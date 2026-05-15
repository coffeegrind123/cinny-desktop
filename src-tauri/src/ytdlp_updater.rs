use crate::ytdlp_manager;
use std::fs;
use std::sync::OnceLock;
use std::time::SystemTime;

const YTDLP_RELEASES_API: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";

struct CachedVersion {
    version: String,
    cached_at: SystemTime,
}

static LATEST_VERSION_CACHE: OnceLock<CachedVersion> = OnceLock::new();
const CACHE_DURATION_SECONDS: u64 = 3600;

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn get_latest_version() -> Result<String, String> {
    if let Some(cached) = LATEST_VERSION_CACHE.get() {
        if let Ok(elapsed) = SystemTime::now().duration_since(cached.cached_at) {
            if elapsed.as_secs() < CACHE_DURATION_SECONDS {
                return Ok(cached.version.clone());
            }
        }
    }

    let client = reqwest::Client::new();
    let response = client
        .get(YTDLP_RELEASES_API)
        .header("User-Agent", "prinny-ytdlp-updater")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch latest version: {}", e))?;

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let version = release.tag_name.trim_start_matches('v').to_string();

    let _ = LATEST_VERSION_CACHE.set(CachedVersion {
        version: version.clone(),
        cached_at: SystemTime::now(),
    });

    Ok(version)
}

pub async fn check_update_available() -> Result<bool, String> {
    let current_version = get_current_version().await?;
    let latest_version = get_latest_version().await?;
    Ok(current_version != latest_version)
}

async fn get_current_version() -> Result<String, String> {
    let ytdlp_path = ytdlp_manager::get_ytdlp_path().await?;
    let output = tokio::process::Command::new(&ytdlp_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to get current version: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get current version".to_string());
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Failed to parse version: {}", e))
}

fn get_platform_asset_name() -> String {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe".to_string()
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "yt-dlp_linux_arm64".to_string()
    } else {
        "yt-dlp_linux".to_string()
    }
}

async fn get_download_url(asset_name: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(YTDLP_RELEASES_API)
        .header("User-Agent", "prinny-ytdlp-updater")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    // Exact match first
    for asset in &release.assets {
        if asset.name == asset_name {
            return Ok(asset.browser_download_url.clone());
        }
    }

    // Partial match
    for asset in &release.assets {
        if asset.name.contains(asset_name) && !asset.name.contains(".sig") {
            return Ok(asset.browser_download_url.clone());
        }
    }

    // Platform-specific fallback
    let platform_patterns: Vec<&str> = if cfg!(target_os = "windows") {
        vec!["yt-dlp.exe"]
    } else if cfg!(target_os = "macos") {
        vec!["macos"]
    } else if cfg!(target_arch = "aarch64") {
        vec!["linux", "arm64", "aarch64"]
    } else {
        vec!["linux"]
    };

    for asset in &release.assets {
        if asset.name.contains("yt-dlp")
            && !asset.name.contains(".sig")
            && !asset.name.ends_with(".tar.gz")
            && !asset.name.ends_with(".zip")
        {
            let name_lower = asset.name.to_lowercase();
            let matches_platform = platform_patterns
                .iter()
                .any(|pattern| name_lower.contains(pattern));

            if matches_platform {
                let is_windows_exe = cfg!(target_os = "windows") && asset.name.ends_with(".exe");
                let is_unix_binary =
                    !cfg!(target_os = "windows") && !asset.name.ends_with(".exe");

                if is_windows_exe || is_unix_binary {
                    return Ok(asset.browser_download_url.clone());
                }
            }
        }
    }

    Err(format!(
        "No suitable yt-dlp binary found for platform: {}",
        asset_name
    ))
}

pub async fn download_ytdlp() -> Result<String, String> {
    let resource_dir = ytdlp_manager::get_bundled_ytdlp_dir()?;
    let asset_name = get_platform_asset_name();
    let download_url = get_download_url(&asset_name).await?;

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .header("User-Agent", "prinny-ytdlp-updater")
        .send()
        .await
        .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    let target_path = ytdlp_manager::get_platform_specific_path(&resource_dir);

    fs::create_dir_all(&resource_dir)
        .map_err(|e| format!("Failed to create resource directory: {}", e))?;

    fs::write(&target_path, bytes)
        .map_err(|e| format!("Failed to write yt-dlp binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    let version = get_latest_version().await?;
    Ok(version)
}
