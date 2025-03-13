//! examples/led

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull}, 

    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use embedded_hal::digital::v2::OutputPin;

    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        i2s: hal::i2s::I2S,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // Check if UICR is set correctly
        let check_uicr_set = cx.device.UICR.nfcpins.read().protect().is_disabled();

        // Set NFC pins to normal GPIO
        if !check_uicr_set {
            cx.device.NVMC.config.write(|w| w.wen().wen());
            while cx.device.NVMC.ready.read().ready().is_busy() {}
            
            cx.device.UICR.nfcpins.write(|w| w.protect().disabled());
            while cx.device.NVMC.ready.read().ready().is_busy() {}

            cx.device.NVMC.config.write(|w| w.wen().ren());
            while cx.device.NVMC.ready.read().ready().is_busy() {}

            // Changes to UICR require a reset to take effect
            cortex_m::peripheral::SCB::sys_reset();
        }

        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let mut sd_mode: Pin<Output<PushPull>> = port0.p0_10.into_push_pull_output(Level::Low).degrade();
        sd_mode.set_high().unwrap();
        let bclk: Pin<Output<PushPull>> = port0.p0_04.into_push_pull_output(Level::Low).degrade();
        let lrclk: Pin<Output<PushPull>> = port0.p0_05.into_push_pull_output(Level::Low).degrade();
        let din: Pin<Output<PushPull>> = port0.p0_31.into_push_pull_output(Level::Low).degrade();

        let pins = hal::i2s::Pins::Controller {
             mck: None, 
             sck: bclk, 
             lrck: lrclk, 
             sdin: None, 
             sdout: Some(din),
        };

        let i2s = hal::i2s::I2S::new(
            cx.device.I2S, pins
        );

        i2s.enable();

        (Shared {}, Local {i2s}, init::Monotonics(mono))
    }

    #[idle(local=[i2s])]
    fn idle(_cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

}
