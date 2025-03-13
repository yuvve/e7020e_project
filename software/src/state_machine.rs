#![allow(dead_code)]

#[derive(Clone, Copy)]
pub enum State {
    Idle,
    Alarm,
    Settings(Settings),
    BackupBattery,
}

#[derive(Clone, Copy)]
pub enum Settings {
    ClockHours(usize),
    ClockMinutes(usize),
    AlarmHours(usize),
    AlarmMinutes(usize),
}

#[derive(Clone, Copy)]
pub enum Event {
    Encoder(EncoderEvent),
    ResetButton,
    TimerEvent(TimerEvent),
    VBUSDisconnected,
    VBUSConnected,
}

#[derive(Clone, Copy)]
pub enum TimerEvent {
    PeriodicUpdate,
    AlarmTriggered,
    Timeout,    // General timeout used for timing out settings/alarm
}

#[derive(Clone, Copy)]
pub enum EncoderEvent {
    Rotated(usize),
    Pressed,
}

pub trait StateMachine {
    fn next(&self, event: Event) -> State;
}

impl StateMachine for State {
    fn next(&self, event: Event) -> State {
        match self {
            State::Idle => match event {
                Event::Encoder(EncoderEvent::Pressed) => State::Settings(Settings::ClockHours(0)),
                Event::VBUSDisconnected => State::BackupBattery,
                Event::TimerEvent(TimerEvent::AlarmTriggered) => State::Alarm,
                Event::TimerEvent(TimerEvent::PeriodicUpdate) => State::Idle,
                _ => State::Idle,
            }

            State::Alarm => match event {
                Event::Encoder(encoder_event) => {
                    match encoder_event {
                        EncoderEvent::Pressed => State::Idle,
                        _ => State::Alarm,
                    }
                }
                Event::TimerEvent(TimerEvent::PeriodicUpdate) => State::Alarm,
                Event::TimerEvent(TimerEvent::Timeout) => State::Idle,
                _ => State::Alarm,
            }

            State::Settings(settings) => match event {
                Event::Encoder(EncoderEvent::Rotated(steps)) => match settings {
                    Settings::ClockHours(_) => State::Settings(Settings::ClockHours(steps)),
                    Settings::ClockMinutes(_) => State::Settings(Settings::ClockMinutes(steps)),
                    Settings::AlarmHours(_) => State::Settings(Settings::AlarmHours(steps)),
                    Settings::AlarmMinutes(_) => State::Settings(Settings::AlarmMinutes(steps)),
                }
                Event::Encoder(EncoderEvent::Pressed) => match settings {
                    Settings::ClockHours(_) => State::Settings(Settings::ClockMinutes(0)),
                    Settings::ClockMinutes(_) => State::Settings(Settings::AlarmHours(0)),
                    Settings::AlarmHours(_) => State::Settings(Settings::AlarmMinutes(0)),
                    Settings::AlarmMinutes(_) => State::Idle,
                }
                Event::TimerEvent(TimerEvent::Timeout) => State::Idle,
                _ => State::Settings(*settings),
            }

            State::BackupBattery => match event {
                Event::VBUSConnected => State::Idle,
                _ => State::BackupBattery,
            }

        }
    }
}