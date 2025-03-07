//! examples/serial_over_usb

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm,
    hal::clocks::*,
    hal::usbd::{UsbPeripheral, Usbd},
    nrf52833_hal as hal,
    rtt_target::{rprintln, rtt_init_print},
    panic_rtt_target as _,
    systick_monotonic::*,
    panic_rtt_target as _,
    usb_device::{
        class_prelude::UsbBusAllocator,
        device::{UsbDevice, StringDescriptors, UsbDeviceBuilder, UsbVidPid},
    },
    usbd_serial::{SerialPort, USB_CLASS_CDC},

};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)
const DATA_IN_BUFFER_SIZE: usize = 64;
const DATA_OUT_BUFFER_SIZE: usize = 64;

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {
        #[lock_free]
        usb_dev: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
        #[lock_free]
        serial: SerialPort<'static, Usbd<UsbPeripheral<'static>>>,
    }

    #[local]
    struct Local {
    }

    #[init(local = [
        clocks: Option<Clocks<ExternalOscillator, Internal, LfOscStopped>> = None,
        usb_bus: Option<UsbBusAllocator<Usbd<UsbPeripheral<'static>>>> = None, 
    ])]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("\n--- init ---");

        //Enable USBD interrupt
        cx.device.USBD.intenset.write(|w| w.sof().set());

        let mono = Systick::new(cx.core.SYST, 64_000_000);
        let device = cx.device;
        let clocks = Clocks::new(device.CLOCK);

        // make static lifetime for clocks
        cx.local.clocks.replace(clocks.enable_ext_hfosc());

        let usb_bus = UsbBusAllocator::new(Usbd::new(UsbPeripheral::new(
            device.USBD,
            // refer to static lifetime
            cx.local.clocks.as_ref().unwrap(),
        )));
        cx.local.usb_bus.replace(usb_bus);

        let serial = SerialPort::new(&cx.local.usb_bus.as_ref().unwrap());

        let usb_dev = UsbDeviceBuilder::new(
            &cx.local.usb_bus.as_ref().unwrap(),
            UsbVidPid(0x16c0, 0x27dd),
        )
        .strings(&[StringDescriptors::default()
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")])
        .unwrap()
        .device_class(USB_CLASS_CDC)
        .max_packet_size_0(64) // (makes control transfers 8x faster)
        .unwrap()
        .build();


        (Shared {usb_dev, serial}, Local {}, init::Monotonics(mono))
    }


    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    // Should NOT be RTIC task
    fn do_stuff_with_data(data: &[u8]) {
        rprintln!("do_stuff_with_data: {:?}", core::str::from_utf8(data));
        write_to_serial(data);
    }

    // Should NOT be RTIC task
    // Just makes it easier to create an array of the correct size
    fn write_to_serial(data: &[u8]) {
        rprintln!("write_to_serial: {:?}", core::str::from_utf8(data));
        let mut len: usize = data.len();
        let size_with_newline = DATA_OUT_BUFFER_SIZE -1;
        if len > size_with_newline {
            rprintln!("Data too large, truncating");
            len = size_with_newline;
        }
        let mut data_out = [0u8; DATA_OUT_BUFFER_SIZE];
        data_out[0..len].copy_from_slice(data);
        data_out[len] = 13;
        data_out::spawn(data_out, len+1).unwrap();
    }

    #[task(shared = [usb_dev, serial])]
    fn data_out(cx: data_out::Context, data: [u8; DATA_OUT_BUFFER_SIZE], len: usize) {
        let serial = cx.shared.serial;
        let usb_dev = cx.shared.usb_dev;

        match serial.write(&data[0..len]) {
            Ok(_) => {
                usb_dev.poll(&mut [serial]);
            }
            Err(_) => {
                rprintln!("Error writing data");
            }
        }
    }

    #[task(capacity = 10, local = [len: usize = 0, data_arr :[u8; DATA_IN_BUFFER_SIZE] = [0; DATA_IN_BUFFER_SIZE]])]
    fn data_in(cx: data_in::Context, data: u8) {
        let len = cx.local.len;
        let data_arr = cx.local.data_arr;

        match data {
            13 => {
                let slice = &data_arr[0..*len];
                rprintln!("Received: {:?}", core::str::from_utf8(slice));
                do_stuff_with_data(slice);
                *len = 0;
            }
            _ => {
                data_arr[*len] = data;
                if *len < data_arr.len() - 1 {
                    *len += 1;
                } else {
                    rprintln!("Buffer full, discarding data");
                    *len = 0;
                }
            }
        }
    }

    #[task(binds=USBD, shared = [usb_dev, serial])]
    fn usb_fs(cx: usb_fs::Context) {
        let usb_dev = cx.shared.usb_dev;
        let serial = cx.shared.serial;
        
        let mut buf = [0u8; 64];
        usb_dev.poll(&mut [serial]);

        match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                for i in 0..count {
                    data_in::spawn(buf[i]).unwrap();
                }
            }
            _ => {}
        }
    }

}
