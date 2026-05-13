//! Pacing source for the DSP worker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

pub trait ClockSource: Send + 'static {
    /// Returns `false` when `stop` is set; `true` on each tick.
    fn wait_for_tick(&mut self, stop: &AtomicBool) -> bool;

    /// Nominal sample rate this clock targets.
    #[allow(dead_code)]
    fn sample_rate(&self) -> u32;
}

/// On overrun, the next deadline resets to "now" rather than bursting through
/// accumulated ticks (which would put rings straight back into desync).
pub struct SystemClockTicker {
    #[allow(dead_code)]
    sample_rate: u32,
    period: Duration,
    next_deadline: Option<Instant>,
}

impl SystemClockTicker {
    pub fn new(sample_rate: u32, block_frames: usize) -> Self {
        let period = Duration::from_nanos(
            (block_frames as u64 * 1_000_000_000) / sample_rate.max(1) as u64,
        );
        Self {
            sample_rate,
            period,
            next_deadline: None,
        }
    }
}

impl ClockSource for SystemClockTicker {
    fn wait_for_tick(&mut self, stop: &AtomicBool) -> bool {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        let now = Instant::now();
        let anchor = match self.next_deadline {
            Some(d) if d > now => {
                thread::sleep(d - now);
                d
            }
            _ => now,
        };
        self.next_deadline = Some(anchor + self.period);
        !stop.load(Ordering::SeqCst)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
