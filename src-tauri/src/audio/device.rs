use std::collections::HashSet;

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[cfg(target_os = "macos")]
use crate::audio::macos_hal;

/// Direction of an audio device endpoint.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Input,
    Output,
}

/// Lightweight DTO sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
}

pub fn list_inputs() -> AppResult<Vec<DeviceInfo>> {
    #[cfg(target_os = "macos")]
    {
        return Ok(unique_named(
            macos_hal::list_input_devices().into_iter().map(|d| d.name).collect(),
            DeviceKind::Input,
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| AppError::Host(e.to_string()))?;
        Ok(collect_cpal(devices, DeviceKind::Input))
    }
}

pub fn list_outputs() -> AppResult<Vec<DeviceInfo>> {
    #[cfg(target_os = "macos")]
    {
        return Ok(unique_named(
            macos_hal::list_output_devices().into_iter().map(|d| d.name).collect(),
            DeviceKind::Output,
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| AppError::Host(e.to_string()))?;
        Ok(collect_cpal(devices, DeviceKind::Output))
    }
}

#[cfg(not(target_os = "macos"))]
fn collect_cpal<I: Iterator<Item = cpal::Device>>(devices: I, kind: DeviceKind) -> Vec<DeviceInfo> {
    devices
        .filter_map(|d| {
            let name = d.name().ok()?;
            Some(DeviceInfo {
                id: name.clone(),
                name,
                kind,
            })
        })
        .collect()
}

fn unique_named(names: Vec<String>, kind: DeviceKind) -> Vec<DeviceInfo> {
    let mut seen = HashSet::new();
    names
        .into_iter()
        .filter(|n| seen.insert(n.clone()))
        .map(|name| DeviceInfo {
            id: name.clone(),
            name,
            kind,
        })
        .collect()
}

/// Find a cpal device by name. We match purely on name — the cpal `Device` is
/// only used to build the stream (AUHAL on macOS), which doesn't need to know
/// or query the device's "current" config. Native sample rate and channel
/// count are resolved separately via the HAL layer on macOS.
///
/// We use `host.devices()` (every AudioDeviceID) rather than `output_devices()`
/// / `input_devices()` because those filter through cpal's active-format probe,
/// which silently drops non-default routes (e.g. internal speakers while
/// headphones are connected).
pub fn find(_kind: DeviceKind, id: &str) -> AppResult<cpal::Device> {
    let host = cpal::default_host();
    host.devices()
        .map_err(|e| AppError::Host(e.to_string()))?
        .find(|d| d.name().map(|n| n == id).unwrap_or(false))
        .ok_or_else(|| AppError::Device(format!("device not found: {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_inputs_without_panicking() {
        let inputs = list_inputs().expect("inputs");
        println!("found {} input device(s):", inputs.len());
        for d in &inputs {
            println!("  - {}", d.name);
        }
    }

    #[test]
    fn enumerates_outputs_without_panicking() {
        let outputs = list_outputs().expect("outputs");
        println!("found {} output device(s):", outputs.len());
        for d in &outputs {
            println!("  - {}", d.name);
        }
    }
}
