#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

mod state_machine;
mod rtc;

use {
    cortex_m::asm, 
    hal::rtc::*,

    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    core::sync::atomic::{AtomicU32, Ordering},
    crate::state_machine::*,
};

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TEMP, RNG, ECB])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        rtc: Rtc<hal::pac::RTC0>,
        time_offset_ticks: AtomicU32,   // Time offset in ticks from 00:00
        alarm_offset_ticks: AtomicU32,  // Alarm offset in ticks from 00:00
        }

    #[local]
    struct Local {
        state_machine: State,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);
        let _clocks = clocks.start_lfclk();

        // Initialize the RTC peripheral
        let rtc = rtc::init(cx.device.RTC0);

        // Simulate user setting the time
        set_time::spawn(06, 20).ok();

        // Simulate user setting the alarm, 
        set_alarm::spawn(06, 25).ok();

        let state_machine = State::Idle;

        (Shared {
            rtc,
            time_offset_ticks: AtomicU32::new(0),
            alarm_offset_ticks: AtomicU32::new(0),
        }, Local {
            state_machine
        }, init::Monotonics())
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(priority = 4, capacity = 10, local = [state_machine])]
    fn state_machine(cx: state_machine::Context, event: Event, arg: u32) {
        match cx.local.state_machine {
            State::Idle => match event {
                Event::TimerEvent(TimerEvent::AlarmTriggered) => {
                    *cx.local.state_machine = cx.local.state_machine.next(event);

                }
                Event::TimerEvent(TimerEvent::PeriodicUpdate) => {
                    let alarm_triggered = false;
                    update_display::spawn(arg, alarm_triggered).ok();

                    *cx.local.state_machine = cx.local.state_machine.next(event);
                }
                _ => {
                    todo!()
                }
            }

            State::Alarm => match event {
                Event::TimerEvent(TimerEvent::PeriodicUpdate) => {
                    let alarm_triggered = true;
                    update_display::spawn(arg, alarm_triggered).ok();

                    *cx.local.state_machine = cx.local.state_machine.next(event);
                }
                _ => {
                    todo!()
                }
            }
            _ => {
                todo!()
            }
        }
    }

    #[task(binds = RTC0, priority = 5, shared = [rtc, &time_offset_ticks])]
    fn rtc_interrupt(mut cx: rtc_interrupt::Context) {
        cx.shared.rtc.lock(|rtc| {
            // Need to check which interrupt has been triggered
            // multiple interrupts can be triggered at the same time

            // Compare 0: Periodic interrupt every minute
            if rtc.is_event_triggered(RtcInterrupt::Compare0) {
                let counter = rtc.get_counter();
                rtc::periodic_interrupt(rtc, counter);

                state_machine::spawn(Event::TimerEvent(TimerEvent::PeriodicUpdate), counter).ok();
            } 
            // Compare 1: Alarm interrupt
            if rtc.is_event_triggered(RtcInterrupt::Compare1) {
                rtc::alarm_interrupt(rtc);

                state_machine::spawn(Event::TimerEvent(TimerEvent::AlarmTriggered), 0).ok();
            }
            // Overflow: RTC counter has reached its maximum value
            if rtc.is_event_triggered(RtcInterrupt::Overflow) {
                rtc::overflow_interrupt(rtc, cx.shared.time_offset_ticks);
            };
        });
    }

    #[task(priority = 1, shared = [rtc, &time_offset_ticks])]
    fn set_time(mut cx: set_time::Context, hour: u8, minute: u8) {
        let time_offset_ticks = rtc::time_to_ticks(hour, minute);

        // Reset RTC counter, set the time offset
        cx.shared.rtc.lock(|rtc| {
            rtc.clear_counter();
        });
        cx.shared.time_offset_ticks.store(time_offset_ticks, Ordering::Relaxed);
    }

    #[task(priority = 1, shared = [rtc, &alarm_offset_ticks, &time_offset_ticks])]
    fn set_alarm(mut cx: set_alarm::Context, hour: u8, minute: u8) {
        let alarm_ticks = rtc::time_to_ticks(hour, minute);
        let counter = cx.shared.rtc.lock(|rtc| rtc.get_counter());
        
        let next_interrupt = rtc::next_alarm_ticks(counter, cx.shared.time_offset_ticks.load(Ordering::Relaxed), alarm_ticks);
        cx.shared.rtc.lock(|rtc| {
            rtc::set_alarm_interrupt(rtc, next_interrupt, cx.shared.alarm_offset_ticks);
        });
    }

    #[task(priority = 3, shared = [&time_offset_ticks])]
    fn update_display(cx: update_display::Context, elapsed_ticks: u32, alarm_triggered: bool) {
        // Compute elapsed time in minutes
        let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
        let (hour, minute) = rtc::ticks_to_time(time_offset_ticks + elapsed_ticks);

        // Update OLED display
        rprintln!("Time: {:02}:{:02}", hour, minute);

        if alarm_triggered {
            rprintln!("BEEP BEEP BEEP");
        }
    }
}