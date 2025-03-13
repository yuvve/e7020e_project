#![no_main]
#![no_std]
#![deny(warnings)]

use {
    cortex_m::asm,
    hal::gpio::{Level, Output, Pin, PushPull},
    nrf52833_hal as hal,
    panic_rtt_target as _,
    rtt_target::{rprintln, rtt_init_print},
    systick_monotonic::*
};

const TIMER_HZ: u32 = 1000;

const WAV_RAW: &[u8] = include_bytes!("SeaBreeze.pcm");

const SEGMENT_SIZE: usize = 1024;
const NUM_SEGMENTS: usize = WAV_RAW.len() / SEGMENT_SIZE;

#[link_section = ".data"]
static mut AUDIO_SAMPLE_BUFFER: [u32; SEGMENT_SIZE / 4] = [0; SEGMENT_SIZE / 4];

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;
    use embedded_hal::digital::v2::OutputPin;
    use nrf52833_hal::i2s::{SampleWidth, Format, Channels, Ratio};

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<TIMER_HZ>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        i2s: hal::i2s::I2S,
        segment_index: usize,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = Systick::new(cx.core.SYST, 64_000_000);

        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);

        let mut sd_mode: Pin<Output<PushPull>> = port0.p0_10.into_push_pull_output(Level::Low).degrade();
        sd_mode.set_high().unwrap();

        let bclk: Pin<Output<PushPull>> = port0.p0_04.into_push_pull_output(Level::Low).degrade();
        let lrclk: Pin<Output<PushPull>> = port0.p0_05.into_push_pull_output(Level::Low).degrade();
        let din: Pin<Output<PushPull>> = port0.p0_31.into_push_pull_output(Level::Low).degrade();

        let pins = hal::i2s::Pins::Controller {
            mck: None, 
            sck: bclk,
            lrck: lrclk,
            sdin: None,
            sdout: Some(din),
        };

        let i2s = hal::i2s::I2S::new(cx.device.I2S, pins);

        i2s.set_tx_enabled(true);
        i2s.set_sample_width(SampleWidth::_16bit);
        i2s.set_format(Format::I2S);
        i2s.set_channels(Channels::Stereo);
        i2s.set_ratio(Ratio::_48x);

        i2s.enable();

        (Shared {}, Local { i2s, segment_index: 0 }, init::Monotonics(mono))
    }

    #[idle(local = [i2s, segment_index])]
    fn idle(cx: idle::Context) -> ! {
        rtt_init_print!();
        rprintln!("Starting transfer");

        loop {
            if *cx.local.segment_index < NUM_SEGMENTS {
                let start = *cx.local.segment_index * SEGMENT_SIZE;
                let end = start + SEGMENT_SIZE;
                let segment = &WAV_RAW[start..end];

                unsafe {
                    for (i, chunk) in segment.chunks(4).enumerate().take(AUDIO_SAMPLE_BUFFER.len()) {
                        AUDIO_SAMPLE_BUFFER[i] = u32::from_le_bytes(chunk.try_into().unwrap());
                    }
                }

                let i2s = &cx.local.i2s;

                unsafe { i2s.set_buffersize(AUDIO_SAMPLE_BUFFER.len() as u32).unwrap(); }

                unsafe {
                    let ptr = AUDIO_SAMPLE_BUFFER.as_ptr() as u32;
                    i2s.set_tx_ptr(ptr).unwrap();
                }

                unsafe {i2s.tx(&AUDIO_SAMPLE_BUFFER);}
                i2s.start();
                rprintln!("I2S started for segment {}", cx.local.segment_index);    
                
                *cx.local.segment_index += 1;
            } else {
                rprintln!("All segments transmitted.");
            }
            asm::wfi();
        }
    }
}
