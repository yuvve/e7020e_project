#![no_main]
#![no_std]
#![allow(dead_code)]
//#![deny(unsafe_code)]
#![deny(warnings)]

mod rtt;
mod display;
mod gpio;
mod pwm;
mod rotary_encoder;
mod rtc;
mod state_machine;
mod thermistor;
mod uicr;
mod backup_mode;
mod cli;
mod speaker;

use {
    cli::*,
    crate::{display::Display, pwm::Pwm0, state_machine::*},
    core::sync::atomic::{AtomicU32, AtomicBool, Ordering},
    cortex_m::asm,
    hal::{
        gpio::*,
        gpiote::*,
        qdec::*,
        rtc::*,
        saadc::*,
        lpcomp::LpComp,
        clocks::{
            Clocks,
            ExternalOscillator,
            Internal,
            LfOscStarted,
        },
        usbd::{
            Usbd,
            UsbPeripheral
        },
    },
    nrf52833_hal as hal, 
    panic_rtt_target as _,
    rtt_target::UpChannel,
    usb_device::device::UsbDevice,
    usbd_serial::{SerialPort, USB_CLASS_CDC},

    usb_device::{
        class_prelude::UsbBusAllocator,
        device::{StringDescriptors, UsbDeviceBuilder, UsbVidPid},
    },
};

#[cfg(feature = "52833-debug")]
use {
    core::fmt::Write,
    rtt_target::rprintln,
};

