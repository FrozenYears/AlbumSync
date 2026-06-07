// 数据库实体与 DTO（serde Serialize 暴露给前端）

use serde::{Deserialize, Serialize};

// ---------- 数据库行（与 file_index、devices 等表对齐） ----------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub is_active: i64,
    pub backup_root: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileIndexRow {
    pub id: i64,
    pub device_id: i64,
    pub rel_path: String,
    pub size: i64,
    pub mtime_unix: i64,
    pub local_status: String,
    pub last_synced_at: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncRunRow {
    pub id: i64,
    pub device_id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: String,
    pub added: i64,
    pub updated: i64,
    pub deleted: i64,
    pub failed: i64,
    pub bytes_downloaded: i64,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TrashRow {
    pub id: i64,
    pub device_id: i64,
    pub original_rel: String,
    pub trash_rel: String,
    pub size: i64,
    pub deleted_at: i64,
    pub expire_at: i64,
}

// ---------- 前端 DTO ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDto {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub backup_root: String,
    pub is_active: bool,
}

impl From<DeviceRow> for DeviceDto {
    fn from(r: DeviceRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            host: r.host,
            port: r.port as u16,
            username: r.username,
            backup_root: r.backup_root,
            is_active: r.is_active != 0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceForm {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub backup_root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunDto {
    pub id: i64,
    pub device_id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: String,
    pub added: u32,
    pub updated: u32,
    pub deleted: u32,
    pub failed: u32,
    pub bytes_downloaded: u64,
    pub error_summary: Option<String>,
}

impl From<SyncRunRow> for SyncRunDto {
    fn from(r: SyncRunRow) -> Self {
        Self {
            id: r.id,
            device_id: r.device_id,
            started_at: r.started_at,
            ended_at: r.ended_at,
            status: r.status,
            added: r.added as u32,
            updated: r.updated as u32,
            deleted: r.deleted as u32,
            failed: r.failed as u32,
            bytes_downloaded: r.bytes_downloaded as u64,
            error_summary: r.error_summary,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItemDto {
    pub id: i64,
    pub device_id: i64,
    pub original_rel: String,
    pub size: u64,
    pub deleted_at: i64,
    pub expire_at: i64,
}

impl From<TrashRow> for TrashItemDto {
    fn from(r: TrashRow) -> Self {
        Self {
            id: r.id,
            device_id: r.device_id,
            original_rel: r.original_rel,
            size: r.size as u64,
            deleted_at: r.deleted_at,
            expire_at: r.expire_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub retention_days: u32,
    pub auto_start: bool,
    pub concurrency: u32,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsForm {
    pub retention_days: u32,
    pub auto_start: bool,
    pub concurrency: u32,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionResult {
    pub ok: bool,
    pub server_banner: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub online: bool,
    pub latency_ms: Option<u32>,
}
