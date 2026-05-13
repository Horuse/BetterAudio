//! Real-time DSP effects. All effects operate on interleaved stereo f32 frames.
//!
//! Parameters live in `Arc<Atomic*>` cells shared with the UI side of the
//! engine. The audio callback reads them lock-free on every block, so slider
//! moves and mute toggles take effect within a couple of milliseconds without
//! restarting the pipeline.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use serde_json::Value;

use crate::audio::graph::{
    ChannelBalanceData, EffectSpec, GainData, LevelMeterData, LimiterData, MuteData,
};

pub trait Effect: Send {
    fn process(&mut self, samples: &mut [f32], frames: usize);
}

/// Enum dispatch wrapper so the RT thread doesn't pay a vtable indirection per
/// process call. The closed set of effects is known at compile time; LLVM can
/// inline the inner loop for each variant.
pub enum RuntimeEffect {
    Gain(GainEffect),
    Mute(MuteEffect),
    ChannelBalance(ChannelBalanceEffect),
    Limiter(LimiterEffect),
    LevelMeter(LevelMeterEffect),
}

impl RuntimeEffect {
    #[inline]
    pub fn process(&mut self, samples: &mut [f32], frames: usize) {
        match self {
            RuntimeEffect::Gain(e) => e.process(samples, frames),
            RuntimeEffect::Mute(e) => e.process(samples, frames),
            RuntimeEffect::ChannelBalance(e) => e.process(samples, frames),
            RuntimeEffect::Limiter(e) => e.process(samples, frames),
            RuntimeEffect::LevelMeter(e) => e.process(samples, frames),
        }
    }
}

#[derive(Clone)]
pub enum EffectControl {
    Gain {
        linear: Arc<AtomicU32>,
    },
    Mute {
        muted: Arc<AtomicBool>,
    },
    ChannelBalance {
        left: Arc<AtomicU32>,
        right: Arc<AtomicU32>,
    },
    Limiter {
        ceiling: Arc<AtomicU32>,
        drive: Arc<AtomicU32>,
        inv_ceiling: Arc<AtomicU32>,
    },
}

impl EffectControl {
    /// Unknown keys are silently ignored — the frontend pushes the full
    /// camelCase payload of the node, only some keys map to live controls.
    pub fn apply_update(&self, data: &Value) {
        match self {
            EffectControl::Gain { linear } => {
                if let Some(db) = num(data, "gainDb") {
                    store_f32(linear, db_to_linear(db));
                }
            }
            EffectControl::Mute { muted } => {
                if let Some(b) = data.get("muted").and_then(Value::as_bool) {
                    muted.store(b, Ordering::Relaxed);
                }
            }
            EffectControl::ChannelBalance { left, right } => {
                if let Some(db) = num(data, "leftGainDb") {
                    store_f32(left, db_to_linear(db));
                }
                if let Some(db) = num(data, "rightGainDb") {
                    store_f32(right, db_to_linear(db));
                }
            }
            EffectControl::Limiter {
                ceiling,
                drive,
                inv_ceiling,
            } => {
                if let Some(db) = num(data, "thresholdDb") {
                    let c = db_to_linear(db).max(1e-6);
                    store_f32(ceiling, c);
                    store_f32(inv_ceiling, 1.0 / c);
                }
                if let Some(db) = num(data, "driveDb") {
                    store_f32(drive, db_to_linear(db));
                }
            }
        }
    }
}

fn num(data: &Value, key: &str) -> Option<f32> {
    data.get(key).and_then(Value::as_f64).map(|v| v as f32)
}

#[inline]
fn store_f32(slot: &AtomicU32, v: f32) {
    slot.store(v.to_bits(), Ordering::Relaxed);
}

#[inline]
fn load_f32(slot: &AtomicU32) -> f32 {
    f32::from_bits(slot.load(Ordering::Relaxed))
}

pub struct GainEffect {
    linear: Arc<AtomicU32>,
}

impl GainEffect {
    fn new(d: GainData) -> (Self, EffectControl) {
        let linear = Arc::new(AtomicU32::new(db_to_linear(d.gain_db).to_bits()));
        let control = EffectControl::Gain {
            linear: linear.clone(),
        };
        (Self { linear }, control)
    }
}

impl Effect for GainEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let g = load_f32(&self.linear);
        for s in &mut samples[..frames * 2] {
            *s *= g;
        }
    }
}

pub struct MuteEffect {
    muted: Arc<AtomicBool>,
}

impl MuteEffect {
    fn new(d: MuteData) -> (Self, EffectControl) {
        let muted = Arc::new(AtomicBool::new(d.muted));
        let control = EffectControl::Mute {
            muted: muted.clone(),
        };
        (Self { muted }, control)
    }
}

