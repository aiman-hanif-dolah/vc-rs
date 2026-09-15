//! Control-thread device discovery shared by the GUI and plugin editor.
//! Never poll or drop this object on an audio callback or during plugin scan.

use crate::Provider;

#[derive(Clone, Debug, Default)]
pub enum DeviceStatus {
    #[default]
    NotStarted,
    Checking,
    DownloadRequired,
    Downloading,
    Unknown(String),
    Detected(Vec<Provider>),
}

impl DeviceStatus {
    pub fn show_picker(&self) -> bool {
        matches!(self, Self::Detected(_) | Self::Unknown(_))
    }

    pub fn availability(&self, provider: Provider) -> Option<bool> {
        // The legacy unrestricted setting is not a hardware type. Its display
        // must not become "unavailable" merely because it is no longer a choice.
        if !Provider::OPENVINO_DEVICES.contains(&provider) {
            return None;
        }
        match self {
            Self::Detected(devices) => Some(devices.contains(&provider)),
            _ => None,
        }
    }

    pub fn label(&self, provider: Provider) -> &'static str {
        match self.availability(provider) {
            Some(true) => "Available",
            Some(false) => "Unavailable",
            None => "Unverified",
        }
    }
}

#[derive(Default)]
pub struct DeviceDiscovery {
    pub status: DeviceStatus,
    worker: Option<std::thread::JoinHandle<Result<DeviceStatus, String>>>,
    allow_download: bool,
}

impl DeviceDiscovery {
    /// Non-blocking UI poll. Automatic discovery never acquires a missing EP.
    /// Only an explicit download action authorizes acquisition, including retries.
    /// The queued action runs here so UI tests can exercise clicks without I/O.
    pub fn poll(&mut self) {
        if self.worker.is_none()
            && matches!(
                self.status,
                DeviceStatus::NotStarted | DeviceStatus::Checking | DeviceStatus::Downloading
            )
        {
            self.start();
        }
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            let result = self.worker.take().expect("finished worker").join();
            self.status = match result {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => DeviceStatus::Unknown(error),
                Err(_) => DeviceStatus::Unknown("OpenVINO device discovery failed".into()),
            };
        }
    }

    pub fn download_and_check(&mut self) {
        if self.worker.is_none() && matches!(self.status, DeviceStatus::DownloadRequired) {
            self.allow_download = true;
            self.status = DeviceStatus::Downloading;
        }
    }

    pub fn retry(&mut self) {
        if self.worker.is_none() && matches!(self.status, DeviceStatus::Unknown(_)) {
            self.status = if self.allow_download {
                DeviceStatus::Downloading
            } else {
                DeviceStatus::Checking
            };
        }
    }

    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        self.status = if self.allow_download {
            DeviceStatus::Downloading
        } else {
            DeviceStatus::Checking
        };
        let allow_download = self.allow_download;
        match std::thread::Builder::new()
            .name("openvino-devices".into())
            .spawn(move || discover(allow_download))
        {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => self.status = DeviceStatus::Unknown(error.to_string()),
        }
    }
}

impl Drop for DeviceDiscovery {
    fn drop(&mut self) {
        // A plugin must not unload its code while a discovery thread runs in it.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn discover(allow_download: bool) -> Result<DeviceStatus, String> {
    #[cfg(all(windows, feature = "windowsml"))]
    {
        crate::windows_ml::probe_openvino_devices(allow_download)
            .map(|devices| devices.map_or(DeviceStatus::DownloadRequired, DeviceStatus::Detected))
            .map_err(|error| format!("{error:#}"))
    }
    #[cfg(not(all(windows, feature = "windowsml")))]
    {
        let _ = allow_download;
        Err("Windows ML is unavailable in this build".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(windows, feature = "windowsml"))]
    #[test]
    #[ignore = "requires an installed OpenVINO EP; prepares it without downloading"]
    fn openvino_installed_device_probe() {
        let DeviceStatus::Detected(devices) =
            discover(false).expect("installed OpenVINO devices can be enumerated")
        else {
            panic!("OpenVINO must be installed for this diagnostic");
        };
        eprintln!("Detected OpenVINO device types: {devices:?}");
        assert!(devices
            .iter()
            .all(|provider| Provider::OPENVINO_DEVICES.contains(provider)));
    }

    #[test]
    fn download_requires_action_and_retry_preserves_authorization() {
        let mut discovery = DeviceDiscovery {
            status: DeviceStatus::DownloadRequired,
            worker: None,
            allow_download: false,
        };
        discovery.poll();
        assert!(discovery.worker.is_none());
        assert!(!discovery.allow_download);
        assert!(!discovery.status.show_picker());
        discovery.download_and_check();
        assert!(discovery.allow_download);
        assert!(matches!(discovery.status, DeviceStatus::Downloading));
        discovery.status = DeviceStatus::Unknown("network failure".into());
        discovery.retry();
        assert!(matches!(discovery.status, DeviceStatus::Downloading));
        let mut automatic = DeviceDiscovery {
            status: DeviceStatus::Unknown("enumeration failure".into()),
            worker: None,
            allow_download: false,
        };
        automatic.retry();
        assert!(!automatic.allow_download);
        assert!(matches!(automatic.status, DeviceStatus::Checking));
    }

    #[test]
    fn unknown_does_not_disable_hardware_and_detected_list_does() {
        let status = DeviceStatus::Unknown("EP not prepared".into());
        assert_eq!(status.availability(Provider::WindowsMlOpenVinoNpu), None);
        let status = DeviceStatus::Detected(vec![Provider::WindowsMlOpenVinoCpu]);
        assert_eq!(status.availability(Provider::WindowsMlOpenVino), None);
        assert_eq!(
            status.availability(Provider::WindowsMlOpenVinoCpu),
            Some(true)
        );
        assert_eq!(
            status.availability(Provider::WindowsMlOpenVinoNpu),
            Some(false)
        );
        assert!(!Provider::OPENVINO_DEVICES.contains(&Provider::WindowsMlOpenVino));
        assert_eq!(
            Provider::from_name("windowsml-openvino"),
            Some(Provider::WindowsMlOpenVino)
        );
    }
}
