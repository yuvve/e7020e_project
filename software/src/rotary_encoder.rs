use {
    crate::{app::*, state_machine::*},
    rtic::Mutex,
    hal::{
        gpio::{Input, Pin, PullUp},
        gpiote::*,
        pac::{GPIOTE, QDEC},
        qdec::*,
    },
    nrf52833_hal as hal,
};

#[cfg(feature = "52833-debug")]
use core::fmt::Write;

const ROTARY_ENCODER_THRESHOLD_SEC: f32 = 0.1;
const LONG_PRESS_THRESHOLD_SEC: f32 = 0.5;
const DEBOUNCE_THRESHOLD_SEC: f32 = 0.1;

pub(crate) fn init(
    qdec: QDEC,
    gpiote: GPIOTE,
    rotation_pins: Pins,
    switch_pin: Pin<Input<PullUp>>,
) -> (Qdec, Gpiote) {
    let qdec = Qdec::new(qdec, rotation_pins, SamplePeriod::_2048us);
    qdec.enable_interrupt(NumSamples::_1smpl)
        .debounce(true)
        .enable();

    let gpiote = Gpiote::new(gpiote);
    gpiote
        .channel0()
        .input_pin(&switch_pin)
        .hi_to_lo()
        .enable_interrupt();
    gpiote
        .channel1()
        .input_pin(&switch_pin)
        .lo_to_hi()
        .enable_interrupt();

    (qdec, gpiote)
}

pub(crate) fn handle_qdec_interrupt(mut cx: qdec_interrupt::Context) {
    #[cfg(feature = "52833-debug")]
    cx.shared.rtt_hw.lock(|rtt_hw| {
        writeln!(rtt_hw, "QDEC interrupt").ok();
    });
    let direction = cx.shared.qdec.lock(|qdec| {
        qdec.reset_events();
        -qdec.read() // Inverted direction
    });

    let now = cortex_m::peripheral::DWT::cycle_count();
    let elapsed_cycles = now.wrapping_sub(*cx.local.last_rotation);
    let elapsed_time = elapsed_cycles as f32 / 64_000_000.0;

    // Filter out debounce noise
    if !(elapsed_time <= ROTARY_ENCODER_THRESHOLD_SEC) {
        *cx.local.last_rotation = now;

        let direction = match direction > 0 {
            true => 1,
            false => -1,
        };
        state_machine::spawn(Event::Encoder(EncoderEvent::Rotated(direction as isize))).ok();
    }
}

pub(crate) fn handle_gpiote_interrupt(mut cx: gpiote_interrupt::Context) {
    #[cfg(feature = "52833-debug")]
    cx.shared.rtt_hw.lock(|rtt_hw| {
        writeln!(rtt_hw, "GPIOTE interrupt").ok();
    });
    let now = cortex_m::peripheral::DWT::cycle_count();
    let elapsed_cycles = now.wrapping_sub(*cx.local.last_press);
    let elapsed_time = elapsed_cycles as f32 / 64_000_000.0;

    // Filter out debounce noise
    if elapsed_time <= DEBOUNCE_THRESHOLD_SEC {
        return;
    }

    // Press event
    cx.shared.gpiote.lock(|gpiote| {
    if gpiote.channel0().is_event_triggered() {
        gpiote.channel0().reset_events();
        *cx.local.last_press = now;
    }});
    // Release event
    cx.shared.gpiote.lock(|gpiote| {
    if gpiote.channel1().is_event_triggered() {
        gpiote.channel1().reset_events();

        match elapsed_time > LONG_PRESS_THRESHOLD_SEC {
            true => state_machine::spawn(Event::Encoder(EncoderEvent::LongPressed)).ok(),
            false => state_machine::spawn(Event::Encoder(EncoderEvent::ShortPressed)).ok(),
        };
    }});
}

pub(crate) fn disable_interrupts(mut cx: rotary_disable_interrupts::Context) {
    cx.shared.gpiote.lock(|gpiote| {
        gpiote.port().disable_interrupt();
    }); 
    cx.shared.qdec.lock(|qdec| {
        qdec.disable_interrupt();
    });
}

pub(crate) fn enable_interrupts(mut cx: rotary_encoder_enable_interrupts::Context) {
    cx.shared.gpiote.lock(|gpiote| {
        gpiote.port().enable_interrupt();
    }); 
    cx.shared.qdec.lock(|qdec| {
        qdec.enable_interrupt(NumSamples::_1smpl)
            .debounce(true)
            .enable();
    });
}