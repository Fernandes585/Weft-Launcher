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
use crate::config::USER_AGENT_VALUE;
use crate::launch::is_version_running;
use crate::error::AppError;

/// Returns the platform-specific executable name for Weft.
pub fn executable_name() -> &'static str {
    if cfg!(target_os = "windows") { "weft.exe" } else { "weft" }
}

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
            eprintln!("Extract error: {}", e);
        }
        extract_result?;

    // Normalize structure (handle top-level folder)
    if let Err(e) = normalize_install_structure(&temp_extract, &tag).await {
            eprintln!("Normalize error: {}", e);
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
        let mut perms = fs::metadata(&temp_executable).await?.permissions();
        perms.set_mode(0o755);
        if let Err(e) = fs::set_permissions(&temp_executable, perms).await {
            eprintln!("Permission error: {}", e);
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
            eprintln!("Rename error: {}", e);
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
    tokio::task::spawn_blocking(move || extract_zip_blocking(&zip_path, &dest))
        .await
        .map_err(|e| {
            InstallError::ExtractFailed(format!(
                "extraction task failed: {}",
                build_error_chain(&e)
            ))
        })?
}

fn extract_zip_blocking(zip_path: &Path, dest: &Path) -> Result<(), InstallError> {
    let file = StdFile::open(zip_path)?;
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

        if !is_safe_archive_path(entry_path) {
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
            "nenhum arquivo extraído (arquivo vazio ou todas as entradas inseguras)".to_string(),
        ));
    }

    Ok(())
}

/// Rejects absolute paths and any `..`, root, or Windows prefix component
/// before anything is written to disk.
fn is_safe_archive_path(path: &Path) -> bool {
    !path.is_absolute()
        && !path.components().any(|c| {
            matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::ParentDir
            )
        })
}

/// Lexically resolves a link target and checks that it cannot leave the
/// extraction root. No `canonicalize` (and therefore no symlink following).
fn link_target_stays_inside(link_path: &Path, target: &Path, relative_to_root: bool) -> bool {
    let mut depth = if relative_to_root {
        0i32
    } else {
        link_path
            .parent()
            .map(|p| {
                p.components()
                    .filter(|c| matches!(c, Component::Normal(_)))
                    .count() as i32
            })
            .unwrap_or(0)
    };

    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// macOS archives often carry `__MACOSX/` folders and `._*` resource forks.
fn is_macos_metadata(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, Component::Normal(name) if name == "__MACOSX"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("._"))
}

/// Build a full error chain message from an error and its sources.
fn build_error_chain(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(src) = source {
        msg.push_str(": ");
        msg.push_str(&src.to_string());
        source = src.source();
    }
    msg
}

async fn extract_tar_gz(tar_gz_path: PathBuf, dest: PathBuf) -> Result<(), InstallError> {
    // Run extraction in blocking task to avoid Send issues with tar crate types
    tokio::task::spawn_blocking(move || extract_tar_gz_blocking(&tar_gz_path, &dest))
        .await
        .map_err(|e| {
            InstallError::ExtractFailed(format!(
                "extraction task failed: {}",
                build_error_chain(&e)
            ))
        })?
}

