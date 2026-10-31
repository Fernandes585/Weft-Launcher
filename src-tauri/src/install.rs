use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File as StdFile;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::Manager;
use thiserror::Error;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zip::result::ZipError;

const USER_AGENT_VALUE: &str = "weft-launcher/0.1.0";

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Erro de rede: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de serialização: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Erro de zip: {0}")]
    Zip(#[from] ZipError),
    #[error("Erro ao criar diretório: {0}")]
    CreateDir(String),
    #[error("SHA256 não confere: esperado {expected}, obtido {actual}")]
    Sha256Mismatch { expected: String, actual: String },
    #[error("Digest SHA256 ausente no asset")]
    MissingDigest,
    #[error("Asset não encontrado para a plataforma atual")]
    AssetNotFound,
    #[error("Versão já instalada: {0}")]
    AlreadyInstalled(String),
    #[error("Versão não instalada: {0}")]
    NotInstalled(String),
    #[error("Limite de taxa da API GitHub excedido")]
    RateLimited,
    #[error("Acesso negado à API GitHub (403)")]
    Forbidden,
    #[error("Sem conexão e sem cache disponível")]
    OfflineNoCache,
}

impl From<InstallError> for String {
    fn from(err: InstallError) -> String {
        err.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub tag: String,
    pub path: String,
    pub executable: String,
    pub installed_at: String,
}

fn get_versions_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, InstallError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InstallError::Io(std::io::Error::other(e)))?;
    Ok(app_data.join("versions"))
}

fn get_downloads_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, InstallError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InstallError::Io(std::io::Error::other(e)))?;
    Ok(app_data.join("downloads"))
}

fn current_platform_asset_suffix() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows-x64.zip";
    #[cfg(target_os = "linux")]
    return "linux-x64.tar.gz";
    #[cfg(target_os = "macos")]
    return "macos-arm64.tar.gz";
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return "";
}

#[tauri::command]
pub async fn install_version(
    app_handle: tauri::AppHandle,
    tag: String,
    asset_url: String,
    asset_name: String,
    expected_sha256: Option<String>,
) -> Result<InstalledVersion, String> {
    let versions_dir = get_versions_dir(&app_handle)?;
    let downloads_dir = get_downloads_dir(&app_handle)?;

    fs::create_dir_all(&versions_dir).await.map_err(|e| e.to_string())?;
    fs::create_dir_all(&downloads_dir).await.map_err(|e| e.to_string())?;

    let version_dir = versions_dir.join(&tag);
    if version_dir.exists() {
        return Err(InstallError::AlreadyInstalled(tag).to_string());
    }

    // Download to temp file
    let temp_download = downloads_dir.join(format!("{}.tmp", &asset_name));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&asset_url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Falha no download: {}", response.status()));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&temp_download).await.map_err(|e| e.to_string())?;
    tokio::io::AsyncWriteExt::write_all(&mut file, &bytes)
        .await
        .map_err(|e| e.to_string())?;
    file.flush().await.map_err(|e| e.to_string())?;

    // Verify SHA256 if provided
    if let Some(expected) = expected_sha256 {
        if !expected.is_empty() {
            let mut file = fs::File::open(&temp_download).await.map_err(|e| e.to_string())?;
            let mut hasher = Sha256::new();
            let mut buffer = vec![0u8; 8192];
            loop {
                let n = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            let actual = format!("{:x}", hasher.finalize());
            if actual != expected {
                fs::remove_file(&temp_download).await.ok();
                return Err(InstallError::Sha256Mismatch {
                    expected,
                    actual,
                }.to_string());
            }
        }
    }

    // Extract to temp directory first, then rename atomically
    let temp_extract = versions_dir.join(format!(".tmp-{}", &tag));
    if temp_extract.exists() {
        fs::remove_dir_all(&temp_extract).await.map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&temp_extract).await.map_err(|e| e.to_string())?;

    // Extract based on file extension
    if asset_name.ends_with(".zip") {
        extract_zip(&temp_download, &temp_extract).await.map_err(|e| e.to_string())?;
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz(&temp_download, &temp_extract).await.map_err(|e| e.to_string())?;
    } else {
        fs::remove_dir_all(&temp_extract).await.ok();
        fs::remove_file(&temp_download).await.ok();
        return Err("Formato de arquivo não suportado".to_string());
    }

    // Find the executable and normalize structure
    normalize_install_structure(&temp_extract, &tag).await.map_err(|e| e.to_string())?;

    // Atomic rename
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir).await.map_err(|e| e.to_string())?;
    }
    fs::rename(&temp_extract, &version_dir).await.map_err(|e| e.to_string())?;

    // Clean up temp download
    fs::remove_file(&temp_download).await.ok();

    let executable_name = if cfg!(target_os = "windows") { "weft.exe" } else { "weft" };
    let executable_path = version_dir.join(executable_name);

    let now = time::OffsetDateTime::now_utc();
    let installed_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    Ok(InstalledVersion {
        tag,
        path: version_dir.to_string_lossy().to_string(),
        executable: executable_path.to_string_lossy().to_string(),
        installed_at,
    })
}

async fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), InstallError> {
    // Use std::fs::File for zip crate compatibility
    let file = StdFile::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_path = entry.mangled_name();

        // Zip slip protection
        if entry_path.components().any(|c| c.as_os_str() == "..") {
            continue;
        }

        let target_path = dest.join(&entry_path);
        if entry.is_dir() {
            fs::create_dir_all(&target_path).await?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            let mut outfile = StdFile::create(&target_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    Ok(())
}

async fn extract_tar_gz(tar_gz_path: &Path, dest: &Path) -> Result<(), InstallError> {
    // Use std::fs::File for tar/flate2 compatibility
    let file = StdFile::open(tar_gz_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    // Collect all entries first to avoid Send issues across await
    let entries: Vec<_> = archive.entries()?.collect::<Result<Vec<_>, _>>()?;

    for mut entry in entries {
        let entry_path = entry.path()?;

        // Path traversal protection
        if entry_path.components().any(|c| c.as_os_str() == "..") {
            continue;
        }

        let target_path = dest.join(&entry_path);
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&target_path).await?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            entry.unpack(&target_path)?;
        }
    }
    Ok(())
}

async fn normalize_install_structure(extract_dir: &Path, _tag: &str) -> Result<(), InstallError> {
    // Check if there's a single top-level folder
    let mut entries = fs::read_dir(extract_dir).await?;
    let mut first_entry = None;
    let mut entry_count = 0;

    while let Some(entry) = entries.next_entry().await? {
        entry_count += 1;
        if first_entry.is_none() {
            first_entry = Some(entry.path());
        }
    }

    // If there's exactly one folder at root, move its contents up
    if entry_count == 1 {
        if let Some(single_dir) = first_entry {
            if single_dir.is_dir() {
                let mut entries = fs::read_dir(&single_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let target = extract_dir.join(entry.file_name());
                    fs::rename(entry.path(), target).await?;
                }
                fs::remove_dir(single_dir).await?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_installed_versions(
    app_handle: tauri::AppHandle,
) -> Result<Vec<InstalledVersion>, String> {
    let versions_dir = get_versions_dir(&app_handle)?;

    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    let mut entries = fs::read_dir(&versions_dir).await.map_err(|e| e.to_string())?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        let executable_name = if cfg!(target_os = "windows") { "weft.exe" } else { "weft" };
        let executable = path.join(executable_name);

        if executable.exists() {
            let metadata = fs::metadata(&path).await.map_err(|e| e.to_string())?;
            let installed_at = metadata
                .created()
                .ok()
                .and_then(|t| {
                    time::OffsetDateTime::from_unix_timestamp(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64).ok()
                })
                .and_then(|dt| dt.format(&time::format_description::well_known::Rfc3339).ok())
                .unwrap_or_default();

            versions.push(InstalledVersion {
                tag,
                path: path.to_string_lossy().to_string(),
                executable: executable.to_string_lossy().to_string(),
                installed_at,
            });
        }
    }

    versions.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
    Ok(versions)
}

#[tauri::command]
pub async fn remove_version(
    app_handle: tauri::AppHandle,
    tag: String,
) -> Result<(), String> {
    let versions_dir = get_versions_dir(&app_handle)?;
    let version_dir = versions_dir.join(&tag);

    if !version_dir.exists() {
        return Err(InstallError::NotInstalled(tag).to_string());
    }

    fs::remove_dir_all(&version_dir).await.map_err(|e| e.to_string())?;
    Ok(())
}