// 统一错误类型与 Tauri 命令错误响应

use serde::Serialize;
use thiserror::Error;

/// 应用统一错误类型。所有 `#[tauri::command]` 返回 `Result<T, AlbumError>`，
/// 序列化后前端拿到 `{ kind, message }` 结构化错误。
#[derive(Debug, Error)]
pub enum AlbumError {
    #[error("FTP error: {0}")]
    Ftp(String),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("database migration error: {0}")]
    DbMigrate(#[from] sqlx::migrate::MigrateError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("keyring error: {0}")]
    Keyring(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("device not found: id={0}")]
    DeviceNotFound(i64),

    #[error("sync was cancelled")]
    Cancelled,

    #[error("tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Serialize)]
pub struct AlbumErrorPayload {
    pub kind: &'static str,
    pub message: String,
}

impl Serialize for AlbumError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let kind = match self {
            AlbumError::Ftp(_) => "ftp",
            AlbumError::Db(_) => "db",
            AlbumError::DbMigrate(_) => "db_migrate",
            AlbumError::Io(_) => "io",
            AlbumError::Keyring(_) => "keyring",
            AlbumError::Config(_) => "config",
            AlbumError::DeviceNotFound(_) => "device_not_found",
            AlbumError::Cancelled => "cancelled",
            AlbumError::Tauri(_) => "tauri",
            AlbumError::Other(_) => "other",
        };
        AlbumErrorPayload {
            kind,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

// keyring 4 把 Error 类型设为非公开（在 keyring_core），不便实现 From。
// 调用方直接 .map_err(|e| AlbumError::Keyring(e.to_string())) 即可。

impl From<suppaftp::FtpError> for AlbumError {
    fn from(e: suppaftp::FtpError) -> Self {
        AlbumError::Ftp(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AlbumError>;
