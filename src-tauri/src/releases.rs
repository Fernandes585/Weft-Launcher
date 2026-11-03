use crate::config::{USER_AGENT_VALUE, WEFT_REPO};
use crate::error::AppError;
use reqwest::header::{
    ETAG, HeaderMap, HeaderValue, IF_NONE_MATCH, InvalidHeaderValue, USER_AGENT,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::fs;

const GITHUB_API_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Debug, Error)]
pub enum ReleasesError {
    #[error("Erro de rede: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de serialização: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Erro de cabeçalho HTTP: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error("Cache corrompido: {0}")]
    CacheCorrupted(String),
    #[error("Asset não encontrado para a plataforma atual")]
    AssetNotFound,
    #[error("Digest SHA256 ausente no asset")]
    MissingDigest,
    #[error("Limite de taxa da API GitHub excedido. Tente novamente mais tarde.")]
    RateLimited,
    #[error("Acesso negado à API GitHub (403). Usando cache se disponível.")]
    Forbidden,
    #[error("Sem conexão e sem cache disponível")]
    OfflineNoCache,
    #[error("Nenhuma versão do Weft disponível para o seu sistema")]
    NoCompatibleVersions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub url: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub published_at: String,
    pub body: String,
    pub prerelease: bool,
    pub asset: AssetInfo,
    #[serde(skip)]
    pub from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedReleases {
    etag: Option<String>,
    releases: Vec<Release>,
    fetched_at: String,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    published_at: String,
    body: Option<String>,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

fn get_cache_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, ReleasesError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| ReleasesError::Io(std::io::Error::other(e)))?;
    let cache_dir = app_data.join("cache");
    Ok(cache_dir)
}

fn get_cache_file(app_handle: &tauri::AppHandle) -> Result<PathBuf, ReleasesError> {
    Ok(get_cache_dir(app_handle)?.join("releases.json"))
}

async fn ensure_cache_dir(app_handle: &tauri::AppHandle) -> Result<(), ReleasesError> {
    let cache_dir = get_cache_dir(app_handle)?;
    fs::create_dir_all(&cache_dir).await?;
    Ok(())
}

fn current_platform_asset_suffix() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x64.zip";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64.tar.gz";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64.tar.gz";
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    return "";
}

/// Pure function: returns the asset suffix for a given OS and architecture.
/// This is separated to enable unit testing without depending on compile-time cfg.
#[allow(dead_code)]
fn asset_suffix(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("windows-x64.zip"),
        ("linux", "x86_64") => Some("linux-x64.tar.gz"),
        ("macos", "aarch64") => Some("macos-arm64.tar.gz"),
        _ => None,
    }
}

/// Pure function: filters releases to keep only those with an asset matching the suffix.
/// Returns the filtered list and a flag indicating if any compatible release was found.
fn filter_releases(
    releases: Vec<GitHubRelease>,
    suffix: &str,
) -> (Vec<Release>, bool) {
    let mut filtered = Vec::new();
    for gh in releases {
        if gh.prerelease {
            continue;
        }
        if let Some(asset) = find_matching_asset(&gh.assets, suffix) {
            filtered.push(Release {
                tag: gh.tag_name,
                name: gh.name,
                published_at: gh.published_at,
                body: gh.body.unwrap_or_default(),
                prerelease: gh.prerelease,
                asset,
                from_cache: false,
            });
        }
    }
    let has_compatible = !filtered.is_empty();
    (filtered, has_compatible)
}

fn find_matching_asset(assets: &[GitHubAsset], suffix: &str) -> Option<AssetInfo> {
    for asset in assets {
        if asset.name.contains("Source code") || asset.name.contains("source code") {
            continue;
        }
        if asset.name.ends_with(suffix) {
            let sha256 = asset
                .digest
                .as_ref()
                .and_then(|d| d.strip_prefix("sha256:"))
                .map(|s| s.to_string());
            return Some(AssetInfo {
                name: asset.name.clone(),
                url: asset.browser_download_url.clone(),
                size: asset.size,
                sha256,
            });
        }
    }
    None
}

