//! examples/rtic_blinky

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::gpio::{Level, Output, Pin, PushPull, }, 
    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    systick_monotonic::*
};

type LED = Pin<Output<PushPull>>;

const TIMER_HZ: u32 = 4; // 4 Hz (250 ms granularity)
const TIME_0: fugit::TimerInstantU64<TIMER_HZ> = fugit::TimerInstantU64::from_ticks(0);

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
        led: LED,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("init");

        // Initialize the monotonic (core clock at 64 MHz)
        let mut mono = Systick::new(cx.core.SYST, 64_000_000);
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);

        // Enable writing to UICR registers
        cx.device.NVMC.config.write(|w| w.wen().wen());

        // Disable NFC pins
        let uicr = cx.device.UICR;
        uicr.nfcpins.write(|w| w.protect().disabled());

        // Disable writing to UICR registers
        cx.device.NVMC.config.write(|w| w.wen().ren());

        // LED
        let led: Pin<Output<PushPull>> = port0.p0_09.into_push_pull_output(Level::High).degrade();

        // Initiate periodic process
        let next_instant = mono.now() + 1.secs();
        blink::spawn_at(next_instant, next_instant).unwrap();

        (Shared {}, Local { led}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            // Puts the device into sleep.
            // However Systick requires the core clock of the MCU to be active
            // Thus we will get about 1.5mA
            asm::wfi();
            rprintln!("wake");
        }
    }

    // Drift free periodic task
    #[task(local = [cnt: u32 = 0, led])]
    fn blink(cx: blink::Context, instant: fugit::TimerInstantU64<TIMER_HZ>) {
        let duration_since_start: fugit::MillisDurationU64 = (instant - TIME_0).convert();
        rprintln!(
            "foo #{:?}, instant {:?}, duration since start {}",
            cx.local.cnt,
            instant,
            duration_since_start
        );
        
        if *cx.local.cnt % 2 == 0 {
             cx.local.led.set_high().ok();

         } else {
             cx.local.led.set_low().ok();

         }

        *cx.local.cnt += 1;

        // Spawn a new message with 1 s offset to spawned time
        let next_instant = instant + 1.secs();
        blink::spawn_at(next_instant, next_instant).unwrap();
    }
}
