//! examples/backup_battery_mode.rs

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    core::fmt::Write,
    nrf52833_hal as hal, 
    rtt_target::{rprintln, rtt_init, UpChannel, ChannelMode},
    panic_rtt_target as _, 
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        rtt_power: UpChannel,
        comp: hal::comp::Comp,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut channels = rtt_init!(
            up: {
                0: {
                    size: 128,
                    mode: ChannelMode::BlockIfFull,
                    name:"Power",
                }
            }
        );
        writeln!(channels.up.0, "init").ok();


        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // Init comparator
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let comp_pin = port0.p0_02.into_floating_input();
        let comp = hal::comp::Comp::new(cx.device.COMP, &comp_pin);
        comp.enable_interrupt(hal::comp::Transition::Up);
        comp.enable_interrupt(hal::comp::Transition::Down);
        comp.enable();
        
        (Shared {}, Local {rtt_power: channels.up.0, comp: comp}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(binds = COMP_LPCOMP, local = [rtt_power, comp])]
    fn comp_lcomp(cx: comp_lcomp::Context) {
        cx.local.rtt_power.write_str("comp_lcomp\n").ok();
        let comp = cx.local.comp;
        
        if comp.event_up().read().bits() != 0 {
            writeln!(cx.local.rtt_power, "Upward crossing\n").ok();
        }

        if comp.event_down().read().bits() != 0 {
            writeln!(cx.local.rtt_power, "Downward crossing\n").ok();
        }
    }
}