#![no_main]
#![no_std]
#![deny(warnings)]

use {
    cortex_m::asm,
    nrf52833_hal as hal,
    panic_rtt_target as _,
    rtt_target::rprintln,
};

const WAV_RAW: &[u8] = include_bytes!("SeaBreeze2.wav");

const WAV_HEADER_SIZE: usize = 44;
const SEGMENT_SIZE: usize = 1024;
const BUFFER_LEN: usize = SEGMENT_SIZE / 4;

#[link_section = ".data"]
static mut AUDIO_SAMPLE_BUFFER: [u32; BUFFER_LEN] = [0; BUFFER_LEN];

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;
    use hal::gpio::{Level, Output, Pin, PushPull};
    use hal::i2s::{SampleWidth, Format, Channels, Ratio, MckFreq};
    use embedded_hal::digital::v2::OutputPin;

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mono = systick_monotonic::Systick::new(cx.core.SYST, 64_000_000);
        (Shared {}, Local {}, init::Monotonics(mono))
    }

    #[idle]
    #[allow(static_mut_refs)]
    fn idle(_cx: idle::Context) -> ! {
        let pcm_raw = &WAV_RAW[WAV_HEADER_SIZE..];
        let num_segments = pcm_raw.len() / SEGMENT_SIZE;

        let dp = unsafe { hal::pac::Peripherals::steal() };

        // Configure Port0 and pins.
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

        // Initialize I2S.
        let mut i2s = hal::i2s::I2S::new(dp.I2S, pins);
        i2s.set_tx_enabled(true);
        i2s.set_sample_width(SampleWidth::_16bit);
        i2s.set_format(Format::I2S);
        i2s.set_channels(Channels::Stereo);
        i2s.set_mck_frequency(MckFreq::_32MDiv10);
        i2s.set_ratio(Ratio::_64x);
        i2s.enable();

        rprintln!("I2S initialized");
        rprintln!("Starting WAV stream");

        let mut segment_index: usize = 0;
        loop {
            if segment_index >= num_segments {
                segment_index = 0;
                rprintln!("Restarting playback");
            }
            let start = segment_index * SEGMENT_SIZE;
            let end = start + SEGMENT_SIZE;
            let segment = &pcm_raw[start..end];

            // Copy the current segment into our audio sample buffer.
            unsafe {
                for (i, chunk) in segment.chunks(4).enumerate().take(BUFFER_LEN) {
                    AUDIO_SAMPLE_BUFFER[i] = u32::from_le_bytes(chunk.try_into().unwrap());
                }
            }

            // Set up I2S DMA buffer pointers.
            unsafe {
                i2s.set_buffersize(BUFFER_LEN as u32).unwrap();
                let ptr = AUDIO_SAMPLE_BUFFER.as_ptr() as u32;
                i2s.set_tx_ptr(ptr).unwrap();
            }
            let tx_buf: &'static [u32] = unsafe {
                core::slice::from_raw_parts(AUDIO_SAMPLE_BUFFER.as_ptr(), BUFFER_LEN)
            };

            i2s.start();

            let transfer = i2s.tx(tx_buf)
                .expect("I2S TX transfer failed");

            let (_returned_buffer, new_i2s) = transfer.wait();
            i2s = new_i2s;

            rprintln!("I2S completed segment {}", segment_index);
            segment_index += 1;
            asm::wfi();
        }
    }
}
