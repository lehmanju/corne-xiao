#![no_main]
#![no_std]

use keyberon::matrix::Matrix;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use xiao_m0::hal::clock::GenericClockController;
use xiao_m0::hal::prelude::nb::block;
use xiao_m0::hal::sercom::v2::uart::{self, BaudMode, Config, Oversampling, Pads, Rx, Tx, Uart};
use xiao_m0::hal::typelevel::NoneT;
use xiao_m0::hal::usb::UsbBus;
use xiao_m0::hal::{self as hal, timer};
use xiao_m0::{usb_allocator, Pins};

use hal::prelude::*;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::{CustomEvent, Event, Layout};
use keyberon::matrix::PressedKeys;
use panic_halt as _;
use rtic::app;
use usb_device::class::UsbClass;
use xiao_m0::hal::gpio::v2::{dynpin, Alternate, DynPin, Pin, D, PB08};
use xiao_m0::pac::{SERCOM4, TC3};

type KeybUsbClass = keyberon::Class<'static, UsbBus, ()>;
type KeybUsbDevice = usb_device::device::UsbDevice<'static, UsbBus>;
type UartRx = Uart<Config<Pads<SERCOM4, Pin<PB08, Alternate<D>>>>, Rx>;
type UartTx = Uart<Config<Pads<SERCOM4, NoneT, Pin<PB08, Alternate<D>>>>, Tx>;
use panic_halt as _;

mod layout;

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

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: KeybUsbDevice,
        usb_class: KeybUsbClass,
        matrix: Matrix<DynPin, DynPin, 6, 4>,
        debouncer: Debouncer<PressedKeys<6, 4>>,
        timer: timer::TimerCounter<TC3>,
        layout: Layout<()>,
        #[cfg(feature = "host")]
        serial_receiver: UartRx,
        #[cfg(not(feature = "host"))]
        serial_sender: UartTx,
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {
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

        // Left / Right hand side
        // depends on whether USB communication is established or not

        // Setup Serial communication

        let clock = &clocks.sercom4_core(&gclk0).unwrap();
        let uart_pin = pins.a6;

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

        #[cfg(feature = "host")]
        {
            let serial_receiver = {
                let pads = uart::Pads::default().rx(uart_pin);
                uart::Config::new(&peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
                    .baud(9600.hz(), BaudMode::Fractional(Oversampling::Bits16))
                    .enable()
            };
            init::LateResources {
                usb_dev,
                usb_class,
                timer,
                debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
                matrix,
                layout: Layout::new(crate::layout::LAYERS),
                serial_receiver,
            }
        }
        #[cfg(not(feature = "host"))]
        {
            let serial_sender = {
                let pads = uart::Pads::default().tx(uart_pin);
                uart::Config::new(&peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
                    .baud(9600.hz(), BaudMode::Fractional(Oversampling::Bits16))
                    .enable()
            };
            init::LateResources {
                usb_dev,
                usb_class,
                timer,
                debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
                matrix,
                layout: Layout::new(crate::layout::LAYERS),
                serial_sender,
            }
        }
    }

    #[task(binds = SERCOM4, priority = 5, spawn = [handle_event], resources = [serial_receiver])]
    fn rx(c: rx::Context) {
        static mut BUF: [u8; 4] = [0; 4];

        #[cfg(feature = "host")]
        {
            // receive events from other half
            // spawn event handler
            let serial_rx = c.resources.serial_receiver;
            if let Ok(b) = serial_rx.read() {
                BUF.rotate_left(1);
                BUF[3] = b;

                if BUF[3] == b'\n' {
                    if let Ok(event) = de(&BUF[..]) {
                        c.spawn.handle_event(event).unwrap();
                    }
                }
            }
        }
    }

    #[task(binds = USB, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        if c.resources.usb_dev.poll(&mut [c.resources.usb_class]) {
            c.resources.usb_class.poll();
        }
    }

    #[task(priority = 3, capacity = 8, resources = [layout])]
    fn handle_event(c: handle_event::Context, event: Event) {
        c.resources.layout.event(event);
    }

    #[task(priority = 3, resources = [usb_dev, usb_class, layout])]
    fn tick_keyberon(mut c: tick_keyberon::Context) {
        let tick = c.resources.layout.tick();
        // if right-hand side do nothing, events have already been sent
        if c.resources.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }

        // else check for custom reset event
        if let CustomEvent::Release(()) = tick {
            unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) }
        }

        // generate and send keyboard report
        let report: KbHidReport = c.resources.layout.keycodes().collect();
        if !c
            .resources
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        while let Ok(0) = c.resources.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TC3,
        priority = 2,
        spawn = [handle_event, tick_keyberon],
        resources = [serial_sender, debouncer, timer, matrix],
    )]
    fn tick(mut c: tick::Context) {
        c.resources.timer.wait().ok();
        // check all events since last tick
        for event in c.resources.debouncer.events(c.resources.matrix.get().get()) {
            #[cfg(not(feature = "host"))]
            {
                for &b in &ser(event.transform(|i, j| (i, 11 - j))) {
                    let res = block!(c.resources.serial_sender.write(b));
                    if res.is_err() {
                        panic!("Error during serial write");
                    }
                }
            }

            // schedule handle_event
            {
                c.spawn.handle_event(event).unwrap();
            }
        }
        // schedule keyberon tick
        c.spawn.tick_keyberon().unwrap();
    }

    extern "C" {
        fn TC4();
        fn TC5();
    }
};

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
