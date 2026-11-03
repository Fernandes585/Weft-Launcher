use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::{OpenerExt, Error as OpenerError};
use thiserror::Error;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;
use crate::error::AppError;
use crate::install::executable_name;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Versão já está rodando: {0}")]
    AlreadyRunning(String),
    #[error("Outro jogo já está rodando: {0}")]
    AnotherGameRunning(String),
    #[error("Executável não encontrado: {0}")]
    ExecutableNotFound(String),
    #[error("Falha ao iniciar o processo: {0}")]
    ProcessSpawnFailed(String),
    #[error("Erro ao criar diretório de logs: {0}")]
    LogDirCreateFailed(String),
    #[error("Pasta do jogo não encontrada: {0}")]
    GameFolderNotFound(String),
    #[error("Erro ao abrir pasta: {0}")]
    OpenerError(#[from] OpenerError),
    #[error("Falha ao abrir pasta do jogo: {0}")]
    OpenGameFolderError(String),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStateEvent {
    pub tag: String,
    pub state: String,
    pub exit_code: Option<i32>,
}

pub struct RunningProcess {
    _tag: String,
    child: tokio::process::Child,
    _log_file: tokio::fs::File,
    tx: mpsc::Sender<()>,
}

type ProcessMap = Arc<Mutex<HashMap<String, RunningProcess>>>;

/// Get the process map state, creating it if needed.
/// Made public so install.rs can check running versions.
pub fn get_process_state(app_handle: &AppHandle) -> Result<ProcessMap, LaunchError> {
    let state = app_handle.try_state::<ProcessMap>();
    match state {
        Some(s) => Ok(s.inner().clone()),
        None => {
            let map: ProcessMap = Arc::new(Mutex::new(HashMap::new()));
            app_handle.manage(map.clone());
            Ok(map)
        }
    }
}

/// Check if a specific version is currently running.
pub async fn is_version_running(app_handle: &AppHandle, tag: &str) -> bool {
    if let Ok(process_map) = get_process_state(app_handle) {
        let map = process_map.lock().unwrap();
        map.contains_key(tag)
    } else {
        false
    }
}

/// Get all currently running version tags.
pub async fn get_running_tags(app_handle: &AppHandle) -> Vec<String> {
    if let Ok(process_map) = get_process_state(app_handle) {
        let map = process_map.lock().unwrap();
        map.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

fn get_versions_dir(app_handle: &AppHandle) -> Result<PathBuf, LaunchError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| LaunchError::Io(std::io::Error::other(e)))?;
    Ok(app_data.join("versions"))
}

fn get_logs_dir(app_handle: &AppHandle) -> Result<PathBuf, LaunchError> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| LaunchError::Io(std::io::Error::other(e)))?;
    let logs_dir = app_data.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| LaunchError::LogDirCreateFailed(e.to_string()))?;
    Ok(logs_dir)
}

