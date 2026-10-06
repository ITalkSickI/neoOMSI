//! Typed CPAL conversion around the same f32 renderer used offline. Scratch is bounded
//! and allocated when opening the device, never by its callback. Chunk boundaries align
//! with device frames; signed/unsigned formats use CPAL's sample conversion semantics.
use cpal::traits::DeviceTrait;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
const SCRATCH_SAMPLES: usize = 4096;

pub(crate) fn build<T, F>(device: &cpal::Device, config: cpal::StreamConfig, mut render: F,
    reopen: Arc<AtomicBool>, lost: Arc<AtomicBool>) -> Result<cpal::Stream, cpal::Error>
where T: cpal::SizedSample + cpal::FromSample<f32>, F: FnMut(&mut [f32]) + Send + 'static {
    let mut scratch = vec![0.0f32; SCRATCH_SAMPLES].into_boxed_slice();
    let channels = config.channels.max(1) as usize;
    let chunk_size = SCRATCH_SAMPLES / channels * channels;
    device.build_output_stream(config, move |data: &mut [T], _| {
        for chunk in data.chunks_mut(chunk_size) {
            render(&mut scratch[..chunk.len()]);
            convert(&scratch[..chunk.len()], chunk);
        }
    }, move |_error| {
        // Error callbacks must not log or format strings. Retry on the game thread.
        lost.store(true, Ordering::Relaxed);
        reopen.store(true, Ordering::Relaxed);
    }, None)
}

pub fn convert<T: cpal::Sample + cpal::FromSample<f32>>(input: &[f32], out: &mut [T]) {
    for (sample, value) in out.iter_mut().zip(input) {
        let value = if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
        *sample = T::from_sample(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_unsigned_float_and_invalid_samples() {
        let input = [-1.0, 0.0, 1.0, f32::NAN];
        let mut signed = [0i16; 4]; convert(&input, &mut signed);
        assert_eq!(signed, [i16::MIN, 0, i16::MAX, 0]);
        let mut unsigned = [0u16; 4]; convert(&input, &mut unsigned);
        assert_eq!(unsigned, [0, 32768, 65535, 32768]);
        let mut floats = [0.0f64; 4]; convert(&input, &mut floats);
        assert_eq!(floats, [-1.0, 0.0, 1.0, 0.0]);
    }
}
