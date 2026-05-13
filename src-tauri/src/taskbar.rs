#[cfg(target_os = "windows")]
mod win {
    use std::sync::Mutex;
    use std::io::Write;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_ICON, LR_LOADFROMFILE};
    use windows::Win32::Graphics::Gdi::HICON;
    use windows::Win32::Foundation::HWND;

    static TASKBAR: Mutex<Option<ITaskbarList3>> = Mutex::new(None);

    fn get_taskbar() -> Option<ITaskbarList3> {
        let mut guard = TASKBAR.lock().unwrap();
        if guard.is_none() {
            unsafe {
                *guard = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok();
            }
        }
        *guard
    }

    fn load_icon_from_bytes(data: &[u8]) -> Option<HICON> {
        // Write ICO data to a temp file so LoadImageW can read it
        let mut path = std::env::temp_dir();
        path.push(format!("prinny_badge_{}.ico", std::process::id()));

        if std::fs::write(&path, data).is_err() {
            return None;
        }

        let path_str = path.to_str()?;
        let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

        let hicon = unsafe {
            LoadImageW(
                None,
                PCWSTR::from_raw(wide.as_ptr()),
                IMAGE_ICON,
                16,
                16,
                LR_LOADFROMFILE,
            )
        };

        // Best-effort cleanup — the file is small and in temp
        let _ = std::fs::remove_file(&path);

        match hicon {
            Ok(handle) if handle.0 != 0 => Some(HICON(handle.0)),
            _ => None,
        }
    }

    /// Set an overlay icon on the Windows taskbar button.
    /// Pass `None` to clear the overlay.
    pub fn set_overlay(hwnd: isize, icon_data: Option<&[u8]>) {
        let taskbar = match get_taskbar() {
            Some(tb) => tb,
            None => return,
        };

        let hicon = icon_data.and_then(|data| load_icon_from_bytes(data));

        unsafe {
            let _ = taskbar.SetOverlayIcon(
                HWND(hwnd as *mut _),
                hicon.unwrap_or(HICON::default()),
                PCWSTR::null(),
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win {
    pub fn set_overlay(_hwnd: isize, _icon_data: Option<&[u8]>) {}
}

pub use win::set_overlay;
