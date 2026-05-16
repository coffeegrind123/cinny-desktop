#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// mod menu;

use tauri::{webview::{NewWindowResponse, WebviewWindowBuilder}, WebviewUrl};
use tauri_plugin_opener::OpenerExt;

mod taskbar;


// Embedded overlay icons for Windows taskbar badge (1-9, 9+)
#[cfg(target_os = "windows")]
const BADGE_ICONS: &[&[u8]] = &[
    &[], // index 0 unused
    include_bytes!("../icons/overlay/badge-1.ico"),
    include_bytes!("../icons/overlay/badge-2.ico"),
    include_bytes!("../icons/overlay/badge-3.ico"),
    include_bytes!("../icons/overlay/badge-4.ico"),
    include_bytes!("../icons/overlay/badge-5.ico"),
    include_bytes!("../icons/overlay/badge-6.ico"),
    include_bytes!("../icons/overlay/badge-7.ico"),
    include_bytes!("../icons/overlay/badge-8.ico"),
    include_bytes!("../icons/overlay/badge-9.ico"),
    include_bytes!("../icons/overlay/badge-9plus.ico"),
];

fn unifiedpush_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("unifiedpush")
        .setup(|_app, api| {
            #[cfg(target_os = "android")]
            {
                let _handle = api.register_android_plugin("in.cinny.app", "UnifiedPushPlugin")?;
            }
            #[cfg(not(target_os = "android"))]
            let _ = &api;
            Ok(())
        })
        .build()
}

fn foreground_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("foreground")
        .setup(|_app, api| {
            #[cfg(target_os = "android")]
            {
                let _handle = api.register_android_plugin("in.cinny.app", "ForegroundServicePlugin")?;
            }
            #[cfg(not(target_os = "android"))]
            let _ = &api;
            Ok(())
        })
        .build()
}

fn message_notification_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("messageNotification")
        .setup(|_app, api| {
            #[cfg(target_os = "android")]
            {
                let _handle = api.register_android_plugin("in.cinny.app", "MessageNotificationPlugin")?;
            }
            #[cfg(not(target_os = "android"))]
            let _ = &api;
            Ok(())
        })
        .build()
}

// Downloads a remote image (typically a Matrix sender/room avatar) and writes
// it to the OS app-cache directory. Returns the absolute path. Used by the
// notification frontend so platform code (notify-rust on desktop, our custom
// Kotlin plugin on Android) can pass a real file path to the toast — both
// notify-rust (Windows winrt-notification path) and Android's
// Notification.Builder.setLargeIcon require an actual file, not a data URI.
//
// The filename is a SHA-256 of the URL so repeat lookups hit the cache.
#[tauri::command]
async fn cache_notification_icon(
    app: tauri::AppHandle,
    url: String,
) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::fs;
    use tauri::Manager;

    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(&hasher.finalize()[..16]);

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("app_cache_dir: {e}"))?;
    let icons_dir = cache_dir.join("notif-icons");
    fs::create_dir_all(&icons_dir).map_err(|e| format!("create_dir_all: {e}"))?;

    let file_path = icons_dir.join(format!("{hash}.img"));
    if file_path.exists() {
        return Ok(file_path.to_string_lossy().to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (compatible; PrinnyNotificationIcon/1.0)",
        )
        .build()
        .map_err(|e| format!("client: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("bytes: {e}"))?;
    fs::write(&file_path, &bytes).map_err(|e| format!("write: {e}"))?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
fn set_badge_count(window: tauri::Window, count: u32) {
    #[cfg(target_os = "windows")]
    {
        let idx = if count == 0 {
            None
        } else if count >= 10 {
            Some(10usize) // badge-9plus
        } else {
            Some(count as usize)
        };
        if let Ok(hwnd) = window.hwnd() {
            taskbar::set_overlay(hwnd.0 as isize, idx.map(|i| BADGE_ICONS[i]));
        }
        return;
    }

    #[cfg(not(any(target_os = "windows", target_os = "android", target_os = "ios")))]
    {
        if count > 0 {
            let _ = window.set_badge_count(Some(count.into()));
        } else {
            let _ = window.set_badge_count(None::<i64>);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let port: u16 = 44548;
    let context = tauri::generate_context!();
    let mut builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            set_badge_count,
            cache_notification_icon,
        ])
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_mobile_push::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(unifiedpush_plugin())
        .plugin(foreground_plugin())
        .plugin(message_notification_plugin());

    #[cfg(not(mobile))]
    {
        builder = builder
            .plugin(tauri_plugin_window_state::Builder::default().build())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .setup(move |app| {
            // Dev: use devUrl from tauri.conf.json (http://localhost:8080) to support HMR
            #[cfg(debug_assertions)]
            let window_url = WebviewUrl::App(Default::default());

            // Release: tauri-plugin-localhost serves bundled frontend assets on this port
            #[cfg(not(debug_assertions))]
            let window_url = {
                let url = format!("http://localhost:{}", port).parse().unwrap();
                WebviewUrl::External(url)
            };

            let app_handle = app.handle().clone();
            let mut window_builder = WebviewWindowBuilder::new(app, "main".to_string(), window_url);

            #[cfg(not(mobile))]
            {
                window_builder = window_builder.title("Cinny");
            }

            #[cfg(not(mobile))]
            {
                window_builder = window_builder.inner_size(800.0, 790.0);
            }

            // Disable Tauri's native drag-drop handler so the WebView fires browser
            // dragenter/dragover/drop events normally. The frontend listens for these
            // at the app root (useGlobalDropListener) to route dropped files into
            // the open conversation's attachment list.
            window_builder = window_builder.disable_drag_drop_handler();

            window_builder
                .on_new_window(move |url, _features| {
                    // blob: URLs are internal to the webview, skip external open
                    if url.scheme() != "blob" {
                        let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    }
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
