#![no_main]
#![no_std]
//#![deny(warnings)]

use cortex_m::singleton;
use nrf52833_hal as hal;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

const WAV_RAW: &[u8] = include_bytes!("../assets/SeaBreeze3.wav");
const WAV_HEADER_SIZE: usize = 44;
const SEGMENT_SIZE: usize = 4;
const BUFFER_LEN: usize = SEGMENT_SIZE / 4;

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;
    use core::convert::TryInto;
    use embedded_hal::digital::v2::OutputPin;
    use hal::gpio::{Level, Output, Pin, PushPull};
    use hal::i2s::{SampleWidth, Format, Channels, Ratio, MckFreq};

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    #[shared]
    struct Shared {
        segment_index: usize,
    }

    #[local]
    struct Local {
        i2s: Option<hal::i2s::I2S>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        
        let mono = systick_monotonic::Systick::new(cx.core.SYST, 64_000_000);
        let dp = cx.device;

        let dma_buf: &'static mut [u32; BUFFER_LEN] =
            singleton!(: [u32; BUFFER_LEN] = [0; BUFFER_LEN]).unwrap();

        let port0 = hal::gpio::p0::Parts::new(dp.P0);
        let mut sd_mode: Pin<Output<PushPull>> =
            port0.p0_10.into_push_pull_output(Level::Low).degrade();
        sd_mode.set_high().unwrap();

        let bclk: Pin<Output<PushPull>> =
            port0.p0_04.into_push_pull_output(Level::Low).degrade();
        let lrclk: Pin<Output<PushPull>> =
            port0.p0_05.into_push_pull_output(Level::Low).degrade();
        let din: Pin<Output<PushPull>> =
            port0.p0_31.into_push_pull_output(Level::Low).degrade();

        let pins = hal::i2s::Pins::Controller {
            mck: None,
            sck: bclk,
            lrck: lrclk,
            sdin: None,
            sdout: Some(din),
        };

        let i2s = hal::i2s::I2S::new(dp.I2S, pins);
        i2s.set_tx_enabled(true);
        i2s.set_sample_width(SampleWidth::_16bit);
        i2s.set_format(Format::I2S);
        i2s.set_channels(Channels::Stereo);
        i2s.set_mck_frequency(MckFreq::_32MDiv10);
        i2s.set_ratio(Ratio::_64x);
        i2s.enable();

        rprintln!("I2S initialized");
        rprintln!("Starting WAV stream");

        let shared = Shared { segment_index: 0 };

        segment_tx::spawn(dma_buf).unwrap();

        (shared, Local { i2s: Some(i2s) }, init::Monotonics(mono))
    }

    #[task(local = [i2s], shared = [segment_index])]
    fn segment_tx(mut cx: segment_tx::Context, dma_buf: &'static mut [u32; BUFFER_LEN]) {
        let pcm_raw = &WAV_RAW[WAV_HEADER_SIZE..];
        let num_segments = pcm_raw.len() / SEGMENT_SIZE;

        let seg_index = cx.shared.segment_index.lock(|segment_index| {
            let cur = *segment_index;
            *segment_index = if cur + 1 >= num_segments { 0 } else { cur + 1 };
            cur
        });

        rprintln!("Playing segment {}", seg_index);

        let start = seg_index * SEGMENT_SIZE;
        let end = start + SEGMENT_SIZE;
        let segment = &pcm_raw[start..end];

        for (i, chunk) in segment.chunks(4).enumerate().take(BUFFER_LEN) {
            dma_buf[i] = u32::from_le_bytes(chunk.try_into().unwrap());
        }

        let i2s = cx.local.i2s.take().unwrap();

        unsafe {
            i2s.set_buffersize(BUFFER_LEN as u32).unwrap();
            let ptr = dma_buf.as_ptr() as u32;
            i2s.set_tx_ptr(ptr).unwrap();
        }

        i2s.start();
        let tx_buf: &'static [u32] = unsafe {
            core::slice::from_raw_parts(dma_buf.as_ptr(), BUFFER_LEN)
        };
        let transfer = i2s.tx(tx_buf).expect("I2S TX transfer failed");

        let (_returned_buffer, new_i2s) = transfer.wait();

        rprintln!("Completed segment {}", seg_index);

        *cx.local.i2s = Some(new_i2s);

        segment_tx::spawn(dma_buf).unwrap();
    }
}