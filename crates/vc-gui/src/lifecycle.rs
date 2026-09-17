use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

#[derive(Default)]
pub(crate) struct CloseGuard {
    pending: bool,
    confirmed: bool,
}

impl CloseGuard {
    pub(crate) fn request(&mut self, state: vc_app::EngineState) -> bool {
        if self.confirmed {
            return false;
        }
        self.pending = matches!(
            state,
            vc_app::EngineState::Starting
                | vc_app::EngineState::Running
                | vc_app::EngineState::Stopping
        );
        self.pending
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending
    }

    pub(crate) fn cancel(&mut self) {
        self.pending = false;
        self.confirmed = false;
    }

    pub(crate) fn confirm(&mut self) {
        self.pending = false;
        self.confirmed = true;
    }
}

pub(crate) fn acquire_instance(path: &Path) -> io::Result<Option<File>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    // The OS releases this lock on exit/crash; a leftover file is not a lock.
    match file.try_lock() {
        Ok(()) => {
            std::fs::write(path.with_extension("pid"), std::process::id().to_string())?;
            Ok(Some(file))
        }
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

pub(crate) fn activate_instance(path: &Path) {
    #[cfg(windows)]
    if let Some(pid) = std::fs::read_to_string(path.with_extension("pid"))
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
    {
        // EnumWindows invokes the callback synchronously; only the owner of the
        // held instance lock is eligible, not another app with the same title.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
                Some(activate_window),
                pid as isize,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = path;
}

#[cfg(windows)]
unsafe extern "system" fn activate_window(
    window: windows_sys::Win32::Foundation::HWND,
    pid: isize,
) -> windows_sys::core::BOOL {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, SetForegroundWindow, ShowWindow,
        SW_RESTORE, SW_SHOW,
    };
    let mut owner = 0;
    let mut title = [0_u16; 64];
    // Buffers remain alive throughout these synchronous Win32 calls.
    unsafe {
        GetWindowThreadProcessId(window, &mut owner);
        if owner != pid as u32 {
            return 1;
        }
        let length = GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
        if String::from_utf16_lossy(&title[..length.max(0) as usize]) != "Sooara" {
            return 1;
        }
        ShowWindow(
            window,
            if IsIconic(window) != 0 {
                SW_RESTORE
            } else {
                SW_SHOW
            },
        );
        SetForegroundWindow(window);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_audio_requires_explicit_exit_confirmation() {
        for state in [
            vc_app::EngineState::Starting,
            vc_app::EngineState::Running,
            vc_app::EngineState::Stopping,
        ] {
            let mut guard = CloseGuard::default();
            assert!(guard.request(state));
            assert!(guard.pending());
            guard.cancel();
            assert!(!guard.pending());
            assert!(guard.request(state));
            guard.confirm();
            assert!(!guard.request(state));
            guard.cancel();
            assert!(guard.request(state));
        }
    }

    #[test]
    fn inactive_audio_does_not_block_exit() {
        for state in [vc_app::EngineState::Stopped, vc_app::EngineState::Error] {
            let mut guard = CloseGuard::default();
            assert!(!guard.request(state));
            assert!(!guard.pending());
        }
    }

    #[test]
    fn instance_lock_excludes_duplicates_and_releases_on_drop() {
        let path = std::env::temp_dir().join(format!(
            "sooara-instance-{}-{}.lock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = acquire_instance(&path).unwrap().unwrap();
        assert!(acquire_instance(&path).unwrap().is_none());
        drop(first);
        let second = acquire_instance(&path).unwrap().unwrap();
        drop(second);
        std::fs::remove_file(path.with_extension("pid")).unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