#[tauri::command]
pub async fn launch_version(app_handle: AppHandle, tag: String) -> Result<(), AppError> {
    let versions_dir = get_versions_dir(&app_handle)?;
    let version_dir = versions_dir.join(&tag);
    let executable_name = executable_name();
    let executable = version_dir.join(executable_name);

    if !executable.exists() {
        return Err(LaunchError::ExecutableNotFound(tag).into());
    }

    let process_map = get_process_state(&app_handle)?;

    // Check if already running (atomic check under lock)
    {
        let map = process_map.lock().unwrap();

        // Check if THIS version is already running
        if map.contains_key(&tag) {
            return Err(LaunchError::AlreadyRunning(tag).into());
        }

        // Check if ANY other game is running (global single-instance)
        if let Some((running_tag, _)) = map.iter().next() {
            return Err(LaunchError::AnotherGameRunning(running_tag.clone()).into());
        }
    } // lock released here

    // Create log file (async, no lock held)
    let logs_dir = get_logs_dir(&app_handle)?;
    let log_path = logs_dir.join(format!("{}.log", tag));
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await?;

    // Prepare command
    let mut cmd = TokioCommand::new(&executable);
    cmd.current_dir(&version_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // Spawn process
    let mut child = cmd
        .spawn()
        .map_err(|e| LaunchError::ProcessSpawnFailed(e.to_string()))?;

    let app_handle_clone = app_handle.clone();
    let tag_clone = tag.clone();

    // If we got here, spawn succeeded. Set up logging and store process.
    let stdout = child.stdout.take().expect("stdout not captured");
    let stderr = child.stderr.take().expect("stderr not captured");

    let log_file_clone = log_file.try_clone().await?;

    let (tx, _rx) = mpsc::channel::<()>(1);

    // Spawn stdout logging task
    let log_file_stdout = log_file_clone
        .try_clone()
        .await?;
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        let mut log_file = log_file_stdout;
        while let Ok(Some(line)) = reader.next_line().await {
            let log_entry = format!("[stdout] {}\n", line);
            let _ = log_file.write_all(log_entry.as_bytes()).await;
        }
    });

    // Spawn stderr logging task
    let log_file_stderr = log_file_clone
        .try_clone()
        .await?;
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut log_file = log_file_stderr;
        while let Ok(Some(line)) = reader.next_line().await {
            let log_entry = format!("[stderr] {}\n", line);
            let _ = log_file.write_all(log_entry.as_bytes()).await;
        }
    });

    // Store process in map
    let running_process = RunningProcess {
        _tag: tag_clone.clone(),
        child,
        _log_file: log_file_clone,
        tx,
    };

    {
        let mut map = process_map.lock().unwrap();
        map.insert(tag_clone.clone(), running_process);
    }

    // Emit "running" event
    let event = GameStateEvent {
        tag: tag_clone.clone(),
        state: "running".to_string(),
        exit_code: None,
    };
    let _ = app_handle_clone.emit("game-state", &event);

    // Spawn task to wait for process completion
    let app_handle_wait = app_handle_clone;
    let process_map_wait = process_map.clone();
    let tag_wait = tag_clone;
    tokio::spawn(async move {
        let process = {
            let mut map = process_map_wait.lock().unwrap();
            map.remove(&tag_wait)
        };

        if let Some(mut process) = process {
            let exit_status = process.child.wait().await;
            let exit_code = exit_status.ok().and_then(|s| s.code());

            // Emit "stopped" event
            let event = GameStateEvent {
                tag: tag_wait.clone(),
                state: "stopped".to_string(),
                exit_code,
            };
            let _ = app_handle_wait.emit("game-state", &event);

            // Signal completion
            let _ = process.tx.send(()).await;
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn get_running_versions(app_handle: AppHandle) -> Result<Vec<String>, AppError> {
    let process_map = get_process_state(&app_handle)?;
    let map = process_map.lock().unwrap();
    Ok(map.keys().cloned().collect())
}

fn default_game_folder() -> Result<PathBuf, LaunchError> {
    let data_dir = dirs::data_dir().ok_or_else(|| {
        LaunchError::OpenGameFolderError("could not determine the data directory".to_string())
    })?;

    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().eq_ignore_ascii_case("weft") && entry.path().is_dir() {
                return Ok(entry.path());
            }
        }
    }

    Ok(data_dir.join("Weft"))
}

async fn game_folder_from_log(app_handle: &AppHandle, tag: &str) -> Option<PathBuf> {
    let logs_dir = get_logs_dir(app_handle).ok()?;
    let log_file = logs_dir.join(format!("{}.log", tag));
    if !log_file.exists() {
        return None;
    }
    let log_content = fs::read_to_string(&log_file).await.ok()?;
    extract_game_folder_from_log(&log_content).map(PathBuf::from)
}

async fn resolve_game_folder(app_handle: &AppHandle, tag: &str) -> Result<PathBuf, LaunchError> {
    if let Some(path) = game_folder_from_log(app_handle, tag).await {
        if path.is_dir() {
            return Ok(path);
        }
        return Err(LaunchError::GameFolderNotFound(path.display().to_string()));
    }

    let fallback = default_game_folder()?;
    if fallback.is_dir() {
        Ok(fallback)
    } else {
        Err(LaunchError::GameFolderNotFound(fallback.display().to_string()))
    }
}

#[tauri::command]
pub async fn open_game_folder(app_handle: AppHandle, tag: String) -> Result<String, AppError> {
    let game_folder = resolve_game_folder(&app_handle, &tag).await?;
    let path_str = game_folder.to_string_lossy().to_string();

    match app_handle.opener().open_path(&path_str, None::<&str>) {
        Ok(_) => Ok(path_str),
        Err(opener_err) => {
            #[cfg(target_os = "linux")]
            {
                if open_folder_on_linux(&path_str) {
                    return Ok(path_str);
                }
                return Err(LaunchError::OpenGameFolderError(format!(
                    "could not open the game folder (tauri opener: {}; xdg-open, gio and kde-open failed)",
                    opener_err
                ))
                .into());
            }

            #[cfg(not(target_os = "linux"))]
            {
                Err(LaunchError::OpenGameFolderError(format!(
                    "failed to open the game folder: {}",
                    opener_err
                ))
                .into())
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn open_folder_on_linux(path: &str) -> bool {
    let attempts: [(&str, Vec<&str>); 3] = [
        ("xdg-open", vec![path]),
        ("gio", vec!["open", path]),
        ("kde-open", vec![path]),
    ];

    for (program, args) in attempts {
        if let Ok(status) = std::process::Command::new(program).args(&args).status()
            && status.success()
        {
            return true;
        }
    }
    false
}

/// Weft logs its folders on startup: `[weft] game folder <path>, cache <path>`.
fn extract_game_folder_from_log(log_content: &str) -> Option<String> {
    const MARKER: &str = "game folder ";
    let mut found = None;

    for line in log_content.lines() {
        let Some(index) = line.rfind(MARKER) else {
            continue;
        };
        let rest = &line[index + MARKER.len()..];
        let path = rest
            .split(", cache")
            .next()
            .unwrap_or(rest)
            .trim()
            .trim_end_matches([',', '"', '\''])
            .trim();
        if !path.is_empty() {
            found = Some(path.to_string());
        }
    }

    found
}

#[tauri::command]
pub async fn get_game_folder_path(app_handle: AppHandle, tag: String) -> Result<String, AppError> {
    let path = resolve_game_folder(&app_handle, &tag).await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn open_log(app_handle: AppHandle, tag: String) -> Result<(), AppError> {
    let logs_dir = get_logs_dir(&app_handle)?;
    let log_file = logs_dir.join(format!("{}.log", tag));
    if !log_file.exists() {
        return Err(LaunchError::GameFolderNotFound(format!("Log file not found: {}", log_file.display())).into());
    }
    app_handle.opener().open_path(log_file.to_string_lossy().as_ref(), None::<&str>).map_err(LaunchError::OpenerError)?;
    Ok(())
}
