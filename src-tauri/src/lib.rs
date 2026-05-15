#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// mod menu;

use tauri::{webview::{NewWindowResponse, WebviewWindowBuilder}, WebviewUrl};
use tauri_plugin_opener::OpenerExt;

mod taskbar;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod ytdlp_manager;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod ytdlp_updater;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod ytdlp_commands;

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

fn ytdlp_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("ytdlp")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                let _handle = _api.register_android_plugin("in.prinny.app", "YtDlpPlugin")?;
            }
            #[cfg(not(target_os = "android"))]
            let _ = &_api;
            Ok(())
        })
        .build()
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
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_get_version,
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_get_video_info,
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_check_update,
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_download_binary,
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_download_video,
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            ytdlp_commands::ytdlp_cancel_download,
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
        .plugin(ytdlp_plugin());

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
                window_builder = window_builder.inner_size(800.0, 700.0);
            }

            window_builder
                .on_new_window(move |url, _features| {
                    let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