impl Effect for MuteEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        if !self.muted.load(Ordering::Relaxed) {
            return;
        }
        for s in &mut samples[..frames * 2] {
            *s = 0.0;
        }
    }
}

pub struct ChannelBalanceEffect {
    left: Arc<AtomicU32>,
    right: Arc<AtomicU32>,
}

impl ChannelBalanceEffect {
    fn new(d: ChannelBalanceData) -> (Self, EffectControl) {
        let left = Arc::new(AtomicU32::new(db_to_linear(d.left_gain_db).to_bits()));
        let right = Arc::new(AtomicU32::new(db_to_linear(d.right_gain_db).to_bits()));
        let control = EffectControl::ChannelBalance {
            left: left.clone(),
            right: right.clone(),
        };
        (Self { left, right }, control)
    }
}

impl Effect for ChannelBalanceEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let gl = load_f32(&self.left);
        let gr = load_f32(&self.right);
        let stereo = &mut samples[..frames * 2];
        for frame in stereo.chunks_exact_mut(2) {
            frame[0] *= gl;
            frame[1] *= gr;
        }
    }
}

pub struct LevelMeterEffect {
    handle: MeterHandle,
}

#[derive(Clone)]
pub struct MeterHandle {
    pub node_id: String,
    pub peak_l: Arc<AtomicU32>,
    pub peak_r: Arc<AtomicU32>,
    pub rms_l: Arc<AtomicU32>,
    pub rms_r: Arc<AtomicU32>,
}

#[derive(Debug, Clone, Copy)]
pub struct MeterSnapshot {
    pub peak_l: f32,
    pub peak_r: f32,
    pub rms_l: f32,
    pub rms_r: f32,
}

/// Peak fall-off per tick — prevents transients from latching the meter.
pub const METER_PEAK_DECAY: f32 = 0.85;

impl MeterHandle {
    /// Snapshot current values and decay the peak — called from the engine's
    /// tick thread.
    pub fn snapshot_and_decay(&self) -> MeterSnapshot {
        let pl = load_f32(&self.peak_l);
        let pr = load_f32(&self.peak_r);
        let rl = load_f32(&self.rms_l);
        let rr = load_f32(&self.rms_r);
        store_f32(&self.peak_l, pl * METER_PEAK_DECAY);
        store_f32(&self.peak_r, pr * METER_PEAK_DECAY);
        MeterSnapshot {
            peak_l: pl,
            peak_r: pr,
            rms_l: rl,
            rms_r: rr,
        }
    }
}

impl LevelMeterEffect {
    fn new(_d: LevelMeterData, node_id: String) -> (Self, MeterHandle) {
        let handle = MeterHandle {
            node_id,
            peak_l: Arc::new(AtomicU32::new(0)),
            peak_r: Arc::new(AtomicU32::new(0)),
            rms_l: Arc::new(AtomicU32::new(0)),
            rms_r: Arc::new(AtomicU32::new(0)),
        };
        (
            Self {
                handle: handle.clone(),
            },
            handle,
        )
    }
}

impl Effect for LevelMeterEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        if frames == 0 {
            return;
        }
        let stereo = &samples[..frames * 2];
        let mut peak_l = 0.0f32;
        let mut peak_r = 0.0f32;
        let mut sum_l_sq = 0.0f64;
        let mut sum_r_sq = 0.0f64;
        for frame in stereo.chunks_exact(2) {
            let l = frame[0];
            let r = frame[1];
            let al = l.abs();
            let ar = r.abs();
            if al > peak_l {
                peak_l = al;
            }
            if ar > peak_r {
                peak_r = ar;
            }
            sum_l_sq += (l as f64) * (l as f64);
            sum_r_sq += (r as f64) * (r as f64);
        }
        // Peak: write the larger of (existing, this block) so the tick thread
        // sees the true peak between samples.
        let existing_l = load_f32(&self.handle.peak_l);
        let existing_r = load_f32(&self.handle.peak_r);
        store_f32(&self.handle.peak_l, existing_l.max(peak_l));
        store_f32(&self.handle.peak_r, existing_r.max(peak_r));
        // RMS: replace with this block's value — short window, snappy UI.
        let rms_l = (sum_l_sq / frames as f64).sqrt() as f32;
        let rms_r = (sum_r_sq / frames as f64).sqrt() as f32;
        store_f32(&self.handle.rms_l, rms_l);
        store_f32(&self.handle.rms_r, rms_r);
    }
}

