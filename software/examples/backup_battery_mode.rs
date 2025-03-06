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
    systick_monotonic::*,
    hal::gpio::{Level, Output, Pin, PushPull}, 
    embedded_hal::digital::v2::OutputPin,
};

const TIMER_HZ: u32 = 1000; // 1000 Hz (1 ms granularity)

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0])]
mod app {
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {
        comp: hal::comp::Comp,
    }

    #[local]
    struct Local {
        rtt_power: UpChannel,
        rtt_trace: UpChannel,
        //comp: hal::lpcomp::LpComp,
        output: Pin<Output<PushPull>>,
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
        comp.vref(hal::comp::VRef::Int1V2);
        comp.power_mode(hal::comp::PowerMode::LowPower);
        //comp.enable_interrupt(hal::comp::Transition::Up);
        //comp.enable_interrupt(hal::comp::Transition::Down);
        comp.enable_interrupt(hal::comp::Transition::Cross);
        comp.hysteresis(false);
        comp.enable();

        //let comp = hal::lpcomp::LpComp::new(cx.device.LPCOMP, &comp_pin);
        //comp.vref(hal::lpcomp::VRef::_4_8Vdd);
        //comp.enable_interrupt(lpcomp::Transition::Cross);
        //comp.enable();

        let mut output: Pin<Output<PushPull>> = port0.p0_15.into_push_pull_output(Level::Low).degrade();
        output.set_high().ok();
        
        (Shared {comp: comp}, Local {rtt_power: channels.up.0, rtt_trace: channels.up.1, output}, init::Monotonics(mono))
    }

    #[idle(local = [rtt_trace], shared = [comp])]
    fn idle(mut cx: idle::Context) -> ! {
        writeln!(cx.local.rtt_trace, "Entering idle!").ok();
        loop {
            let comp_read = cx.shared.comp.lock(|c| c.read());
            writeln!(cx.local.rtt_trace, "Comp {:?}", comp_read).ok();
            asm::wfi();
        }
    }

    #[task(binds = COMP_LPCOMP, local = [rtt_power, output], shared = [comp], priority=5)]
    fn comp_lcomp(mut cx: comp_lcomp::Context) {
        let comp_read = cx.shared.comp.lock(|c| c.read());

        match comp_read {
            hal::comp::CompResult::Above => {writeln!(cx.local.rtt_power, "Above").ok();}
            hal::comp::CompResult::Below => {writeln!(cx.local.rtt_power, "Below").ok();}
        }
        
        let (is_up, is_down, is_cross) = cx.shared.comp.lock(|c| {
            (c.is_up(), c.is_down(), c.is_cross())
        });

        if is_up {
            writeln!(cx.local.rtt_power, "Upward crossing").ok();
        }

        if is_down {
            writeln!(cx.local.rtt_power, "Downward crossing").ok();
        }

        if is_cross {
            writeln!(cx.local.rtt_power, "Crossing").ok();
        }

        cx.shared.comp.lock(|c| c.reset_events());
    }
}