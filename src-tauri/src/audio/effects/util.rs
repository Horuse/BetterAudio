use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;

#[inline]
pub(super) fn db_to_linear(db: f32) -> f32 {
    if db <= -60.0 {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

#[inline]
pub(super) fn store_f32(slot: &AtomicU32, v: f32) {
    slot.store(v.to_bits(), Ordering::Relaxed);
}

#[inline]
pub(super) fn load_f32(slot: &AtomicU32) -> f32 {
    f32::from_bits(slot.load(Ordering::Relaxed))
}

pub(super) fn num(data: &Value, key: &str) -> Option<f32> {
    data.get(key).and_then(Value::as_f64).map(|v| v as f32)
}
