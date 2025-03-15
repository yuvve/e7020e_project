#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

mod state_machine;
mod time;
mod temp;

use {
    cortex_m::asm, 
    hal::rtc::*,
    hal::saadc::*,
    hal::gpio::{p0::P0_03, Disconnected},

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
        temperature: f32,
        }

    #[local]
    struct Local {
        state_machine: State,
        saadc: Saadc,
        saadc_pin: P0_03<Disconnected>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);

        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);
        let _clocks = clocks.start_lfclk();

        // Initialize the RTC peripheral
        let rtc = time::init(cx.device.RTC0);

        // Initialize the thermistor, read initial temp
        let saadc = temp::init(cx.device.SAADC);
        let saadc_pin = port0.p0_03;
        read_temperature::spawn().ok();

        // Simulate user setting the time
        set_time::spawn(06, 20).ok();

        // Simulate user setting the alarm, 
        set_alarm::spawn(06, 25).ok();

        let state_machine = State::Idle;

        (Shared {
            rtc,
            time_offset_ticks: AtomicU32::new(0),
            alarm_offset_ticks: AtomicU32::new(0),
            temperature: 0.0,
        }, Local {
            state_machine,
            saadc,
            saadc_pin
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
                    let elapsed_ticks = arg;
                    update_display::spawn(elapsed_ticks, alarm_triggered).ok();
                    read_temperature::spawn().ok();

                    *cx.local.state_machine = cx.local.state_machine.next(event);
                }
                _ => {
                    todo!()
                }
            }

            State::Alarm => match event {
                Event::TimerEvent(TimerEvent::PeriodicUpdate) => {
                    let alarm_triggered = true;
                    let elapsed_ticks = arg;
                    update_display::spawn(elapsed_ticks, alarm_triggered).ok();
                    read_temperature::spawn().ok();

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
    fn rtc_interrupt(cx: rtc_interrupt::Context) {
        time::handle_interrupt(cx);
    }

    #[task(priority = 1, shared = [rtc, &time_offset_ticks])]
    fn set_time(cx: set_time::Context, hour: u8, minute: u8) {
        time::set_time(cx, hour, minute);
    }

    #[task(priority = 1, shared = [rtc, &alarm_offset_ticks, &time_offset_ticks])]
    fn set_alarm(cx: set_alarm::Context, hour: u8, minute: u8) {
        time::set_alarm(cx, hour, minute);
    }

    #[task(priority = 1, local = [saadc, saadc_pin], shared = [temperature])]
    fn read_temperature(cx: read_temperature::Context) {
        temp::read(cx);
    }

    #[task(priority = 3, shared = [&time_offset_ticks, temperature])]
    fn update_display(mut cx: update_display::Context, elapsed_ticks: u32, alarm_triggered: bool) {
        // Compute elapsed time in minutes
        let time_offset_ticks = cx.shared.time_offset_ticks.load(Ordering::Relaxed);
        let (hour, minute) = time::ticks_to_time(time_offset_ticks + elapsed_ticks);

        let temperature = cx.shared.temperature.lock(|temperature| *temperature);
        // Update OLED display
        rprintln!("Time: {:02}:{:02}, Temperature: {:.1}C", hour, minute, temperature);

        if alarm_triggered {
            rprintln!("BEEP BEEP BEEP");
        }
    }
}