#[allow(dead_code)]
fn select_asset_for_platform(assets: &[GitHubAsset]) -> Result<AssetInfo, ReleasesError> {
    let suffix = current_platform_asset_suffix();
    if suffix.is_empty() {
        return Err(ReleasesError::AssetNotFound);
    }

    find_matching_asset(assets, suffix).ok_or(ReleasesError::AssetNotFound)
}

fn parse_github_releases(gh_releases: Vec<GitHubRelease>) -> Result<Vec<Release>, ReleasesError> {
    let suffix = current_platform_asset_suffix();
    if suffix.is_empty() {
        return Err(ReleasesError::AssetNotFound);
    }

    let (releases, has_compatible) = filter_releases(gh_releases, suffix);
    
    if !has_compatible {
        return Err(ReleasesError::NoCompatibleVersions);
    }
    
    Ok(releases)
}

fn github_api_url() -> String {
    format!("https://api.github.com/repos/{}/releases", WEFT_REPO)
}

async fn fetch_from_github(
    client: &reqwest::Client,
    etag: Option<&str>,
) -> Result<(Vec<Release>, Option<String>), ReleasesError> {
    let url = github_api_url();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    if let Some(etag) = etag {
        headers.insert(IF_NONE_MATCH, HeaderValue::from_str(etag)?);
    }

    let response = client.get(&url).headers(headers).send().await?;

    let new_etag = response
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match response.status().as_u16() {
        200 => {
            let gh_releases: Vec<GitHubRelease> = response.json().await?;
            let releases = parse_github_releases(gh_releases)?;
            Ok((releases, new_etag))
        }
        304 => Ok((Vec::new(), None)),
        403 => {
            let remaining = response
                .headers()
                .get("X-RateLimit-Remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            if remaining == 0 {
                Err(ReleasesError::RateLimited)
            } else {
                Err(ReleasesError::Forbidden)
            }
        }
        429 => Err(ReleasesError::RateLimited),
        _ => Err(ReleasesError::Network(
            response.error_for_status().unwrap_err(),
        )),
    }
}

async fn load_cache(
    app_handle: &tauri::AppHandle,
) -> Result<Option<CachedReleases>, ReleasesError> {
    let cache_file = get_cache_file(app_handle)?;
    if !cache_file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&cache_file).await?;
    let cached: CachedReleases = serde_json::from_str(&content)?;
    Ok(Some(cached))
}

async fn save_cache(
    app_handle: &tauri::AppHandle,
    etag: Option<String>,
    releases: &[Release],
) -> Result<(), ReleasesError> {
    ensure_cache_dir(app_handle).await?;
    let cache_file = get_cache_file(app_handle)?;
    let now = OffsetDateTime::now_utc();
    let fetched_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let cached = CachedReleases {
        etag,
        releases: releases.to_vec(),
        fetched_at,
    };
    let content = serde_json::to_string_pretty(&cached)?;
    fs::write(&cache_file, content).await?;
    Ok(())
}

#[tauri::command]
pub async fn list_versions(
    app_handle: tauri::AppHandle,
    force_refresh: bool,
) -> Result<Vec<Release>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(GITHUB_API_TIMEOUT)
        .build()
        .map_err(ReleasesError::Network)?;

    let cached = load_cache(&app_handle).await?;

    if !force_refresh && let Some(cached) = cached {
        let mut releases = cached.releases;
        for r in &mut releases {
            r.from_cache = true;
        }
        return Ok(releases);
    }

    let etag = cached.as_ref().and_then(|c| c.etag.clone());
    let etag_ref = etag.as_deref();

    match fetch_from_github(&client, etag_ref).await {
        Ok((new_releases, new_etag)) => {
            let final_releases = match cached {
                Some(cached) if new_releases.is_empty() => {
                    let mut releases = cached.releases;
                    for r in &mut releases {
                        r.from_cache = true;
                    }
                    releases
                }
                _ => new_releases,
            };

            if !final_releases.is_empty() {
                save_cache(&app_handle, new_etag, &final_releases).await?;
            }

            Ok(final_releases)
        }
        Err(e) => {
            if let Some(cached) = cached {
                let mut releases = cached.releases;
                for r in &mut releases {
                    r.from_cache = true;
                }
                Ok(releases)
            } else {
                Err(e.into())
            }
        }
    }
}
