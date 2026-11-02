use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File as StdFile;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use tauri::{Emitter, Manager};
use thiserror::Error;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zip::result::ZipError;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// Import from launch module to check running versions
use crate::launch::is_version_running;
use crate::error::AppError;

/// Returns the platform-specific executable name for Weft.
pub fn executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "weft.exe"
    } else {
        "weft"
    }
}

/// Logs an error to launcher.log in the app data directory.
async fn log_error(app_handle: &tauri::AppHandle, stage: &str, err: &std::io::Error) {
    if let Ok(app_data) = app_handle.path().app_data_dir() {
        let log_path = app_data.join("launcher.log");
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let log_entry = format!("[{}] [{}] {}\n", timestamp, stage, err);
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
        {
            let _ = file.write_all(log_entry.as_bytes()).await;
        }
    }
}

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
    #[error("Digest SHA256 obrigatório não fornecido")]
    HashMissing,
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
    #[error("Falha no download: {0}")]
    DownloadFailed(String),
    #[error("Falha na extração: {0}")]
    ExtractFailed(String),
    #[error("Versão está em execução: {0}")]
    GameRunning(String),
    #[error("Versão em uso (arquivo bloqueado): {0}")]
    VersionInUse(String),
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

// Cleanup helper
async fn cleanup_temp(temp_download: PathBuf, temp_extract: PathBuf) {
    fs::remove_file(temp_download).await.ok();
    fs::remove_dir_all(temp_extract).await.ok();
}

