use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Error)]
#[error("{code}{}", .detail.as_ref().map(|d| format!(": {}", d)).unwrap_or_default())]
pub struct AppError {
    pub code: &'static str,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &'static str, detail: Option<String>) -> Self {
        Self { code, detail }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::new("Io", Some(err.to_string()))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::new("Network", Some(err.to_string()))
    }
}

impl From<crate::releases::ReleasesError> for AppError {
    fn from(err: crate::releases::ReleasesError) -> Self {
        use crate::releases::ReleasesError::*;
        match err {
            Network(e) => AppError::new("Network", Some(e.to_string())),
            Io(e) => AppError::new("Io", Some(e.to_string())),
            Serde(e) => AppError::new("Serde", Some(e.to_string())),
            InvalidHeader(e) => AppError::new("InvalidHeader", Some(e.to_string())),
            CacheCorrupted(msg) => AppError::new("CacheCorrupted", Some(msg)),
            AssetNotFound => AppError::new("AssetNotFound", None),
            MissingDigest => AppError::new("MissingDigest", None),
            RateLimited => AppError::new("RateLimited", None),
            Forbidden => AppError::new("Forbidden", None),
            OfflineNoCache => AppError::new("OfflineNoCache", None),
        }
    }
}

impl From<crate::install::InstallError> for AppError {
    fn from(err: crate::install::InstallError) -> Self {
        use crate::install::InstallError::*;
        match err {
            Network(e) => AppError::new("Network", Some(e.to_string())),
            Io(e) => AppError::new("Io", Some(e.to_string())),
            Serde(e) => AppError::new("Serde", Some(e.to_string())),
            Zip(e) => AppError::new("Zip", Some(e.to_string())),
            CreateDir(msg) => AppError::new("CreateDir", Some(msg)),
            Sha256Mismatch { expected, actual } => AppError::new(
                "Sha256Mismatch",
                Some(format!("esperado {}, obtido {}", expected, actual)),
            ),
            MissingDigest => AppError::new("MissingDigest", None),
            HashMissing => AppError::new("HashMissing", None),
            AssetNotFound => AppError::new("AssetNotFound", None),
            AlreadyInstalled(tag) => AppError::new("AlreadyInstalled", Some(tag)),
            NotInstalled(tag) => AppError::new("NotInstalled", Some(tag)),
            RateLimited => AppError::new("RateLimited", None),
            Forbidden => AppError::new("Forbidden", None),
            OfflineNoCache => AppError::new("OfflineNoCache", None),
            DownloadFailed(msg) => AppError::new("DownloadFailed", Some(msg)),
            ExtractFailed(msg) => AppError::new("ExtractFailed", Some(msg)),
            GameRunning(tag) => AppError::new("GameRunning", Some(tag)),
            VersionInUse(tag) => AppError::new("VersionInUse", Some(tag)),
        }
    }
}

impl From<crate::launch::LaunchError> for AppError {
    fn from(err: crate::launch::LaunchError) -> Self {
        use crate::launch::LaunchError::*;
        match err {
            Io(e) => AppError::new("Io", Some(e.to_string())),
            AlreadyRunning(tag) => AppError::new("AlreadyRunning", Some(tag)),
            AnotherGameRunning(tag) => AppError::new("AnotherGameRunning", Some(tag)),
            ExecutableNotFound(path_or_tag) => {
                AppError::new("ExecutableNotFound", Some(path_or_tag))
            }
            ProcessSpawnFailed(msg) => AppError::new("ProcessSpawnFailed", Some(msg)),
            LogDirCreateFailed(msg) => AppError::new("LogDirCreateFailed", Some(msg)),
            GameFolderNotFound(msg) => AppError::new("GameFolderNotFound", Some(msg)),
            UnsupportedPlatform(msg) => AppError::new("UnsupportedPlatform", Some(msg)),
            OpenerError(e) => AppError::new("OpenerError", Some(e.to_string())),
        }
    }
}
