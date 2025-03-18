#![no_main]
#![no_std]
#![deny(unsafe_code)]
//#![deny(warnings)]

mod state_machine;
mod rtc;
mod thermistor;
mod rotary_encoder;
mod uicr;
mod pwm;
mod display;

use {
    crate::{state_machine::*, display::Display, pwm::Pwm0},
    core::sync::atomic::{AtomicU32, Ordering},
    cortex_m::asm, 
    hal::{
        rtc::*,
        saadc::*,
        qdec::*,
        gpiote::*,
        gpio::{Level, p0::P0_03, Disconnected, Output, Pin, PushPull},
    },
    nrf52833_hal as hal, 
    panic_rtt_target as _, 
    rtt_target::{rprintln, rtt_init_print}, 
};



#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TEMP, RNG, ECB])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        rtc: Rtc<hal::pac::RTC1>,
        time_offset_ticks: AtomicU32,   // Time offset in ticks from 00:00
        alarm_offset_ticks: AtomicU32,  // Alarm offset in ticks from 00:00
        temperature: f32,
        #[lock_free]
        pwm: Pwm0,
        display: Display,
        }

    #[local]
    struct Local {
        state_machine: State,
        saadc: Saadc,
        saadc_pin: P0_03<Disconnected>,
        qdec: Qdec,
        gpiote:Gpiote,
    }

    #[init(local = [ 
        SEQBUF0: [u16; 100] = pwm::PWM_DUTY_CYCLE_SEQUENCE,
        SEQBUF1: [u16; 100] = pwm::PWM_DUTY_CYCLE_SEQUENCE,
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        let SEQBUF0 = cx.local.SEQBUF0;
        let SEQBUF1 = cx.local.SEQBUF1;

        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let port1 = hal::gpio::p1::Parts::new(cx.device.P1);
        
        // Enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);
        let _clocks = clocks.start_lfclk().enable_ext_hfosc();

        // Initialize UICR
        uicr::init(cx.device.UICR, cx.device.NVMC);

        // Initialize PWM
        let led_pin: Pin<Output<PushPull>> = port0.p0_09.into_push_pull_output(Level::Low).degrade();
        let amp_fan_hum_pin = port0.p0_10.into_push_pull_output(Level::Low).degrade();
        let haptic_pin = port0.p0_20.into_push_pull_output(Level::Low).degrade();
        let pwm = pwm::init(cx.device.PWM0, led_pin, amp_fan_hum_pin, haptic_pin);

        // Initialize the RTC peripheral
        let rtc = rtc::init(cx.device.RTC1);

        // Initialize the rotary encoder and switch
        let rotation_pins = hal::qdec::Pins {
            a: port0.p0_30.into_pullup_input().degrade(),
            b: port0.p0_29.into_pullup_input().degrade(),
            led: None,
        };
        let switch_pin = port0.p0_28.into_pullup_input().degrade();
        let (qdec, gpiote) = rotary_encoder::init(
            cx.device.QDEC, cx.device.GPIOTE, rotation_pins, switch_pin);

        // Initialize the OLED display
        let scl_pin = port0.p0_11.into_floating_input().degrade();
        let sda_pin = port1.p1_09.into_floating_input().degrade();
        let twim_pins = hal::twim::Pins { scl: scl_pin, sda: sda_pin };
        let display = display::init(cx.device.TWIM0, twim_pins);
        
        // Initialize the thermistor, read initial temp
        let saadc = thermistor::init(cx.device.SAADC);
        let saadc_pin = port0.p0_03;
        read_temperature::spawn().ok();

        // Simulate user setting the time
        let time_ticks = rtc::time_to_ticks(06, 20);
        set_time::spawn(time_ticks).ok();
        update_display::spawn(time_ticks, display::Section::Display, false).ok();

        // Simulate user setting the alarm, 
        let alarm_ticks = rtc::time_to_ticks(06, 25);
        set_alarm::spawn(alarm_ticks).ok();

        let state_machine = State::Idle;
        (Shared {
            rtc,
            time_offset_ticks: AtomicU32::new(time_ticks),
            alarm_offset_ticks: AtomicU32::new(alarm_ticks),
            temperature: 0.0,
            pwm: pwm.load(Some(SEQBUF0), Some(SEQBUF1), false).ok(),
            display,
        }, Local {
            state_machine,
            saadc,
            saadc_pin,
            qdec,
            gpiote,
        }, init::Monotonics())
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(priority = 4, capacity = 10, local = [state_machine, current_ticks: u32 = 0, temp_ticks: u32 = 0], shared = [&time_offset_ticks, &alarm_offset_ticks])]
    fn state_machine(cx: state_machine::Context, event: Event) {
        let state = *cx.local.state_machine;
        let next_state = state.next(event);
        *cx.local.state_machine = next_state;
        //rprintln!("State: {:?}, Event: {:?} -> State: {:?}", state, event, next_state);

        match event{
            Event::Timer(TimerEvent::PeriodicUpdate(counter)) => {
                let new_time = cx.shared.time_offset_ticks.load(Ordering::Relaxed) + counter % rtc::TICKS_PER_DAY;
                *cx.local.current_ticks = new_time;
                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                read_temperature::spawn().ok();

                match state {
                    State::Idle => {
                        update_display::spawn(new_time, display::Section::Display, false).ok();
                    }
                    _ => {}
                }
            }
            Event::Timer(TimerEvent::AlarmTriggered) => {
                start_pwm::spawn().ok();
                disable_alarm::spawn().ok();
                set_blinking::spawn(rtc::BLINK_TICKS).ok();
            }
            Event::Timer(TimerEvent::Timeout) => {
                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                disable_blinking::spawn().ok();
                update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
            }
            Event::Timer(TimerEvent::Blink) => {
                set_blinking::spawn(rtc::BLINK_TICKS).ok();
                match state {
                    State::Alarm => {
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, true).ok();
                    }
                    State::Settings(settings) => {
                        match settings {
                            Settings::ClockHours => {
                                update_display::spawn(*cx.local.temp_ticks, display::Section::Hour, true).ok();
                            }
                            Settings::ClockMinutes => {
                                update_display::spawn(*cx.local.temp_ticks, display::Section::Minute, true).ok();
                            }
                            Settings::AlarmHours => {
                                update_display::spawn(*cx.local.temp_ticks, display::Section::Hour, true).ok();
                            }
                            Settings::AlarmMinutes => {
                                update_display::spawn(*cx.local.temp_ticks, display::Section::Minute, true).ok();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Encoder(EncoderEvent::ShortPressed) => {
                match state {
                    State::Idle => {
                        let alarm_time = cx.shared.alarm_offset_ticks.load(Ordering::Relaxed);  
                        *cx.local.temp_ticks = alarm_time;
    
                        disable_alarm::spawn().ok();
                        set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                        set_blinking::spawn(rtc::BLINK_TICKS).ok();
                        update_display::spawn(alarm_time, display::Section::Display, false).ok();
                    }
                    State::Alarm => {
                        stop_pwm::spawn().ok();
                        disable_blinking::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    State::Settings(settings) => {
                        match settings {
                            Settings::ClockHours => {
                            }
                            Settings::ClockMinutes => {
                                set_time::spawn(*cx.local.temp_ticks).ok();
                                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                                set_alarm::spawn(cx.shared.alarm_offset_ticks.load(Ordering::Relaxed)).ok();
                                disable_blinking::spawn().ok();
                                update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                            }
                            Settings::AlarmHours => {
                            }
                            Settings::AlarmMinutes => {
                                set_alarm::spawn(*cx.local.temp_ticks).ok();
                                disable_blinking::spawn().ok();
                                update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                            }
                        }
                    }
                    _ => {
                        todo!()
                    }
                }
            }
            Event::Encoder(EncoderEvent::LongPressed) => {
                match state {
                    State::Idle => {
                        let temp = *cx.local.current_ticks;
                        *cx.local.temp_ticks = temp;

                        disable_periodic_update::spawn().ok();
                        disable_alarm::spawn().ok();
                        set_blinking::spawn(rtc::BLINK_TICKS).ok();
                        set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                    }
                    State::Alarm => {
                        stop_pwm::spawn().ok();
                        disable_blinking::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    _ => {}
                }
                
            }
            Event::Encoder(EncoderEvent::Rotated(direction)) => {
                match state {
                    State::Idle => {
                        // Should we set the volume here?
                    }
                    State::Alarm => {}
                    State::Settings(settings) => {
                        let mut diff = direction;
                        match settings {
                            Settings::ClockHours => {
                                diff = diff * rtc::TICKS_PER_HOUR as isize;
                            }
                            Settings::ClockMinutes => {
                                diff = diff * rtc::TICKS_PER_MINUTE as isize;
                            }
                            Settings::AlarmHours => {
                                diff = diff * rtc::TICKS_PER_HOUR as isize;
                            }
                            Settings::AlarmMinutes => {
                                diff = diff * rtc::TICKS_PER_MINUTE as isize;
                            }
                        }
                        let temp = *cx.local.temp_ticks;
                        let new_time = (temp as isize + diff) as u32 % rtc::TICKS_PER_DAY;
                        *cx.local.temp_ticks = new_time;
                        
                        update_display::spawn(new_time, display::Section::Display, false).ok();
                    }
                    _ => {
                        todo!()
                    }
                }
            }
            _ => {
                todo!()
            }
        }
    }

    #[task(binds = RTC1, priority = 5, shared = [rtc, &time_offset_ticks])]
    fn rtc_interrupt(cx: rtc_interrupt::Context) {
        rtc::handle_interrupt(cx);
    }

    #[task(binds = QDEC, priority = 5,  local = [qdec, compare_cycle: u32 = 0])]
    fn qdec_interrupt(cx: qdec_interrupt::Context) {
        rotary_encoder::handle_qdec_interrupt(cx);
    }

    #[task(binds = GPIOTE, priority = 5, local = [gpiote, last_press: u32 = 0])]
    fn gpiote_interrupt(cx: gpiote_interrupt::Context) {
        rotary_encoder::handle_gpiote_interrupt(cx);
    }

    #[task(priority = 3, shared = [rtc, &time_offset_ticks])]
    fn set_time(cx: set_time::Context, ticks: u32) {
        rtc::set_time(cx, ticks);
    }

    #[task(priority = 3, shared = [rtc, &alarm_offset_ticks, &time_offset_ticks])]
    fn set_alarm(cx: set_alarm::Context, ticks: u32) {
        rtc::set_alarm(cx, ticks);
    }

    #[task(priority = 3, shared = [rtc])]
    fn disable_alarm(cx: disable_alarm::Context) {
        rtc::disable_alarm(cx);
    }

    #[task(priority = 3, shared = [rtc])]
    fn set_periodic_update(cx: set_periodic_update::Context, interval_minutes: u32) {
        rtc::set_periodic_update(cx, interval_minutes);
    }

    #[task(priority = 3, shared = [rtc])]
    fn disable_periodic_update(cx: disable_periodic_update::Context) {
        rtc::disable_periodic_update(cx);
    }

    #[task(priority = 1, shared = [rtc, &time_offset_ticks])]
    fn set_timeout(cx: set_timeout::Context, ticks: u32) {
        rtc::set_timeout(cx, ticks);
    }

    #[task(priority = 1, shared = [rtc])]
    fn disable_timeout(cx: disable_timeout::Context) {
        rtc::disable_timeout(cx);
    }

    #[task(priority = 1, shared = [rtc])]
    fn set_blinking(cx: set_blinking::Context, interval_ticks: u32) {
        rtc::set_blinking(cx, interval_ticks);
    }

    #[task(priority = 1, shared = [rtc])]
    fn disable_blinking(cx: disable_blinking::Context) {
        rtc::disable_blinking(cx);
    }

    #[task(priority = 1, local = [saadc, saadc_pin], shared = [temperature])]
    fn read_temperature(cx: read_temperature::Context) {
        thermistor::read(cx);
    }

    #[task(priority = 1, shared = [pwm])]
    fn start_pwm(cx: start_pwm::Context) {
        pwm::start(cx);
    }

    #[task(priority = 1, shared = [pwm])]
    fn stop_pwm(cx: stop_pwm::Context) {
        pwm::stop(cx);
    }

    #[task(priority = 3, shared = [display, temperature], local = [on: bool = true])]
    fn update_display(cx: update_display::Context, ticks: u32, section: display::Section, blink: bool) {
        display::update_display_rtt(cx, ticks, section, blink);
    }

    #[task(priority = 3, shared = [display])]
    fn clear_display(cx: clear_display::Context) {
        display::clear(cx);
    }
}