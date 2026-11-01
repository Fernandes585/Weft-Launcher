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
}

/// Remove ANSI escape codes from a string.
fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Potential ANSI escape sequence: ESC [
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we find a letter (typically 'm' for SGR)
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            // Not an ANSI sequence, keep the ESC char
            result.push(c);
        } else if c == '[' {
            // Handle case where ESC was already stripped: [...m
            // Peek ahead to see if this looks like an SGR sequence
            let mut peeked = chars.clone();
            let mut looks_like_sgr = false;
            let mut has_digit = false;
            for pc in peeked.by_ref() {
                if pc.is_ascii_digit() {
                    has_digit = true;
                } else if pc == ';' {
                    if !has_digit {
                        break;
                    }
                    has_digit = false;
                } else if pc == 'm' && has_digit {
                    looks_like_sgr = true;
                    break;
                } else {
                    break;
                }
            }
            if looks_like_sgr {
                // Consume the sequence
                for pc in chars.by_ref() {
                    if pc == 'm' {
                        break;
                    }
                }
                continue;
            }
            // Not an SGR sequence, keep the bracket
            result.push(c);
        } else {
            result.push(c);
        }
    }
    result
}

/// Extract game folder path from a log line containing "game folder <path>, cache <path>".
fn extract_game_folder_path(line: &str) -> Option<String> {
    let cleaned = strip_ansi_codes(line);
    let prefix = "game folder ";
    let suffix = ", cache ";
    let start = cleaned.find(prefix)?;
    let start = start + prefix.len();
    let end = cleaned[start..].find(suffix)?;
    Some(cleaned[start..start + end].trim().to_string())
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
    let executable_name = if cfg!(target_os = "windows") {
        "weft.exe"
    } else {
        "weft"
    };
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

#[tauri::command]
pub async fn open_game_folder(app_handle: AppHandle, _tag: String) -> Result<(), AppError> {
    let logs_dir = get_logs_dir(&app_handle)?;

    // Read all log files, sort by modification time (newest first)
    let mut entries = fs::read_dir(&logs_dir).await?;
    let mut log_files: Vec<_> = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("log")
            && let Ok(metadata) = entry.metadata().await
            && let Ok(modified) = metadata.modified()
        {
            log_files.push((path, modified));
        }
    }

    // Sort by modification time, newest first
    log_files.sort_by_key(|a| std::cmp::Reverse(a.1));

    // Search each log file for the "game folder" line
    for (log_path, _) in log_files {
        let file = fs::File::open(&log_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(game_folder) = extract_game_folder_path(&line) {
                let game_folder_path = PathBuf::from(&game_folder);
                if game_folder_path.exists() {
                    app_handle
                        .opener()
                        .open_path(game_folder_path.to_string_lossy().as_ref(), None::<&str>)
                        .map_err(LaunchError::OpenerError)?;
                    return Ok(());
                }
                // Path from log doesn't exist, continue searching
            }
        }
    }

    // No valid game folder found in any log
    Err(LaunchError::GameFolderNotFound(
        "Game folder not found in logs (game may not have been launched yet)".to_string(),
    )
    .into())
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
