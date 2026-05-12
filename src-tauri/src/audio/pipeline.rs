//! Build and run a multi-input / multi-output pipeline.
//!
//! Each input is connected to N outputs via per-pair SPSC ring buffers carrying
//! interleaved stereo f32 at the **input device's** sample rate. The output side
//! resamples to the output device's rate (or the file recorder's fixed rate),
//! applies the per-bridge effect chain, and sums all bridges into the device or
//! file.
//!
//! Threads:
//! - One cpal input callback per input device (RT thread, broadcasts to N rings).
//! - One cpal output callback per Speaker output (RT thread, mixes + DSP).
//! - One worker thread per File Recording output (drains rings, mixes, writes).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

use crate::audio::device::{self, DeviceKind};
use crate::audio::effects::{build_chain, Effect};
use crate::audio::graph::{
    EffectChain, InputSpec, OutputSpec, ValidGraph, ValidInput, ValidOutput,
};
use crate::audio::recorder::WavRecorder;
use crate::audio::resample::StereoResampler;
use crate::audio::streams;
use crate::error::{AppError, AppResult};

const STATE_EVENT: &str = "audio://state";

/// Ring buffer length (in stereo f32 samples) per bridge. Chosen to absorb
/// ~250 ms of jitter at 48 kHz stereo (~24 000 samples) — plenty of headroom.
const RING_CAPACITY: usize = 24_000;

/// Block size used by the resampler. 256 frames @ 48 kHz ≈ 5.3 ms.
const RESAMPLE_CHUNK: usize = 256;

/// File recording sample rate. Fixed at 48 kHz for stereo lossless WAV.
const RECORDER_SR: u32 = 48_000;

/// File recorder worker loop pacing.
const RECORDER_POLL: Duration = Duration::from_millis(5);

/// Owned handles to every resource a started pipeline holds. `Drop` stops all
/// streams and joins the file recorder workers (in order).
pub struct ActivePipeline {
    _input_streams: Vec<InputHandle>,
    _speaker_streams: Vec<cpal::Stream>,
    _workers: Vec<RecorderWorker>,
}

/// Unified RAII handle for the different input source backends. The wrapped
/// value is held only for its `Drop` side-effect: the cpal stream stops on
/// drop, the SCK capture tears down the SCStream on drop.
#[allow(dead_code)]
enum InputHandle {
    Cpal(cpal::Stream),
    #[cfg(target_os = "macos")]
    Sck(crate::audio::sck_capture::SckCapture),
}

struct RecorderWorker {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Drop for RecorderWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Per (input, output) bridge state on the consumer side.
struct BridgeConsumer {
    consumer: Consumer<f32>,
    /// `None` when input SR == output SR. Resampling is skipped entirely.
    resampler: Option<StereoResampler>,
    /// Holds incoming samples (input SR) until we have a full resampler chunk.
    input_staging: Vec<f32>,
    /// Holds resampled-then-effected stereo samples (output SR) waiting to be
    /// mixed into the device or file block. FIFO queue, no allocations on push
    /// because we reserve capacity up front.
    output_staging: std::collections::VecDeque<f32>,
    effects: Vec<Box<dyn Effect>>,
    /// Scratch buffer for resampler output / pass-through chunks. Reused.
    chunk_tmp: Vec<f32>,
}

impl BridgeConsumer {
    fn new(
        consumer: Consumer<f32>,
        input_sr: u32,
        output_sr: u32,
        effects: &EffectChain,
    ) -> AppResult<Self> {
        let resampler = if input_sr == output_sr {
            None
        } else {
            Some(StereoResampler::new(input_sr, output_sr, RESAMPLE_CHUNK)?)
        };
        let out_max = resampler.as_ref().map(|r| r.out_max()).unwrap_or(RESAMPLE_CHUNK);
        Ok(Self {
            consumer,
            resampler,
            input_staging: Vec::with_capacity(RESAMPLE_CHUNK * 2 + 8),
            output_staging: std::collections::VecDeque::with_capacity(out_max * 4),
            effects: build_chain(effects),
            chunk_tmp: Vec::with_capacity(out_max * 2),
        })
    }

