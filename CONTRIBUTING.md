# Contributing to Splitwave

Thanks for your interest in helping out.

## Reporting issues

Pick the right template in
[`.github/ISSUE_TEMPLATE`](./.github/ISSUE_TEMPLATE):

- **Bug** -- something's broken or misbehaving
- **Audio quality** -- glitches, clicks, distortion, dropouts, sync drift
- **Feature** -- new functionality or improvement
- **Crash** -- usually auto-filled by the in-app **Report on GitHub** button
  (it ships diagnostics: app version, OS version, thread, stack trace)

Before opening: search existing issues for duplicates. If unsure whether
something is a bug or expected, start a Discussion instead.

## Development setup

Prerequisites and commands are in the [README](./README.md). Quick version:

```bash
bun install
bun run tauri dev
```

## Coding conventions

The rules the codebase actually follows live in [`CLAUDE.md`](./CLAUDE.md).
Most important:

- **Real-time audio path** (`cpal` / SCK callbacks, `DspWorker::run`): no
  allocations, no locks, no syscalls. Shared state goes through
  `Arc<Atomic*>` cells.
- **Smallest viable change.** Avoid drive-by refactors mixed into feature
  work.
- **Comments only when the *why* is non-obvious.** Names handle the *what*.
- **Svelte 5 runes only.** No `export let`, no stores in component scope.
- **IDs** use `@paralleldrive/cuid2` (not nanoid, not uuid).
- **TS types** flow from Rust via `ts-rs` -- never hand-edit
  `src/lib/modules/pipeline/generated/`. `cargo test` regenerates them.

## Pull requests

- Run `bun run check` and
  `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` before
  pushing.
- Commit messages: `type(scope): subject`, lowercase, no trailing period.
- Keep PRs focused; one concern per PR.
- CI runs the same checks on macOS and must pass.
