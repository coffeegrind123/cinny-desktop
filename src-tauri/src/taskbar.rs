#[cfg(target_os = "windows")]
mod win {
    use std::cell::RefCell;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::WindowsAndMessaging::{
        LoadImageW, IMAGE_ICON, LR_LOADFROMFILE, HICON,
    };
    use windows::Win32::Foundation::HWND;

    thread_local! {
        static TASKBAR: RefCell<Option<ITaskbarList3>> = RefCell::new(None);
    }

    fn get_taskbar() -> Option<ITaskbarList3> {
        TASKBAR.with(|cell| {
            if cell.borrow().is_none() {
                unsafe {
                    *cell.borrow_mut() =
                        CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok();
                }
            }
            cell.borrow().clone()
        })
    }

    fn load_icon_from_bytes(data: &[u8]) -> Option<HICON> {
        let mut path = std::env::temp_dir();
        path.push(format!("prinny_badge_{}.ico", std::process::id()));

        if std::fs::write(&path, data).is_err() {
            return None;
        }

        let path_str = path.to_str()?;
        let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

        let handle = unsafe {
            LoadImageW(
                None,
                PCWSTR::from_raw(wide.as_ptr()),
                IMAGE_ICON,
                16,
                16,
                LR_LOADFROMFILE,
            )
        };

        let _ = std::fs::remove_file(&path);

        match handle {
            Ok(h) => {
                // h.0 is HANDLE(*mut c_void). Check != null, then cast to HICON.
                let p: *mut std::ffi::c_void = h.0;
                if p.is_null() {
                    None
                } else {
                    Some(HICON(p))
                }
            }
            Err(_) => None,
        }
    }

    /// hwnd_raw: raw HWND as isize (from tauri::Window::hwnd())
    pub fn set_overlay(hwnd_raw: isize, icon_data: Option<&[u8]>) {
        let taskbar = match get_taskbar() {
            Some(tb) => tb,
            None => return,
        };

        let hicon = icon_data
            .and_then(|data| load_icon_from_bytes(data))
            .unwrap_or(HICON(std::ptr::null_mut()));

        unsafe {
            let _ = taskbar.SetOverlayIcon(
                HWND(hwnd_raw as *mut _),
                hicon,
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
