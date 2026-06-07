// 设备管理命令

use tauri::State;

use crate::config;
use crate::db::models::*;
use crate::db::Database;
use crate::error::{AlbumError, Result};
use crate::ftp;

#[tauri::command]
pub async fn get_active_device(db: State<'_, Database>) -> Result<Option<DeviceDto>> {
    config::get_active_device(db.pool()).await
}

#[tauri::command]
pub async fn save_device(
    db: State<'_, Database>,
    form: DeviceForm,
) -> Result<DeviceDto> {
    if form.name.is_empty() || form.host.is_empty() || form.username.is_empty() {
        return Err(AlbumError::Config("名称/主机/用户名不能为空".into()));
    }
    if form.backup_root.is_empty() {
        return Err(AlbumError::Config("备份目录不能为空".into()));
    }
    // 确保备份目录可创建
    std::fs::create_dir_all(&form.backup_root)?;
    let now = unix_now();
    config::save_device(db.pool(), &form, now).await
}

#[tauri::command]
pub async fn delete_device(db: State<'_, Database>, id: i64) -> Result<()> {
    config::delete_device(db.pool(), id).await
}

#[tauri::command]
pub async fn test_connection(form: DeviceForm) -> Result<ConnectionResult> {
    match ftp::test_connection(&form.host, form.port, &form.username, &form.password).await {
        Ok(banner) => Ok(ConnectionResult { ok: true, server_banner: Some(banner), error: None }),
        Err(e) => Ok(ConnectionResult { ok: false, server_banner: None, error: Some(e.to_string()) }),
    }
}

#[tauri::command]
pub async fn device_status(
    db: State<'_, Database>,
    id: i64,
) -> Result<DeviceStatus> {
    use std::time::{Duration, Instant};
    let row = crate::db::queries::get_device(db.pool(), id)
        .await?
        .ok_or(AlbumError::DeviceNotFound(id))?;
    let addr = format!("{}:{}", row.host, row.port);
    let start = Instant::now();
    let online = tokio::time::timeout(
        Duration::from_millis(800),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .is_some();
    let latency_ms = if online {
        Some(start.elapsed().as_millis() as u32)
    } else {
        None
    };
    Ok(DeviceStatus { online, latency_ms })
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