/// Recursively copies a directory from src to dst.
async fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(dst).await?;
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type().await?;
        if file_type.is_dir() {
            Box::pin(copy_dir_all(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn install_version(
    app_handle: tauri::AppHandle,
    tag: String,
    asset_url: String,
    asset_name: String,
    expected_sha256: Option<String>,
) -> Result<InstalledVersion, AppError> {
    let versions_dir = get_versions_dir(&app_handle)?;
    let downloads_dir = get_downloads_dir(&app_handle)?;

    fs::create_dir_all(&versions_dir).await?;
    fs::create_dir_all(&downloads_dir).await?;

    let version_dir = versions_dir.join(&tag);
    let executable_name = executable_name();
    let executable_path = version_dir.join(executable_name);

    // Check if already properly installed (directory exists AND executable exists)
    if executable_path.exists() {
        return Err(InstallError::AlreadyInstalled(tag).into());
    }

    // Directory exists but executable is missing -> remove stale directory and reinstall
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir).await?;
    }

    // Require SHA256
    let expected_sha256 = expected_sha256
        .filter(|s| !s.is_empty())
        .ok_or_else(|| InstallError::HashMissing)?;

    // Download to temp file with progress
    let temp_download = downloads_dir.join(format!("{}.tmp", asset_name));
    let temp_extract = versions_dir.join(format!(".tmp-{}", tag));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client
        .get(&asset_url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .send()
        .await
        .map_err(|e| InstallError::DownloadFailed(e.to_string()))?;

    if !response.status().is_success() {
        cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
        return Err(InstallError::DownloadFailed(format!("HTTP {}", response.status())).into());
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_reported_percent = 0u32;
    let mut last_reported_bytes = 0u64;
    let mut last_report_time = std::time::Instant::now();

    #[cfg(debug_assertions)]
    let download_start = std::time::Instant::now();

    let mut file = fs::File::create(&temp_download).await?;

    // Stream download with progress
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        file.write_all(&chunk).await?;

        downloaded += chunk.len() as u64;

        // Throttle progress: emit every ~1% or 100KB
        let should_report = if total_size > 0 {
            let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            percent > last_reported_percent
        } else {
            downloaded >= last_reported_bytes + 100_000
        };

        if should_report {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_report_time).as_secs_f64();
            let bytes_since_last = downloaded.saturating_sub(last_reported_bytes);
            let speed_mbps = if elapsed > 0.0 {
                (bytes_since_last as f64 * 8.0 / 1_000_000.0) / elapsed
            } else {
                0.0
            };

            let percent = if total_size > 0 {
                ((downloaded as f64 / total_size as f64) * 100.0) as u32
            } else {
                0
            };
            let _ = app_handle.emit(
                "install-progress",
                serde_json::json!({
                    "tag": tag,
                    "downloaded": downloaded,
                    "total": total_size,
                    "percent": percent,
                    "stage": "downloading",
                    "speedMbps": speed_mbps
                }),
            );
            last_reported_percent = if total_size > 0 {
                ((downloaded as f64 / total_size as f64) * 100.0) as u32
            } else {
                0
            };
            last_reported_bytes = downloaded;
            last_report_time = now;
        }

        #[cfg(debug_assertions)]
        {
            if let Ok(val) = std::env::var("WEFT_THROTTLE_KBPS")
                && let Ok(kbps) = val.parse::<f64>()
                && kbps > 0.0
            {
                let bytes_per_sec = kbps * 1024.0;
                let expected_duration = downloaded as f64 / bytes_per_sec;
                let actual_duration = download_start.elapsed().as_secs_f64();
                if expected_duration > actual_duration {
                    let sleep_secs = expected_duration - actual_duration;
                    tokio::time::sleep(std::time::Duration::from_secs_f64(sleep_secs)).await;
                }
            }
        }
    }

    file.flush().await?;
    drop(file);

    // Final download progress
    let _ = app_handle.emit(
        "install-progress",
        serde_json::json!({
            "tag": tag,
            "downloaded": downloaded,
            "total": total_size,
            "percent": 100,
            "stage": "verifying",
            "speedMbps": 0.0
        }),
    );

    // Verify SHA256
    let mut file = fs::File::open(&temp_download).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha256 {
        cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
        return Err(InstallError::Sha256Mismatch {
            expected: expected_sha256,
            actual,
        }
        .into());
    }
    drop(file);

    // Extract to temp directory
    let _ = app_handle.emit(
        "install-progress",
        serde_json::json!({
            "tag": tag,
            "downloaded": downloaded,
            "total": total_size,
            "percent": 100,
            "stage": "extracting",
            "speedMbps": 0.0
        }),
    );

    if temp_extract.exists() {
        fs::remove_dir_all(&temp_extract).await?;
    }
    fs::create_dir_all(&temp_extract).await?;

    // Extract based on file extension
    let extract_result = if asset_name.ends_with(".zip") {
        extract_zip(temp_download.clone(), temp_extract.clone()).await
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz(temp_download.clone(), temp_extract.clone()).await
    } else {
        cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
        return Err(
            InstallError::ExtractFailed("Formato de arquivo não suportado".to_string()).into(),
        );
    };

    if let Err(e) = &extract_result {
        log_error(&app_handle, "extracting", &std::io::Error::other(e.to_string())).await;
    }
    extract_result?;

    // Normalize structure (handle top-level folder)
    if let Err(e) = normalize_install_structure(&temp_extract, &tag).await {
        log_error(&app_handle, "normalizing", &std::io::Error::other(e.to_string())).await;
        return Err(e.into());
    }

    // Verify executable exists in temp directory before finalizing
    let temp_executable = temp_extract.join(executable_name);
    if !temp_executable.exists() {
        // List first 10 items in temp directory for diagnostics
        let mut items = Vec::new();
        if let Ok(mut entries) = fs::read_dir(&temp_extract).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                items.push(entry.file_name().to_string_lossy().to_string());
                if items.len() >= 10 {
                    break;
                }
            }
        }
        let items_str = if items.is_empty() {
            " (diretório vazio)".to_string()
        } else {
            format!(" (primeiros itens: {})", items.join(", "))
        };
        cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
        return Err(
            InstallError::ExtractFailed(format!(
                "Executável não encontrado após extração{}",
                items_str
            ))
            .into(),
        );
    }

    // On Unix, set executable permission on the binary before finalizing
    #[cfg(unix)]
    {
        use std::fs::Permissions;
        let mut perms = fs::metadata(&temp_executable).await?.permissions();
        perms.set_mode(0o755);
        if let Err(e) = fs::set_permissions(&temp_executable, perms).await {
            log_error(&app_handle, "setting permissions", &e).await;
            cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
            return Err(InstallError::ExtractFailed(format!(
                "Falha ao definir permissão de execução: {}",
                e
            ))
            .into());
        }
    }

    // Finalizing
    let _ = app_handle.emit(
        "install-progress",
        serde_json::json!({
            "tag": tag,
            "downloaded": downloaded,
            "total": total_size,
            "percent": 100,
            "stage": "finalizing",
            "speedMbps": 0.0
        }),
    );

    // Atomic rename with EXDEV fallback (copy + delete)
    if version_dir.exists() {
        fs::remove_dir_all(&version_dir).await?;
    }
    if let Err(e) = fs::rename(&temp_extract, &version_dir).await {
        // If rename fails due to cross-device (EXDEV), fallback to copy + delete
        if e.kind() == ErrorKind::CrossesDevices {
            // Copy recursively
            copy_dir_all(&temp_extract, &version_dir).await?;
            // Remove temp dir
            fs::remove_dir_all(&temp_extract).await?;
        } else {
            log_error(&app_handle, "finalizing (rename)", &e).await;
            cleanup_temp(temp_download.clone(), temp_extract.clone()).await;
            return Err(e.into());
        }
    }

    // Clean up temp download
    fs::remove_file(&temp_download).await.ok();

    // Final verification: executable exists and is executable (on Unix)
    let final_executable = version_dir.join(executable_name);
    if !final_executable.exists() {
        fs::remove_dir_all(&version_dir).await.ok();
        return Err(
            InstallError::ExtractFailed("Executável não encontrado após finalização".to_string())
                .into(),
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(&final_executable).await?;
        // Check if executable bit is set (any of user/group/other)
        if metadata.mode() & 0o111 == 0 {
            fs::remove_dir_all(&version_dir).await.ok();
            return Err(
                InstallError::ExtractFailed(
                    "Executável não tem permissão de execução após finalização".to_string(),
                )
                .into(),
            );
        }
    }

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

async fn extract_zip(zip_path: PathBuf, dest: PathBuf) -> Result<(), InstallError> {
    // Run extraction in blocking task to avoid Send issues with zip crate types
    tokio::task::spawn_blocking(move || -> Result<(), InstallError> {
        let file = StdFile::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut extracted_count = 0;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;

            // Use enclosed_name() which returns None if path tries to escape
            let Some(entry_path) = entry.enclosed_name() else {
                return Err(InstallError::ExtractFailed(
                    "entrada insegura no arquivo (zip-slip)".to_string(),
                ));
            };

            // Reject absolute paths, Windows prefixes, and parent directory components
            if entry_path.components().any(|c| {
                matches!(
                    c,
                    Component::RootDir | Component::Prefix(_) | Component::ParentDir
                )
            }) {
                return Err(InstallError::ExtractFailed(
                    "entrada insegura no arquivo".to_string(),
                ));
            }

            let target_path = dest.join(entry_path);

            if entry.is_dir() {
                std::fs::create_dir_all(&target_path)?;
            } else {
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = StdFile::create(&target_path)?;
                std::io::copy(&mut entry, &mut outfile)?;
            }
            extracted_count += 1;
        }

        if extracted_count == 0 {
            return Err(InstallError::ExtractFailed(
                "nenhum arquivo extraído (arquivo vazio ou todas as entradas inseguras)"
                    .to_string(),
            ));
        }

        Ok(())
    })
    .await
    .map_err(|e| InstallError::ExtractFailed(e.to_string()))?
}

