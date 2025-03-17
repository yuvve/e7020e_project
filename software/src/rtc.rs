use {
    crate::{
        app::*, 
        state_machine::*
    }, 
    hal::{
        pac::RTC0, 
        rtc::*
    }, 
    nrf52833_hal::{self as hal}, 
    rtic::Mutex,
    core::sync::atomic::Ordering, 
};

const RTC_PRESCALER: u32 = 4095; // 8 Hz RTC frequency, max prescaler value
const MAX_TICKS: u32 = 16_777_216; // 24 bit max value for RTC counter
pub const TICKS_PER_SECOND: u32 = 8;
pub const TICKS_PER_MINUTE: u32 = TICKS_PER_SECOND;// * 60; // Interrupt every second for demonstration purpose, will be 8*60 in production
pub const TICKS_PER_HOUR: u32 = TICKS_PER_MINUTE * 60;
pub const TICKS_PER_DAY: u32 = TICKS_PER_MINUTE * 60 * 24;
pub const TIMEOUT_SETTINGS_TICKS: u32 = TICKS_PER_MINUTE;

pub(crate) fn init(rtc: RTC0) -> Rtc<hal::pac::RTC0> {
    let mut rtc = hal::rtc::Rtc::new(rtc, RTC_PRESCALER).unwrap();
    // Start periodic interrupt every minute, to update OLED display
    rtc.set_compare(RtcCompareReg::Compare0, TICKS_PER_MINUTE).unwrap();
    rtc.enable_interrupt(RtcInterrupt::Compare0, None);
    rtc.enable_interrupt(RtcInterrupt::Overflow, None);
    rtc.enable_counter();
    rtc
}

pub(crate) fn set_alarm(mut cx: set_alarm::Context, ticks:  u32) {
    let next_interrupt = next_alarm_ticks(cx.shared.time_offset_ticks.load(Ordering::Relaxed), ticks);
    cx.shared.rtc.lock(|rtc| {
        rtc.set_compare(RtcCompareReg::Compare1, next_interrupt).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Compare1, None);
    });
    cx.shared.alarm_offset_ticks.store(ticks, Ordering::Relaxed);
}

pub(crate) fn disable_alarm(mut cx: disable_alarm::Context) {
    cx.shared.rtc.lock(|rtc| {
        rtc.disable_interrupt(RtcInterrupt::Compare1, None);
    });
}

pub(crate) fn set_time(mut cx: set_time::Context, ticks: u32) {
    cx.shared.rtc.lock(|rtc| {
        rtc.clear_counter();
    });
    cx.shared.time_offset_ticks.store(ticks, Ordering::Relaxed);
}

pub(crate) fn set_current_time(mut cx: set_current_time::Context) {
    let counter = cx.shared.rtc.lock(|rtc| rtc.get_counter());
    let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
    let new_offset = (counter + time_offset_ticks) % TICKS_PER_DAY;
    cx.shared.rtc.lock(|rtc| {
        rtc.clear_counter();
    });
    cx.shared.time_offset_ticks.store(new_offset, Ordering::Relaxed);
}

pub(crate) fn set_timeout(mut cx: set_timeout::Context, ticks: u32) {
    let counter = cx.shared.rtc.lock(|rtc| rtc.get_counter());
    let timeout_ticks = counter + ticks;
    cx.shared.rtc.lock(|rtc| {
        rtc.set_compare(RtcCompareReg::Compare2, timeout_ticks).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Compare2, None);
    });
}

pub(crate) fn set_periodic_update(mut cx: set_periodic_update::Context, interval_ticks: u32) {
    let counter = cx.shared.rtc.lock(|rtc| rtc.get_counter());

    // Align the next interrupt to the nearest future multiple of interval_ticks
    let next_interrupt = ((counter / interval_ticks) + 1) * interval_ticks;

    cx.shared.rtc.lock(|rtc| {
        rtc.set_compare(RtcCompareReg::Compare0, next_interrupt % MAX_TICKS).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Compare0, None);
    });
}

pub(crate) fn disable_periodic_update(mut cx: disable_periodic_update::Context) {
    cx.shared.rtc.lock(|rtc| {
        rtc.disable_interrupt(RtcInterrupt::Compare0, None);
    });
}

pub(crate) fn time_to_ticks(hour: u8, minute: u8) -> u32 {
    let minutes = (hour as u32) * 60 + (minute as u32);
    let time_offset_ticks = minutes * TICKS_PER_MINUTE;

    time_offset_ticks
}

pub(crate) fn ticks_to_time(ticks: u32) -> (u8, u8) {
    let minutes = ticks / TICKS_PER_MINUTE;
    let hour = ((minutes / 60) % 24) as u8;
    let minute = (minutes % 60) as u8;

    (hour as u8, minute as u8)
}

pub(crate) fn handle_interrupt(mut cx: rtc_interrupt::Context) {
    cx.shared.rtc.lock(|rtc| {
        // Need to check which interrupt has been triggered
        // multiple interrupts can be triggered at the same time

        // Compare 0: Periodic interrupt every minute
        if rtc.is_event_triggered(RtcInterrupt::Compare0) {
            rtc.reset_event(RtcInterrupt::Compare0);

            let counter = rtc.get_counter();
            state_machine::spawn(Event::Timer(TimerEvent::PeriodicUpdate(counter))).ok();
        } 
        // Compare 1: Alarm interrupt
        if rtc.is_event_triggered(RtcInterrupt::Compare1) {
            rtc.reset_event(RtcInterrupt::Compare1);
            state_machine::spawn(Event::Timer(TimerEvent::AlarmTriggered)).ok();
        }
        // Compare 2: Timeout interrupt
        if rtc.is_event_triggered(RtcInterrupt::Compare2) {
            rtc.reset_event(RtcInterrupt::Compare2);
            state_machine::spawn(Event::Timer(TimerEvent::Timeout)).ok();
        }
        // Overflow: RTC counter has reached its maximum value
        if rtc.is_event_triggered(RtcInterrupt::Overflow) {
            rtc.reset_event(RtcInterrupt::Overflow);

            let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
            // Update the time offset to current time, adjusting for overflow
            let new_offset = (MAX_TICKS + time_offset_ticks) % TICKS_PER_DAY;
            
            cx.shared.time_offset_ticks.store(new_offset, Ordering::Relaxed);
        };
    });
}

fn next_alarm_ticks(time_offset_ticks: u32, alarm_offset_ticks: u32) -> u32 {
    let current_time = (time_offset_ticks ) % TICKS_PER_DAY;
    let next_alarm_ticks = if current_time > alarm_offset_ticks {
        // Alarm time has already passed today, set it for tomorrow
        TICKS_PER_DAY - current_time + alarm_offset_ticks
    } else {
        alarm_offset_ticks - current_time
    };

    // Modulo MAX_TICKS, adjusting for RTC counter overflow
    next_alarm_ticks % MAX_TICKS
}