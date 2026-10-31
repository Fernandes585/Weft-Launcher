use serde::{Deserialize, Serialize};
use std::fs::File as StdFile;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Manager;
use thiserror::Error;
use tokio::fs;
use time::OffsetDateTime;
use walkdir::WalkDir;
use zip::result::ZipError;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de serialização: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Erro de zip: {0}")]
    Zip(#[from] ZipError),
    #[error("Erro de walkdir: {0}")]
    Walkdir(#[from] walkdir::Error),
    #[error("Instância já existe: {0}")]
    AlreadyExists(String),
    #[error("Instância não encontrada: {0}")]
    NotFound(String),
    #[error("Versão não instalada: {0}")]
    VersionNotInstalled(String),
    #[error("Backup falhou: {0}")]
    BackupFailed(String),
}

impl From<InstanceError> for String {
    fn from(err: InstanceError) -> String {
        err.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub version_tag: String,
    pub run_dir: String,
    pub created_at: String,
    pub last_played: Option<String>,
    pub last_version: Option<String>,
}

fn get_instances_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, InstanceError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InstanceError::Io(std::io::Error::other(e)))?;
    Ok(app_data.join("instances"))
}

fn get_backups_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, InstanceError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| InstanceError::Io(std::io::Error::other(e)))?;
    Ok(app_data.join("backups"))
}

fn get_instances_file(app_handle: &tauri::AppHandle) -> Result<PathBuf, InstanceError> {
    Ok(get_instances_dir(app_handle)?.join("instances.json"))
}

async fn ensure_instances_dir(app_handle: &tauri::AppHandle) -> Result<(), InstanceError> {
    let dir = get_instances_dir(app_handle)?;
    fs::create_dir_all(&dir).await?;
    Ok(())
}

async fn load_instances(app_handle: &tauri::AppHandle) -> Result<Vec<Instance>, InstanceError> {
    let file = get_instances_file(app_handle)?;
    if !file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&file).await?;
    let instances: Vec<Instance> = serde_json::from_str(&content)?;
    Ok(instances)
}

async fn save_instances(app_handle: &tauri::AppHandle, instances: &[Instance]) -> Result<(), InstanceError> {
    ensure_instances_dir(app_handle).await?;
    let file = get_instances_file(app_handle)?;
    let content = serde_json::to_string_pretty(instances)?;
    fs::write(&file, content).await?;
    Ok(())
}

