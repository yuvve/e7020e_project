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
    AlarmDetected,
    Encoder(EncoderEvent),
    ResetButton,
    Timeout,
    VBUSDisconnected,
    VBUSConnected,
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
                Event::AlarmDetected => State::Alarm,
                _ => State::Idle,
            }

            State::Alarm => match event {
                Event::Encoder(encoder_event) => {
                    match encoder_event {
                        EncoderEvent::Pressed => State::Idle,
                        _ => State::Alarm,
                    }
                }
                Event::Timeout => State::Idle,
                _ => State::Alarm,
            }

            State::Settings(settings) => match event {
                Event::Encoder(EncoderEvent::Rotated(steps)) => match settings {
                    Settings::ClockHours => State::Settings(Settings::ClockHours(steps)),
                    Settings::ClockMinutes => State::Settings(Settings::ClockMinutes(steps)),
                    Settings::AlarmHours => State::Settings(Settings::AlarmHours(steps)),
                    Settings::AlarmMinutes => State::Settings(Settings::AlarmMinutes(steps)),
                }
                Event::Encoder(EncoderEvent::Pressed) => match settings {
                    Settings::ClockHours => State::Settings(Settings::ClockMinutes(0)),
                    Settings::ClockMinutes => State::Settings(Settings::AlarmHours(0)),
                    Settings::AlarmHours => State::Settings(Settings::AlarmMinutes(0)),
                    Settings::AlarmMinutes => State::Idle,
                }
                Event::Timeout => State::Idle,
                _ => State::Settings(*settings),
            }

            State::BackupBattery => match event {
                Event::VBUSConnected => State::Idle,
                _ => State::BackupBattery,
            }

        }
    }
}