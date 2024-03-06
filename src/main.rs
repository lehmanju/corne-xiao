#![no_main]
#![no_std]

use keyberon::matrix::Matrix;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use xiao_m0::hal::clock::GenericClockController;
use xiao_m0::hal::prelude::nb::block;
use xiao_m0::hal::sercom::v2::uart::{
    self, BaudMode, Config, Flags, Oversampling, Pads, Rx, Tx, Uart,
};
use xiao_m0::hal::time::Hertz;
use xiao_m0::hal::typelevel::NoneT;
use xiao_m0::hal::usb::UsbBus;
use xiao_m0::hal::{self as hal, timer};
use xiao_m0::{usb_allocator, Pins};

use hal::prelude::*;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::{Event, Layout};
use panic_halt as _;
use rtic::app;
use usb_device::class::UsbClass;
use xiao_m0::hal::gpio::v2::{dynpin, Alternate, DynPin, Pin, D, PB08};
use xiao_m0::pac::{PM, SERCOM4, TC3};

type KeybUsbClass = keyberon::Class<'static, UsbBus, ()>;
type KeybUsbDevice = usb_device::device::UsbDevice<'static, UsbBus>;
type UartRx = Uart<Config<Pads<SERCOM4, Pin<PB08, Alternate<D>>>>, Rx>;
type UartTx = Uart<Config<Pads<SERCOM4, NoneT, Pin<PB08, Alternate<D>>>>, Tx>;
use panic_halt as _;

mod layout;

pub enum Serial<R, T> {
    Rx(R),
    Tx(T),
}

pub struct SerialPeripherals {
    frequency: Hertz,
    pm: PM,
}

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, dynpin::Error> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(_) => panic!("DynPin unwrap error"),
        }
    }
}

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
//static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;

#[app(device = crate::hal::pac, peripherals = true, dispatchers = [TC4, TC5])]
mod app {
    use super::*;
    #[shared]
    struct Shared {
        usb_dev: KeybUsbDevice,
        usb_class: KeybUsbClass,
        #[lock_free]
        layout: Layout<12, 4, 4, ()>,
        serial: Option<Serial<UartRx, UartTx>>,
    }

    #[local]
    struct Local {
        matrix: Matrix<DynPin, DynPin, 6, 4>,
        debouncer: Debouncer<[[bool; 6]; 4]>,
        timer: timer::TimerCounter<TC3>,
        buf: [u8; 4],
        serial_peripherals: SerialPeripherals,
    }

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut peripherals = c.device;

        // Initialize USB for keyberon

