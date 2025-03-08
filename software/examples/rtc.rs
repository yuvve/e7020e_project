//! examples/rtc

#![no_main]
#![no_std]
#![deny(unsafe_code)]
//#![deny(warnings)]

use {
    cortex_m::asm, 
    hal::rtc::*,

    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
    core::sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

const RTC_PRESCALER: u32 = 4095; // 8 Hz RTC frequency, max prescaler value
const TICKS_PER_MINUTE: u32 = 8; // Interrupt every second for demonstration purpose, will be 8*60 in production
const TICKS_PER_DAY: u32 = TICKS_PER_MINUTE * 60 * 24;
const MAX_TICKS: u32 = 16_777_216; // 24 bit max value for RTC counter

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TIMER0, TIMER1])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        rtc: Rtc<hal::pac::RTC0>,
        alarm_triggered: AtomicBool,
        time_offset_ticks: AtomicU32,   // Time offset in ticks from 00:00
        alarm_offset_ticks: AtomicU32,  // Alarm offset in ticks from 00:00
        }

    #[local]
    struct Local {
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);
        let _clocks = clocks.start_lfclk();

        // RTC
        let mut rtc = hal::rtc::Rtc::new(cx.device.RTC0, RTC_PRESCALER).unwrap();
        // Start periodic interrupt every minute, to update OLED display
        rtc.set_compare(RtcCompareReg::Compare0, TICKS_PER_MINUTE).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Compare0, None);
        rtc.enable_interrupt(RtcInterrupt::Overflow, None);
        rtc.enable_counter();

        // Simulate user setting the time
        set_time::spawn(06, 20).ok();

        // Simulate user setting the alarm, 
        set_alarm::spawn(06, 25).ok();

        (Shared {
            rtc,
            alarm_triggered: AtomicBool::new(false),
            time_offset_ticks: AtomicU32::new(0),
            alarm_offset_ticks: AtomicU32::new(0),
        }, Local {}, init::Monotonics())
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(binds = RTC0, priority = 3, shared = [rtc, &alarm_triggered, &time_offset_ticks])]
    fn rtc_interrupt(mut cx: rtc_interrupt::Context) {
        cx.shared.rtc.lock(|rtc| {
            // Need to check which interrupt has been triggered
            // multiple interrupts can be triggered at the same time

            // Compare 0: Periodic interrupt every minute
            if rtc.is_event_triggered(RtcInterrupt::Compare0) {
                rtc.reset_event(RtcInterrupt::Compare0);
                
                // Set the next interrupt, next minute
                let counter = rtc.get_counter();
                let next_interrupt = (counter + TICKS_PER_MINUTE) % MAX_TICKS;
                rtc.set_compare(RtcCompareReg::Compare0, next_interrupt).ok();

                // This can be spawned in the state_machine
                update_display::spawn(counter).ok();
            } 
            // Compare 1: Alarm interrupt
            if rtc.is_event_triggered(RtcInterrupt::Compare1) {
                rtc.reset_event(RtcInterrupt::Compare1);
                cx.shared.alarm_triggered.store(true, Ordering::Relaxed);

            }
            // Overflow: RTC counter has reached its maximum value
            if rtc.is_event_triggered(RtcInterrupt::Overflow) {
                rtc.reset_event(RtcInterrupt::Overflow);

                // Update the time offset to current time, adjusting for overflow
                let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
                let new_offset = (MAX_TICKS + time_offset_ticks) % TICKS_PER_DAY;
                cx.shared.time_offset_ticks.store(new_offset, Ordering::Relaxed);
            };
        });
    }

    #[task(shared = [rtc, &time_offset_ticks])]
    fn set_time(mut cx: set_time::Context, hour: u8, minute: u8) {
        let minutes = (hour as u32) * 60 + (minute as u32);
        let time_offset_ticks = minutes * TICKS_PER_MINUTE;
    
        // Reset RTC counter, set the time offset
        cx.shared.rtc.lock(|rtc| {
            rtc.clear_counter();
        });
        cx.shared.time_offset_ticks.store(time_offset_ticks, Ordering::Relaxed);
    }

    #[task(shared = [rtc, &alarm_offset_ticks, &time_offset_ticks])]
    fn set_alarm(mut cx: set_alarm::Context, hour: u8, minute: u8) {
        let alarm_minutes = (hour as u32) * 60 + (minute as u32);
        let alarm_ticks = alarm_minutes * TICKS_PER_MINUTE;
        
        let counter = cx.shared.rtc.lock(|rtc| rtc.get_counter());
        let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
        let current_time = (counter + time_offset_ticks ) % TICKS_PER_DAY;

        let next_alarm_ticks = if current_time > alarm_ticks {
            // Alarm time has already passed today, set it for tomorrow
            TICKS_PER_DAY - current_time + alarm_ticks
        } else {
            alarm_ticks - current_time + counter
        };

        // Modulo MAX_TICKS, adjusting for RTC counter overflow
        let next_interrupt = next_alarm_ticks % MAX_TICKS;
        cx.shared.rtc.lock(|rtc| {
            rtc.set_compare(RtcCompareReg::Compare1, next_interrupt).ok();
            rtc.enable_interrupt(RtcInterrupt::Compare1, None);
        });
        cx.shared.alarm_offset_ticks.store(alarm_ticks, Ordering::Relaxed);
    }

    #[task(shared = [&alarm_triggered, &time_offset_ticks])]
    fn update_display(cx: update_display::Context, elapsed_ticks: u32) {
        // Compute elapsed time in minutes
        let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
        let total_minutes = (elapsed_ticks + time_offset_ticks) / TICKS_PER_MINUTE;
        
        let hour = ((total_minutes / 60) % 24) as u8;
        let minute = (total_minutes % 60) as u8;

        // Update OLED display
        rprintln!("Time: {:02}:{:02}", hour, minute);

        // This will be handled by the state machine, e.g. if in alarm state
        let alarm_triggered = cx.shared.alarm_triggered.load(Ordering::Relaxed);
        if alarm_triggered {
            rprintln!("BEEP BEEP BEEP");
            cx.shared.alarm_triggered.store(false, Ordering::Relaxed);
            // Set the next alarm, 5 minutes from now, for demonstration purpose
            let next_total_minutes = total_minutes + 5;
            let next_hour = ((next_total_minutes / 60) % 24) as u8;
            let next_minute = (next_total_minutes % 60) as u8;
            set_alarm::spawn(next_hour, next_minute).ok();
        }
    }
}