fn extract_tar_gz_blocking(tar_gz_path: &Path, dest: &Path) -> Result<(), InstallError> {
    let file = StdFile::open(tar_gz_path).map_err(|e| {
        InstallError::ExtractFailed(format!(
            "failed to open `{}`: {}",
            tar_gz_path.display(),
            build_error_chain(&e)
        ))
    })?;
    let decoder = flate2::read::MultiGzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(true);

    let mut extracted_count = 0;

    // gzip input is not seekable: each entry must be consumed while the
    // iterator still owns it. Never collect entries into a Vec first.
    for entry_result in archive.entries()? {
        let mut entry = entry_result.map_err(|e| {
            InstallError::ExtractFailed(format!(
                "failed to read tar entry: {}",
                build_error_chain(&e)
            ))
        })?;

        let entry_path = entry
            .path()
            .map_err(|e| {
                InstallError::ExtractFailed(format!(
                    "failed to read entry path: {}",
                    build_error_chain(&e)
                ))
            })?
            .into_owned();

        if is_macos_metadata(&entry_path) {
            continue;
        }

        if !is_safe_archive_path(&entry_path) {
            return Err(InstallError::ExtractFailed(
                "entrada insegura no arquivo".to_string(),
            ));
        }

        let entry_type = entry.header().entry_type();

        if entry_type.is_dir() || entry_type.is_file() {
            let unpacked = entry.unpack_in(dest).map_err(|e| {
                InstallError::ExtractFailed(format!(
                    "failed to unpack `{}`: {}",
                    entry_path.display(),
                    build_error_chain(&e)
                ))
            })?;
            if !unpacked {
                return Err(InstallError::ExtractFailed(
                    "entrada insegura no arquivo".to_string(),
                ));
            }
            extracted_count += 1;
            continue;
        }

        if entry_type.is_symlink() || entry_type.is_hard_link() {
            let target = entry
                .link_name()
                .map_err(|e| {
                    InstallError::ExtractFailed(format!(
                        "failed to read link target of `{}`: {}",
                        entry_path.display(),
                        build_error_chain(&e)
                    ))
                })?
                .ok_or_else(|| {
                    InstallError::ExtractFailed(format!(
                        "link entry `{}` has no target",
                        entry_path.display()
                    ))
                })?;

            let stays_inside = if entry_type.is_symlink() {
                link_target_stays_inside(&entry_path, &target, false)
            } else {
                link_target_stays_inside(&entry_path, &target, true)
            };

            if !stays_inside {
                return Err(InstallError::ExtractFailed(format!(
                    "entrada insegura no arquivo: link `{}` aponta para `{}`",
                    entry_path.display(),
                    target.display()
                )));
            }

            let unpacked = entry.unpack_in(dest).map_err(|e| {
                InstallError::ExtractFailed(format!(
                    "failed to unpack link `{}`: {}",
                    entry_path.display(),
                    build_error_chain(&e)
                ))
            })?;
            if !unpacked {
                return Err(InstallError::ExtractFailed(
                    "entrada insegura no arquivo".to_string(),
                ));
            }
            extracted_count += 1;
        }
        // Device nodes, FIFOs and extension headers are not part of the game
        // assets and are ignored.
    }

    if extracted_count == 0 {
        return Err(InstallError::ExtractFailed(
            "no files extracted (archive empty or all entries unsafe)".to_string(),
        ));
    }

    Ok(())
}

/// Returns an unused sibling path used to temporarily rename the top-level
/// folder while its contents are lifted to the extraction root.
fn unique_staging_dir(extract_dir: &Path) -> PathBuf {
    let mut counter = 0u32;
    loop {
        let candidate = extract_dir.join(format!(".normalize-{}-{}", std::process::id(), counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
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
    drop(entries);

    // If there's exactly one folder at root, move its contents up
    if entry_count == 1
        && let Some(single_dir) = first_entry
        && single_dir.is_dir()
    {
        // Rename the folder before moving anything: on case-insensitive
        // filesystems (default macOS/Windows) a child such as `weft` would
        // otherwise resolve to the folder itself (`Weft/`).
        let staging = unique_staging_dir(extract_dir);
        fs::rename(&single_dir, &staging).await?;

        // Collect entries first to avoid concurrent modification issues
        let mut entries_to_move = Vec::new();
        let mut entries = fs::read_dir(&staging).await?;
        while let Some(entry) = entries.next_entry().await? {
            entries_to_move.push(entry);
        }
        drop(entries);

        for entry in entries_to_move {
            let file_name = entry.file_name();
            let source = staging.join(&file_name);
            let target = extract_dir.join(&file_name);
            // On Windows, remove target first if it exists (rename fails if target exists)
            if target.exists() {
                if target.is_dir() {
                    fs::remove_dir_all(&target).await?;
                } else {
                    fs::remove_file(&target).await?;
                }
            }
            // On Windows, rename can fail for files; use copy+remove as fallback
            #[cfg(windows)]
            {
                // DirEntry metadata is more reliable than path metadata right
                // after extraction on Windows.
                let is_file = entry
                    .metadata()
                    .await
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false);
                if is_file {
                    // On Windows, file might not be immediately accessible after tar extraction
                    // Retry a few times
                    let mut last_err = None;
                    for _attempt in 0..5 {
                        match fs::copy(&source, &target).await {
                            Ok(_) => {
                                last_err = None;
                                break;
                            }
                            Err(e) => {
                                last_err = Some(e);
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }
                        }
                    }
                    if let Some(e) = last_err {
                        return Err(e.into());
                    }
                    fs::remove_file(&source).await?;
                } else {
                    fs::rename(&source, &target).await?;
                }
            }
            #[cfg(not(windows))]
            {
                fs::rename(&source, &target).await?;
            }
        }
        fs::remove_dir(&staging).await?;
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
mod tests;