        let mut clocks = GenericClockController::with_internal_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );
        let pins = Pins::new(peripherals.PORT);
        let bus_allocator = unsafe {
            USB_ALLOCATOR = Some(usb_allocator(
                peripherals.USB,
                &mut clocks,
                &mut peripherals.PM,
                pins.usb_dm,
                pins.usb_dp,
            ));
            USB_ALLOCATOR.as_ref().unwrap()
        };

        let usb_class = keyberon::new_class(bus_allocator, ());
        let usb_dev = keyberon::new_device(bus_allocator);

        // Configure timer

        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tcc2_tc3(&gclk0).unwrap();
        let mut timer =
            timer::TimerCounter::tc3_(&timer_clock, peripherals.TC3, &mut peripherals.PM);

        timer.start(1.khz());
        timer.enable_interrupt();

        // Setup Serial communication
        // default configuration is for sending

        let clock = &clocks.sercom4_core(&gclk0).unwrap();
        let uart_pin = pins.a6;
        let serial = {
            let pads = uart::Pads::default().tx(uart_pin);
            let uart = uart::Config::new(&peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
                .baud(9600.hz(), BaudMode::Fractional(Oversampling::Bits16))
                .enable();
            Some(Serial::Tx(uart))
        };

        let serial_peripherals = SerialPeripherals {
            frequency: clock.freq(),
            pm: peripherals.PM,
        };

        // Setup keyboard matrix
        let matrix = match Matrix::new(
            [
                pins.a0.into_pull_up_input().into(),
                pins.a1.into_pull_up_input().into(),
                pins.a3.into_pull_up_input().into(),
                pins.a2.into_pull_up_input().into(),
                pins.a5.into_pull_up_input().into(),
                pins.a4.into_pull_up_input().into(),
            ],
            [
                pins.a7.into_push_pull_output().into(),
                pins.a8.into_push_pull_output().into(),
                pins.a9.into_push_pull_output().into(),
                pins.a10.into_push_pull_output().into(),
            ],
        ) {
            Ok(val) => val,
            Err(_) => panic!("Error creating matrix"),
        };

        (
            Shared {
                usb_dev,
                usb_class,
                layout: Layout::new(&crate::layout::LAYERS),
                serial,
            },
            Local {
                timer,
                debouncer: Debouncer::new([[false; 6]; 4], [[false; 6]; 4], 5),
                matrix,
                buf: [0; 4],
                serial_peripherals,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = SERCOM4, priority = 4, shared = [serial], local= [buf])]
    fn rx(mut c: rx::Context) {
        // receive events from other half
        // spawn event handler
        c.shared.serial.lock(|ser_rx| {
            if let Serial::Rx(rx) = ser_rx.as_mut().unwrap() {
                if let Ok(b) = rx.read() {
                    c.local.buf.rotate_left(1);
                    c.local.buf[3] = b;

                    if c.local.buf[3] == b'\n' {
                        if let Ok(event) = de(&c.local.buf[..]) {
                            handle_event::spawn(event).unwrap();
                        }
                    }
                }
            }
        });
    }

    #[task(binds = USB, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        (c.shared.usb_dev, c.shared.usb_class).lock(|usb_dev, usb_class| {
            if usb_dev.poll(&mut [usb_class]) {
                usb_class.poll();
            }
        });
    }

    #[task(priority = 2, capacity = 8, shared = [layout])]
    fn handle_event(c: handle_event::Context, event: Event) {
        c.shared.layout.event(event);
    }

    #[task(priority = 2, shared = [usb_dev, usb_class, layout])]
    fn tick_keyberon(mut c: tick_keyberon::Context) {
        let _tick = c.shared.layout.tick();
        // if right-hand side do nothing, events have already been sent
        if c.shared.usb_dev.lock(|d| d.state()) == UsbDeviceState::Default {
            return;
        }

        /*
        // else check for custom reset event
        if let CustomEvent::Release(()) = tick {
            unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) }
        }*/

        // generate and send keyboard report
        let report: KbHidReport = c.shared.layout.keycodes().collect();
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TC3,
        priority = 1,
        shared = [serial,  usb_dev],
        local = [debouncer, timer, serial_peripherals, matrix]
    )]
    fn tick(mut c: tick::Context) {
        c.local.timer.wait().ok();

        let mut receiving = false;

        // determine send/receive half
        c.shared.usb_dev.lock(|dev| {
            if dev.state() == UsbDeviceState::Configured {
                // receiving on uart
                receiving = true;
            }
        });

        c.shared.serial.lock(|serial| {
            let value = serial.take().unwrap();
            let result = match value {
                Serial::Rx(rx) => {
                    if !receiving {
                        let (sercom, pads) = rx.disable().free();
                        let uart_pin = pads.free().0;
                        let pads = uart::Pads::default().tx(uart_pin);
                        let uart = uart::Config::new(
                            &c.local.serial_peripherals.pm,
                            sercom,
                            pads,
                            c.local.serial_peripherals.frequency,
                        )
                        .baud(9600.hz(), BaudMode::Fractional(Oversampling::Bits16))
                        .enable();

                        Serial::Tx(uart)
                    } else {
                        Serial::Rx(rx)
                    }
                }
                Serial::Tx(tx) => {
                    if receiving {
                        let (sercom, pads) = tx.disable().free();
                        let uart_pin = pads.free().1;
                        let pads = uart::Pads::default().rx(uart_pin);
                        let mut uart = uart::Config::new(
                            &c.local.serial_peripherals.pm,
                            sercom,
                            pads,
                            c.local.serial_peripherals.frequency,
                        )
                        .baud(9600.hz(), BaudMode::Fractional(Oversampling::Bits16))
                        .enable();
                        uart.enable_interrupts(Flags::RXC);
                        Serial::Rx(uart)
                    } else {
                        Serial::Tx(tx)
                    }
                }
            };
            serial.replace(result);
        });

        // check all events since last tick
        for event in c
            .local
            .debouncer
            .events(c.local.matrix.get().get())
            .map(|e| {
                if cfg!(feature = "right") {
                    e.transform(|i, j| (i, 11 - j))
                } else {
                    e
                }
            })
        {
            // send events to other keyboard half if right side
            c.shared.serial.lock(|serial| {
                if let Serial::Tx(tx) = serial.as_mut().unwrap() {
                    for &b in &ser(event) {
                        let res = block!(tx.write(b));
                        if res.is_err() {
                            panic!("Error during serial write");
                        }
                    }
                }
            });

            // schedule handle_event
            {
                handle_event::spawn(event).unwrap();
            }
        }
        // schedule keyberon tick
        tick_keyberon::spawn().unwrap();
    }
}

fn de(bytes: &[u8]) -> Result<Event, ()> {
    match *bytes {
        [b'P', i, j, b'\n'] => Ok(Event::Press(i, j)),
        [b'R', i, j, b'\n'] => Ok(Event::Release(i, j)),
        _ => Err(()),
    }
}
fn ser(e: Event) -> [u8; 4] {
    match e {
        Event::Press(i, j) => [b'P', i, j, b'\n'],
        Event::Release(i, j) => [b'R', i, j, b'\n'],
    }
}
