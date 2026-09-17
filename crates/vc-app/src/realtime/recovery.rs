use super::{DeviceList, RealtimeConfig};
use std::time::{Duration, Instant};

const RETRY_INTERVAL: Duration = Duration::from_secs(3);
const MAX_OPEN_ATTEMPTS: usize = 3;

pub(super) struct DeviceRecovery {
    pub(super) config: RealtimeConfig,
    next_check: Instant,
    attempts: usize,
}

impl DeviceRecovery {
    pub(super) fn new(config: RealtimeConfig, now: Instant) -> Option<Self> {
        // Reopening a default/partial route could select another device. Debug
        // capture files also must never be reopened and overwritten by recovery.
        if config
            .input_device
            .as_ref()
            .is_none_or(|name| name.is_empty())
            || config
                .output_device
                .as_ref()
                .is_none_or(|name| name.is_empty())
            || config.debug_input_wav.is_some()
            || config.debug_output_wav.is_some()
        {
            return None;
        }
        Some(Self {
            config,
            next_check: now + RETRY_INTERVAL,
            attempts: 0,
        })
    }

    pub(super) fn due(&self, now: Instant) -> bool {
        now >= self.next_check
    }

    pub(super) fn ready(&mut self, devices: &DeviceList, now: Instant) -> bool {
        self.next_check = now + RETRY_INTERVAL;
        let unique = |names: &[String], selected: Option<&String>| {
            selected.is_some_and(|selected| {
                names.iter().filter(|name| *name == selected).count() == 1
                    && names
                        .iter()
                        .filter(|name| name.to_lowercase().contains(&selected.to_lowercase()))
                        .count()
                        == 1
            })
        };
        devices.error.is_none()
            && unique(&devices.inputs, self.config.input_device.as_ref())
            && unique(&devices.outputs, self.config.output_device.as_ref())
            && self
                .config
                .monitor_device
                .as_ref()
                .is_none_or(|monitor| unique(&devices.outputs, Some(monitor)))
    }

    pub(super) fn failed(&mut self, now: Instant) -> bool {
        self.attempts += 1;
        self.next_check = now + RETRY_INTERVAL;
        self.attempts >= MAX_OPEN_ATTEMPTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> RealtimeConfig {
        RealtimeConfig {
            input_device: Some("Mic".into()),
            output_device: Some("Cable".into()),
            monitor_device: Some("Headphones".into()),
            ..Default::default()
        }
    }

    #[test]
    fn waits_for_unique_exact_devices_including_monitor() {
        let now = Instant::now();
        let mut recovery = DeviceRecovery::new(config(), now).unwrap();
        assert!(!recovery.due(now));
        assert!(recovery.due(now + RETRY_INTERVAL));
        let mut devices = DeviceList {
            inputs: vec!["Mic".into()],
            outputs: vec!["Cable".into()],
            error: None,
        };
        assert!(!recovery.ready(&devices, now));
        devices.outputs.push("Headphones".into());
        assert!(recovery.ready(&devices, now));
        devices.inputs.push("Mic".into());
        assert!(!recovery.ready(&devices, now));
        devices.inputs = vec!["Mic replacement".into()];
        assert!(!recovery.ready(&devices, now));
    }

    #[test]
    fn defaults_and_debug_capture_never_auto_reopen() {
        let now = Instant::now();
        assert!(DeviceRecovery::new(RealtimeConfig::default(), now).is_none());
        let mut capture = config();
        capture.debug_output_wav = Some("capture.wav".into());
        assert!(DeviceRecovery::new(capture, now).is_none());
    }

    #[test]
    fn open_failures_are_bounded() {
        let now = Instant::now();
        let mut recovery = DeviceRecovery::new(config(), now).unwrap();
        assert!(!recovery.failed(now));
        assert!(!recovery.failed(now));
        assert!(recovery.failed(now));
    }

    #[test]
    fn waiting_does_not_consume_open_attempts_and_respects_poll_interval() {
        let now = Instant::now();
        let mut recovery = DeviceRecovery::new(config(), now).unwrap();
        let unavailable = DeviceList::default();
        for index in 1..10 {
            let check = now + RETRY_INTERVAL * index;
            assert!(recovery.due(check));
            assert!(!recovery.ready(&unavailable, check));
            assert!(!recovery.due(check));
        }
        assert!(!recovery.failed(now));
        assert!(!recovery.failed(now));
        assert!(recovery.failed(now));
    }

    #[test]
    fn enumeration_errors_prevent_start_even_with_matching_names() {
        let now = Instant::now();
        let mut recovery = DeviceRecovery::new(config(), now).unwrap();
        let devices = DeviceList {
            inputs: vec!["Mic".into()],
            outputs: vec!["Cable".into(), "Headphones".into()],
            error: Some("Device enumeration failed".into()),
        };
        assert!(!recovery.ready(&devices, now));
    }
}
