// 回收站命令

use tauri::State;

use crate::db::{models::TrashItemDto, queries, Database};
use crate::error::{AlbumError, Result};
use crate::trash;

#[tauri::command]
pub async fn list_trash(
    db: State<'_, Database>,
    device_id: i64,
    search: Option<String>,
) -> Result<Vec<TrashItemDto>> {
    let rows = queries::list_trash(db.pool(), device_id, search.as_deref()).await?;
    Ok(rows.into_iter().map(TrashItemDto::from).collect())
}

#[tauri::command]
pub async fn restore_trash(
    db: State<'_, Database>,
    ids: Vec<i64>,
) -> Result<()> {
    let rows = queries::get_trash_items(db.pool(), &ids).await?;
    let now = unix_now();
    for row in rows {
        let device_row = queries::get_device(db.pool(), row.device_id)
            .await?
            .ok_or(AlbumError::DeviceNotFound(row.device_id))?;
        let backup_root = std::path::PathBuf::from(device_row.backup_root);
        trash::restore_one(db.pool(), &backup_root, &row, now).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn purge_trash(
    db: State<'_, Database>,
    ids: Vec<i64>,
) -> Result<()> {
    let rows = queries::get_trash_items(db.pool(), &ids).await?;
    for row in &rows {
        let device_row = queries::get_device(db.pool(), row.device_id)
            .await?
            .ok_or(AlbumError::DeviceNotFound(row.device_id))?;
        let backup_root = std::path::PathBuf::from(device_row.backup_root);
        trash::purge_one(&backup_root, &row.trash_rel).await?;
    }
    let row_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    queries::delete_trash_rows(db.pool(), &row_ids).await?;
    Ok(())
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
