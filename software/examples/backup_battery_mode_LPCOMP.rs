//! examples/backup_battery_mode_LPCOMP.rs

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
    use embedded_hal::digital::v2::StatefulOutputPin;

    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {
    }

    #[local]
    struct Local {
        rtt_power: UpChannel,
        rtt_trace: UpChannel,
        comp: hal::lpcomp::LpComp,
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

        let comp = hal::lpcomp::LpComp::new(cx.device.LPCOMP, &comp_pin);
        comp.vref(hal::lpcomp::VRef::_4_8Vdd);
        comp.enable_interrupt(hal::lpcomp::Transition::Cross);
        comp.enable();

        let mut output: Pin<Output<PushPull>> = port0.p0_15.into_push_pull_output(Level::Low).degrade();
        output.set_high().ok();
        
        (Shared {}, Local {rtt_power: channels.up.0, rtt_trace: channels.up.1, output, comp}, init::Monotonics(mono))
    }

    #[idle(local = [rtt_trace])]
    fn idle(cx: idle::Context) -> ! {
        writeln!(cx.local.rtt_trace, "Entering idle!").ok();
        loop {
            asm::wfi();
        }
    }

    #[task(binds = COMP_LPCOMP, local = [rtt_power, output, comp], priority=1)]
    fn comp_lcomp(cx: comp_lcomp::Context) {
        let comp = cx.local.comp;
        let comp_read = comp.read();
        
        if cx.local.output.is_set_high().ok().unwrap() {
            cx.local.output.set_low().ok();
        } else {
            cx.local.output.set_high().ok();
        }

        match comp_read {
            hal::lpcomp::CompResult::Above => {writeln!(cx.local.rtt_power, "Above").ok();}
            hal::lpcomp::CompResult::Below => {writeln!(cx.local.rtt_power, "Below").ok();}
        }
        

        if comp.is_up() {
            writeln!(cx.local.rtt_power, "Upward crossing").ok();
        }

        if comp.is_down() {
            writeln!(cx.local.rtt_power, "Downward crossing").ok();
        }

        if comp.is_cross() {
            writeln!(cx.local.rtt_power, "Crossing").ok();
        }

        comp.reset_events();
    }
}