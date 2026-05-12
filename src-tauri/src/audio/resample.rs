//! High-quality sinc-based resampler for stereo f32 streams.
//!
//! Wraps `rubato::SincFixedIn` for fixed-chunk-size input. The caller is
//! responsible for feeding exactly `chunk_size` frames at a time; the resampler
//! emits a variable (but bounded) number of output frames per call.
//!
//! All buffers are pre-allocated. `process_chunk` does not allocate.

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::error::{AppError, AppResult};

pub struct StereoResampler {
    inner: SincFixedIn<f32>,
    in_planar: [Vec<f32>; 2],
    out_planar: [Vec<f32>; 2],
    chunk_in: usize,
    out_max: usize,
}

impl StereoResampler {
    /// Build a resampler converting from `from_rate` Hz to `to_rate` Hz.
    /// `chunk_size` is the number of input frames fed per `process_chunk` call.
    pub fn new(from_rate: u32, to_rate: u32, chunk_size: usize) -> AppResult<Self> {
        let ratio = to_rate as f64 / from_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        let inner = SincFixedIn::<f32>::new(ratio, 1.05, params, chunk_size, 2)
            .map_err(|e| AppError::Stream(format!("resampler init: {e}")))?;

        let out_max = inner.output_frames_max();
        let in_planar = [vec![0.0_f32; chunk_size], vec![0.0_f32; chunk_size]];
        let out_planar = [Vec::with_capacity(out_max), Vec::with_capacity(out_max)];

        Ok(Self {
            inner,
            in_planar,
            out_planar,
            chunk_in: chunk_size,
            out_max,
        })
    }

    pub fn chunk_in(&self) -> usize {
        self.chunk_in
    }
    pub fn out_max(&self) -> usize {
        self.out_max
    }

    /// Process one fixed-size chunk of interleaved stereo input. Appends
    /// produced frames (interleaved stereo) to `dst`.
    pub fn process_chunk(&mut self, interleaved_in: &[f32], dst: &mut Vec<f32>) -> AppResult<()> {
        debug_assert_eq!(interleaved_in.len(), self.chunk_in * 2);

        // Deinterleave into planar input buffers.
        for (i, frame) in interleaved_in.chunks_exact(2).enumerate() {
            self.in_planar[0][i] = frame[0];
            self.in_planar[1][i] = frame[1];
        }

        // Ensure output buffers can hold at least out_max frames without realloc.
        for v in &mut self.out_planar {
            v.clear();
            v.reserve(self.out_max);
        }

        self.inner
            .process_into_buffer(&self.in_planar, &mut self.out_planar, None)
            .map_err(|e| AppError::Stream(format!("resampler process: {e}")))?;

        let produced = self.out_planar[0].len();
        for i in 0..produced {
            dst.push(self.out_planar[0][i]);
            dst.push(self.out_planar[1][i]);
        }
        Ok(())
    }
}
