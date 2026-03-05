use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_API_URL: &str = "https://api.github.com/repos/davidroeca/NotarAI/releases/latest";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;
const VERSION_CHECK_TIMEOUT_SECS: u64 = 5;
const DOWNLOAD_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCache {
    pub latest_version: String,
    pub checked_at: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstallMethod {
    CargoInstall,
    DevBuild,
    GithubRelease,
}

#[derive(Debug)]
pub struct UpdateStatus {
    pub current: Version,
    pub latest: Version,
    pub update_available: bool,
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("notarai"))
}

fn cache_path() -> Option<PathBuf> {
    cache_dir().map(|d| d.join("update_check.json"))
}

fn read_cache(path: &Path) -> Option<UpdateCache> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache(path: &Path, cache: &UpdateCache) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        fs::write(path, json).ok();
    }
}

fn make_agent(timeout_secs: u64) -> Result<ureq::Agent, String> {
    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
            .user_agent(format!("notarai/{}", env!("CARGO_PKG_VERSION")))
            .build(),
    );
    Ok(agent)
}

pub fn fetch_latest_version(timeout_secs: u64) -> Result<Version, String> {
    let agent = make_agent(timeout_secs)?;
    let mut response = agent
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("HTTP request failed: {e}"))?;
    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;
    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No tag_name in response".to_string())?;
    let version_str = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(version_str).map_err(|e| format!("Invalid version '{version_str}': {e}"))
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

pub fn check_for_update() -> Result<UpdateStatus, String> {
    let current = current_version();

    if let Some(cp) = cache_path()
        && let Some(cached) = read_cache(&cp)
    {
        let age = now_epoch_secs().saturating_sub(cached.checked_at);
        if age < CACHE_TTL_SECS
            && let Ok(latest) = Version::parse(&cached.latest_version)
        {
            return Ok(UpdateStatus {
                update_available: latest > current,
                current,
                latest,
            });
        }
    }

    let latest = fetch_latest_version(VERSION_CHECK_TIMEOUT_SECS)?;

    if let Some(cp) = cache_path() {
        write_cache(
            &cp,
            &UpdateCache {
                latest_version: latest.to_string(),
                checked_at: now_epoch_secs(),
            },
        );
    }

    Ok(UpdateStatus {
        update_available: latest > current,
        current,
        latest,
    })
}

pub fn check_for_update_no_cache() -> Result<UpdateStatus, String> {
    let current = current_version();
    let latest = fetch_latest_version(VERSION_CHECK_TIMEOUT_SECS)?;

    if let Some(cp) = cache_path() {
        write_cache(
            &cp,
            &UpdateCache {
                latest_version: latest.to_string(),
                checked_at: now_epoch_secs(),
            },
        );
    }

    Ok(UpdateStatus {
        update_available: latest > current,
        current,
        latest,
    })
}

pub fn detect_install_method() -> InstallMethod {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return InstallMethod::GithubRelease,
    };
    let path_str = exe.to_string_lossy();

    if cfg!(debug_assertions) || path_str.contains("target/") {
        return InstallMethod::DevBuild;
    }

    if path_str.contains(".cargo/bin") {
        return InstallMethod::CargoInstall;
    }

    InstallMethod::GithubRelease
}

pub fn release_binary_name() -> String {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    match (os, arch) {
        ("linux", "x86_64") => "notarai-x86_64-linux-musl",
        ("linux", "aarch64") => "notarai-aarch64-linux-musl",
        ("macos", "x86_64") => "notarai-x86_64-apple-darwin",
        ("macos", "aarch64") => "notarai-aarch64-apple-darwin",
        ("windows", "x86_64") => "notarai-x86_64-windows.exe",
        ("windows", "aarch64") => "notarai-aarch64-windows.exe",
        _ => "notarai",
    }
    .to_string()
}

pub fn download_and_replace(version: &Version) -> Result<(), String> {
    let binary_name = release_binary_name();
    let url =
        format!("https://github.com/davidroeca/NotarAI/releases/download/v{version}/{binary_name}");

    let exe_path =
        std::env::current_exe().map_err(|e| format!("Cannot determine exe path: {e}"))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Cannot determine exe directory".to_string())?;

    eprintln!("Downloading {binary_name} v{version}...");

    let agent = make_agent(DOWNLOAD_TIMEOUT_SECS)?;
    let mut response = agent
        .get(&url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("Failed to read download: {e}"))?;

    let temp_path = exe_dir.join(".notarai-update-tmp");

    fs::write(&temp_path, &body).map_err(|e| format!("Failed to write temp file: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    #[cfg(windows)]
    {
        // Windows cannot replace a running executable directly.
        // Rename the current exe to .old, then move the new one in.
        let old_path = exe_path.with_extension("old.exe");
        if old_path.exists() {
            fs::remove_file(&old_path).ok();
        }
        fs::rename(&exe_path, &old_path).map_err(|e| {
            format!("Failed to rename current exe: {e}. Try running as administrator.")
        })?;
        if let Err(e) = fs::rename(&temp_path, &exe_path) {
            // Try to restore the original
            fs::rename(&old_path, &exe_path).ok();
            return Err(format!("Failed to install new binary: {e}"));
        }
        eprintln!(
            "Note: old binary saved as {}. You can delete it.",
            old_path.display()
        );
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp_path, &exe_path).map_err(|e| {
            // Clean up temp file
            fs::remove_file(&temp_path).ok();
            format!("Failed to replace binary: {e}. Try: sudo notarai update")
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_is_valid() {
        let v = current_version();
        assert!(!v.to_string().is_empty());
    }

    #[test]
    fn detect_install_method_returns_dev_in_tests() {
        assert_eq!(detect_install_method(), InstallMethod::DevBuild);
    }

    #[test]
    fn release_binary_name_is_non_empty() {
        let name = release_binary_name();
        assert!(name.starts_with("notarai"));
    }

    #[test]
    fn cache_serialization_roundtrip() {
        let cache = UpdateCache {
            latest_version: "1.2.3".to_string(),
            checked_at: 1234567890,
        };
        let json = serde_json::to_string(&cache).expect("serialize");
        let parsed: UpdateCache = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.latest_version, "1.2.3");
        assert_eq!(parsed.checked_at, 1234567890);
    }

    #[test]
    fn version_comparison() {
        let v1 = Version::parse("0.3.0").unwrap();
        let v2 = Version::parse("0.4.0").unwrap();
        assert!(v2 > v1);

        let v3 = Version::parse("0.3.0").unwrap();
        assert!(!(v3 > v1));
    }
}
