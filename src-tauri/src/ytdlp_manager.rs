use std::path::{Path, PathBuf};
use std::env;
use tokio::sync::OnceCell;

static YTDLP_PATH_CACHE: OnceCell<Result<PathBuf, String>> = OnceCell::const_new();

pub async fn get_ytdlp_path() -> Result<PathBuf, String> {
    YTDLP_PATH_CACHE
        .get_or_init(|| async { find_ytdlp_path().await })
        .await
        .clone()
}

#[allow(dead_code)]
pub fn clear_ytdlp_cache() {} // OnceCell path is stable for process lifetime

async fn find_ytdlp_path() -> Result<PathBuf, String> {
    let system_name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };

    if let Ok(path) = which::which(system_name) {
        return Ok(path);
    }

    // Fallback to bundled binary in resources directory
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut resource_paths: Vec<PathBuf> = Vec::new();

            #[cfg(target_os = "macos")]
            {
                let exe_str = exe_dir.to_string_lossy();
                if exe_str.contains("Contents/MacOS") {
                    resource_paths.push(exe_dir.join("../Resources/resources"));
                } else {
                    resource_paths.push(exe_dir.join("../Resources/resources"));
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                resource_paths.push(exe_dir.join("resources"));
                resource_paths.push(exe_dir.join("../resources"));
            }

            resource_paths.push(exe_dir.join("../../resources"));
            resource_paths.push(PathBuf::from("src-tauri/resources"));

            for resource_dir in resource_paths {
                let bundled_path = get_platform_specific_path(&resource_dir);
                if bundled_path.exists() {
                    return Ok(bundled_path);
                }
            }
        }
    }

    Err("yt-dlp not found. Please install yt-dlp or use the updater to download it.".to_string())
}

pub fn get_platform_specific_path(resource_dir: &Path) -> PathBuf {
    let mut path = resource_dir.to_path_buf();

    if cfg!(target_os = "windows") {
        path.push("yt-dlp.exe");
    } else if cfg!(target_os = "macos") {
        path.push("yt-dlp_macos");
    } else if cfg!(target_arch = "aarch64") {
        path.push("yt-dlp_linux_arm64");
    } else {
        path.push("yt-dlp_linux");
    }

    path
}

pub fn get_bundled_ytdlp_dir() -> Result<PathBuf, String> {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[allow(unused_mut)]
            let mut resource_paths = vec![
                exe_dir.join("../Resources/resources"),
                exe_dir.join("resources"),
                exe_dir.join("../resources"),
                exe_dir.join("../../resources"),
                PathBuf::from("src-tauri/resources"),
            ];

            #[cfg(target_os = "macos")]
            {
                let exe_str = exe_dir.to_string_lossy();
                if exe_str.contains("Contents/MacOS") {
                    resource_paths.insert(0, exe_dir.join("../../Resources/resources"));
                }
                resource_paths.insert(0, exe_dir.join("../Resources/resources"));
            }

            for resource_dir in resource_paths {
                if resource_dir.exists() {
                    return Ok(resource_dir);
                }
            }
        }
    }

    let fallback_dir = PathBuf::from("src-tauri/resources");
    std::fs::create_dir_all(&fallback_dir)
        .map_err(|e| format!("Failed to create resource directory: {}", e))?;
    Ok(fallback_dir)
}
