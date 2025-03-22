#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Idle,
    Alarm,
    Settings(Settings),
    BackupBattery,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Settings {
    ClockHours,
    ClockMinutes,
    AlarmHours,
    AlarmMinutes,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Encoder(EncoderEvent),
    ResetButton,
    Timer(TimerEvent),
    VBUSDisconnected,
    VBUSConnected,
}

#[derive(Clone, Copy, Debug)]
pub enum TimerEvent {
    PeriodicUpdate(u32), // Event contains RTC counter value
    AlarmTriggered,
    Timeout, // General timeout used for timing out settings/alarm
    Blink,   // Used for blinking the alarm and settings display
}

#[derive(Clone, Copy, Debug)]
pub enum EncoderEvent {
    Rotated(isize),
    ShortPressed,
    LongPressed,
}

pub trait StateMachine {
    fn next(&self, event: Event) -> State;
}

impl StateMachine for State {
    fn next(&self, event: Event) -> State {
        match self {
            State::Idle => match event {
                Event::Encoder(EncoderEvent::ShortPressed) => State::Settings(Settings::AlarmHours),
                Event::Encoder(EncoderEvent::LongPressed) => State::Settings(Settings::ClockHours),
                Event::VBUSDisconnected => State::BackupBattery,
                Event::Timer(TimerEvent::AlarmTriggered) => State::Alarm,
                _ => State::Idle,
            },

            State::Alarm => match event {
                Event::Encoder(encoder_event) => match encoder_event {
                    EncoderEvent::ShortPressed => State::Idle,
                    EncoderEvent::LongPressed => State::Idle,
                    _ => State::Alarm,
                },
                Event::Timer(TimerEvent::PeriodicUpdate(_)) => State::Alarm,
                Event::Timer(TimerEvent::Timeout) => State::Idle,
                _ => State::Alarm,
            },

            State::Settings(settings) => match event {
                Event::Encoder(EncoderEvent::Rotated(_)) => match settings {
                    Settings::ClockHours => State::Settings(Settings::ClockHours),
                    Settings::ClockMinutes => State::Settings(Settings::ClockMinutes),
                    Settings::AlarmHours => State::Settings(Settings::AlarmHours),
                    Settings::AlarmMinutes => State::Settings(Settings::AlarmMinutes),
                },
                Event::Encoder(EncoderEvent::ShortPressed) => match settings {
                    Settings::ClockHours => State::Settings(Settings::ClockMinutes),
                    Settings::ClockMinutes => State::Idle,
                    Settings::AlarmHours => State::Settings(Settings::AlarmMinutes),
                    Settings::AlarmMinutes => State::Idle,
                },
                Event::Timer(TimerEvent::Timeout) => State::Idle,
                _ => State::Settings(*settings),
            },

            State::BackupBattery => match event {
                Event::VBUSConnected => State::Idle,
                _ => State::BackupBattery,
            },
        }
    }
}