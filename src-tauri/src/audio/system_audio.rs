//! NSWorkspace-backed enumeration of running applications used to populate the
//! "App Audio" picker. Actual audio capture lives in `audio/sck_capture.rs`
//! (Swift bridge to ScreenCaptureKit).

use serde::Serialize;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize)]
pub struct AudioApplication {
    #[serde(rename = "bundleId")]
    pub bundle_id: String,
    pub name: String,
}

#[cfg(target_os = "macos")]
pub fn list_audio_applications() -> AppResult<Vec<AudioApplication>> {
    Ok(macos::list_running_applications())
}

#[cfg(not(target_os = "macos"))]
pub fn list_audio_applications() -> AppResult<Vec<AudioApplication>> {
    Ok(Vec::new())
}

#[cfg(target_os = "macos")]
mod macos {
    use std::collections::HashSet;

    use objc2_app_kit::{NSRunningApplication, NSWorkspace};

    use super::AudioApplication;

    pub fn list_running_applications() -> Vec<AudioApplication> {
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(apps.len());
        for app in apps.iter() {
            let Some(bundle_id) = bundle_identifier(&app) else {
                continue;
            };
            let name = localized_name(&app).unwrap_or_else(|| bundle_id.clone());
            // Dedupe — multiple `NSRunningApplication` entries can share a bundle id
            // (helpers, login items). The user just wants one entry per app.
            if seen.insert(bundle_id.clone()) {
                out.push(AudioApplication { bundle_id, name });
            }
        }
        // Sort by display name for predictable UI ordering.
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out
    }

    fn bundle_identifier(app: &NSRunningApplication) -> Option<String> {
        app.bundleIdentifier().map(|s| s.to_string())
    }

    fn localized_name(app: &NSRunningApplication) -> Option<String> {
        app.localizedName().map(|s| s.to_string())
    }
}