async fn extract_tar_gz(tar_gz_path: PathBuf, dest: PathBuf) -> Result<(), InstallError> {
    // Run extraction in blocking task to avoid Send issues with tar crate types
    tokio::task::spawn_blocking(move || -> Result<(), InstallError> {
        let file = StdFile::open(&tar_gz_path)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let mut extracted_count = 0;

        // Collect all entries first
        let entries: Vec<_> = archive.entries()?.collect::<Result<Vec<_>, _>>()?;

        for mut entry in entries {
            let entry_path = entry.path()?;

            // Path traversal protection: reject .. components, absolute paths, and Windows prefixes
            if entry_path.components().any(|c| {
                matches!(
                    c,
                    Component::RootDir | Component::Prefix(_) | Component::ParentDir
                )
            }) {
                return Err(InstallError::ExtractFailed(
                    "entrada insegura no arquivo".to_string(),
                ));
            }

            let target_path = dest.join(entry_path);

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&target_path)?;
            } else {
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                entry.unpack(&target_path)?;
            }
            extracted_count += 1;
        }

        if extracted_count == 0 {
            return Err(InstallError::ExtractFailed(
                "nenhum arquivo extraído (arquivo vazio ou todas as entradas inseguras)"
                    .to_string(),
            ));
        }

        Ok(())
    })
    .await
    .map_err(|e| InstallError::ExtractFailed(e.to_string()))?
}

