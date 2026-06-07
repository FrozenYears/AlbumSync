// 应用配置 + 设备信息组合层
//
// 负责：
//   - 从 app_settings 表读写 SettingsDto
//   - 从 devices 表 + keyring 组合出完整的 DeviceConfig
//   - 提供首次启动判定（is_onboarded）

use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::db::{models::*, queries};
use crate::error::{AlbumError, Result};

pub mod credential;

/// 完整设备配置（含明文密码，仅在内存中存在）
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub backup_root: PathBuf,
}

impl DeviceConfig {
    pub async fn load_active(pool: &SqlitePool) -> Result<Option<Self>> {
        let mut devices = queries::list_active_devices(pool).await?;
        let Some(row) = devices.pop() else { return Ok(None) };
        let password = credential::load_password(&row.username, &row.host, row.port as u16)?;
        Ok(Some(Self {
            id: row.id,
            name: row.name,
            host: row.host,
            port: row.port as u16,
            username: row.username,
            password,
            backup_root: PathBuf::from(row.backup_root),
        }))
    }
}

/// 取出活跃设备（不含密码）
pub async fn get_active_device(pool: &SqlitePool) -> Result<Option<DeviceDto>> {
    let mut devices = queries::list_active_devices(pool).await?;
    Ok(devices.pop().map(DeviceDto::from))
}

/// 创建/更新设备（v0.1 设计：只支持 1 台，旧的会先 deactivate）
pub async fn save_device(
    pool: &SqlitePool,
    form: &DeviceForm,
    now: i64,
) -> Result<DeviceDto> {
    let mut tx = pool.begin().await?;

    // 标记旧的为 inactive
    sqlx::query("UPDATE devices SET is_active = 0 WHERE is_active = 1")
        .execute(&mut *tx)
        .await?;

    // 插入新的
    let id = sqlx::query(
        "INSERT INTO devices(name, host, port, username, is_active, backup_root, created_at, updated_at) \
         VALUES(?, ?, ?, ?, 1, ?, ?, ?)",
    )
    .bind(&form.name)
    .bind(&form.host)
    .bind(form.port as i64)
    .bind(&form.username)
    .bind(&form.backup_root)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    tx.commit().await?;

    // 写凭据
    credential::save_password(&form.username, &form.host, form.port, &form.password)?;

    let row = queries::get_device(pool, id)
        .await?
        .ok_or_else(|| AlbumError::Other("inserted device disappeared".into()))?;
    Ok(DeviceDto::from(row))
}

pub async fn delete_device(pool: &SqlitePool, id: i64) -> Result<()> {
    let Some(device) = queries::get_device(pool, id).await? else {
        return Err(AlbumError::DeviceNotFound(id));
    };
    queries::delete_device(pool, id).await?;
    let _ = credential::delete_password(&device.username, &device.host, device.port as u16);
    Ok(())
}

// ============== Settings ==============

const K_RETENTION: &str = "retention_days";
const K_AUTOSTART: &str = "auto_start";
const K_CONCURRENCY: &str = "concurrency";
const K_INCLUDE: &str = "include_globs";
const K_EXCLUDE: &str = "exclude_globs";

pub async fn get_settings(pool: &SqlitePool) -> Result<SettingsDto> {
    let retention: u32 = parse_setting(pool, K_RETENTION, 30).await?;
    let auto_start: bool = parse_setting(pool, K_AUTOSTART, false).await?;
    let concurrency: u32 = parse_setting(pool, K_CONCURRENCY, 4).await?;
    let include: Vec<String> = parse_setting_json(pool, K_INCLUDE, default_includes()).await?;
    let exclude: Vec<String> = parse_setting_json(pool, K_EXCLUDE, default_excludes()).await?;
    Ok(SettingsDto {
        retention_days: retention,
        auto_start,
        concurrency,
        include_globs: include,
        exclude_globs: exclude,
    })
}

pub async fn update_settings(pool: &SqlitePool, form: &SettingsForm) -> Result<()> {
    let mut tx = pool.begin().await?;
    write_setting(&mut tx, K_RETENTION, &form.retention_days.to_string()).await?;
    write_setting(&mut tx, K_AUTOSTART, if form.auto_start { "true" } else { "false" }).await?;
    write_setting(&mut tx, K_CONCURRENCY, &form.concurrency.to_string()).await?;
    let include_json = serde_json::to_string(&form.include_globs)
        .map_err(|e| AlbumError::Config(e.to_string()))?;
    let exclude_json = serde_json::to_string(&form.exclude_globs)
        .map_err(|e| AlbumError::Config(e.to_string()))?;
    write_setting(&mut tx, K_INCLUDE, &include_json).await?;
    write_setting(&mut tx, K_EXCLUDE, &exclude_json).await?;
    tx.commit().await?;
    Ok(())
}

async fn parse_setting<T: std::str::FromStr>(
    pool: &SqlitePool,
    key: &str,
    default: T,
) -> Result<T> {
    let Some(s) = queries::get_setting(pool, key).await? else { return Ok(default) };
    Ok(s.parse().unwrap_or(default))
}

async fn parse_setting_json<T: serde::de::DeserializeOwned>(
    pool: &SqlitePool,
    key: &str,
    default: T,
) -> Result<T> {
    let Some(s) = queries::get_setting(pool, key).await? else { return Ok(default) };
    serde_json::from_str(&s).map_err(|e| AlbumError::Config(e.to_string()))
}

async fn write_setting(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings(key, value) VALUES(?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn default_includes() -> Vec<String> {
    vec![
        "DCIM/**/*".into(),
        "Pictures/**/*".into(),
        "Tencent/MicroMsg/WeiXin/**/*".into(),
        "Tencent/QQ_Images/**/*".into(),
        "Movies/**/*".into(),
    ]
}

fn default_excludes() -> Vec<String> {
    vec![
        "**/.thumbnails/**".into(),
        "**/cache/**".into(),
        "**/*.tmp".into(),
    ]
}
