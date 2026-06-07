// 集中放置 SQL 查询，便于审查
//
// 风格：每个函数返回 Result<T>，参数显式传 &SqlitePool。
// 业务逻辑（如何用）在 config/sync/trash 等模块；这里只管 SQL 形状。

use sqlx::SqlitePool;

use crate::error::Result;

use super::models::{DeviceRow, FileIndexRow, SyncRunRow, TrashRow};

// ============== devices ==============

pub async fn list_active_devices(pool: &SqlitePool) -> Result<Vec<DeviceRow>> {
    let rows = sqlx::query_as::<_, DeviceRow>(
        "SELECT id, name, host, port, username, is_active, backup_root, created_at, updated_at \
         FROM devices WHERE is_active = 1 ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_device(pool: &SqlitePool, id: i64) -> Result<Option<DeviceRow>> {
    let row = sqlx::query_as::<_, DeviceRow>(
        "SELECT id, name, host, port, username, is_active, backup_root, created_at, updated_at \
         FROM devices WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_device(
    pool: &SqlitePool,
    name: &str,
    host: &str,
    port: u16,
    username: &str,
    backup_root: &str,
    now: i64,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO devices(name, host, port, username, is_active, backup_root, created_at, updated_at) \
         VALUES(?, ?, ?, ?, 1, ?, ?, ?)",
    )
    .bind(name)
    .bind(host)
    .bind(port as i64)
    .bind(username)
    .bind(backup_root)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn delete_device(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM devices WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ============== file_index ==============

pub async fn list_file_index_for_device(
    pool: &SqlitePool,
    device_id: i64,
) -> Result<Vec<FileIndexRow>> {
    let rows = sqlx::query_as::<_, FileIndexRow>(
        "SELECT id, device_id, rel_path, size, mtime_unix, local_status, last_synced_at \
         FROM file_index WHERE device_id = ?",
    )
    .bind(device_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_file_index(
    pool: &SqlitePool,
    device_id: i64,
    rel_path: &str,
    size: i64,
    mtime_unix: i64,
    local_status: &str,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO file_index(device_id, rel_path, size, mtime_unix, local_status, last_synced_at) \
         VALUES(?, ?, ?, ?, ?, ?) \
         ON CONFLICT(device_id, rel_path) DO UPDATE SET \
            size = excluded.size, \
            mtime_unix = excluded.mtime_unix, \
            local_status = excluded.local_status, \
            last_synced_at = excluded.last_synced_at",
    )
    .bind(device_id)
    .bind(rel_path)
    .bind(size)
    .bind(mtime_unix)
    .bind(local_status)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_file_index(pool: &SqlitePool, device_id: i64, rel_path: &str) -> Result<()> {
    sqlx::query("DELETE FROM file_index WHERE device_id = ? AND rel_path = ?")
        .bind(device_id)
        .bind(rel_path)
        .execute(pool)
        .await?;
    Ok(())
}

// ============== sync_runs ==============

pub async fn start_sync_run(pool: &SqlitePool, device_id: i64, now: i64) -> Result<i64> {
    let res =
        sqlx::query("INSERT INTO sync_runs(device_id, started_at, status) VALUES(?, ?, 'running')")
            .bind(device_id)
            .bind(now)
            .execute(pool)
            .await?;
    Ok(res.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub async fn finish_sync_run(
    pool: &SqlitePool,
    run_id: i64,
    status: &str,
    added: u32,
    updated: u32,
    deleted: u32,
    failed: u32,
    bytes_downloaded: u64,
    error_summary: Option<&str>,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE sync_runs SET \
            ended_at = ?, status = ?, added = ?, updated = ?, deleted = ?, \
            failed = ?, bytes_downloaded = ?, error_summary = ? \
         WHERE id = ?",
    )
    .bind(now)
    .bind(status)
    .bind(added as i64)
    .bind(updated as i64)
    .bind(deleted as i64)
    .bind(failed as i64)
    .bind(bytes_downloaded as i64)
    .bind(error_summary)
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_sync_failure(
    pool: &SqlitePool,
    run_id: i64,
    rel_path: &str,
    reason: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO sync_failures(run_id, rel_path, reason) VALUES(?, ?, ?)")
        .bind(run_id)
        .bind(rel_path)
        .bind(reason)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_sync_runs(
    pool: &SqlitePool,
    device_id: i64,
    limit: i64,
) -> Result<Vec<SyncRunRow>> {
    let rows = sqlx::query_as::<_, SyncRunRow>(
        "SELECT id, device_id, started_at, ended_at, status, added, updated, \
                deleted, failed, bytes_downloaded, error_summary \
         FROM sync_runs WHERE device_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(device_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ============== trash ==============

pub async fn insert_trash(
    pool: &SqlitePool,
    device_id: i64,
    original_rel: &str,
    trash_rel: &str,
    size: i64,
    deleted_at: i64,
    expire_at: i64,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO trash(device_id, original_rel, trash_rel, size, deleted_at, expire_at) \
         VALUES(?, ?, ?, ?, ?, ?)",
    )
    .bind(device_id)
    .bind(original_rel)
    .bind(trash_rel)
    .bind(size)
    .bind(deleted_at)
    .bind(expire_at)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn list_trash(
    pool: &SqlitePool,
    device_id: i64,
    search: Option<&str>,
) -> Result<Vec<TrashRow>> {
    let rows = if let Some(q) = search {
        let like = format!("%{q}%");
        sqlx::query_as::<_, TrashRow>(
            "SELECT id, device_id, original_rel, trash_rel, size, deleted_at, expire_at \
             FROM trash WHERE device_id = ? AND original_rel LIKE ? \
             ORDER BY deleted_at DESC",
        )
        .bind(device_id)
        .bind(like)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, TrashRow>(
            "SELECT id, device_id, original_rel, trash_rel, size, deleted_at, expire_at \
             FROM trash WHERE device_id = ? ORDER BY deleted_at DESC",
        )
        .bind(device_id)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

pub async fn get_trash_items(pool: &SqlitePool, ids: &[i64]) -> Result<Vec<TrashRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT id, device_id, original_rel, trash_rel, size, deleted_at, expire_at \
         FROM trash WHERE id IN (",
    );
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    qb.push(")");
    Ok(qb.build_query_as::<TrashRow>().fetch_all(pool).await?)
}

pub async fn delete_trash_rows(pool: &SqlitePool, ids: &[i64]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut qb = sqlx::QueryBuilder::new("DELETE FROM trash WHERE id IN (");
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    qb.push(")");
    let r = qb.build().execute(pool).await?;
    Ok(r.rows_affected())
}

pub async fn list_expired_trash(pool: &SqlitePool, now: i64) -> Result<Vec<TrashRow>> {
    let rows = sqlx::query_as::<_, TrashRow>(
        "SELECT id, device_id, original_rel, trash_rel, size, deleted_at, expire_at \
         FROM trash WHERE expire_at <= ?",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ============== app_settings ==============

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|t| t.0))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings(key, value) VALUES(?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
