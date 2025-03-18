use {
    nrf52833_hal as hal, 
    hal::gpio::{
        p0::{P0_03, Parts as P0Parts},
        p1::Parts as P1Parts,
        Disconnected, 
        Input, 
        Output, 
        Pin, 
        PullUp, 
        PushPull,
        Level},
    hal::pac::{P0, P1},
};

pub(crate) struct  Pins {
    pub(crate) led: Pin<Output<PushPull>>,
    pub(crate) amp_fan_hum: Pin<Output<PushPull>>,
    pub(crate) haptic: Pin<Output<PushPull>>,
    pub(crate) rotary_encoder: hal::qdec::Pins,
    pub(crate) rotary_switch: Pin<Input<PullUp>>,
    pub(crate) oled: hal::twim::Pins,
    pub(crate) saadc: P0_03<Disconnected>,
}

pub(crate) fn init(p0: P0, p1: P1) -> Pins {
    let port0 = P0Parts::new(p0);
    let port1 = P1Parts::new(p1);

    let led = port0.p0_09.into_push_pull_output(Level::Low).degrade();
    let amp_fan_hum = port0.p0_10.into_push_pull_output(Level::Low).degrade();
    let haptic = port0.p0_20.into_push_pull_output(Level::Low).degrade();
    let rotary_encoder = hal::qdec::Pins {
        a: port0.p0_30.into_pullup_input().degrade(),
        b: port0.p0_29.into_pullup_input().degrade(),
        led: None
    };
    let rotary_switch = port0.p0_28.into_pullup_input().degrade();
    let oled = hal::twim::Pins {
        scl: port0.p0_11.into_floating_input().degrade(),
        sda: port1.p1_09.into_floating_input().degrade(),
    };
    let saadc = port0.p0_03;

    Pins {
        led,
        amp_fan_hum,
        haptic,
        rotary_encoder,
        rotary_switch,
        oled,
        saadc,
    }
}