#[rtic::app(device = nrf52833_hal::pac, dispatchers= [TEMP, RNG, ECB, FPU, PDM, CCM_AAR, SWI5_EGU5])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        rtt_hw: UpChannel,
        rtt_serial: UpChannel,
        rtc: Rtc<hal::pac::RTC1>,
        time_offset_ticks: AtomicU32,  // Time offset in ticks from 00:00
        alarm_offset_ticks: AtomicU32, // Alarm offset in ticks from 00:00
        amp_on: AtomicBool,
        temperature: f32,
        #[lock_free]
        pwm: Pwm0,
        display: Display,
        amp_fan_hum_pin: Pin<Output<PushPull>>,
        #[lock_free]
        usb_dev: UsbDevice<'static, Usbd<UsbPeripheral<'static>>>,
        #[lock_free]
        serial: SerialPort<'static, Usbd<UsbPeripheral<'static>>>, 
        gpiote: Gpiote,
        qdec: Qdec,
    }

    #[local]
    struct Local {
        rtt_display: UpChannel,
        rtt_state: UpChannel,
        rtt_speaker: UpChannel,
        state_machine: State,
        saadc: Saadc,
        saadc_pin: p0::P0_03<Disconnected>,
        comp: LpComp,
        dma_buf: [u32; speaker::BUFFER_LEN],
        i2s: Option<hal::i2s::I2S>,
    }

    #[init(local = [
        SEQBUF0: [u16; pwm::SEQUENCE_LENGTH*4] = [0u16; pwm::SEQUENCE_LENGTH*4],
        SEQBUF1: [u16; pwm::SEQUENCE_LENGTH*4] = [0u16; pwm::SEQUENCE_LENGTH*4],
        clocks: Option<Clocks<ExternalOscillator, Internal, LfOscStarted>> = None,
        usb_bus: Option<UsbBusAllocator<Usbd<UsbPeripheral<'static>>>> = None, 
    ])]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let (rtt_display, rtt_hw, rtt_state, rtt_serial, rtt_speaker) = rtt::init();

        let SEQBUF0 = cx.local.SEQBUF0;
        let SEQBUF1 = cx.local.SEQBUF1;

        //Enable USBD interrupt
        cx.device.USBD.intenset.write(|w| w.sof().set());

        // Need to set up the 32kHz clock source for the RTC
        let clocks = hal::clocks::Clocks::new(cx.device.CLOCK);

        // make static lifetime for clocks
        cx.local.clocks.replace(clocks.enable_ext_hfosc().start_lfclk());

        let usb_bus = UsbBusAllocator::new(Usbd::new(UsbPeripheral::new(
            cx.device.USBD,
            // refer to static lifetime
            cx.local.clocks.as_ref().unwrap(),
        )));
        cx.local.usb_bus.replace(usb_bus);

        let serial = SerialPort::new(&cx.local.usb_bus.as_ref().unwrap());

        let usb_dev = UsbDeviceBuilder::new(
            &cx.local.usb_bus.as_ref().unwrap(),
            UsbVidPid(0x16c0, 0x27dd),
        )
        .strings(&[StringDescriptors::default()
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")])
        .unwrap()
        .device_class(USB_CLASS_CDC)
        .max_packet_size_0(64) // (makes control transfers 8x faster)
        .unwrap()
        .build();

        // Enable cycle counter
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Initialize GPIO pins
        let pins = gpio::init(cx.device.P0, cx.device.P1);

        // Initialize UICR
        uicr::init(cx.device.UICR, cx.device.NVMC);

        // Initialize PWM
        let pwm = pwm::init(cx.device.PWM0, pins.led, pins.haptic);
        let pwm = pwm.load(Some(SEQBUF0), Some(SEQBUF1), false).ok();
        load_pwm_sequence::spawn().ok();

        // Initialize the RTC peripheral
        let rtc = rtc::init(cx.device.RTC1);

        // Initialize the rotary encoder and switch
        let (qdec, gpiote) = rotary_encoder::init(
            cx.device.QDEC,
            cx.device.GPIOTE,
            pins.rotary_encoder,
            pins.rotary_switch,
        );

        // Initialize the OLED display
        let display = display::init(cx.device.TWIM0, pins.oled);

        // Initialize the thermistor, read initial temp
        let saadc = thermistor::init(cx.device.SAADC);
        read_temperature::spawn().ok();

        // Simulate user setting the time
        let time_ticks = rtc::time_to_ticks(06, 20);
        set_time::spawn(time_ticks).ok();

        // Simulate user setting the alarm,
        let alarm_ticks = rtc::time_to_ticks(06, 21);
        set_alarm::spawn(alarm_ticks).ok();

        let comp = backup_mode::init(cx.device.LPCOMP, pins.vdetect);

        let i2s = speaker::init(cx.device.I2S, pins.speaker);
        enable_display::spawn().ok();
        (
            Shared {
                rtt_serial,
                rtt_hw,
                rtc,
                time_offset_ticks: AtomicU32::new(time_ticks),
                alarm_offset_ticks: AtomicU32::new(alarm_ticks),
                amp_on: AtomicBool::new(false),
                temperature: 0.0,
                pwm,
                display,
                amp_fan_hum_pin: pins.amp_fan_hum,
                usb_dev,
                serial,
                gpiote,
                qdec,
            },
            Local {
                rtt_display,
                rtt_state,
                rtt_speaker,
                state_machine: State::Idle,
                saadc,
                saadc_pin: pins.saadc,
                comp,
                dma_buf: [0u32; speaker::BUFFER_LEN],
                i2s: Some(i2s),
            },
            init::Monotonics(),
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        #[cfg(feature = "52833-debug")]
        rprintln!("idle");
        loop {
            asm::wfi();
        }
    }

    #[task(
        priority = 2, 
        capacity = 10, 
        local = [state_machine, current_ticks: u32 = 0, temp_ticks: u32 = 0, rtt_state], 
        shared = [&time_offset_ticks, &alarm_offset_ticks, &amp_on])]
    fn state_machine(cx: state_machine::Context, event: Event) {
        let state = *cx.local.state_machine;
        let next_state = state.next(event);
        *cx.local.state_machine = next_state;
        #[cfg(feature = "52833-debug")]
        writeln!(
            cx.local.rtt_state, 
            "State: {:?}, Event: {:?} -> State: {:?}", state, event, next_state
        ).ok();

        match event {
            Event::Timer(TimerEvent::PeriodicUpdate(counter)) => {
                let new_time = cx.shared.time_offset_ticks.load(Ordering::Relaxed)
                    + counter % rtc::TICKS_PER_DAY;
                *cx.local.current_ticks = new_time;
                read_temperature::spawn().ok();
                set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();

                match state {
                    State::Idle => {
                        update_display::spawn(new_time, display::Section::Display, false).ok();
                    }
                    _ => {}
                }
            }
            Event::Timer(TimerEvent::AlarmTriggered) => {
                match state {
                    State::Idle => {
                        cx.shared.amp_on.store(true, Ordering::Relaxed); 
                        turn_on_amp_fan_hum::spawn().ok();
                        disable_alarm::spawn().ok();
                        start_pwm::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::AlarmIcon, false).ok();
                        play_next_audio_segment::spawn().ok();
                    }
                    _ => {}
                }
            }
            Event::Timer(TimerEvent::Timeout) => {
                match state {
                    State::Settings(_) => {
                        disable_blinking::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    State::Alarm => {
                        disable_alarm_components(&cx);
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    _ => {}
                }
            }
            Event::Timer(TimerEvent::Blink) => {
                match state {
                    State::Alarm => {
                        update_display::spawn(*cx.local.current_ticks, display::Section::AlarmIcon, true).ok();
                        set_blinking::spawn(rtc::BLINK_TICKS).ok();

                    }
                    State::Settings(settings) => match settings {
                        Settings::ClockHours => {
                            update_display::spawn(*cx.local.temp_ticks, display::Section::Hour, true).ok();
                            set_blinking::spawn(rtc::BLINK_TICKS).ok();

                        }
                        Settings::ClockMinutes => {
                            update_display::spawn(*cx.local.temp_ticks, display::Section::Minute, true).ok();
                            set_blinking::spawn(rtc::BLINK_TICKS).ok();

                        }
                        Settings::AlarmHours => {
                            update_display::spawn(*cx.local.temp_ticks, display::Section::Hour, true).ok();
                            set_blinking::spawn(rtc::BLINK_TICKS).ok();

                        }
                        Settings::AlarmMinutes => {
                            update_display::spawn(*cx.local.temp_ticks, display::Section::Minute, true).ok();
                            set_blinking::spawn(rtc::BLINK_TICKS).ok();
                        }
                    },
                    _ => {}
                }
            }
            Event::Encoder(EncoderEvent::ShortPressed) => match state {
                State::Idle => {
                    let alarm_time = cx.shared.alarm_offset_ticks.load(Ordering::Relaxed);
                    *cx.local.temp_ticks = alarm_time;

                    disable_alarm::spawn().ok();
                    set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                    set_blinking::spawn(rtc::BLINK_TICKS).ok();
                    update_display::spawn(alarm_time, display::Section::Display, false).ok();
                }
                State::Alarm => {
                    disable_alarm_components(&cx);
                    update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                }
                State::Settings(settings) => match settings {
                    Settings::ClockMinutes => {
                        disable_blinking::spawn().ok();
                        set_time::spawn(*cx.local.temp_ticks).ok();
                        set_alarm::spawn(cx.shared.alarm_offset_ticks.load(Ordering::Relaxed)).ok();
                        update_display::spawn(*cx.local.temp_ticks, display::Section::Display, false).ok();
                        set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                    }
                    Settings::AlarmMinutes => {
                        set_alarm::spawn(*cx.local.temp_ticks).ok();
                        disable_blinking::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    _ => {}
                },
                _ => {}
            },
            Event::Encoder(EncoderEvent::LongPressed) => match state {
                State::Idle => {
                    let temp = *cx.local.current_ticks;
                    *cx.local.temp_ticks = temp;

                    disable_periodic_update::spawn().ok();
                    disable_alarm::spawn().ok();
                    set_blinking::spawn(rtc::BLINK_TICKS).ok();
                    set_timeout::spawn(rtc::TIMEOUT_SETTINGS_TICKS).ok();
                }
                State::Alarm => {
                    disable_alarm_components(&cx);
                    update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                }
                _ => {}
            },
            Event::Encoder(EncoderEvent::Rotated(direction)) => {
                match state {
                    State::Settings(settings) => {
                        let mut diff = direction;
                        match settings {
                            Settings::ClockHours => {
                                diff = diff * rtc::TICKS_PER_HOUR as isize;
                                let temp = *cx.local.temp_ticks;
                                let new_time = (temp as isize + diff).rem_euclid(rtc::TICKS_PER_DAY as isize) as u32;
                                *cx.local.temp_ticks = new_time;
                                update_display::spawn(new_time, display::Section::Display, false).ok();
                            }
                            Settings::ClockMinutes => {
                                diff = diff * rtc::TICKS_PER_MINUTE as isize;
                                let temp = *cx.local.temp_ticks;
                                let new_time = (temp as isize + diff).rem_euclid(rtc::TICKS_PER_DAY as isize) as u32;
                                *cx.local.temp_ticks = new_time;
                                update_display::spawn(new_time, display::Section::Display, false).ok();
                            }
                            Settings::AlarmHours => {
                                diff = diff * rtc::TICKS_PER_HOUR as isize;
                                let temp = *cx.local.temp_ticks;
                                let new_time = (temp as isize + diff).rem_euclid(rtc::TICKS_PER_DAY as isize) as u32;
                                *cx.local.temp_ticks = new_time;
                                update_display::spawn(new_time, display::Section::Display, false).ok();
                            }
                            Settings::AlarmMinutes => {
                                diff = diff * rtc::TICKS_PER_MINUTE as isize;
                                let temp = *cx.local.temp_ticks;
                                let new_time = (temp as isize + diff).rem_euclid(rtc::TICKS_PER_DAY as isize) as u32;
                                *cx.local.temp_ticks = new_time;
                                update_display::spawn(new_time, display::Section::Display, false).ok();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::VBUSConnected => {
                match state {
                    State::BackupBattery => { // Just in case
                        set_alarm::spawn(*cx.local.current_ticks).ok();
                        set_periodic_update::spawn(rtc::TICKS_PER_MINUTE).ok();
                        rotary_encoder_enable_interrupts::spawn().ok();
                        enable_display::spawn().ok();
                        update_display::spawn(*cx.local.current_ticks, display::Section::Display, false).ok();
                    }
                    _ => {}
                }
            }
            Event::VBUSDisconnected => {
                match state {
                    State::BackupBattery => {} // Just in case
                    _ => {
                        rotary_disable_interrupts::spawn().ok();
                        disable_alarm::spawn().ok();
                        disable_periodic_update::spawn().ok();
                        disable_timeout::spawn().ok();
                        disable_display::spawn().ok();
                        disable_alarm_components(&cx);
                    }
                }
            }
            _ => {}
        }
    }

    #[task(binds = RTC1, priority = 4, shared = [rtc, &time_offset_ticks, rtt_hw])]
    fn rtc_interrupt(cx: rtc_interrupt::Context) {
        rtc::handle_interrupt(cx);
    }

    #[task(binds = QDEC, priority = 4,  local = [last_rotation: u32 = 0], shared = [rtt_hw, qdec])]
    fn qdec_interrupt(cx: qdec_interrupt::Context) {
        rotary_encoder::handle_qdec_interrupt(cx);
    }

    #[task(binds = GPIOTE, priority = 4, local = [last_press: u32 = 0], shared = [gpiote, rtt_hw])]
    fn gpiote_interrupt(cx: gpiote_interrupt::Context) {
        rotary_encoder::handle_gpiote_interrupt(cx);
    }

    #[task(binds = COMP_LPCOMP, priority = 5, local = [comp], shared=[rtt_hw])]
    fn comp_lcomp(cx: comp_lcomp::Context) {
        backup_mode::comp_lcomp(cx);
    }

    #[task(binds=USBD, priority = 4, shared = [usb_dev, serial, rtt_hw])]
    fn usb_fs(cx: usb_fs::Context) {
        cli::usb_fs(cx);
    }

    #[task(shared = [gpiote, rtt_hw, qdec], priority = 5)]
    fn rotary_disable_interrupts(cx: rotary_disable_interrupts::Context) {
        rotary_encoder::disable_interrupts(cx);
    }

    #[task(shared = [gpiote, rtt_hw, qdec], priority = 3)]
    fn rotary_encoder_enable_interrupts(cx: rotary_encoder_enable_interrupts::Context) {
        rotary_encoder::enable_interrupts(cx);
    }

    #[task(priority = 3, shared = [rtc, &time_offset_ticks])]
    fn set_time(cx: set_time::Context, ticks: u32) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Setting time, ticks: {}", ticks);
        rtc::set_time(cx, ticks);
    }

    #[task(priority = 3, shared = [rtc, &alarm_offset_ticks, &time_offset_ticks])]
    fn set_alarm(cx: set_alarm::Context, ticks: u32) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Setting alarm, ticks: {}", ticks);
        rtc::set_alarm(cx, ticks);
    }

    #[task(priority = 5, shared = [rtc])]
    fn disable_alarm(cx: disable_alarm::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Disabling alarm");
        rtc::disable_alarm(cx);
    }

    #[task(priority = 1, shared = [rtc])]
    fn set_periodic_update(cx: set_periodic_update::Context, interval_minutes: u32) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Setting periodic update, ticks: {}", interval_minutes);
        rtc::set_periodic_update(cx, interval_minutes);
    }

    #[task(priority = 5, shared = [rtc])]
    fn disable_periodic_update(cx: disable_periodic_update::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Disabling periodic update");
        rtc::disable_periodic_update(cx);
    }

    #[task(priority = 3, shared = [rtc, &time_offset_ticks])]
    fn set_timeout(cx: set_timeout::Context, ticks: u32) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Setting timeout, ticks: {}", ticks);
        rtc::set_timeout(cx, ticks);
    }

    #[task(priority = 3, shared = [rtc])]
    fn disable_timeout(cx: disable_timeout::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Disabling timeout");
        rtc::disable_timeout(cx);
    }

    #[task(priority = 3, shared = [rtc])]
    fn set_blinking(cx: set_blinking::Context, interval_ticks: u32) {
        #[cfg(feature = "52833-debug")]
        rprintln!("Setting blinking, interval_ticks: {}", interval_ticks);
        rtc::set_blinking(cx, interval_ticks);
    }

    #[task(priority = 5, shared = [rtc])]
    fn disable_blinking(cx: disable_blinking::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("disable_blinking");
        rtc::disable_blinking(cx);
    }

    #[task(priority = 3, local = [saadc, saadc_pin], shared = [temperature])]
    fn read_temperature(cx: read_temperature::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("read_temperature");
        thermistor::read(cx);
    }

    #[task(priority = 3, shared = [pwm])]
    fn load_pwm_sequence(cx: load_pwm_sequence::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("load_pwm_sequence");
        pwm::load_pwm_sequence(cx);
    }

    #[task(priority = 3, shared = [pwm])]
    fn start_pwm(cx: start_pwm::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("start_pwm");
        pwm::start(cx);
    }

    #[task(priority = 3, shared = [pwm])]
    fn stop_pwm(cx: stop_pwm::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("stop_pwm");
        pwm::stop(cx);
    }

    #[task(priority = 5, shared = [display, temperature], local = [on: bool = true, rtt_display])]
    fn update_display(
        cx: update_display::Context,
        ticks: u32,
        section: display::Section,
        blink: bool,
    ) {
        #[cfg(feature = "52833-debug")]
        rprintln!("update_display");
        //display::update_display_rtt(cx, ticks, section, blink);
        display::update_display(cx, ticks, section, blink);
    }

    #[task(priority = 3, shared = [display])]
    fn enable_display(cx: enable_display::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("enable_display");
        display::enable_display(cx);
    }

    #[task(priority = 5, shared = [display])]
    fn disable_display(cx: disable_display::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("disable_display");
        display::disable_display(cx);
    }

    #[task(priority = 3, shared = [amp_fan_hum_pin])]
    fn turn_on_amp_fan_hum(cx: turn_on_amp_fan_hum::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("turn_on_amp_fan_hum");
        gpio::turn_on_amp_fan_hum(cx);
    }

    #[task(priority = 5, shared = [amp_fan_hum_pin])]
    fn turn_off_amp_fan_hum(cx: turn_off_amp_fan_hum::Context) {
        #[cfg(feature = "52833-debug")]
        rprintln!("turn_off_amp_fan_hum");
        gpio::turn_off_amp_fan_hum(cx);
    }

    #[task(priority = 4, shared = [usb_dev, serial, rtt_serial])]
    fn data_out(cx: data_out::Context, data: [u8; DATA_OUT_BUFFER_SIZE], len: usize) {
        #[cfg(feature = "52833-debug")]
        rprintln!("data_out");
        cli::data_out(cx, data, len);
    }
    #[task(priority = 3, capacity = 10, local = [len: usize = 0, data_arr :[u8; DATA_IN_BUFFER_SIZE] = [0; DATA_IN_BUFFER_SIZE]], shared = [rtt_serial])]
    fn data_in(cx: data_in::Context, data: u8){
        #[cfg(feature = "52833-debug")]
        rprintln!("data_in");
        cli::data_in(cx, data);
    }

    #[task(priority = 3, shared = [rtt_serial])]
    fn cli_commands(cx: cli_commands::Context, command: CliCommand) {
        #[cfg(feature = "52833-debug")]
        rprintln!("cli_commands");
        cli::cli_commands(cx, command);
    }

    #[task(priority = 1, shared = [&amp_on], local = [i2s, dma_buf, segment_index: u32 = 0, rtt_speaker])]
    fn play_next_audio_segment(cx: play_next_audio_segment::Context) {
        speaker::next_segment(cx);
    }

    fn disable_alarm_components(cx: &state_machine::Context) {
        cx.shared.amp_on.store(false, Ordering::Relaxed);
        turn_off_amp_fan_hum::spawn().ok();
        stop_pwm::spawn().ok();
        disable_blinking::spawn().ok();
    }
}
