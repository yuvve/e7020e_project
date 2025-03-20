use {
    crate::app::*,
    core::sync::atomic::Ordering,
    hal::{
        i2s::{Channels, Format, MckFreq, Pins, Ratio, SampleWidth},
        pac::I2S,
    },
    nrf52833_hal as hal,
    rtt_target::rprintln,
};

const WAV_RAW: &[u8] = include_bytes!("../assets/SeaBreeze3.wav");
const WAV_HEADER_SIZE: usize = 44;
const SEGMENT_SIZE: usize = 1024;
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

pub(crate) fn next_segment(cx: next_segment::Context) {
    if !cx.shared.amp_on.load(Ordering::Relaxed) {
        return;
    }

    let dma_buf = cx.local.dma_buf;
    let pcm_raw = &WAV_RAW[WAV_HEADER_SIZE..];
    let num_segments = pcm_raw.len() / SEGMENT_SIZE;

    let mut seg_index = *cx.local.segment_index as usize;
    seg_index = if seg_index + 1 >= num_segments {
        0
    } else {
        seg_index + 1
    };

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
    let tx_buf: &'static [u32] =
        unsafe { core::slice::from_raw_parts(dma_buf.as_ptr(), BUFFER_LEN) };
    let transfer = i2s.tx(tx_buf).expect("I2S TX transfer failed");

    let (_returned_buffer, new_i2s) = transfer.wait();

    rprintln!("Completed segment {}", seg_index);

    *cx.local.i2s = Some(new_i2s);

    next_segment::spawn().ok();
}
