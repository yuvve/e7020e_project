use {
    core::sync::atomic::{AtomicU32, Ordering}, hal::{pac::RTC0, rtc::*}, nrf52833_hal as hal, 
    rtic::Mutex
};

const RTC_PRESCALER: u32 = 4095; // 8 Hz RTC frequency, max prescaler value
const TICKS_PER_MINUTE: u32 = 8; // Interrupt every second for demonstration purpose, will be 8*60 in production
const TICKS_PER_DAY: u32 = TICKS_PER_MINUTE * 60 * 24;
const MAX_TICKS: u32 = 16_777_216; // 24 bit max value for RTC counter

pub(crate) fn init(rtc: RTC0) -> Rtc<hal::pac::RTC0> {
    let mut rtc = hal::rtc::Rtc::new(rtc, RTC_PRESCALER).unwrap();
    // Start periodic interrupt every minute, to update OLED display
    rtc.set_compare(RtcCompareReg::Compare0, TICKS_PER_MINUTE).unwrap();
    rtc.enable_interrupt(RtcInterrupt::Compare0, None);
    rtc.enable_interrupt(RtcInterrupt::Overflow, None);
    rtc.enable_counter();
    rtc
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

pub(crate) fn periodic_interrupt(rtc: &mut Rtc<hal::pac::RTC0>, counter: u32) {
    rtc.reset_event(RtcInterrupt::Compare0);

    let next_interrupt = (counter + TICKS_PER_MINUTE) % MAX_TICKS;
    rtc.set_compare(RtcCompareReg::Compare0, next_interrupt).ok();
}

pub(crate) fn alarm_interrupt(rtc: &mut Rtc<hal::pac::RTC0>) {
    rtc.reset_event(RtcInterrupt::Compare1);
}

pub(crate) fn overflow_interrupt(rtc: &mut Rtc<hal::pac::RTC0>, time_offset: &AtomicU32) -> u32 {
    rtc.reset_event(RtcInterrupt::Overflow);

    let time_offset_ticks = time_offset.load(Ordering::Relaxed);
    // Update the time offset to current time, adjusting for overflow
    let new_offset = (MAX_TICKS + time_offset_ticks) % TICKS_PER_DAY;
    
    time_offset.store(new_offset, Ordering::Relaxed);
    new_offset
}

pub(crate) fn set_alarm(mut cx: crate::app::__rtic_internal_set_alarmSharedResources, hour: u8, minute: u8) {
    let alarm_ticks = time_to_ticks(hour, minute);
    let counter = cx.rtc.lock(|rtc| rtc.get_counter());
    
    let next_interrupt = next_alarm_ticks(counter, cx.time_offset_ticks.load(Ordering::Relaxed), alarm_ticks);
    cx.rtc.lock(|rtc| {
        rtc.set_compare(RtcCompareReg::Compare1, next_interrupt).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Compare1, None);
    });
    cx.alarm_offset_ticks.store(alarm_ticks, Ordering::Relaxed);
}

pub(crate) fn set_time(mut cx: crate::app::__rtic_internal_set_timeSharedResources, hour: u8, minute: u8) {
    let time_offset_ticks = time_to_ticks(hour, minute);
    cx.rtc.lock(|rtc| {
        rtc.clear_counter();
    });
    cx.time_offset_ticks.store(time_offset_ticks, Ordering::Relaxed);
}

pub(crate) fn next_alarm_ticks(counter: u32, time_offset_ticks: u32, alarm_offset_ticks: u32) -> u32 {
    let current_time = (counter + time_offset_ticks ) % TICKS_PER_DAY;
    let next_alarm_ticks = if current_time > alarm_offset_ticks {
        // Alarm time has already passed today, set it for tomorrow
        TICKS_PER_DAY - current_time + alarm_offset_ticks
    } else {
        alarm_offset_ticks - current_time + counter
    };

    // Modulo MAX_TICKS, adjusting for RTC counter overflow
    next_alarm_ticks % MAX_TICKS
}
