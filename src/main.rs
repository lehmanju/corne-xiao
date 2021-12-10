#![no_main]
#![no_std]

use keyberon::matrix::Matrix;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use xiao_m0::hal::sercom::v2::uart;
use xiao_m0::hal::usb::UsbBus;
use xiao_m0::hal::{self as hal, timer};

use hal::gpio::v2::{Floating, Input};
use hal::prelude::*;
use rtic::app;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::{Layout, Event};
use keyberon::matrix::{PressedKeys};
use panic_halt as _;
use xiao_m0::hal::gpio::v2::DynPin;
use xiao_m0::pac::{TC3, usb};

type UsbClass = keyberon::Class<'static, UsbBus, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, UsbBus>;

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
        tx: uart::Tx,
        rx: uart::Rx,
    }

    #[init]
    fn init(mut c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut rcc = c
            .device
            .RCC
            .configure()
            .hsi48()
            .enable_crs(c.device.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut c.device.FLASH);

        let gpioa = c.device.GPIOA.split(&mut rcc);
        let gpiob = c.device.GPIOB.split(&mut rcc);

        let usb = usb::Peripheral {
            usb: c.device.USB,
            pin_dm: gpioa.pa11,
            pin_dp: gpioa.pa12,
        };
        *USB_BUS = Some(usb::UsbBusType::new(usb));
        let usb_bus = USB_BUS.as_ref().unwrap();

        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timers::Timer::tim3(c.device.TIM3, 1.khz(), &mut rcc);
        timer.listen(timers::Event::TimeOut);

        let pb12: &gpiob::PB12<Input<Floating>> = &gpiob.pb12;
        let is_left = pb12.is_low().get();
        let transform: fn(Event) -> Event = if is_left {
            |e| e
        } else {
            |e| e.transform(|i, j| (i, 11 - j))
        };

        let (pa9, pa10) = (gpioa.pa9, gpioa.pa10);
        let pins = cortex_m::interrupt::free(move |cs| {
            (pa9.into_alternate_af1(cs), pa10.into_alternate_af1(cs))
        });
        let mut serial = serial::Serial::usart1(c.device.USART1, pins, 38_400.bps(), &mut rcc);
        serial.listen(serial::Event::Rxne);
        let (tx, rx) = serial.split();

        let pa15 = gpioa.pa15;
        let matrix = cortex_m::interrupt::free(move |cs| {
            Matrix::new(
                [
                    pa15.into_pull_up_input(cs).downgrade(),
                    gpiob.pb3.into_pull_up_input(cs).downgrade(),
                    gpiob.pb4.into_pull_up_input(cs).downgrade(),
                    gpiob.pb5.into_pull_up_input(cs).downgrade(),
                    gpiob.pb8.into_pull_up_input(cs).downgrade(),
                    gpiob.pb9.into_pull_up_input(cs).downgrade(),
                ],
                [
                    gpiob.pb0.into_push_pull_output(cs).downgrade(),
                    gpiob.pb1.into_push_pull_output(cs).downgrade(),
                    gpiob.pb2.into_push_pull_output(cs).downgrade(),
                    gpiob.pb10.into_push_pull_output(cs).downgrade(),
                ],
            )
        });

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            debouncer: Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5),
            matrix: matrix.get(),
            layout: Layout::new(crate::layout::LAYERS),
            transform,
            tx,
            rx,
        }
    }

    #[task(binds = USART1, priority = 5, spawn = [handle_event], resources = [rx])]
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
        binds = TIM3,
        priority = 2,
        spawn = [handle_event, tick_keyberon],
        resources = [matrix, debouncer, timer, &transform, tx],
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