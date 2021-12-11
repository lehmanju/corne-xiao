#![no_main]
#![no_std]

use keyberon::matrix::Matrix;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use xiao_m0::hal::clock::GenericClockController;
use xiao_m0::hal::sercom::v2::uart::{self, BaudMode, Config, Oversampling, Pads, Rx, Tx, Uart};
use xiao_m0::hal::typelevel::NoneT;
use xiao_m0::hal::usb::UsbBus;
use xiao_m0::hal::{self as hal, timer};
use xiao_m0::{usb_allocator, Pins};

use hal::prelude::*;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::{Event, Layout};
use keyberon::matrix::PressedKeys;
use panic_halt as _;
use rtic::app;
use xiao_m0::hal::gpio::v2::{Alternate, DynPin, Pin, D, PB08};
use xiao_m0::pac::{usb, SERCOM4, TC3};

type UsbClass = keyberon::Class<'static, UsbBus, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, UsbBus>;
type UartRx = Uart<Config<Pads<SERCOM4, Pin<PB08, Alternate<D>>>>, Rx>;
type UartTx = Uart<Config<Pads<SERCOM4, NoneT, Pin<PB08, Alternate<D>>>>, Tx>;

use panic_halt as _;

mod layout;

pub enum Serial<R, T> {
    Rx(R),
    Tx(T),
}

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, Infallible> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => match e {},
        }
    }
}

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        matrix: Matrix<DynPin, DynPin, 6, 4>,
        debouncer: Debouncer<PressedKeys<6, 4>>,
        timer: timer::TimerCounter<TC3>,
        layout: Layout<()>,
        transform: fn(Event) -> Event,
        serial: Serial<UartRx, UartTx>,
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

        let peripherals = c.device;
        let core = c.core;

        // Initialize USB for keyberon

        let mut clocks = GenericClockController::with_external_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );
        let pins = Pins::new(peripherals.PORT);
        let bus_allocator = usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        );

        let usb_class = keyberon::new_class(&bus_allocator, ());
        let usb_dev = keyberon::new_device(&bus_allocator);

        // Configure timer

        let gclk0 = clocks.gclk0();
        let timer_clock = clocks.tcc2_tc3(&gclk0).unwrap();
        let mut timer =
            timer::TimerCounter::tc3_(&timer_clock, peripherals.TC3, &mut peripherals.PM);
        timer.start(1.khz());

        // Left / Right hand side
        // depends on whether USB communication is established or not

        let is_left = usb_dev.state() == UsbDeviceState::Configured;
        let transform: fn(Event) -> Event = if is_left {
            |e| e
        } else {
            |e| e.transform(|i, j| (i, 11 - j))
        };

        // Setup Serial communication
        let clock = &clocks.sercom4_core(&gclk0).unwrap();
        let uart_pin = pins.a6;
        let serial = if is_left {
            let pads = uart::Pads::default().rx(uart_pin);
            let uart =
                uart::Config::new(&mut peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
                    .baud(
                        38_400.bps().into(),
                        BaudMode::Fractional(Oversampling::Bits16),
                    )
                    .enable();
            Serial::Rx(uart)
        } else {
            let pads = uart::Pads::default().tx(uart_pin);
            let uart =
                uart::Config::new(&mut peripherals.PM, peripherals.SERCOM4, pads, clock.freq())
                    .baud(
                        38_400.bps().into(),
                        BaudMode::Fractional(Oversampling::Bits16),
                    )
                    .enable();
            Serial::Tx(uart)
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
                pins.a7.into_pull_up_input().into(),
                pins.a8.into_pull_up_input().into(),
                pins.a9.into_pull_up_input().into(),
                pins.a10.into_pull_up_input().into(),
            ],
        ) {
            Ok(val) => val,
            Err(_) => panic!("Error creating matrix"),
        };


        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
            matrix,
            layout: Layout::new(crate::layout::LAYERS),
            transform,
            serial,
        }
    }

    #[task(binds = SERCOM4, priority = 5, spawn = [handle_event], resources = [serial])]
    fn rx(c: rx::Context) {
        static mut BUF: [u8; 4] = [0; 4];

        if let Ok(b) = c.resources.rx.read() {
            BUF.rotate_left(1);
            BUF[3] = b;

            if BUF[3] == b'\n' {
                if let Ok(event) = de(&BUF[..]) {
                    c.spawn.handle_event(event).unwrap();
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
        if c.resources.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        match tick {
            CustomEvent::Release(()) => unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) },
            _ => (),
        }
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

    #[task(
        binds = TC3,
        priority = 2,
        spawn = [handle_event, tick_keyberon],
        resources = [matrix, debouncer, timer, &transform, serial],
    )]
    fn tick(c: tick::Context) {
        c.resources.timer.wait().ok();

        for event in c
            .resources
            .debouncer
            .events(c.resources.matrix.get().get())
            .map(c.resources.transform)
        {
            for &b in &ser(event) {
                block!(c.resources.tx.write(b)).get();
            }
            c.spawn.handle_event(event).unwrap();
        }
        c.spawn.tick_keyberon().unwrap();
    }

    extern "C" {
        fn CEC_CAN();
    }
};
