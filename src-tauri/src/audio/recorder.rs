//! WAV file recorder for the `FileRecording` output node.
//!
//! Pro defaults: 32-bit float PCM, stereo, fixed internal sample rate.
//! Writes are buffered through `hound::WavWriter`'s internal `BufWriter`.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::error::{AppError, AppResult};

pub struct WavRecorder {
    writer: WavWriter<BufWriter<File>>,
}

impl WavRecorder {
    pub fn create(path: &Path, sample_rate: u32) -> AppResult<Self> {
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let writer = WavWriter::create(path, spec)
            .map_err(|e| AppError::Stream(format!("create {}: {e}", path.display())))?;
        Ok(Self { writer })
    }

    /// Append a block of interleaved stereo f32 samples. Length must be even.
    pub fn write_stereo(&mut self, samples: &[f32]) -> AppResult<()> {
        debug_assert!(samples.len() % 2 == 0, "stereo buffer must be even length");
        for &s in samples {
            self.writer
                .write_sample(s)
                .map_err(|e| AppError::Stream(format!("write wav: {e}")))?;
        }
        Ok(())
    }

    /// Force any buffered bytes to disk. Called periodically so a hard crash
    /// loses at most one flush interval of audio. Note: hound only updates the
    /// RIFF header on `finalize`, so a crash still leaves a file with an
    /// outdated `data` chunk size — `ffmpeg -i broken.wav fixed.wav` rebuilds
    /// it. The actual PCM bytes are intact.
    pub fn flush(&mut self) -> AppResult<()> {
        self.writer
            .flush()
            .map_err(|e| AppError::Stream(format!("flush wav: {e}")))
    }

    pub fn finalize(self) -> AppResult<()> {
        self.writer
            .finalize()
            .map_err(|e| AppError::Stream(format!("finalize wav: {e}")))
    }
}
