//! examples/backup_battery_mode.rs

#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

use {
    cortex_m::asm, 
    core::fmt::Write,
    nrf52833_hal as hal, 
    rtt_target::{rtt_init, UpChannel, ChannelMode},
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
    struct Shared {
        counter: u32,
    }

    #[local]
    struct Local {
        rtt_power: UpChannel,
        rtt_trace: UpChannel,
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
                1 : {
                    size: 128,
                    mode: ChannelMode::BlockIfFull,
                    name: "Trace",
                }
            }
        );
        writeln!(channels.up.0, "Power RTT channel initialized!").ok();
        writeln!(channels.up.1, "Trace RTT channel initialized!").ok();


        // Initialize the monotonic (core clock at 64 MHz)
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        // Init comparator
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let comp_pin = port0.p0_02.into_floating_input();
        let comp = hal::comp::Comp::new(cx.device.COMP, &comp_pin);
        comp.vref(hal::comp::VRef::Vdd);
        comp.power_mode(hal::comp::PowerMode::LowPower);
        //comp.enable_interrupt(hal::comp::Transition::Up);
        //comp.enable_interrupt(hal::comp::Transition::Down);
        comp.enable_interrupt(hal::comp::Transition::Cross);
        comp.enable();
        
        (Shared {counter: 0}, Local {rtt_power: channels.up.0, rtt_trace: channels.up.1, comp: comp}, init::Monotonics(mono))
    }

    #[idle(local = [rtt_trace], shared = [counter])]
    fn idle(mut cx: idle::Context) -> ! {
        writeln!(cx.local.rtt_trace, "Entering idle!").ok();
        loop {
            let counter = cx.shared.counter.lock(|c| *c);
            if counter > 0 {
                writeln!(cx.local.rtt_trace, "Counter: {}", counter).ok();
            }
            asm::wfi();
        }
    }

    #[task(binds = COMP_LPCOMP, local = [rtt_power, comp], shared = [counter])]
    fn comp_lcomp(mut cx: comp_lcomp::Context) {
        writeln!(cx.local.rtt_power, "Comparator interrupt!").ok();
        cx.shared.counter.lock(|c| *c += 1);

        let comp = cx.local.comp;
        
        if comp.event_up().read().bits() != 0 {
            writeln!(cx.local.rtt_power, "Upward crossing").ok();
        }

        if comp.event_down().read().bits() != 0 {
            writeln!(cx.local.rtt_power, "Downward crossing").ok();
        }

        if comp.event_cross().read().bits() != 0 {
            writeln!(cx.local.rtt_power, "Crossing").ok();
        }
    }
}