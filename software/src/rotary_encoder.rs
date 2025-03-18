use {
    crate::{
        app::*,
        state_machine::*
    }, 
    hal::{
        gpio::{Input, Pin, PullUp},
        gpiote::*, 
        pac::{GPIOTE, QDEC}, 
        qdec::*
    }, 
    nrf52833_hal::{self as hal}, 
};

const ROTARY_ENCODER_THRESHOLD_SEC: f32 = 0.1;
const LONG_PRESS_THRESHOLD_SEC: f32 = 1.0;
const DEBOUNCE_THRESHOLD_SEC: f32 = 0.1;

pub(crate) fn init(qdec: QDEC, gpiote: GPIOTE, rotation_pins: Pins, switch_pin: Pin<Input<PullUp>>) -> (Qdec, Gpiote) {
    let qdec = Qdec::new(qdec, rotation_pins, SamplePeriod::_2048us);
    qdec.enable_interrupt(NumSamples::_1smpl)
    .debounce(true)
    .enable();

    let gpiote = Gpiote::new(gpiote);
    gpiote.channel0().input_pin(&switch_pin)
        .hi_to_lo()
        .enable_interrupt();
    gpiote.channel1().input_pin(&switch_pin)
        .lo_to_hi()
        .enable_interrupt();

    (qdec, gpiote)
}

pub(crate) fn handle_qdec_interrupt(cx: qdec_interrupt::Context) {
    let qdec = cx.local.qdec;
    qdec.reset_events();
    let direction = -qdec.read(); // Inverted direction

    let now = cortex_m::peripheral::DWT::cycle_count();
    let elapsed_cycles = now.wrapping_sub(*cx.local.compare_cycle);
    let elapsed_time = elapsed_cycles as f32 / 64_000_000.0;

    // Filter out debounce noise
    if !(elapsed_time <= ROTARY_ENCODER_THRESHOLD_SEC) {
        *cx.local.compare_cycle = now;

        state_machine::spawn(Event::Encoder(EncoderEvent::Rotated(direction as isize))).ok();
    }
}

pub(crate) fn handle_gpiote_interrupt(cx: gpiote_interrupt::Context) {
    let gpiote = cx.local.gpiote;

    let now = cortex_m::peripheral::DWT::cycle_count();
    let elapsed_cycles = now.wrapping_sub(*cx.local.last_press);
    let elapsed_time = elapsed_cycles as f32 / 64_000_000.0;

    // Filter out debounce noise
    if elapsed_time <= DEBOUNCE_THRESHOLD_SEC {
        return;
    }

    // Press event
    if gpiote.channel0().is_event_triggered() {
        gpiote.channel0().reset_events();
        *cx.local.last_press = now;
    }
    // Release event
    if gpiote.channel1().is_event_triggered() {
        gpiote.channel1().reset_events();
        
        match elapsed_time > LONG_PRESS_THRESHOLD_SEC {
            true => state_machine::spawn(Event::Encoder(EncoderEvent::LongPressed)).ok(),
            false => state_machine::spawn(Event::Encoder(EncoderEvent::ShortPressed)).ok(),
        };
    }
}