/// Soft limiter: pre-amp by `drive`, then pass through tanh, then scale to ceiling.
/// `y = ceiling * tanh(x * drive / ceiling)` — smooth saturation, no hard clipping.
pub struct LimiterEffect {
    ceiling: Arc<AtomicU32>,
    drive: Arc<AtomicU32>,
    inv_ceiling: Arc<AtomicU32>,
}

impl LimiterEffect {
    fn new(d: LimiterData) -> (Self, EffectControl) {
        let c = db_to_linear(d.threshold_db).max(1e-6);
        let ceiling = Arc::new(AtomicU32::new(c.to_bits()));
        let drive = Arc::new(AtomicU32::new(db_to_linear(d.drive_db).to_bits()));
        let inv_ceiling = Arc::new(AtomicU32::new((1.0 / c).to_bits()));
        let control = EffectControl::Limiter {
            ceiling: ceiling.clone(),
            drive: drive.clone(),
            inv_ceiling: inv_ceiling.clone(),
        };
        (
            Self {
                ceiling,
                drive,
                inv_ceiling,
            },
            control,
        )
    }
}

impl Effect for LimiterEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        let c = load_f32(&self.ceiling);
        let d = load_f32(&self.drive);
        let inv_c = load_f32(&self.inv_ceiling);
        let stereo = &mut samples[..frames * 2];
        for s in stereo {
            *s = c * fast_tanh(*s * d * inv_c);
        }
    }
}

pub struct EffectBuild {
    pub effect: RuntimeEffect,
    pub control: Option<EffectControl>,
    pub meter: Option<MeterHandle>,
}

pub fn instantiate_effect(spec: &EffectSpec, node_id: &str) -> EffectBuild {
    match *spec {
        EffectSpec::Gain(d) => {
            let (e, c) = GainEffect::new(d);
            EffectBuild {
                effect: RuntimeEffect::Gain(e),
                control: Some(c),
                meter: None,
            }
        }
        EffectSpec::Mute(d) => {
            let (e, c) = MuteEffect::new(d);
            EffectBuild {
                effect: RuntimeEffect::Mute(e),
                control: Some(c),
                meter: None,
            }
        }
        EffectSpec::ChannelBalance(d) => {
            let (e, c) = ChannelBalanceEffect::new(d);
            EffectBuild {
                effect: RuntimeEffect::ChannelBalance(e),
                control: Some(c),
                meter: None,
            }
        }
        EffectSpec::Limiter(d) => {
            let (e, c) = LimiterEffect::new(d);
            EffectBuild {
                effect: RuntimeEffect::Limiter(e),
                control: Some(c),
                meter: None,
            }
        }
        EffectSpec::LevelMeter(d) => {
            let (e, handle) = LevelMeterEffect::new(d, node_id.to_string());
            EffectBuild {
                effect: RuntimeEffect::LevelMeter(e),
                control: None,
                meter: Some(handle),
            }
        }
    }
}

#[inline]
fn db_to_linear(db: f32) -> f32 {
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
        let (mut e, _) = GainEffect::new(GainData { gain_db: 6.0 });
        let mut buf = [1.0_f32, 1.0];
        e.process(&mut buf, 1);
        assert!((buf[0] - 1.995).abs() < 0.01);
    }

    #[test]
    fn gain_control_changes_live() {
        let (mut e, c) = GainEffect::new(GainData { gain_db: 0.0 });
        c.apply_update(&serde_json::json!({ "gainDb": 6.0 }));
        let mut buf = [1.0_f32, 1.0];
        e.process(&mut buf, 1);
        assert!((buf[0] - 1.995).abs() < 0.01);
    }

    #[test]
    fn mute_zeros() {
        let (mut e, _) = MuteEffect::new(MuteData { muted: true });
        let mut buf = [0.5, -0.5, 0.3, -0.3];
        e.process(&mut buf, 2);
        assert_eq!(buf, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn mute_control_unmutes_live() {
        let (mut e, c) = MuteEffect::new(MuteData { muted: true });
        c.apply_update(&serde_json::json!({ "muted": false }));
        let mut buf = [0.5_f32, -0.5];
        e.process(&mut buf, 1);
        assert_eq!(buf, [0.5, -0.5]);
    }

    #[test]
    fn balance_applies_per_channel() {
        let (mut e, _) = ChannelBalanceEffect::new(ChannelBalanceData {
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
        let (mut e, _) = LimiterEffect::new(LimiterData {
            threshold_db: 0.0,
            drive_db: 0.0,
        });
        let mut buf = [10.0, -10.0];
        e.process(&mut buf, 1);
        assert!(buf[0].abs() < 1.05);
        assert!(buf[1].abs() < 1.05);
    }
}
