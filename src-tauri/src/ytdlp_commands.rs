use crate::ytdlp_manager;
use crate::ytdlp_updater;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Emitter;
use tokio::sync::oneshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub duration: Option<u64>,
    pub uploader: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YtdlpVersionInfo {
    pub version: String,
    pub source: String, // "path" or "bundled"
}

static CANCEL_SENDER: Mutex<Option<oneshot::Sender<()>>> = Mutex::new(None);

#[tauri::command]
pub async fn ytdlp_get_version() -> Result<YtdlpVersionInfo, String> {
    let ytdlp_path = ytdlp_manager::get_ytdlp_path().await?;

    let source = if ytdlp_path.to_string_lossy().contains("resources") {
        "bundled"
    } else {
        "path"
    };

    let output = tokio::process::Command::new(&ytdlp_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get yt-dlp version".to_string());
    }

    let version = String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Failed to parse version: {}", e))?;

    Ok(YtdlpVersionInfo { version, source: source.to_string() })
}

#[tauri::command]
pub async fn ytdlp_get_video_info(url: String) -> Result<VideoInfo, String> {
    let ytdlp_path = ytdlp_manager::get_ytdlp_path()
        .await
        .map_err(|e| format!("yt-dlp not available: {}", e))?;

    let output = tokio::process::Command::new(&ytdlp_path)
        .arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg(&url)
        .output()
        .await
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", error_msg));
    }

    let json_output = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to parse yt-dlp output: {}", e))?;

    let info: serde_json::Value = serde_json::from_str(&json_output)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(VideoInfo {
        title: info["title"].as_str().unwrap_or("Unknown").to_string(),
        duration: info["duration"].as_u64(),
        uploader: info["uploader"].as_str().map(|s| s.to_string()),
    })
}

#[tauri::command]
pub async fn ytdlp_check_update() -> Result<bool, String> {
    ytdlp_updater::check_update_available().await
}

#[tauri::command]
pub async fn ytdlp_download_binary() -> Result<String, String> {
    ytdlp_updater::download_ytdlp().await
}

#[tauri::command]
pub async fn ytdlp_cancel_download() -> Result<(), String> {
    let mut sender_guard = CANCEL_SENDER
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(sender) = sender_guard.take() {
        let _ = sender.send(());
    }
    Ok(())
}

#[tauri::command]
pub async fn ytdlp_download_video(
    url: String,
    quality: Option<String>,
    window: tauri::Window,
) -> Result<String, String> {
    let ytdlp_path = ytdlp_manager::get_ytdlp_path()
        .await
        .map_err(|e| format!("yt-dlp not available: {}", e))?;

    let download_dir = dirs::download_dir()
        .ok_or_else(|| "Failed to get Downloads directory".to_string())?;

    let mut cmd = tokio::process::Command::new(&ytdlp_path);
    cmd.arg("--output")
        .arg(format!(
            "{}/%(title)s.%(ext)s",
            download_dir.to_string_lossy()
        ))
        .arg("--newline")
        .arg("--progress")
        .arg("--no-warnings")
        .arg("--merge-output-format")
        .arg("mp4");

    // Select a reasonable default format: best MP4 that plays everywhere
    if let Some(ref q) = quality {
        if q == "best" || q == "worst" {
            let selector = if q == "best" {
                "bestvideo[ext=mp4][vcodec^=avc1]+bestaudio[ext=mp4][acodec^=mp4a]/bestvideo[ext=mp4]+bestaudio[ext=mp4]/best[ext=mp4]"
            } else {
                "worstvideo[ext=mp4][vcodec^=avc1]+worstaudio[ext=mp4][acodec^=mp4a]/worstvideo[ext=mp4]+worstaudio[ext=mp4]/worst[ext=mp4]"
            };
            cmd.arg("-f").arg(selector);
        } else {
            cmd.arg("-f")
                .arg(format!("{}+bestaudio[ext=mp4]/best[ext=mp4]", q));
        }
    } else {
        // Default: best MP4 for maximum compatibility
        cmd.arg("-f").arg("bestvideo[ext=mp4][vcodec^=avc1]+bestaudio[ext=mp4][acodec^=mp4a]/bestvideo[ext=mp4]+bestaudio[ext=mp4]/best[ext=mp4]");
    }

    cmd.arg(&url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    // Setup cancellation channel
    let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
    {
        let mut sender_guard = CANCEL_SENDER
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        *sender_guard = Some(cancel_tx);
    }

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let (progress_cancel_tx, mut progress_cancel_rx) = oneshot::channel::<()>();

    let window_clone = window.clone();
    let progress_task = tokio::spawn(async move {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        loop {
            tokio::select! {
                _ = &mut progress_cancel_rx => break,
                result = stdout_reader.read_until(b'\n', &mut stdout_buf) => {
                    match result {
                        Ok(0) => break,
                        Ok(_) => {
                            let line = String::from_utf8_lossy(&stdout_buf);
                            let line = line.trim_end_matches('\n').trim_end_matches('\r');
                            if !line.is_empty() {
                                let _ = window_clone.emit("ytdlp-output", line.to_string());
                            }
                            stdout_buf.clear();
                        }
                        Err(_) => break,
                    }
                }
                result = stderr_reader.read_until(b'\n', &mut stderr_buf) => {
                    match result {
                        Ok(0) => break,
                        Ok(_) => {
                            let line = String::from_utf8_lossy(&stderr_buf);
                            let line = line.trim_end_matches('\n').trim_end_matches('\r');
                            if !line.is_empty() {
                                let _ = window_clone.emit("ytdlp-output", line.to_string());
                            }
                            stderr_buf.clear();
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    let status = tokio::select! {
        result = child.wait() => {
            result.map_err(|e| format!("Failed to wait for process: {}", e))?
        }
        _ = &mut cancel_rx => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            {
                let mut sender_guard = CANCEL_SENDER.lock().map_err(|e| format!("Lock error: {}", e))?;
                *sender_guard = None;
            }
            let _ = progress_cancel_tx.send(());
            let _ = progress_task.await;
            return Err("Download cancelled".to_string());
        }
    };

    let _ = progress_cancel_tx.send(());
    let _ = progress_task.await;

    {
        let mut sender_guard = CANCEL_SENDER
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        *sender_guard = None;
    }

    if !status.success() {
        return Err("Download failed".to_string());
    }

    let _ = window.emit("ytdlp-output", "DOWNLOAD_COMPLETE".to_string());

    Ok(format!(
        "Download completed to: {}",
        download_dir.to_string_lossy()
    ))
}