    /// Pull one frame (L,R) from the staging FIFO, refilling from the input
    /// ring if necessary. Returns [0,0] on underrun.
    #[inline]
    fn pop_frame(&mut self) -> [f32; 2] {
        if self.output_staging.len() < 2 {
            self.try_refill_one_chunk();
        }
        let l = self.output_staging.pop_front().unwrap_or(0.0);
        let r = self.output_staging.pop_front().unwrap_or(0.0);
        [l, r]
    }

    fn try_refill_one_chunk(&mut self) {
        if let Some(rs) = &mut self.resampler {
            let needed = rs.chunk_in() * 2;
            while self.input_staging.len() < needed {
                match self.consumer.pop() {
                    Ok(s) => self.input_staging.push(s),
                    Err(_) => return,
                }
            }
            self.chunk_tmp.clear();
            if rs
                .process_chunk(&self.input_staging[..needed], &mut self.chunk_tmp)
                .is_err()
            {
                self.input_staging.drain(..needed);
                return;
            }
            self.input_staging.drain(..needed);
        } else {
            // Pass-through path: no SR conversion.
            self.chunk_tmp.clear();
            for _ in 0..(RESAMPLE_CHUNK * 2) {
                match self.consumer.pop() {
                    Ok(s) => self.chunk_tmp.push(s),
                    Err(_) => break,
                }
            }
        }
        let frames = self.chunk_tmp.len() / 2;
        if frames == 0 {
            return;
        }
        // Truncate any odd-sample tail (orphan L without R) before effects/mix.
        self.chunk_tmp.truncate(frames * 2);
        for eff in &mut self.effects {
            eff.process(&mut self.chunk_tmp, frames);
        }
        self.output_staging.extend(self.chunk_tmp.iter().copied());
    }
}

/// Build everything from a validated graph and start playing.
pub fn build(graph: &ValidGraph, app: AppHandle) -> AppResult<ActivePipeline> {
    if graph.bridges.is_empty() {
        return Err(AppError::Validation(
            "no routing — connect at least one input to at least one output".into(),
        ));
    }

    // 1. Resolve every input device + its native config (only ones with bridges).
    let mut input_native_sr: HashMap<String, u32> = HashMap::new();
    let mut input_runtime: HashMap<String, ResolvedInput> = HashMap::new();
    for inp in &graph.inputs {
        if !graph.bridges.iter().any(|b| b.input_id == inp.id) {
            continue;
        }
        let resolved = resolve_input(inp)?;
        input_native_sr.insert(inp.id.clone(), resolved.sample_rate());
        input_runtime.insert(inp.id.clone(), resolved);
    }

    // 2. Resolve every output device + its native config.
    let mut output_runtime: HashMap<String, ResolvedOutput> = HashMap::new();
    for out in &graph.outputs {
        if !graph.bridges.iter().any(|b| b.output_id == out.id) {
            continue;
        }
        let resolved = resolve_output(out)?;
        output_runtime.insert(out.id.clone(), resolved);
    }

    // 3. Create per-bridge ring buffers.
    let mut producers_by_input: HashMap<String, Vec<Producer<f32>>> = HashMap::new();
    // BridgeConsumer is wrapped so we can move it into the output side.
    let mut consumers_by_output: HashMap<String, Vec<BridgeConsumer>> = HashMap::new();
    for bridge in &graph.bridges {
        let input_sr = *input_native_sr
            .get(&bridge.input_id)
            .ok_or_else(|| AppError::Validation("bridge references unknown input".into()))?;
        let output_sr = output_runtime
            .get(&bridge.output_id)
            .map(|o| o.sample_rate())
            .ok_or_else(|| AppError::Validation("bridge references unknown output".into()))?;

        let (producer, consumer) = RingBuffer::<f32>::new(RING_CAPACITY);
        producers_by_input
            .entry(bridge.input_id.clone())
            .or_default()
            .push(producer);
        let bc = BridgeConsumer::new(consumer, input_sr, output_sr, &bridge.effects)?;
        consumers_by_output
            .entry(bridge.output_id.clone())
            .or_default()
            .push(bc);
    }

    // 4. Start input streams (capturing into all subscriber rings).
    let mut input_streams = Vec::with_capacity(input_runtime.len());
    for (input_id, resolved) in input_runtime {
        let producers = producers_by_input
            .remove(&input_id)
            .unwrap_or_default();
        let stream = start_input_stream(resolved, producers, &app)?;
        input_streams.push(stream);
    }

    // 5. Start outputs (speaker streams + file recorder workers).
    let mut speaker_streams = Vec::new();
    let mut workers = Vec::new();
    for (output_id, resolved) in output_runtime {
        let consumers = consumers_by_output.remove(&output_id).unwrap_or_default();
        match resolved {
            ResolvedOutput::Speaker(spec) => {
                speaker_streams.push(start_speaker_stream(spec, consumers, &app)?);
            }
            ResolvedOutput::File { path, .. } => {
                workers.push(start_recorder_worker(path, consumers)?);
            }
        }
    }

    info!(
        inputs = input_streams.len(),
        speakers = speaker_streams.len(),
        recorders = workers.len(),
        bridges = graph.bridges.len(),
        "pipeline started"
    );

    Ok(ActivePipeline {
        _input_streams: input_streams,
        _speaker_streams: speaker_streams,
        _workers: workers,
    })
}

// ---------- input resolution ----------

enum ResolvedInput {
    Cpal {
        device: cpal::Device,
        config: cpal::StreamConfig,
        sample_format: cpal::SampleFormat,
        src_channels: usize,
        sample_rate: u32,
    },
    SystemAudio {
        sample_rate: u32,
        exclude_current_app: bool,
    },
    AppAudio {
        sample_rate: u32,
        bundle_id: String,
    },
}

impl ResolvedInput {
    fn sample_rate(&self) -> u32 {
        match self {
            ResolvedInput::Cpal { sample_rate, .. } => *sample_rate,
            ResolvedInput::SystemAudio { sample_rate, .. } => *sample_rate,
            ResolvedInput::AppAudio { sample_rate, .. } => *sample_rate,
        }
    }
}

fn resolve_input(inp: &ValidInput) -> AppResult<ResolvedInput> {
    match &inp.spec {
        InputSpec::Microphone { device_id } => {
            let device = device::find(DeviceKind::Input, device_id)?;
            let native = native_config(DeviceKind::Input, &device, device_id)?;
            Ok(ResolvedInput::Cpal {
                device,
                config: native.config,
                sample_format: native.sample_format,
                src_channels: native.channels as usize,
                sample_rate: native.sample_rate,
            })
        }
        InputSpec::SystemAudio {
            exclude_current_app,
        } => Ok(ResolvedInput::SystemAudio {
            sample_rate: RECORDER_SR,
            exclude_current_app: *exclude_current_app,
        }),
        InputSpec::AppAudio { bundle_id } => Ok(ResolvedInput::AppAudio {
            sample_rate: RECORDER_SR,
            bundle_id: bundle_id.clone(),
        }),
    }
}

fn start_input_stream(
    resolved: ResolvedInput,
    producers: Vec<Producer<f32>>,
    app: &AppHandle,
) -> AppResult<InputHandle> {
    let app_err = app.clone();
    let err_cb = move |e: cpal::StreamError| {
        let _ = app_err.emit(
            STATE_EVENT,
            json!({ "kind": "error", "message": format!("input: {e}") }),
        );
    };

    match resolved {
        ResolvedInput::Cpal {
            device,
            config,
            sample_format,
            src_channels,
            ..
        } => {
            let stream = streams::build_input_stream(
                &device,
                &config,
                sample_format,
                src_channels,
                producers,
                err_cb,
            )?;
            Ok(InputHandle::Cpal(stream))
        }
        #[cfg(target_os = "macos")]
        ResolvedInput::SystemAudio {
            sample_rate,
            exclude_current_app,
        } => {
            info!(
                sample_rate,
                exclude_current_app, "starting system-audio capture (ScreenCaptureKit)"
            );
            let capture = crate::audio::sck_capture::SckCapture::start_system(
                exclude_current_app,
                sample_rate,
                SCK_CHANNELS as u32,
                producers,
            )?;
            Ok(InputHandle::Sck(capture))
        }
        #[cfg(target_os = "macos")]
        ResolvedInput::AppAudio {
            sample_rate,
            bundle_id,
        } => {
            info!(sample_rate, %bundle_id, "starting app-audio capture (ScreenCaptureKit)");
            let capture = crate::audio::sck_capture::SckCapture::start_app(
                &bundle_id,
                sample_rate,
                SCK_CHANNELS as u32,
                producers,
            )?;
            Ok(InputHandle::Sck(capture))
        }
        #[cfg(not(target_os = "macos"))]
        ResolvedInput::SystemAudio { .. } | ResolvedInput::AppAudio { .. } => {
            drop(producers);
            Err(AppError::Stream(
                "System/App Audio capture is only supported on macOS".into(),
            ))
        }
    }
}

/// ScreenCaptureKit always delivers interleaved stereo by configuration.
const SCK_CHANNELS: usize = 2;

// ---------- output resolution ----------

struct SpeakerResolved {
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    out_channels: usize,
    sample_rate: u32,
}

enum ResolvedOutput {
    Speaker(SpeakerResolved),
    File {
        path: PathBuf,
        sample_rate: u32,
    },
}

impl ResolvedOutput {
    fn sample_rate(&self) -> u32 {
        match self {
            ResolvedOutput::Speaker(s) => s.sample_rate,
            ResolvedOutput::File { sample_rate, .. } => *sample_rate,
        }
    }
}

fn resolve_output(out: &ValidOutput) -> AppResult<ResolvedOutput> {
    match &out.spec {
        OutputSpec::Speaker { device_id } => {
            let device = device::find(DeviceKind::Output, device_id)?;
            let native = native_config(DeviceKind::Output, &device, device_id)?;
            Ok(ResolvedOutput::Speaker(SpeakerResolved {
                device,
                config: native.config,
                sample_format: native.sample_format,
                out_channels: native.channels as usize,
                sample_rate: native.sample_rate,
            }))
        }
        OutputSpec::FileRecording { file_path } => Ok(ResolvedOutput::File {
            path: PathBuf::from(file_path),
            sample_rate: RECORDER_SR,
        }),
    }
}

// ---------- native config resolution ----------
//
// We never ask cpal "what is this device's default/supported config?":
//   - `default_*_config` reads the *currently active* CoreAudio stream format,
//     which is absent for non-default routes (built-in speakers while AirPods
//     are connected) → "Invalid property value".
//   - `supported_*_configs` reads `kAudioStreamPropertyAvailableVirtualFormats`,
//     which is also empty for those same non-default routes.
//
// AUHAL (cpal's underlying output unit on macOS) does NOT need to be told the
// device's "current" format up front — it accepts whatever StreamConfig we
// hand it and asks CoreAudio to convert. So we read the device's nominal
// sample rate and channel count *directly* from CoreAudio HAL (which works
// regardless of routing state) and feed those into `build_*_stream`.
//
// Sample format is always `f32` — the universal macOS audio type and the
// internal pipeline format.

struct NativeConfig {
    config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    sample_rate: u32,
    channels: u16,
}

#[cfg(target_os = "macos")]
fn native_config(
    kind: DeviceKind,
    _device: &cpal::Device,
    name: &str,
) -> AppResult<NativeConfig> {
    use crate::audio::macos_hal;
    let hal = match kind {
        DeviceKind::Input => macos_hal::find_input_device(name),
        DeviceKind::Output => macos_hal::find_output_device(name),
    }
    .ok_or_else(|| {
        AppError::Device(format!(
            "{kind:?} device {name:?} disappeared between enumeration and open"
        ))
    })?;

    let channels: u16 = hal
        .channels
        .try_into()
        .map_err(|_| AppError::Device(format!("device {name:?} has {} channels (too many)", hal.channels)))?;

    Ok(NativeConfig {
        config: cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(hal.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        },
        sample_format: cpal::SampleFormat::F32,
        sample_rate: hal.sample_rate,
        channels,
    })
}

#[cfg(not(target_os = "macos"))]
fn native_config(
    kind: DeviceKind,
    device: &cpal::Device,
    name: &str,
) -> AppResult<NativeConfig> {
    // On Linux/Windows cpal's `supported_*_configs` is reliable for any device
    // the OS exposes — no inactive-route quirk like macOS. Pick the range with
    // the highest max sample rate; force f32 sample format.
    let configs: Vec<cpal::SupportedStreamConfigRange> = match kind {
        DeviceKind::Input => device
            .supported_input_configs()
            .map_err(|e| AppError::Device(format!("query input configs for {name:?}: {e}")))?
            .collect(),
        DeviceKind::Output => device
            .supported_output_configs()
            .map_err(|e| AppError::Device(format!("query output configs for {name:?}: {e}")))?
            .collect(),
    };
    let best = configs
        .into_iter()
        .max_by_key(|c| c.max_sample_rate().0)
        .ok_or_else(|| AppError::Device(format!("device {name:?} exposes no configs")))?
        .with_max_sample_rate();
    Ok(NativeConfig {
        config: cpal::StreamConfig {
            channels: best.channels(),
            sample_rate: best.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        },
        sample_format: cpal::SampleFormat::F32,
        sample_rate: best.sample_rate().0,
        channels: best.channels(),
    })
}

// ---------- output runtime: speakers ----------

fn start_speaker_stream(
    spec: SpeakerResolved,
    mut consumers: Vec<BridgeConsumer>,
    app: &AppHandle,
) -> AppResult<cpal::Stream> {
    info!(
        sample_rate = spec.sample_rate,
        channels = spec.out_channels,
        format = ?spec.sample_format,
        "opening speaker stream",
    );
    let app_err = app.clone();
    let err_cb = move |e: cpal::StreamError| {
        let _ = app_err.emit(
            STATE_EVENT,
            json!({ "kind": "error", "message": format!("output: {e}") }),
        );
    };

    let fill = move |stereo_out: &mut [f32], frames: usize| {
        // Zero first; we accumulate into this buffer.
        for s in stereo_out.iter_mut() {
            *s = 0.0;
        }
        for i in 0..frames {
            let mut l = 0.0_f32;
            let mut r = 0.0_f32;
            for bc in consumers.iter_mut() {
                let [bl, br] = bc.pop_frame();
                l += bl;
                r += br;
            }
            stereo_out[i * 2] = l;
            stereo_out[i * 2 + 1] = r;
        }
    };

    streams::build_output_stream(
        &spec.device,
        &spec.config,
        spec.sample_format,
        spec.out_channels,
        fill,
        err_cb,
    )
}

// ---------- output runtime: file recording ----------

fn start_recorder_worker(
    path: PathBuf,
    consumers: Vec<BridgeConsumer>,
) -> AppResult<RecorderWorker> {
    let mut recorder = WavRecorder::create(&path, RECORDER_SR)?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    let mut consumers = consumers;

    let join = thread::Builder::new()
        .name(format!("recorder:{}", path.display()))
        .spawn(move || {
            const BLOCK_FRAMES: usize = 1024;
            let mut block: Vec<f32> = vec![0.0; BLOCK_FRAMES * 2];
            while !stop_thread.load(Ordering::SeqCst) {
                // Fill one block by polling each bridge once per frame.
                for s in block.iter_mut() {
                    *s = 0.0;
                }
                let mut produced_any = false;
                for i in 0..BLOCK_FRAMES {
                    let mut l = 0.0_f32;
                    let mut r = 0.0_f32;
                    let mut bridge_has_data = false;
                    for bc in consumers.iter_mut() {
                        let [bl, br] = bc.pop_frame();
                        if bl != 0.0 || br != 0.0 {
                            bridge_has_data = true;
                        }
                        l += bl;
                        r += br;
                    }
                    block[i * 2] = l;
                    block[i * 2 + 1] = r;
                    if bridge_has_data {
                        produced_any = true;
                    }
                }
                if let Err(e) = recorder.write_stereo(&block) {
                    warn!(error = %e, "wav write failed; stopping recorder");
                    break;
                }
                if !produced_any {
                    // Inputs not feeding us yet → don't busy-loop.
                    thread::sleep(RECORDER_POLL);
                }
            }
            if let Err(e) = recorder.finalize() {
                warn!(error = %e, "wav finalize failed");
            }
        })
        .map_err(|e| AppError::Stream(format!("spawn recorder thread: {e}")))?;

    Ok(RecorderWorker {
        stop,
        join: Some(join),
    })
}

