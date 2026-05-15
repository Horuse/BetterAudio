# Splitwave

[![Downloads](https://img.shields.io/github/downloads/Horuse/Splitwave/total.svg)](https://github.com/Horuse/Splitwave/releases/latest)

![Splitwave preview](./preview.png)

Audio routing for macOS. Build a node graph of inputs, effects, and outputs;
the engine processes audio in real time with sample-accurate DSP and writes to
files in any of six formats.

## Features

- **Inputs:** microphones, system audio, per-application audio
  (ScreenCaptureKit), WAV files
- **Outputs:** physical speakers/interfaces, file recording in WAV (16/24-bit
  PCM + 32-float), FLAC, AIFF, Opus, MP3, AAC (M4A)
- **Effects:** Gain, Mute, Channel Balance, Saturator, 10-band Graphic EQ,
  Brick-wall Limiter with look-ahead, Compressor (with sidechain), Noise Gate
  (with sidechain), Stereo Delay, Algorithmic Reverb (Freeverb), Level Meter,
  EBU R128 LUFS Meter

## Stack

- **Frontend:** SvelteKit 5 (runes), Tauri 2, xyflow, Tailwind 4
- **Engine:** Rust -- `cpal` (device I/O), `rtrb` (SPSC ring buffers), `rubato`
  (resampling), `hound` (WAV), `flac-codec`, `opus`, `mp3lame-encoder`,
  `ebur128`
- **macOS-specific:** custom Swift static library for ScreenCaptureKit,
  compiled by `build.rs` via `swiftc`; CoreAudio HAL FFI for device
  enumeration

## Development

### Prerequisites

- macOS 13+ with Xcode Command Line Tools (`xcode-select --install`) -- gives
  you `swiftc` and the SDKs Tauri needs.
- [Rust](https://rustup.rs) (stable toolchain)
- [Bun](https://bun.sh) (`curl -fsSL https://bun.sh/install | bash`)

### Setup

```bash
bun install
bun run tauri dev
```

### Useful commands

```bash
bun run check                                # svelte-check + tsc
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml   # also regenerates ts-rs bindings
bun run tauri build --bundles app                # local .app build
```

### Project layout

```
src/lib/modules/
  audio/        cpal/SCK bridge, device enumeration, meter store
  flow/         xyflow node graph editor, sidebar, context menu
  pipeline/     pipeline state, snapshot history, ts-rs generated types
  form/         shared form primitives (combobox, slider)
  error/        global error modal (Rust panics + JS errors)
  updater/      auto-update modal + skip-version persistence
  app_info/     OS / app version cache
src-tauri/src/
  audio/        DSP engine, effects, encoders, pipeline DAG
  commands.rs   Tauri command surface
  lib.rs        app entry, plugin wiring, panic hook
```
