//! Helpers for cpal sample-format dispatch and channel layout conversion.
//!
//! Everything is lossless where the source format can express the target.
//! Internal pipeline format is `f32` interleaved stereo.

use cpal::Sample;

/// Convert an interleaved buffer of arbitrary sample type to interleaved f32 stereo.
/// `src` is `frames * src_channels` long. `dst` must hold `frames * 2`.
///
/// Channel layout rules:
/// - 1 channel  → duplicate (L = R = mono)
/// - 2 channels → pass-through (L, R)
/// - ≥3 channels → take the first two channels (front L/R per WAVEFORMATEXTENSIBLE)
pub fn convert_to_stereo<T>(src: &[T], dst: &mut [f32], src_channels: usize)
where
    T: Sample,
    f32: cpal::FromSample<T>,
{
    let frames = src.len() / src_channels;
    debug_assert!(dst.len() >= frames * 2);

    match src_channels {
        0 => {}
        1 => {
            for (i, &s) in src.iter().enumerate() {
                let v = s.to_sample::<f32>();
                dst[i * 2] = v;
                dst[i * 2 + 1] = v;
            }
        }
        2 => {
            // Direct interleave conversion.
            for (i, frame) in src.chunks_exact(2).enumerate() {
                dst[i * 2] = frame[0].to_sample::<f32>();
                dst[i * 2 + 1] = frame[1].to_sample::<f32>();
            }
        }
        n => {
            for (i, frame) in src.chunks_exact(n).enumerate() {
                dst[i * 2] = frame[0].to_sample::<f32>();
                dst[i * 2 + 1] = frame[1].to_sample::<f32>();
            }
        }
    }
}

/// Write a block of interleaved stereo f32 frames into an interleaved buffer of
/// `M` channels. M is determined by `dst.chunks_mut(channels)`.
///
/// - M = 1 → average L + R
/// - M = 2 → pass-through
/// - M ≥ 3 → L, R, then zero the remaining channels (no upmixing)
pub fn write_stereo_to_output(stereo: &[f32], dst: &mut [f32], channels: usize) {
    match channels {
        0 => {}
        1 => {
            for (i, frame) in dst.iter_mut().enumerate() {
                let l = stereo[i * 2];
                let r = stereo[i * 2 + 1];
                *frame = 0.5 * (l + r);
            }
        }
        2 => {
            dst.copy_from_slice(&stereo[..dst.len()]);
        }
        m => {
            for (i, frame) in dst.chunks_exact_mut(m).enumerate() {
                frame[0] = stereo[i * 2];
                frame[1] = stereo[i * 2 + 1];
                for ch in &mut frame[2..] {
                    *ch = 0.0;
                }
            }
        }
    }
}
