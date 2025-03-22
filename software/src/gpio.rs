use {
    crate::app::*, embedded_hal::digital::v2::OutputPin, hal::{gpio::{
        p0::{Parts as P0Parts, P0_02, P0_03}, p1::Parts as P1Parts, Disconnected, Floating, Input, Level, Output, Pin, PullUp, PushPull
    }, pac::{P0, P1}}, nrf52833_hal::{self as hal}, rtic::Mutex
};

pub(crate) struct Pins {
    pub(crate) led: Pin<Output<PushPull>>,
    pub(crate) amp_fan_hum: Pin<Output<PushPull>>,
    pub(crate) haptic: Pin<Output<PushPull>>,
    pub(crate) rotary_encoder: hal::qdec::Pins,
    pub(crate) rotary_switch: Pin<Input<PullUp>>,
    pub(crate) oled: hal::twim::Pins,
    pub(crate) saadc: P0_03<Disconnected>,
    pub(crate) vdetect: P0_02<Input<Floating>>,
    pub(crate) speaker: hal::i2s::Pins,
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
        led: None,
    };
    let rotary_switch = port0.p0_28.into_pullup_input().degrade();
    let oled = hal::twim::Pins {
        scl: port0.p0_11.into_floating_input().degrade(),
        sda: port1.p1_09.into_floating_input().degrade(),
    };
    let saadc = port0.p0_03;

    let vdetect = port0.p0_02.into_floating_input();

    let bclk: Pin<Output<PushPull>> =
        port0.p0_04.into_push_pull_output(Level::Low).degrade();
    let lrclk: Pin<Output<PushPull>> =
        port0.p0_05.into_push_pull_output(Level::Low).degrade();
    let din: Pin<Output<PushPull>> =
        port0.p0_31.into_push_pull_output(Level::Low).degrade();

    let speaker = hal::i2s::Pins::Controller {
        mck: None,
        sck: bclk,
        lrck: lrclk,
        sdin: None,
        sdout: Some(din),
    };

    Pins {
        led,
        amp_fan_hum,
        haptic,
        rotary_encoder,
        rotary_switch,
        oled,
        saadc,
        vdetect,
        speaker,
    }
}

pub(crate) fn turn_on_amp_fan_hum(mut cx: turn_on_amp_fan_hum::Context) {
    cx.shared.amp_fan_hum_pin.lock(|amp_fan_hum_pin| {
        amp_fan_hum_pin.set_high().ok();
    });
}

pub(crate) fn turn_off_amp_fan_hum(mut cx: turn_off_amp_fan_hum::Context) {
    cx.shared.amp_fan_hum_pin.lock(|amp_fan_hum_pin| {
        amp_fan_hum_pin.set_low().ok();
    });
}
