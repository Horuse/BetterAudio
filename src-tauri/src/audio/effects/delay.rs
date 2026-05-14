use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::audio::graph::DelayData;

use super::util::load_f32;
use super::{Effect, EffectControl};

const MAX_DELAY_MS: f32 = 2000.0;

/// Stereo delay line with feedback and dry/wet mix. Ring sized to 2 s @ build
/// SR — live `time_ms` changes just shift the read offset, no realloc.
pub struct DelayEffect {
    time_ms: Arc<AtomicU32>,
    feedback: Arc<AtomicU32>,
    mix: Arc<AtomicU32>,
    sample_rate: u32,
    /// Stereo-interleaved ring; capacity = MAX_DELAY_MS @ sample_rate.
    buf: Box<[f32]>,
    write: usize,
    /// Per-block lerp target for `time_ms` — prevents discontinuity clicks
    /// when the slider moves.
    current_delay_frames: f32,
}

impl DelayEffect {
    pub fn new(d: DelayData, sample_rate: u32) -> (Self, EffectControl) {
        let time_ms = Arc::new(AtomicU32::new(d.time_ms.max(1.0).to_bits()));
        let feedback = Arc::new(AtomicU32::new(d.feedback.clamp(0.0, 0.95).to_bits()));
        let mix = Arc::new(AtomicU32::new(d.mix.clamp(0.0, 1.0).to_bits()));
        let control = EffectControl::Delay {
            time_ms: time_ms.clone(),
            feedback: feedback.clone(),
            mix: mix.clone(),
        };
        (
            Self::from_state(time_ms, feedback, mix, sample_rate),
            control,
        )
    }

    pub fn from_state(
        time_ms: Arc<AtomicU32>,
        feedback: Arc<AtomicU32>,
        mix: Arc<AtomicU32>,
        sample_rate: u32,
    ) -> Self {
        let cap = (MAX_DELAY_MS * 0.001 * sample_rate as f32) as usize * 2;
        let initial_delay_frames = load_f32(&time_ms).max(1.0) * 0.001 * sample_rate as f32;
        Self {
            time_ms,
            feedback,
            mix,
            sample_rate,
            buf: vec![0.0; cap].into_boxed_slice(),
            write: 0,
            current_delay_frames: initial_delay_frames,
        }
    }
}

impl Effect for DelayEffect {
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        if frames == 0 {
            return;
        }
        let target_delay = (load_f32(&self.time_ms).max(1.0) * 0.001 * self.sample_rate as f32)
            .min(MAX_DELAY_MS * 0.001 * self.sample_rate as f32);
        // Single-pole smoothing over one block: critically damped, ≈10 ms
        // settle so slider sweeps don't click.
        let smooth = 1.0 - (-1.0 / (0.01 * self.sample_rate as f32 / frames as f32)).exp();
        self.current_delay_frames += (target_delay - self.current_delay_frames) * smooth;
        let delay_samples = (self.current_delay_frames as usize).max(1) * 2;
        let feedback = load_f32(&self.feedback).clamp(0.0, 0.95);
        let mix = load_f32(&self.mix).clamp(0.0, 1.0);
        let dry = 1.0 - mix;

        let cap = self.buf.len();
        let stereo = &mut samples[..frames * 2];
        for frame in stereo.chunks_exact_mut(2) {
            let read = (self.write + cap - delay_samples) % cap;
            let dl = self.buf[read];
            let dr = self.buf[read + 1];
            let il = frame[0];
            let ir = frame[1];
            self.buf[self.write] = il + dl * feedback;
            self.buf[self.write + 1] = ir + dr * feedback;
            self.write = (self.write + 2) % cap;
            frame[0] = il * dry + dl * mix;
            frame[1] = ir * dry + dr * mix;
        }
    }
}
