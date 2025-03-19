#![no_main]
#![no_std]
#![deny(warnings)]

use cortex_m::asm;
use cortex_m::singleton;
use nrf52833_hal as hal;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

const SEGMENT_SIZE: usize = 1024;
const BUFFER_LEN: usize = SEGMENT_SIZE / 4;
const SAMPLE_RATE: f32 = 16000.0;
const TWO_PI: f32 = 6.283185307179586;
const MIN_FREQUENCY: f32 = 500.0;
const MAX_FREQUENCY: f32 = 8000.0;
const SWEEP_STEP: f32 = (MAX_FREQUENCY - MIN_FREQUENCY) / 8000.0;

#[rtic::app(device = nrf52833_hal::pac, dispatchers = [TIMER0])]
mod app {
    use super::*;
    use embedded_hal::digital::v2::OutputPin;
    use hal::gpio::{Level, Output, Pin, PushPull};
    use hal::i2s::{SampleWidth, Format, Channels, Ratio, MckFreq};

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = systick_monotonic::Systick<1000>;

    #[shared]
    struct Shared {
        sweep: SweepData,
    }

    pub struct SweepData {
        pub phase: f32,
        pub frequency: f32,
        pub direction: f32,
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
        rprintln!("Starting frequency sweep sine wave stream");

        let shared = Shared {
            sweep: SweepData {
                phase: 0.0,
                frequency: MIN_FREQUENCY,
                direction: 1.0,
            },
        };

        segment_tx::spawn(dma_buf).unwrap();

        (shared, Local { i2s: Some(i2s) }, init::Monotonics(mono))
    }

    #[task(local = [i2s], shared = [sweep])]
    fn segment_tx(mut cx: segment_tx::Context, dma_buf: &'static mut [u32; BUFFER_LEN]) {
        cx.shared.sweep.lock(|sweep| {
            for sample in dma_buf.iter_mut() {
                let phase_increment = TWO_PI * sweep.frequency / SAMPLE_RATE;
                let sample_val = libm::sinf(sweep.phase) * 30000.0;
                let sample_i16 = sample_val as i16;
                let packed: u32 =
                    (sample_i16 as u16 as u32) | ((sample_i16 as u16 as u32) << 16);
                *sample = packed;

                sweep.phase += phase_increment;
                if sweep.phase >= TWO_PI {
                    sweep.phase -= TWO_PI;
                }

                sweep.frequency += sweep.direction * SWEEP_STEP;
                if sweep.frequency >= MAX_FREQUENCY {
                    sweep.frequency = MAX_FREQUENCY;
                    sweep.direction = -1.0;
                } else if sweep.frequency <= MIN_FREQUENCY {
                    sweep.frequency = MIN_FREQUENCY;
                    sweep.direction = 1.0;
                }
            }
        });

        rprintln!("Playing sine wave segment with frequency sweep");

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

        rprintln!("Completed sine wave segment");
        *cx.local.i2s = Some(new_i2s);
        segment_tx::spawn(dma_buf).unwrap();
        asm::wfi();
    }
}
