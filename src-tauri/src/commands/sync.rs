// 同步触发与中止命令

use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::config::DeviceConfig;
use crate::db::Database;
use crate::error::{AlbumError, Result};
use crate::events::SyncEvent;
use crate::sync::{self, SyncState};

#[tauri::command]
pub async fn sync_start(
    db: State<'_, Database>,
    state: State<'_, Arc<SyncState>>,
    device_id: i64,
    on_event: Channel<SyncEvent>,
) -> Result<()> {
    // 检查是否已有正在跑的任务
    {
        let inflight = state.inflight.lock().await;
        if let Some(r) = inflight.as_ref() {
            return Err(AlbumError::Other(format!(
                "已有同步任务进行中（run #{}）",
                r.run_id
            )));
        }
    }

    let device = match crate::db::queries::get_device(db.pool(), device_id).await? {
        Some(row) => {
            let password = crate::config::credential::load_password(
                &row.username,
                &row.host,
                row.port as u16,
            )?;
            DeviceConfig {
                id: row.id,
                name: row.name.clone(),
                host: row.host.clone(),
                port: row.port as u16,
                username: row.username.clone(),
                password,
                backup_root: std::path::PathBuf::from(row.backup_root),
            }
        }
        None => return Err(AlbumError::DeviceNotFound(device_id)),
    };

    let pool = db.pool().clone();
    let state_arc: Arc<SyncState> = state.inner().clone();
    tokio::spawn(async move {
        if let Err(e) = sync::run(pool, state_arc, device, on_event).await {
            tracing::error!(error = %e, "sync task failed");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn sync_abort(state: State<'_, Arc<SyncState>>) -> Result<()> {
    let inflight = state.inflight.lock().await;
    if let Some(r) = inflight.as_ref() {
        r.cancel.cancel();
    }
    Ok(())
}
