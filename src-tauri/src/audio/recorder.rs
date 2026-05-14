//! WAV file recorder for the `FileRecording` output node.
//!
//! Each `flush` patches the RIFF/fact/data chunk sizes so the file on disk
//! is always a valid WAV at the last flush boundary — no `ffmpeg` repair
//! after a process crash.
//!
//! Format: IEEE_FLOAT 32-bit stereo with the basic 18-byte `fmt` (not
//! `WAVE_FORMAT_EXTENSIBLE` — wider editor compatibility).

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::{AppError, AppResult};

const CHANNELS: u16 = 2;
const BITS_PER_SAMPLE: u16 = 32;
const FORMAT_IEEE_FLOAT: u16 = 3;
const BYTES_PER_SAMPLE: u32 = (BITS_PER_SAMPLE / 8) as u32;
const BLOCK_ALIGN: u16 = CHANNELS * (BITS_PER_SAMPLE / 8);

const HEADER_SIZE: u64 = 58;
const OFFSET_RIFF_SIZE: u64 = 4;
const OFFSET_FACT_SAMPLES: u64 = 46;
const OFFSET_DATA_SIZE: u64 = 54;

pub struct WavRecorder {
    inner: BufWriter<File>,
    samples_per_channel: u64,
}

impl WavRecorder {
    pub fn create(path: &Path, sample_rate: u32) -> AppResult<Self> {
        let file = File::create(path)
            .map_err(|e| AppError::Stream(format!("create {}: {e}", path.display())))?;
        let mut inner = BufWriter::new(file);
        write_header(&mut inner, sample_rate, 0)
            .map_err(|e| AppError::Stream(format!("write wav header: {e}")))?;
        Ok(Self {
            inner,
            samples_per_channel: 0,
        })
    }

    pub fn write_stereo(&mut self, samples: &[f32]) -> AppResult<()> {
        debug_assert!(samples.len() % 2 == 0, "stereo buffer must be even length");
        let mut buf = [0u8; 8];
        for pair in samples.chunks_exact(2) {
            buf[0..4].copy_from_slice(&pair[0].to_le_bytes());
            buf[4..8].copy_from_slice(&pair[1].to_le_bytes());
            self.inner
                .write_all(&buf)
                .map_err(|e| AppError::Stream(format!("write wav: {e}")))?;
        }
        self.samples_per_channel += (samples.len() / 2) as u64;
        Ok(())
    }

    pub fn flush(&mut self) -> AppResult<()> {
        self.inner
            .flush()
            .map_err(|e| AppError::Stream(format!("flush wav: {e}")))?;
        let data_size = self.samples_per_channel * (CHANNELS as u64) * (BYTES_PER_SAMPLE as u64);
        // WAV size fields are u32 — saturate (≈6 h of stereo float at 48 k).
        let data_size_u32 = u32::try_from(data_size).unwrap_or(u32::MAX);
        let riff_size_u32 = data_size_u32.saturating_add((HEADER_SIZE - 8) as u32);
        let samples_u32 = u32::try_from(self.samples_per_channel).unwrap_or(u32::MAX);

        let file = self.inner.get_mut();
        file.seek(SeekFrom::Start(OFFSET_RIFF_SIZE))
            .map_err(|e| AppError::Stream(format!("seek wav: {e}")))?;
        file.write_all(&riff_size_u32.to_le_bytes())
            .map_err(|e| AppError::Stream(format!("patch riff size: {e}")))?;
        file.seek(SeekFrom::Start(OFFSET_FACT_SAMPLES))
            .map_err(|e| AppError::Stream(format!("seek wav: {e}")))?;
        file.write_all(&samples_u32.to_le_bytes())
            .map_err(|e| AppError::Stream(format!("patch fact samples: {e}")))?;
        file.seek(SeekFrom::Start(OFFSET_DATA_SIZE))
            .map_err(|e| AppError::Stream(format!("seek wav: {e}")))?;
        file.write_all(&data_size_u32.to_le_bytes())
            .map_err(|e| AppError::Stream(format!("patch data size: {e}")))?;
        file.seek(SeekFrom::End(0))
            .map_err(|e| AppError::Stream(format!("seek wav end: {e}")))?;
        Ok(())
    }

    pub fn finalize(mut self) -> AppResult<()> {
        self.flush()
    }
}

fn write_header(w: &mut impl Write, sample_rate: u32, samples_per_channel: u32) -> std::io::Result<()> {
    let data_size = samples_per_channel.saturating_mul((CHANNELS as u32) * BYTES_PER_SAMPLE);
    let riff_size = data_size.saturating_add((HEADER_SIZE - 8) as u32);
    let byte_rate = sample_rate.saturating_mul((CHANNELS as u32) * BYTES_PER_SAMPLE);

    w.write_all(b"RIFF")?;
    w.write_all(&riff_size.to_le_bytes())?;
    w.write_all(b"WAVE")?;

    w.write_all(b"fmt ")?;
    w.write_all(&18u32.to_le_bytes())?;
    w.write_all(&FORMAT_IEEE_FLOAT.to_le_bytes())?;
    w.write_all(&CHANNELS.to_le_bytes())?;
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&byte_rate.to_le_bytes())?;
    w.write_all(&BLOCK_ALIGN.to_le_bytes())?;
    w.write_all(&BITS_PER_SAMPLE.to_le_bytes())?;
    w.write_all(&0u16.to_le_bytes())?;

    w.write_all(b"fact")?;
    w.write_all(&4u32.to_le_bytes())?;
    w.write_all(&samples_per_channel.to_le_bytes())?;

    w.write_all(b"data")?;
    w.write_all(&data_size.to_le_bytes())?;
    Ok(())
}
