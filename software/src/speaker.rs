use {
    crate::app::*,
    core::sync::atomic::Ordering,
    hal::{
        i2s::{Channels, Format, MckFreq, Pins, Ratio, SampleWidth},
        pac::I2S,
    },
    nrf52833_hal as hal,
};

#[cfg(feature = "52833-debug")]
use core::fmt::Write;

const WAV_RAW: &[u8] = include_bytes!("../assets/Silent.wav");
const WAV_HEADER_SIZE: usize = 44;
const SEGMENT_SIZE: usize = 4;
pub const BUFFER_LEN: usize = SEGMENT_SIZE / 4;

pub(crate) fn init(i2s: I2S, pins: Pins) -> hal::i2s::I2S {
    let i2s = hal::i2s::I2S::new(i2s, pins);
    i2s.set_tx_enabled(true);
    i2s.set_sample_width(SampleWidth::_16bit);
    i2s.set_format(Format::I2S);
    i2s.set_channels(Channels::Stereo);
    i2s.set_mck_frequency(MckFreq::_32MDiv10);
    i2s.set_ratio(Ratio::_64x);
    i2s.enable();

    i2s
}

pub(crate) fn next_segment(cx: play_next_audio_segment::Context) {
    if !cx.shared.amp_on.load(Ordering::Relaxed) {
        return;
    }

    let dma_buf = cx.local.dma_buf;
    let pcm_raw = &WAV_RAW[WAV_HEADER_SIZE..];
    let num_segments = pcm_raw.len() / SEGMENT_SIZE;

    let curr_segment = *cx.local.segment_index;
    let seg_index = *cx.local.segment_index as usize;

    *cx.local.segment_index = if curr_segment + 1 >= num_segments as u32 {
        0
    } else {
        curr_segment + 1
    };

    #[cfg(feature = "52833-debug")]
    writeln!(cx.local.rtt_speaker, "Playing segment {}", seg_index).ok();

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
    let tx_buf: &'static [u32] =
        unsafe { core::slice::from_raw_parts(dma_buf.as_ptr(), BUFFER_LEN) };
    let transfer = i2s.tx(tx_buf).expect("I2S TX transfer failed");

    let (_returned_buffer, new_i2s) = transfer.wait();

    #[cfg(feature = "52833-debug")]
    writeln!(cx.local.rtt_speaker, "Completed segment {}", seg_index).ok();

    *cx.local.i2s = Some(new_i2s);

    play_next_audio_segment::spawn().ok();
}
