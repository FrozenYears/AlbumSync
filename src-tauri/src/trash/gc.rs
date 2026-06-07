// 后台 GC：定期清理过期的 trash 项

use std::path::PathBuf;
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::db::queries;
use crate::error::Result;
use crate::events::{TrashGcFinishedPayload, EVT_TRASH_GC_FINISHED};

/// 跑一次 GC：删 expire_at <= now 的所有 trash 项（文件 + 数据库行）
pub async fn run_once(
    pool: &SqlitePool,
    backup_root: &std::path::Path,
    now: i64,
) -> Result<TrashGcFinishedPayload> {
    let expired = queries::list_expired_trash(pool, now).await?;
    let mut purged = 0u32;
    let mut freed: u64 = 0;
    for row in &expired {
        let abs = backup_root.join(&row.trash_rel);
        match tokio::fs::remove_file(&abs).await {
            Ok(()) | Err(_) => {} // 文件可能已不存在，照常清数据库行
        }
        freed += row.size as u64;
        purged += 1;
    }
    if !expired.is_empty() {
        let ids: Vec<i64> = expired.iter().map(|r| r.id).collect();
        queries::delete_trash_rows(pool, &ids).await?;
    }
    Ok(TrashGcFinishedPayload {
        purged,
        freed_bytes: freed,
    })
}

/// 启动后台 GC 循环：启动时立即跑一次，之后每 6 小时一次。
///
/// `sync_lock`：与同步引擎共享的"是否正在同步"锁。GC 在同步进行时跳过本轮，避免与
/// trash 写入竞态（不致死锁但语义更干净）。
pub fn spawn_gc_task(
    app: AppHandle,
    pool: SqlitePool,
    backup_root_factory: impl Fn() -> Option<PathBuf> + Send + 'static,
    sync_lock: std::sync::Arc<Mutex<()>>,
) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(6 * 3600);
        loop {
            // 等同步空闲
            let _g = sync_lock.lock().await;
            let now = unix_now();
            if let Some(root) = backup_root_factory() {
                match run_once(&pool, &root, now).await {
                    Ok(p) => {
                        tracing::info!(purged = p.purged, freed = p.freed_bytes, "trash gc done");
                        let _ = app.emit(EVT_TRASH_GC_FINISHED, p);
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "trash gc failed");
                    }
                }
            }
            drop(_g);
            tokio::time::sleep(interval).await;
        }
    });
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
