//! Rust binding for the Swift `SCKAudioCapture` static library (native/).
//!
//! Architecture: the Swift side owns the `SCStream` and its delegate; on each
//! audio `CMSampleBuffer` it interleaves f32 samples and invokes our C
//! callback, which pushes them into a `rtrb::Producer<f32>` shared with the
//! pipeline engine.
//!
//! All public functions block on Swift's async start completion via a
//! `DispatchSemaphore` on the Swift side; the wait is bounded (10 s).

use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

use rtrb::Producer;

use crate::error::{AppError, AppResult};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultCode {
    Ok = 0,
    OsVersion = 1,
    PermissionDenied = 2,
    AppNotFound = 3,
    StreamError = 4,
    Internal = 5,
}

impl ResultCode {
    fn from_raw(v: i32) -> Self {
        match v {
            0 => ResultCode::Ok,
            1 => ResultCode::OsVersion,
            2 => ResultCode::PermissionDenied,
            3 => ResultCode::AppNotFound,
            4 => ResultCode::StreamError,
            _ => ResultCode::Internal,
        }
    }

    fn into_error(self, context: &str) -> AppError {
        let msg = match self {
            ResultCode::Ok => return AppError::Stream(format!("{context}: unexpected Ok in error path")),
            ResultCode::OsVersion => "macOS 13.0+ required for ScreenCaptureKit",
            ResultCode::PermissionDenied => {
                "Screen Recording permission denied — enable it in System Settings → Privacy & Security → Screen Recording"
            }
            ResultCode::AppNotFound => "selected application is not running or has no audio",
            ResultCode::StreamError => "ScreenCaptureKit stream failed to start",
            ResultCode::Internal => "ScreenCaptureKit internal error (timeout)",
        };
        AppError::Stream(format!("{context}: {msg}"))
    }
}

type SampleCallback = extern "C" fn(
    user_data: *mut c_void,
    samples: *const f32,
    frames: i32,
    channels: i32,
);

extern "C" {
    fn ba_sck_create() -> *mut c_void;
    fn ba_sck_destroy(handle: *mut c_void);
    fn ba_sck_start_app(
        handle: *mut c_void,
        bundle_id: *const c_char,
        sample_rate: i32,
        channels: i32,
        callback: SampleCallback,
        user_data: *mut c_void,
    ) -> i32;
    fn ba_sck_start_system(
        handle: *mut c_void,
        exclude_current_app: i32,
        sample_rate: i32,
        channels: i32,
        callback: SampleCallback,
        user_data: *mut c_void,
    ) -> i32;
    fn ba_sck_stop(handle: *mut c_void);
}

/// RAII handle owning the live SCStream + ring producer state.
pub struct SckCapture {
    handle: *mut c_void,
    /// Heap-allocated state passed as `user_data` to the Swift callback. Lives
    /// at least until Drop stops the stream, at which point Swift has
    /// guaranteed no further callbacks will fire.
    _state: Box<CallbackState>,
}

unsafe impl Send for SckCapture {}

struct CallbackState {
    /// One producer per bridge that subscribes to this SCK source. Wrapped in
    /// `Mutex` only because rtrb's `Producer<f32>` is `!Sync` — SCK delivers
    /// buffers on a serial dispatch queue, so the mutex is uncontended.
    producers: Mutex<Vec<Producer<f32>>>,
}

extern "C" fn sample_trampoline(
    user_data: *mut c_void,
    samples: *const f32,
    frames: i32,
    channels: i32,
) {
    if user_data.is_null() || samples.is_null() || frames <= 0 || channels <= 0 {
        return;
    }
    let state = unsafe { &*(user_data as *const CallbackState) };
    let n = (frames as usize) * (channels as usize);
    let slice = unsafe { std::slice::from_raw_parts(samples, n) };
    let Ok(mut producers) = state.producers.lock() else {
        return;
    };
    for prod in producers.iter_mut() {
        for &sample in slice {
            if prod.push(sample).is_err() {
                // Ring full — consumer hasn't drained yet. Drop the rest of
                // this block for this subscriber; the engine catches up next
                // callback.
                break;
            }
        }
    }
}

impl SckCapture {
    /// Start capturing audio from one application (by bundle identifier),
    /// fanning out the captured samples to every supplied producer ring.
    pub fn start_app(
        bundle_id: &str,
        sample_rate: u32,
        channels: u32,
        producers: Vec<Producer<f32>>,
    ) -> AppResult<Self> {
        let handle = unsafe { ba_sck_create() };
        if handle.is_null() {
            return Err(AppError::Stream("ScreenCaptureKit requires macOS 13.0+".into()));
        }

        let state = Box::new(CallbackState {
            producers: Mutex::new(producers),
        });
        let state_ptr = state.as_ref() as *const CallbackState as *mut c_void;

        let bundle_cstr = CString::new(bundle_id)
            .map_err(|_| AppError::Validation("bundle id contains nul byte".into()))?;

        let code = unsafe {
            ba_sck_start_app(
                handle,
                bundle_cstr.as_ptr(),
                sample_rate as i32,
                channels as i32,
                sample_trampoline,
                state_ptr,
            )
        };
        let rc = ResultCode::from_raw(code);
        if rc != ResultCode::Ok {
            unsafe { ba_sck_destroy(handle) };
            return Err(rc.into_error(&format!("app audio capture ({bundle_id})")));
        }

        Ok(SckCapture {
            handle,
            _state: state,
        })
    }

    /// Start capturing system-wide audio. When `exclude_current_app` is true,
    /// our own process's audio is omitted from the mix (prevents feedback loops
    /// when System Audio → speaker is wired through BetterAudio itself).
    pub fn start_system(
        exclude_current_app: bool,
        sample_rate: u32,
        channels: u32,
        producers: Vec<Producer<f32>>,
    ) -> AppResult<Self> {
        let handle = unsafe { ba_sck_create() };
        if handle.is_null() {
            return Err(AppError::Stream("ScreenCaptureKit requires macOS 13.0+".into()));
        }

        let state = Box::new(CallbackState {
            producers: Mutex::new(producers),
        });
        let state_ptr = state.as_ref() as *const CallbackState as *mut c_void;

        let code = unsafe {
            ba_sck_start_system(
                handle,
                if exclude_current_app { 1 } else { 0 },
                sample_rate as i32,
                channels as i32,
                sample_trampoline,
                state_ptr,
            )
        };
        let rc = ResultCode::from_raw(code);
        if rc != ResultCode::Ok {
            unsafe { ba_sck_destroy(handle) };
            return Err(rc.into_error("system audio capture"));
        }

        Ok(SckCapture {
            handle,
            _state: state,
        })
    }
}

impl Drop for SckCapture {
    fn drop(&mut self) {
        unsafe {
            ba_sck_stop(self.handle);
            ba_sck_destroy(self.handle);
        }
    }
}
