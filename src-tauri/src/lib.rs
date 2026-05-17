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
                let _handle = api.register_android_plugin("in.prinny.app", "UnifiedPushPlugin")?;
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
                let _handle = api.register_android_plugin("in.prinny.app", "ForegroundServicePlugin")?;
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
                let _handle = api.register_android_plugin("in.prinny.app", "MessageNotificationPlugin")?;
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
    auth_header: Option<String>,
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

    // Hit cache by checking for any file that matches the hash with a real
    // image extension. Old `.img` entries (without a recognized extension)
    // are deliberately skipped so they get re-fetched with a proper ext —
    // Windows toast won't render `<image src="file:///…/foo.img" />`.
    for ext in ["png", "jpg", "jpeg", "gif", "webp", "bmp"] {
        let candidate = icons_dir.join(format!("{hash}.{ext}"));
        if candidate.exists() {
            return Ok(candidate.to_string_lossy().to_string());
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (compatible; PrinnyNotificationIcon/1.0)",
        )
        .build()
        .map_err(|e| format!("client: {e}"))?;
    let mut req = client.get(&url);
    if let Some(auth) = auth_header.filter(|s| !s.is_empty()) {
        req = req.header(reqwest::header::AUTHORIZATION, auth);
    }
    let resp = req.send().await.map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').next())
        .map(|s| s.trim().to_ascii_lowercase());
    let bytes = resp.bytes().await.map_err(|e| format!("bytes: {e}"))?;

    let ext = match content_type.as_deref() {
        Some("image/jpeg") | Some("image/jpg") | Some("image/pjpeg") => "jpg",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        Some("image/bmp") => "bmp",
        Some("image/png") => "png",
        _ => match bytes.first_chunk::<4>() {
            // sniff magic bytes when the server didn't tell us
            Some([0xFF, 0xD8, 0xFF, _]) => "jpg",
            Some([0x89, 0x50, 0x4E, 0x47]) => "png",
            Some([0x47, 0x49, 0x46, _]) => "gif",
            Some([0x52, 0x49, 0x46, 0x46]) => "webp",
            Some([0x42, 0x4D, _, _]) => "bmp",
            _ => "png",
        },
    };

    let file_path = icons_dir.join(format!("{hash}.{ext}"));
    fs::write(&file_path, &bytes).map_err(|e| format!("write: {e}"))?;

    Ok(file_path.to_string_lossy().to_string())
}

// Reads a file dropped onto the window via Tauri's native drag-drop event and
// returns its bytes plus inferred MIME. Used in place of WebView2's HTML5
// DragEvent path because WebView2 hands JS zero-byte File stubs from
// dataTransfer.files on Windows — no real content reaches the page.
#[derive(serde::Serialize)]
struct DroppedFile {
    name: String,
    mime: String,
    bytes: Vec<u8>,
}

// Proxy a remote URL through Rust reqwest and return the raw bytes. We can't
// use the @tauri-apps/plugin-http path for this because its guest-js layer
// constructs a browser `Headers` object, which silently drops forbidden
// headers (User-Agent, Referer). The request then reaches reqwest with the
// default `reqwest/x.x` UA and video.twimg.com 403s it. This command sends
// a real Chrome UA and no Referer (twimg serves when Referer is absent).
#[tauri::command]
async fn fetch_remote_bytes(url: String) -> Result<tauri::ipc::Response, String> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36",
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
    let bytes = resp.bytes().await.map_err(|e| format!("bytes: {e}"))?;
    Ok(tauri::ipc::Response::new(bytes.to_vec()))
}

#[tauri::command]
async fn read_dropped_file(path: String) -> Result<DroppedFile, String> {
    let path_buf = std::path::PathBuf::from(&path);
    let name = path_buf
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "file".to_string());
    let mime = mime_guess::from_path(&path_buf)
        .first_or_octet_stream()
        .essence_str()
        .to_string();
    let bytes = std::fs::read(&path_buf).map_err(|e| format!("read {}: {}", path, e))?;
    Ok(DroppedFile { name, mime, bytes })
}

// Windows toast with a real `appLogoOverride` avatar in the top-left.
//
// `tauri-plugin-notification` v2.3.3 calls `notify_rust::Notification::icon()`,
// but on Windows that field is silently dropped — `notify-rust`'s Windows
// backend (windows.rs) only reads `path_to_image` and even then renders the
// image as a regular `<image id="1" src=…>` (inline below the body) rather
// than as the app logo. Building the toast directly via
// `tauri-winrt-notification` lets us emit the proper
// `<image placement="appLogoOverride" hint-crop="circle" src=…>` element so
// the avatar renders in the standard top-left position used by Discord,
// Element, Slack, etc.
//
// The activation handler emits a `notification://activated` Tauri event so
// the JS-side click listener can route the click back to the originating
// room — the same flow `tauri-plugin-notification`'s `onAction` listener
// provides on other platforms.
#[cfg(target_os = "windows")]
#[tauri::command]
fn send_windows_message_toast(
    app: tauri::AppHandle,
    title: String,
    body: String,
    icon_path: Option<String>,
    room_id: String,
    event_id: String,
) -> Result<(), String> {
    use std::path::PathBuf;
    use tauri::Emitter;
    use tauri_winrt_notification::{IconCrop, Toast};

    // Must match the AppUserModelID registered by the NSIS installer
    // (`bundle.identifier` in tauri.conf.json). When this doesn't match,
    // Windows silently drops the toast.
    let app_id = app.config().identifier.clone();
    let app_handle = app.clone();

    // Showing the toast triggers WinRT activation events on the calling
    // thread. tauri-plugin-notification's desktop backend spawns a thread
    // for the same reason — see plugins/notification/src/desktop.rs in
    // the Tauri repo. Running it on the tokio executor thread directly
    // intermittently fires CO_E_NOTINITIALIZED.
    std::thread::spawn(move || {
        let mut toast = Toast::new(&app_id).title(&title).text1(&body);

        if let Some(path) = icon_path.filter(|s| !s.is_empty()) {
            let p = PathBuf::from(path);
            if p.exists() {
                toast = toast.icon(&p, IconCrop::Circular, &title);
            }
        }

        toast = toast.add_button("Open", "open");

        toast = toast.on_activated(move |_action| {
            let _ = app_handle.emit(
                "notification://activated",
                serde_json::json!({
                    "roomId": room_id,
                    "eventId": event_id,
                }),
            );
            Ok(())
        });

        if let Err(e) = toast.show() {
            eprintln!("[notif] tauri-winrt-notification toast.show failed: {e:?}");
        }
    });

    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
fn send_windows_message_toast(
    _title: String,
    _body: String,
    _icon_path: Option<String>,
    _room_id: String,
    _event_id: String,
) -> Result<(), String> {
    Err("send_windows_message_toast is Windows-only".to_string())
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
            read_dropped_file,
            fetch_remote_bytes,
            send_windows_message_toast,
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
                window_builder = window_builder.inner_size(800.0, 800.0);
            }

            // Keep Tauri's native drag-drop handler enabled. WebView2 (Windows)
            // hands JS zero-byte File stubs from dataTransfer.files when the OS
            // drag-drop path is bypassed, so the frontend listens for Tauri's
            // own drag-drop event (real OS paths) via useTauriDragDropListener
            // and reads bytes through read_dropped_file.

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
