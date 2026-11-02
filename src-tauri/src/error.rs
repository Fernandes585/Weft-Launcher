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
        AppError::new("io", Some(err.to_string()))
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
            Network(e) => AppError::new("network", Some(e.to_string())),
            Io(e) => AppError::new("io", Some(e.to_string())),
            Serde(e) => AppError::new("serialization", Some(e.to_string())),
            InvalidHeader(e) => AppError::new("httpHeader", Some(e.to_string())),
            CacheCorrupted(msg) => AppError::new("cacheCorrupted", Some(msg)),
            AssetNotFound => AppError::new("assetNotFound", None),
            MissingDigest => AppError::new("missingDigest", None),
            RateLimited => AppError::new("rateLimited", None),
            Forbidden => AppError::new("forbidden", None),
            OfflineNoCache => AppError::new("offlineNoCache", None),
            NoCompatibleVersions => AppError::new("noCompatibleVersions", None),
        }
    }
}

impl From<crate::install::InstallError> for AppError {
    fn from(err: crate::install::InstallError) -> Self {
        use crate::install::InstallError::*;
        match err {
            Network(e) => AppError::new("network", Some(e.to_string())),
            Io(e) => AppError::new("io", Some(e.to_string())),
            Serde(e) => AppError::new("serialization", Some(e.to_string())),
            Zip(e) => AppError::new("zip", Some(e.to_string())),
            CreateDir(msg) => AppError::new("createDir", Some(msg)),
            Sha256Mismatch { expected, actual } => AppError::new(
                "sha256Mismatch",
                Some(format!("esperado {}, obtido {}", expected, actual)),
            ),
            MissingDigest => AppError::new("missingDigest", None),
            HashMissing => AppError::new("hashMissing", None),
            AssetNotFound => AppError::new("assetNotFound", None),
            AlreadyInstalled(tag) => AppError::new("alreadyInstalled", Some(tag)),
            NotInstalled(tag) => AppError::new("notInstalled", Some(tag)),
            RateLimited => AppError::new("rateLimited", None),
            Forbidden => AppError::new("forbidden", None),
            OfflineNoCache => AppError::new("offlineNoCache", None),
            DownloadFailed(msg) => AppError::new("downloadFailed", Some(msg)),
            ExtractFailed(msg) => AppError::new("extractFailed", Some(msg)),
            GameRunning(tag) => AppError::new("gameRunning", Some(tag)),
            VersionInUse(tag) => AppError::new("versionInUse", Some(tag)),
        }
    }
}

impl From<crate::launch::LaunchError> for AppError {
    fn from(err: crate::launch::LaunchError) -> Self {
        use crate::launch::LaunchError::*;
        match err {
            Io(e) => AppError::new("io", Some(e.to_string())),
            AlreadyRunning(tag) => AppError::new("alreadyRunning", Some(tag)),
            AnotherGameRunning(tag) => AppError::new("anotherGameRunning", Some(tag)),
            ExecutableNotFound(path_or_tag) => {
                AppError::new("executableNotFound", Some(path_or_tag))
            }
            ProcessSpawnFailed(msg) => AppError::new("processSpawnFailed", Some(msg)),
            LogDirCreateFailed(msg) => AppError::new("logDirCreateFailed", Some(msg)),
            GameFolderNotFound(msg) => AppError::new("gameFolderNotFound", Some(msg)),
            OpenerError(e) => AppError::new("openerError", Some(e.to_string())),
        }
    }
}
