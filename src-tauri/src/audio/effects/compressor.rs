use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::audio::graph::CompressorData;

use super::util::{load_f32, store_f32};
use super::{Effect, EffectControl};

pub struct CompressorEffect {
    threshold_db: Arc<AtomicU32>,
    ratio: Arc<AtomicU32>,
    attack_ms: Arc<AtomicU32>,
    release_ms: Arc<AtomicU32>,
    knee_db: Arc<AtomicU32>,
    makeup_db: Arc<AtomicU32>,
    sample_rate: u32,
    envelope: f32,
    /// Min gain (0-1 linear, no makeup) across the last block. 1.0 = no GR.
    pub gr_lin: Arc<AtomicU32>,
}

impl CompressorEffect {
    pub fn new(d: CompressorData, sample_rate: u32) -> (Self, EffectControl, Arc<AtomicU32>) {
        let threshold_db = Arc::new(AtomicU32::new(d.threshold_db.to_bits()));
        let ratio = Arc::new(AtomicU32::new(d.ratio.max(1.0).to_bits()));
        let attack_ms = Arc::new(AtomicU32::new(d.attack_ms.max(0.01).to_bits()));
        let release_ms = Arc::new(AtomicU32::new(d.release_ms.max(0.1).to_bits()));
        let knee_db = Arc::new(AtomicU32::new(d.knee_db.max(0.0).to_bits()));
        let makeup_db = Arc::new(AtomicU32::new(d.makeup_db.to_bits()));
        let gr_lin = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let control = EffectControl::Compressor {
            threshold_db: threshold_db.clone(),
            ratio: ratio.clone(),
            attack_ms: attack_ms.clone(),
            release_ms: release_ms.clone(),
            knee_db: knee_db.clone(),
            makeup_db: makeup_db.clone(),
        };
        (
            Self {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                sample_rate,
                envelope: 0.0,
                gr_lin: gr_lin.clone(),
            },
            control,
            gr_lin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_state(
        threshold_db: Arc<AtomicU32>,
        ratio: Arc<AtomicU32>,
        attack_ms: Arc<AtomicU32>,
        release_ms: Arc<AtomicU32>,
        knee_db: Arc<AtomicU32>,
        makeup_db: Arc<AtomicU32>,
        sample_rate: u32,
        gr_lin: Arc<AtomicU32>,
    ) -> Self {
        Self {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            sample_rate,
            envelope: 0.0,
            gr_lin,
        }
    }

    pub fn process_with_sidechain(
        &mut self,
        main: &mut [f32],
        sidechain: Option<&[f32]>,
        frames: usize,
    ) {
        self.process_inner(main, sidechain, frames);
    }

    fn process_inner(&mut self, main: &mut [f32], sidechain: Option<&[f32]>, frames: usize) {
        let threshold_db = load_f32(&self.threshold_db);
        let ratio = load_f32(&self.ratio).max(1.0);
        let attack_ms = load_f32(&self.attack_ms).max(0.01);
        let release_ms = load_f32(&self.release_ms).max(0.1);
        let knee_db = load_f32(&self.knee_db).max(0.0);
        let makeup_db = load_f32(&self.makeup_db);

        let sr = self.sample_rate as f32;
        let attack_coeff = 1.0 - (-1.0 / (attack_ms * 0.001 * sr)).exp();
        let release_coeff = 1.0 - (-1.0 / (release_ms * 0.001 * sr)).exp();
        let inv_ratio = 1.0 / ratio;
        let makeup_lin = 10f32.powf(makeup_db / 20.0);
        let half_knee = knee_db * 0.5;

        let main_buf = &mut main[..frames * 2];
        let side = sidechain.filter(|s| s.len() >= frames * 2);
        let mut block_min_gr = 1.0f32;
        for (f, frame) in main_buf.chunks_exact_mut(2).enumerate() {
            let detected = match side {
                Some(s) => s[f * 2].abs().max(s[f * 2 + 1].abs()),
                None => frame[0].abs().max(frame[1].abs()),
            };
            if detected > self.envelope {
                self.envelope += (detected - self.envelope) * attack_coeff;
            } else {
                self.envelope += (detected - self.envelope) * release_coeff;
            }

            let env_db = if self.envelope < 1e-6 {
                -120.0
            } else {
                20.0 * self.envelope.log10()
            };
            let over = env_db - threshold_db;
            let gain_red_db = if knee_db > 0.0 && over > -half_knee && over < half_knee {
                let x = over + half_knee;
                (1.0 - inv_ratio) * x * x / (2.0 * knee_db)
            } else if over > 0.0 {
                over * (1.0 - inv_ratio)
            } else {
                0.0
            };
            let gr_only = 10f32.powf(-gain_red_db / 20.0);
            if gr_only < block_min_gr {
                block_min_gr = gr_only;
            }
            frame[0] *= gr_only * makeup_lin;
            frame[1] *= gr_only * makeup_lin;
        }
        store_f32(&self.gr_lin, block_min_gr);
    }
}

impl Effect for CompressorEffect {
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        self.process_inner(samples, None, frames);
    }
}