async fn normalize_install_structure(extract_dir: &Path, _tag: &str) -> Result<(), InstallError> {
    // Check if there's a single top-level folder (like "Weft/")
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
    if entry_count == 1
        && let Some(single_dir) = first_entry
        && single_dir.is_dir()
    {
        // Collect entries first to avoid concurrent modification issues
        let mut entries_to_move = Vec::new();
        let mut entries = fs::read_dir(&single_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            // Get metadata from DirEntry directly (more reliable on Windows)
            let metadata = entry.metadata().await.ok();
            entries_to_move.push((entry, metadata));
        }
        for (entry, metadata) in entries_to_move {
            let source = entry.path();
            let target = extract_dir.join(entry.file_name());
            eprintln!("Normalize: moving {:?} -> {:?}", source, target);
            // Ensure source exists
            if !source.exists() {
                eprintln!("Source does not exist, skipping");
                continue;
            }
            // On Windows, remove target first if it exists (rename fails if target exists)
            if target.exists() {
                if target.is_dir() {
                    fs::remove_dir_all(&target).await?;
                } else {
                    fs::remove_file(&target).await?;
                }
            }
            eprintln!("  About to rename...");
            // On Windows, rename can fail for files; use copy+remove as fallback
            #[cfg(windows)]
            {
                let is_file = metadata.as_ref().map(|m| m.file_type().is_file()).unwrap_or(false);
                eprintln!("  Windows branch, is_file={}", is_file);
                if is_file {
                    eprintln!("  Trying copy...");
                    // On Windows, file might not be immediately accessible after tar extraction
                    // Retry a few times
                    let mut last_err = None;
                    for attempt in 0..5 {
                        match fs::copy(&source, &target).await {
                            Ok(_) => {
                                eprintln!("  Copy succeeded on attempt {}", attempt + 1);
                                last_err = None;
                                break;
                            }
                            Err(e) => {
                                let err_msg = e.to_string();
                                last_err = Some(e);
                                eprintln!("  Copy attempt {} failed: {}, retrying...", attempt + 1, err_msg);
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }
                        }
                    }
                    if let Some(e) = last_err {
                        return Err(e.into());
                    }
                    eprintln!("  Copy succeeded, removing source...");
                    fs::remove_file(&source).await?;
                    eprintln!("  Copy+remove succeeded");
                } else {
                    fs::rename(&source, &target).await?;
                    eprintln!("  Rename succeeded");
                }
            }
            #[cfg(not(windows))]
            {
                fs::rename(&source, &target).await?;
                eprintln!("  Rename succeeded");
            }
        }
        eprintln!("Removing single_dir: {:?}", single_dir);
        fs::remove_dir(single_dir).await?;
        eprintln!("remove_dir succeeded");
    }
    // If there are multiple entries or a single file at root, that's fine - already normalized

    Ok(())
}

