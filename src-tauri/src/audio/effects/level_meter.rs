use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::audio::graph::LevelMeterData;

use super::util::{load_f32, store_f32};
use super::Effect;

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
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            peak_l: Arc::new(AtomicU32::new(0)),
            peak_r: Arc::new(AtomicU32::new(0)),
            rms_l: Arc::new(AtomicU32::new(0)),
            rms_r: Arc::new(AtomicU32::new(0)),
        }
    }

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
    pub fn new(_d: LevelMeterData, node_id: String) -> (Self, MeterHandle) {
        let handle = MeterHandle::new(node_id);
        (
            Self {
                handle: handle.clone(),
            },
            handle,
        )
    }

    pub fn from_handle(handle: MeterHandle) -> Self {
        Self { handle }
    }
}

impl Effect for LevelMeterEffect {
    #[inline]
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        update_meter(&self.handle, &samples[..frames * 2]);
    }
}

/// `stereo` is interleaved L/R f32; odd-length truncates the trailing half-frame.
pub fn update_meter(handle: &MeterHandle, stereo: &[f32]) {
    let frames = stereo.len() / 2;
    if frames == 0 {
        return;
    }
    let stereo = &stereo[..frames * 2];
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
    let existing_l = load_f32(&handle.peak_l);
    let existing_r = load_f32(&handle.peak_r);
    store_f32(&handle.peak_l, existing_l.max(peak_l));
    store_f32(&handle.peak_r, existing_r.max(peak_r));
    let rms_l = (sum_l_sq / frames as f64).sqrt() as f32;
    let rms_r = (sum_r_sq / frames as f64).sqrt() as f32;
    store_f32(&handle.rms_l, rms_l);
    store_f32(&handle.rms_r, rms_r);
}
