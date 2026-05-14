use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::audio::graph::ReverbData;

use super::util::load_f32;
use super::{Effect, EffectControl};

/// Freeverb. Tuning constants are 44.1 kHz; we scale buffer lengths to the
/// host SR. Right channel uses `STEREO_SPREAD`-sample longer buffers — the
/// length offset is what gives the wet field stereo image.
pub struct ReverbEffect {
    room_size: Arc<AtomicU32>,
    damping: Arc<AtomicU32>,
    width: Arc<AtomicU32>,
    mix: Arc<AtomicU32>,
    comb_l: [Comb; 8],
    comb_r: [Comb; 8],
    allpass_l: [Allpass; 4],
    allpass_r: [Allpass; 4],
}

struct Comb {
    buf: Box<[f32]>,
    pos: usize,
    filter_store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)].into_boxed_slice(),
            pos: 0,
            filter_store: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.pos];
        self.filter_store = out * (1.0 - damp) + self.filter_store * damp;
        self.buf[self.pos] = input + self.filter_store * feedback;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        out
    }
}

struct Allpass {
    buf: Box<[f32]>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)].into_boxed_slice(),
            pos: 0,
        }
    }

    /// Fixed g = 0.5 — not a true allpass, but it's what defines the
    /// Freeverb sound. Don't parametrise it.
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buf[self.pos];
        let output = -input + buf_out;
        self.buf[self.pos] = input + buf_out * 0.5;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        output
    }
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const REVERB_INPUT_GAIN: f32 = 0.015;
const REVERB_SCALE_ROOM: f32 = 0.28;
const REVERB_OFFSET_ROOM: f32 = 0.7;
const REVERB_SCALE_DAMP: f32 = 0.4;

impl ReverbEffect {
    pub fn new(d: ReverbData, sample_rate: u32) -> (Self, EffectControl) {
        let room_size = Arc::new(AtomicU32::new(d.room_size.clamp(0.0, 1.0).to_bits()));
        let damping = Arc::new(AtomicU32::new(d.damping.clamp(0.0, 1.0).to_bits()));
        let width = Arc::new(AtomicU32::new(d.width.clamp(0.0, 1.0).to_bits()));
        let mix = Arc::new(AtomicU32::new(d.mix.clamp(0.0, 1.0).to_bits()));
        let control = EffectControl::Reverb {
            room_size: room_size.clone(),
            damping: damping.clone(),
            width: width.clone(),
            mix: mix.clone(),
        };
        (
            Self::from_state(room_size, damping, width, mix, sample_rate),
            control,
        )
    }

    pub fn from_state(
        room_size: Arc<AtomicU32>,
        damping: Arc<AtomicU32>,
        width: Arc<AtomicU32>,
        mix: Arc<AtomicU32>,
        sample_rate: u32,
    ) -> Self {
        let scale = sample_rate as f32 / 44100.0;
        let comb_len = |n: usize| (n as f32 * scale) as usize;
        Self {
            room_size,
            damping,
            width,
            mix,
            comb_l: std::array::from_fn(|i| Comb::new(comb_len(COMB_TUNING[i]))),
            comb_r: std::array::from_fn(|i| Comb::new(comb_len(COMB_TUNING[i] + STEREO_SPREAD))),
            allpass_l: std::array::from_fn(|i| Allpass::new(comb_len(ALLPASS_TUNING[i]))),
            allpass_r: std::array::from_fn(|i| {
                Allpass::new(comb_len(ALLPASS_TUNING[i] + STEREO_SPREAD))
            }),
        }
    }
}

impl Effect for ReverbEffect {
    fn process(&mut self, samples: &mut [f32], frames: usize) {
        if frames == 0 {
            return;
        }
        let room = load_f32(&self.room_size).clamp(0.0, 1.0);
        let damping = load_f32(&self.damping).clamp(0.0, 1.0);
        let width = load_f32(&self.width).clamp(0.0, 1.0);
        let mix = load_f32(&self.mix).clamp(0.0, 1.0);
        let feedback = room * REVERB_SCALE_ROOM + REVERB_OFFSET_ROOM;
        let damp = damping * REVERB_SCALE_DAMP;
        let dry = 1.0 - mix;
        // width=1: strict L/R wet; width=0: mono wet image (cross-channel mix)
        let wet1 = mix * (width * 0.5 + 0.5);
        let wet2 = mix * (1.0 - width) * 0.5;

        let stereo = &mut samples[..frames * 2];
        for frame in stereo.chunks_exact_mut(2) {
            let il = frame[0];
            let ir = frame[1];
            let input = (il + ir) * REVERB_INPUT_GAIN;
            let mut out_l = 0.0;
            let mut out_r = 0.0;
            for c in &mut self.comb_l {
                out_l += c.process(input, feedback, damp);
            }
            for c in &mut self.comb_r {
                out_r += c.process(input, feedback, damp);
            }
            for ap in &mut self.allpass_l {
                out_l = ap.process(out_l);
            }
            for ap in &mut self.allpass_r {
                out_r = ap.process(out_r);
            }
            frame[0] = il * dry + out_l * wet1 + out_r * wet2;
            frame[1] = ir * dry + out_r * wet1 + out_l * wet2;
        }
    }
}
