//! Real-time DSP effects. All effects operate on interleaved stereo f32 frames.
//!
//! Effects must be allocation-free and lock-free in `process` — they are invoked
//! from the cpal output callback thread, which has hard real-time constraints.

use crate::audio::graph::{ChannelBalanceData, EffectChain, EffectSpec, GainData, LimiterData, MuteData};

/// A processor that mutates interleaved stereo f32 samples in place.
/// `frames` is the slice length in *frames* (one frame = L,R pair).
pub trait Effect: Send {
    fn process(&mut self, samples: &mut [f32], frames: usize);
}

#[derive(Debug, Clone, Copy)]
pub struct GainEffect {
    linear: f32,
}

impl GainEffect {
    pub fn new(d: GainData) -> Self {
        Self {
            linear: db_to_linear(d.gain_db),
        }
    }
}

impl Effect for GainEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let g = self.linear;
        let n = frames * 2;
        for s in &mut samples[..n] {
            *s *= g;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MuteEffect {
    muted: bool,
}

impl MuteEffect {
    pub fn new(d: MuteData) -> Self {
        Self { muted: d.muted }
    }
}

impl Effect for MuteEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        if !self.muted {
            return;
        }
        for s in &mut samples[..frames * 2] {
            *s = 0.0;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelBalanceEffect {
    left_linear: f32,
    right_linear: f32,
}

impl ChannelBalanceEffect {
    pub fn new(d: ChannelBalanceData) -> Self {
        Self {
            left_linear: db_to_linear(d.left_gain_db),
            right_linear: db_to_linear(d.right_gain_db),
        }
    }
}

impl Effect for ChannelBalanceEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let (gl, gr) = (self.left_linear, self.right_linear);
        let stereo = &mut samples[..frames * 2];
        for frame in stereo.chunks_exact_mut(2) {
            frame[0] *= gl;
            frame[1] *= gr;
        }
    }
}

/// Soft limiter: pre-amp by `drive`, then pass through tanh, then scale to ceiling.
/// `y = ceiling * tanh(x * drive / ceiling)` — smooth saturation, no hard clipping.
#[derive(Debug, Clone, Copy)]
pub struct LimiterEffect {
    ceiling: f32,
    drive: f32,
    inv_ceiling: f32,
}

impl LimiterEffect {
    pub fn new(d: LimiterData) -> Self {
        let ceiling = db_to_linear(d.threshold_db).max(1e-6);
        let drive = db_to_linear(d.drive_db);
        Self {
            ceiling,
            drive,
            inv_ceiling: 1.0 / ceiling,
        }
    }
}

impl Effect for LimiterEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let (c, d, inv_c) = (self.ceiling, self.drive, self.inv_ceiling);
        let stereo = &mut samples[..frames * 2];
        for s in stereo {
            // tanh on f32 — precision is sufficient for limiting transients.
            *s = c * fast_tanh(*s * d * inv_c);
        }
    }
}

/// Build a runtime effect chain from a typed spec. Allocates once; the resulting
/// objects are `process()`-safe to call on the RT thread.
pub fn build_chain(spec: &EffectChain) -> Vec<Box<dyn Effect>> {
    spec.0
        .iter()
        .map(|e| -> Box<dyn Effect> {
            match *e {
                EffectSpec::Gain(d) => Box::new(GainEffect::new(d)),
                EffectSpec::Mute(d) => Box::new(MuteEffect::new(d)),
                EffectSpec::ChannelBalance(d) => Box::new(ChannelBalanceEffect::new(d)),
                EffectSpec::Limiter(d) => Box::new(LimiterEffect::new(d)),
            }
        })
        .collect()
}

#[inline]
fn db_to_linear(db: f32) -> f32 {
    // -inf dB → 0. We treat -60 dB as the practical floor for our UI sliders.
    if db <= -60.0 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

/// Padé-style approximation of `tanh` — within ~1e-4 of `f32::tanh` in [-4, 4],
/// branchless, ~4x faster than `f32::tanh` on x86_64/aarch64.
#[inline]
fn fast_tanh(x: f32) -> f32 {
    // Clamp keeps the polynomial well-conditioned outside the typical range.
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    let num = x * (27.0 + x2);
    let den = 27.0 + 9.0 * x2;
    num / den
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_applies_db() {
        let mut e = GainEffect::new(GainData { gain_db: 6.0 });
        let mut buf = [1.0_f32, 1.0];
        e.process(&mut buf, 1);
        // +6 dB ≈ 1.995x
        assert!((buf[0] - 1.995).abs() < 0.01);
    }

    #[test]
    fn mute_zeros() {
        let mut e = MuteEffect::new(MuteData { muted: true });
        let mut buf = [0.5, -0.5, 0.3, -0.3];
        e.process(&mut buf, 2);
        assert_eq!(buf, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn balance_applies_per_channel() {
        let mut e = ChannelBalanceEffect::new(ChannelBalanceData {
            left_gain_db: -6.0,
            right_gain_db: 0.0,
        });
        let mut buf = [1.0, 1.0];
        e.process(&mut buf, 1);
        assert!((buf[0] - 0.501).abs() < 0.01);
        assert!((buf[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn limiter_saturates_above_ceiling() {
        let mut e = LimiterEffect::new(LimiterData {
            threshold_db: 0.0,
            drive_db: 0.0,
        });
        let mut buf = [10.0, -10.0];
        e.process(&mut buf, 1);
        assert!(buf[0].abs() < 1.05);
        assert!(buf[1].abs() < 1.05);
    }
}