#[tauri::command]
pub async fn get_installed_versions(
    app_handle: tauri::AppHandle,
) -> Result<Vec<InstalledVersion>, AppError> {
    let versions_dir = get_versions_dir(&app_handle)?;

    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    let mut entries = fs::read_dir(&versions_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        let executable_name = executable_name();
        let executable = path.join(executable_name);

        if executable.exists() {
            let metadata = fs::metadata(&path).await?;
            let installed_at = metadata
                .created()
                .ok()
                .and_then(|t| {
                    time::OffsetDateTime::from_unix_timestamp(
                        t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                    )
                    .ok()
                })
                .and_then(|dt| {
                    dt.format(&time::format_description::well_known::Rfc3339)
                        .ok()
                })
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
pub async fn remove_version(app_handle: tauri::AppHandle, tag: String) -> Result<(), AppError> {
    let versions_dir = get_versions_dir(&app_handle)?;
    let version_dir = versions_dir.join(&tag);

    if !version_dir.exists() {
        return Err(InstallError::NotInstalled(tag).into());
    }

    // Check if version is currently running
    if is_version_running(&app_handle, &tag).await {
        return Err(InstallError::GameRunning(tag).into());
    }

    // Try to rename the folder first (atomic on Windows, fails if file in use)
    let removing_dir = versions_dir.join(format!(".removing-{}", tag));
    match fs::rename(&version_dir, &removing_dir).await {
        Ok(_) => {
            // Rename succeeded, now delete the renamed folder
            fs::remove_dir_all(&removing_dir).await?;
            Ok(())
        }
        Err(e) => {
            // Rename failed - likely file in use on Windows
            // Check if it's a "file in use" type error
            if e.kind() == std::io::ErrorKind::PermissionDenied
                || e.raw_os_error() == Some(32) // ERROR_SHARING_VIOLATION on Windows
                || e.raw_os_error() == Some(26)
            // ETXTBSY on Unix
            {
                Err(InstallError::VersionInUse(tag).into())
            } else {
                Err(e.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::fs;

    /// Creates a minimal valid tar.gz in memory with structure:
    /// Weft/weft (executable) - uses platform-specific name
    /// Weft/assets/a.txt
    /// Weft/run/
    fn create_test_tar_gz() -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;

        let exe_name = executable_name();
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut ar = Builder::new(&mut gz);
            // Weft/ (directory) - explicit parent directory
            let mut header = tar::Header::new_gnu();
            header.set_path("Weft/").unwrap();
            header.set_mode(0o755);
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_cksum();
            ar.append(&header, &b""[..]).unwrap();

            // Weft/assets/ (directory) - explicit parent directory
            let mut header = tar::Header::new_gnu();
            header.set_path("Weft/assets/").unwrap();
            header.set_mode(0o755);
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_cksum();
            ar.append(&header, &b""[..]).unwrap();

            // Weft/<exe_name> (executable)
            let exe_path = format!("Weft/{}", exe_name);
            let mut header = tar::Header::new_gnu();
            header.set_path(&exe_path).unwrap();
            header.set_mode(0o755);
            header.set_size(4);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            ar.append(&header, &b"fake"[..]).unwrap();

            // Weft/assets/a.txt
            let mut header = tar::Header::new_gnu();
            header.set_path("Weft/assets/a.txt").unwrap();
            header.set_mode(0o644);
            header.set_size(1);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            ar.append(&header, &b"a"[..]).unwrap();

            // Weft/run/ (directory)
            let mut header = tar::Header::new_gnu();
            header.set_path("Weft/run/").unwrap();
            header.set_mode(0o755);
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_cksum();
            ar.append(&header, &b""[..]).unwrap();
        }
        gz.finish().unwrap()
    }

    /// Creates a minimal zip with executable at root (platform-specific name)
    fn create_test_zip() -> Vec<u8> {
        use zip::write::FileOptions;
        use zip::CompressionMethod;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = FileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o755);
            let exe_name = executable_name();
            zip.start_file(exe_name, options).unwrap();
            zip.write_all(b"fake").unwrap();
            zip.finish().unwrap();
        }
        buf
    }

    /// Creates a tar.gz with path traversal attempt (../fora.txt)
    /// Uses raw tar format to bypass tar crate's path validation during creation
    fn create_malicious_tar_gz() -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        {
            // Manually write a tar entry with ../fora.txt path
            // Tar format: 512-byte header + padded data
            let mut header = [0u8; 512];
            // Name field (100 bytes)
            let name = b"../fora.txt";
            header[0..name.len()].copy_from_slice(name);
            // Mode (8 bytes, octal)
            let mode = b"0000644 ";
            header[100..108].copy_from_slice(mode);
            // UID (8 bytes)
            header[108..116].copy_from_slice(b"0000000 ");
            // GID (8 bytes)
            header[116..124].copy_from_slice(b"0000000 ");
            // Size (12 bytes, octal) - 4 bytes
            let size = b"00000000004 ";
            header[124..136].copy_from_slice(size);
            // Mtime (12 bytes)
            header[136..148].copy_from_slice(b"00000000000 ");
            // Checksum (8 bytes) - will compute
            // Typeflag (1 byte) - regular file
            header[156] = b'0';
            // Magic (6 bytes) - "ustar\0"
            header[257..263].copy_from_slice(b"ustar\0");
            // Version (2 bytes) - "00"
            header[263..265].copy_from_slice(b"00");
            // Compute checksum (sum of all bytes, with checksum field treated as spaces)
            let mut checksum: u32 = 0;
            for (i, &byte) in header.iter().enumerate() {
                if i >= 148 && i < 156 {
                    checksum += b' ' as u32;
                } else {
                    checksum += byte as u32;
                }
            }
            // Format as 6 octal digits + null + space (8 bytes total)
            let checksum_str = format!("{:06o}\0 ", checksum);
            header[148..156].copy_from_slice(checksum_str.as_bytes());

            gz.write_all(&header).unwrap();
            gz.write_all(b"evil").unwrap();
            // Pad to 512 bytes
            gz.write_all(&[0u8; 508]).unwrap();
            // End of archive (two 512-byte blocks of zeros)
            gz.write_all(&[0u8; 1024]).unwrap();
        }
        gz.finish().unwrap()
    }

    #[tokio::test]
    async fn test_tar_gz_extraction_normalizes_structure() {
        let tmp = tempdir().unwrap();
        let tar_gz_path = tmp.path().join("test.tar.gz");
        let extract_dir = tmp.path().join("extract");

        fs::write(&tar_gz_path, create_test_tar_gz()).await.unwrap();
        fs::create_dir_all(&extract_dir).await.unwrap();

        // Extract
        extract_tar_gz(tar_gz_path, extract_dir.clone()).await.unwrap();

        // Debug: list directory contents after extraction
        let mut entries = fs::read_dir(&extract_dir).await.unwrap();
        eprintln!("After extraction, extract_dir contents:");
        while let Some(entry) = entries.next_entry().await.unwrap() {
            eprintln!("  {:?} (is_dir={})", entry.path(), entry.path().is_dir());
            if entry.path().is_dir() {
                let mut sub = fs::read_dir(entry.path()).await.unwrap();
                while let Some(sub_entry) = sub.next_entry().await.unwrap() {
                    eprintln!("    {:?}", sub_entry.path());
                }
            }
        }

        // Normalize (should move Weft/* up)
        normalize_install_structure(&extract_dir, "v1.0.0").await.unwrap();

        // Verify structure: weft executable at root, assets/ and run/ at root
        let weft_path = extract_dir.join(executable_name());
        assert!(weft_path.exists(), "weft executable should exist at root");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&weft_path).await.unwrap();
            assert_eq!(metadata.mode() & 0o111, 0o111, "executable should have execute bits");
        }

        assert!(extract_dir.join("assets").join("a.txt").exists(), "assets/a.txt should exist");
        assert!(extract_dir.join("run").exists(), "run/ directory should exist");

        // No leftover Weft/ directory
        assert!(!extract_dir.join("Weft").exists(), "Weft/ directory should be removed");
    }

    #[tokio::test]
    async fn test_zip_extraction_executable_at_root() {
        let tmp = tempdir().unwrap();
        let zip_path = tmp.path().join("test.zip");
        let extract_dir = tmp.path().join("extract");

        fs::write(&zip_path, create_test_zip()).await.unwrap();
        fs::create_dir_all(&extract_dir).await.unwrap();

        // Extract
        extract_zip(zip_path, extract_dir.clone()).await.unwrap();

        // Verify weft executable at root
        let weft_path = extract_dir.join(executable_name());
        assert!(weft_path.exists(), "weft executable should exist at root");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&weft_path).await.unwrap();
            assert_eq!(metadata.mode() & 0o111, 0o111, "executable should have execute bits");
        }
    }

    #[tokio::test]
    async fn test_tar_gz_rejects_path_traversal() {
        let tmp = tempdir().unwrap();
        let tar_gz_path = tmp.path().join("malicious.tar.gz");
        let extract_dir = tmp.path().join("extract");

        fs::write(&tar_gz_path, create_malicious_tar_gz()).await.unwrap();
        fs::create_dir_all(&extract_dir).await.unwrap();

        // Should fail with ExtractFailed
        let result = extract_tar_gz(tar_gz_path, extract_dir).await;
        assert!(result.is_err(), "should reject path traversal");
        let err = result.unwrap_err();
        assert!(matches!(err, InstallError::ExtractFailed(_)), "should be ExtractFailed");
    }
}