#[tauri::command]
pub async fn list_instances(app_handle: tauri::AppHandle) -> Result<Vec<Instance>, String> {
    load_instances(&app_handle).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_instance(
    app_handle: tauri::AppHandle,
    name: String,
    version_tag: String,
) -> Result<Instance, String> {
    // Validate name
    if name.trim().is_empty() {
        return Err("Nome da instância não pode ser vazio".to_string());
    }
    if name.contains('/') || name.contains('\\') || name.contains(':') || name.contains('*') || name.contains('?') || name.contains('"') || name.contains('<') || name.contains('>') || name.contains('|') {
        return Err("Nome contém caracteres inválidos".to_string());
    }

    // Check if version is installed
    let versions_dir = get_instances_dir(&app_handle)?.parent().unwrap().join("versions");
    let version_dir = versions_dir.join(&version_tag);
    let executable_name = if cfg!(target_os = "windows") { "weft.exe" } else { "weft" };
    let executable = version_dir.join(executable_name);

    if !executable.exists() {
        return Err(InstanceError::VersionNotInstalled(version_tag).to_string());
    }

    // Check if instance already exists
    let mut instances = load_instances(&app_handle).await.map_err(|e| e.to_string())?;
    if instances.iter().any(|i| i.name == name) {
        return Err(InstanceError::AlreadyExists(name).to_string());
    }

    // Create run directory
    let instances_dir = get_instances_dir(&app_handle)?;
    let run_dir = instances_dir.join(&name).join("run");
    fs::create_dir_all(&run_dir).await.map_err(|e| e.to_string())?;

    let now = OffsetDateTime::now_utc();
    let created_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let instance = Instance {
        name: name.clone(),
        version_tag,
        run_dir: run_dir.to_string_lossy().to_string(),
        created_at,
        last_played: None,
        last_version: None,
    };

    instances.push(instance.clone());
    save_instances(&app_handle, &instances).await.map_err(|e| e.to_string())?;

    Ok(instance)
}

async fn backup_instance_run_dir(
    app_handle: &tauri::AppHandle,
    instance_name: &str,
    run_dir: &Path,
) -> Result<(), InstanceError> {
    let backups_dir = get_backups_dir(app_handle)?;
    let instance_backup_dir = backups_dir.join(instance_name);
    fs::create_dir_all(&instance_backup_dir).await?;

    let now = OffsetDateTime::now_utc();
    let timestamp = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
        .replace(':', "-")
        .replace('T', "_")
        .replace('Z', "");

    let backup_name = format!("{}.zip", timestamp);
    let backup_path = instance_backup_dir.join(&backup_name);

    // Create zip of run directory - use std::fs::File for zip crate compatibility
    let file = StdFile::create(&backup_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let run_dir_canonical = run_dir.canonicalize()?;
    let mut buffer = Vec::new();

    for entry in WalkDir::new(&run_dir_canonical) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(&run_dir_canonical).unwrap();

        if path.is_dir() {
            zip.add_directory(relative.to_string_lossy().to_string(), options)?;
        } else {
            zip.start_file(relative.to_string_lossy().to_string(), options)?;
            let mut file = StdFile::open(path)?;
            std::io::copy(&mut file, &mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        }
    }

    zip.finish()?;
    Ok(())
}

#[tauri::command]
pub async fn launch_instance(
    app_handle: tauri::AppHandle,
    name: String,
    version_tag: String,
) -> Result<(), String> {
    let mut instances = load_instances(&app_handle).await.map_err(|e| e.to_string())?;

    let instance_index = instances.iter().position(|i| i.name == name)
        .ok_or_else(|| InstanceError::NotFound(name.clone()).to_string())?;

    let instance = &mut instances[instance_index];

    // Check if version changed - if so, backup first
    if instance.last_version.as_ref() != Some(&version_tag) {
        let run_dir = Path::new(&instance.run_dir);
        if run_dir.exists() {
            backup_instance_run_dir(&app_handle, &name, run_dir).await
                .map_err(|e| format!("Falha no backup: {}", e))?;
        }
        instance.last_version = Some(version_tag.clone());
    }

    // Update instance version
    instance.version_tag = version_tag.clone();

    // Get executable path
    let versions_dir = get_instances_dir(&app_handle)?.parent().unwrap().join("versions");
    let version_dir = versions_dir.join(&version_tag);
    let executable_name = if cfg!(target_os = "windows") { "weft.exe" } else { "weft" };
    let executable = version_dir.join(executable_name);

    if !executable.exists() {
        return Err(InstanceError::VersionNotInstalled(version_tag).to_string());
    }

    // Launch the game with --run-dir
    let run_dir = Path::new(&instance.run_dir);
    let mut cmd = std::process::Command::new(&executable);
    cmd.arg("--run-dir").arg(run_dir);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    cmd.spawn().map_err(|e| format!("Falha ao iniciar o jogo: {}", e))?;

    // Update last_played
    let now = OffsetDateTime::now_utc();
    instance.last_played = Some(
        now.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    );

    save_instances(&app_handle, &instances).await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_instance(
    app_handle: tauri::AppHandle,
    name: String,
) -> Result<(), String> {
    let mut instances = load_instances(&app_handle).await.map_err(|e| e.to_string())?;

    let instance_index = instances.iter().position(|i| i.name == name)
        .ok_or_else(|| InstanceError::NotFound(name.clone()).to_string())?;

    let instance = instances.remove(instance_index);

    // Remove run directory
    let run_dir = Path::new(&instance.run_dir);
    if run_dir.exists() {
        fs::remove_dir_all(run_dir).await.map_err(|e| e.to_string())?;
    }

    // Remove instance directory if empty
    let instance_dir = run_dir.parent();
    if let Some(dir) = instance_dir {
        match fs::read_dir(dir).await {
            Ok(mut entries) => {
                if entries.next_entry().await.unwrap_or(None).is_none() {
                    fs::remove_dir(dir).await.ok();
                }
            }
            Err(_) => {}
        }
    }

    save_instances(&app_handle, &instances).await.map_err(|e| e.to_string())?;
    Ok(())
}