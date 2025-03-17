#![no_main]
#![no_std]
#![deny(unsafe_code)]
#![deny(warnings)]

mod state_machine;
mod rtc;
mod thermistor;
mod rotary_encoder;


use {
    crate::state_machine::*,
    core::sync::atomic::{AtomicU32, Ordering},
    cortex_m::asm, 
    hal::{
        rtc::*,
        saadc::*,
        qdec::*,
        gpiote::*,
        gpio::{p0::P0_03, Disconnected},
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
        rtc: Rtc<hal::pac::RTC0>,
        time_offset_ticks: AtomicU32,   // Time offset in ticks from 00:00
        alarm_offset_ticks: AtomicU32,  // Alarm offset in ticks from 00:00
        current_ticks: AtomicU32, // Temporary offset for settings
        temperature: f32,
        }

    #[local]
    struct Local {
        state_machine: State,
        saadc: Saadc,
        saadc_pin: P0_03<Disconnected>,
        qdec: Qdec,
        gpiote:Gpiote
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        
        // Enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Check if UICR is set correctly
        let check_uicr_set = 
        cx.device.UICR.pselreset[0].read().connect().is_connected()
        | cx.device.UICR.pselreset[1].read().connect().is_connected();

        if !check_uicr_set {
            cx.device.NVMC.config.write(|w| w.wen().wen());
            while cx.device.NVMC.ready.read().ready().is_busy() {}

            // Set nReset pin        
            for i in 0..2 {
                cx.device.UICR.pselreset[i].write(|w| {
                    w.pin().variant(thermistor::RESET_PIN);
                    w.port().variant(thermistor::RESET_PORT);
                    w.connect().connected();
                    w
                });
                while !cx.device.NVMC.ready.read().ready().is_ready() {}
            }
            cx.device.NVMC.config.write(|w| w.wen().ren());
            while cx.device.NVMC.ready.read().ready().is_busy() {}
            
            // Changes to UICR require a reset to take effect
            cortex_m::peripheral::SCB::sys_reset();
        };

        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);
        let _clocks = clocks.start_lfclk();

        // Initialize the RTC peripheral
        let rtc = rtc::init(cx.device.RTC0);

        // Initialize the rotary encoder and switch
        let rotation_pins = hal::qdec::Pins {
            a: port0.p0_30.into_pullup_input().degrade(),
            b: port0.p0_29.into_pullup_input().degrade(),
            led: None,
        };
        let switch_pin = port0.p0_28.into_pullup_input().degrade();
        let (qdec, gpiote) = rotary_encoder::init(
            cx.device.QDEC, cx.device.GPIOTE, rotation_pins, switch_pin);

        // Initialize the thermistor, read initial temp
        let saadc = thermistor::init(cx.device.SAADC);
        let saadc_pin = port0.p0_03;
        read_temperature::spawn().ok();

        // Simulate user setting the time
        let time_ticks = rtc::time_to_ticks(06, 20);
        set_time::spawn(time_ticks).ok();

        // Simulate user setting the alarm, 
        let alarm_ticks = rtc::time_to_ticks(06, 25);
        set_alarm::spawn(alarm_ticks).ok();

        let state_machine = State::Idle;

        (Shared {
            rtc,
            time_offset_ticks: AtomicU32::new(0),
            alarm_offset_ticks: AtomicU32::new(0),
            current_ticks: AtomicU32::new(0),
            temperature: 0.0,
        }, Local {
            state_machine,
            saadc,
            saadc_pin,
            qdec,
            gpiote
        }, init::Monotonics())
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(priority = 4, capacity = 10, local = [state_machine, current_ticks: u32 = 0, temp_ticks: u32 = 0], shared = [&time_offset_ticks, &current_ticks, &alarm_offset_ticks, temperature])]
    fn state_machine(cx: state_machine::Context, event: Event) {
        let state_machine = *cx.local.state_machine;
        let next_state = state_machine.next(event);
        *cx.local.state_machine = next_state;
        rprintln!("State: {:?}, Event: {:?} -> State: {:?}", state_machine, event, next_state);

        match event{
            Event::Timer(TimerEvent::PeriodicUpdate(counter)) => {
                let new_time = cx.shared.time_offset_ticks.load(Ordering::Relaxed) + counter % rtc::TICKS_PER_DAY;
                *cx.local.current_ticks = new_time;

                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                update_display::spawn(next_state, new_time).ok();
                read_temperature::spawn().ok();
            }
            Event::Timer(TimerEvent::AlarmTriggered) => {
                disable_alarm::spawn().ok();
            }
            Event::Encoder(EncoderEvent::ShortPressed) => {
                match state_machine {
                    State::Idle => {
                        // Set temp_ticks to current alarm
                        let alarm_time = cx.shared.alarm_offset_ticks.load(Ordering::Relaxed);  
                        *cx.local.temp_ticks = alarm_time;
    
                        disable_periodic_update::spawn().ok();
                        disable_alarm::spawn().ok();
                        set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                        update_display::spawn(next_state, alarm_time).ok();
                    }
                    State::Alarm => {
                        update_display::spawn(next_state, *cx.local.current_ticks).ok();
                    }
                    State::Settings(settings) => {
                        match settings {
                            Settings::ClockHours => {
                                update_display::spawn(next_state, *cx.local.temp_ticks).ok();
                            }
                            Settings::ClockMinutes => {
                                set_time::spawn(*cx.local.temp_ticks).ok();
                                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                                update_display::spawn(next_state, *cx.local.temp_ticks).ok();
                            }
                            Settings::AlarmHours => {
                                update_display::spawn(next_state, *cx.local.temp_ticks).ok();
                            }
                            Settings::AlarmMinutes => {
                                set_alarm::spawn(*cx.local.temp_ticks).ok();
                                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                                update_display::spawn(next_state, *cx.local.current_ticks).ok();
                            }
                        }
                    }
                    _ => {
                        todo!()
                    }
                }
            }
            Event::Encoder(EncoderEvent::LongPressed) => {
                match state_machine {
                    State::Idle => {
                        let temp = *cx.local.current_ticks;
                        *cx.local.temp_ticks = temp;

                        disable_periodic_update::spawn().ok();
                        disable_alarm::spawn().ok();
                        set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                        update_display::spawn(next_state, temp).ok();
                    }
                    State::Alarm => {
                        update_display::spawn(next_state, *cx.local.current_ticks).ok();
                    }
                    _ => {}
                }
                
            }
            Event::Encoder(EncoderEvent::Rotated(direction)) => {
                match state_machine {
                    State::Idle => {}
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
                        
                        update_display::spawn(next_state, new_time).ok();
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

    #[task(binds = RTC0, priority = 5, shared = [rtc, &time_offset_ticks])]
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

    #[task(priority = 3, shared = [rtc, &time_offset_ticks])]
    fn set_current_time(cx: set_current_time::Context) {
        rtc::set_current_time(cx);
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

    #[task(priority = 1, local = [saadc, saadc_pin], shared = [temperature])]
    fn read_temperature(cx: read_temperature::Context) {
        thermistor::read(cx);
    }

    #[task(priority = 3, shared = [temperature])]
    fn update_display(mut cx: update_display::Context, state: State, ticks: u32) {
        let temperature = cx.shared.temperature.lock(|temperature| *temperature);

        let (hour, minute) = rtc::ticks_to_time(ticks as u32);

        match state {
            State::Idle => {

                rprintln!("Time: {:02}:{:02}, Temperature: {:.2} C", hour, minute, temperature);
            }
            State::Alarm => {
                rprintln!("Time: {:02}:{:02}, Temperature: {:.2} C", hour, minute, temperature);
                rprintln!("BEEP BEEP BEEP");
            }
            State::Settings(settings) => {
                match settings {
                    Settings::ClockHours => {
                        rprintln!("Time: _{:02}_:{:02}", hour, minute);
                    }
                    Settings::ClockMinutes => {
                        rprintln!("Time: {:02}:_{:02}_", hour, minute);
                    }
                    Settings::AlarmHours => {
                        rprintln!("Alarm: _{:02}_:{:02}", hour, minute);
                    }
                    Settings::AlarmMinutes => {
                        rprintln!("Alarm: {:02}:_{:02}_", hour, minute);
                    }
                }
            }
            _ => {
                todo!()
            }
        }
    }
}