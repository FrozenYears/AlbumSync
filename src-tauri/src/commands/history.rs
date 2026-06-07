// 历史记录命令

use tauri::State;

use crate::db::{models::SyncRunDto, queries, Database};
use crate::error::Result;

#[tauri::command]
pub async fn list_sync_runs(
    db: State<'_, Database>,
    device_id: i64,
    limit: Option<i64>,
) -> Result<Vec<SyncRunDto>> {
    let lim = limit.unwrap_or(50).clamp(1, 500);
    let rows = queries::list_sync_runs(db.pool(), device_id, lim).await?;
    Ok(rows.into_iter().map(SyncRunDto::from).collect())
}
