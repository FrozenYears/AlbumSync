// 软删除回收站
//
// 删除时：把文件从 backup_root 移动到 backup_root/.trash/<时间戳>__<原相对路径>
// 恢复时：把文件从 .trash 复原到原位置 + 重新插入 file_index
//
// 路径转义：相对路径里的 / \ 等保留，因为我们用一个时间戳前缀避免名字冲突，
// 而不是 flatten 路径。

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use tokio::fs;

use crate::db::{models::*, queries};
use crate::error::{AlbumError, Result};

pub mod gc;

const TRASH_DIRNAME: &str = ".trash";
const SECS_PER_DAY: i64 = 86_400;

/// 把一个文件移到 trash 目录（异步）。失败时返回错误但不删除原文件。
///
/// 返回 trash_rel（相对 backup_root）+ size
pub async fn move_to_trash(
    pool: &SqlitePool,
    device_id: i64,
    backup_root: &Path,
    original_rel: &str,
    now: i64,
    retention_days: u32,
) -> Result<TrashRow> {
    let original = backup_root.join(original_rel);
    let size = fs::metadata(&original).await?.len() as i64;

    // .trash/<timestamp>/<original_rel>
    let ts_dir = format_unix(now);
    let trash_rel_pb: PathBuf = PathBuf::from(TRASH_DIRNAME).join(&ts_dir).join(original_rel);
    let trash_abs = backup_root.join(&trash_rel_pb);

    if let Some(parent) = trash_abs.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::rename(&original, &trash_abs).await?;

    let expire_at = now + SECS_PER_DAY * retention_days as i64;
    let trash_rel = trash_rel_pb.to_string_lossy().replace('\\', "/");
    let id = queries::insert_trash(pool, device_id, original_rel, &trash_rel, size, now, expire_at).await?;

    Ok(TrashRow {
        id,
        device_id,
        original_rel: original_rel.to_string(),
        trash_rel,
        size,
        deleted_at: now,
        expire_at,
    })
}

/// 恢复 trash 项到原位置
pub async fn restore_one(
    pool: &SqlitePool,
    backup_root: &Path,
    row: &TrashRow,
    now: i64,
) -> Result<()> {
    let from = backup_root.join(&row.trash_rel);
    let to = backup_root.join(&row.original_rel);
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).await?;
    }
    if to.exists() {
        // 原位置有同名文件（用户已重新下载）→ 跳过恢复，仅删 trash 项
        let _ = fs::remove_file(&from).await;
    } else {
        fs::rename(&from, &to).await?;
    }

    // 重新插入 file_index（视作"刚同步过"）
    queries::upsert_file_index(
        pool,
        row.device_id,
        &row.original_rel,
        row.size,
        now, // 用 now 作为 mtime；下次同步会校正
        "present",
        now,
    )
    .await?;

    queries::delete_trash_rows(pool, &[row.id]).await?;
    Ok(())
}

/// 立刻物理删除
pub async fn purge_one(backup_root: &Path, trash_rel: &str) -> Result<()> {
    let abs = backup_root.join(trash_rel);
    match fs::remove_file(&abs).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AlbumError::Io(e)),
    }
}

fn format_unix(now: i64) -> String {
    // 不依赖 chrono：手动生成 YYYYMMDD-HHMMSS（UTC）
    fn is_leap(y: i64) -> bool { (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 }
    let secs_in_day = now.rem_euclid(SECS_PER_DAY);
    let mut days = now.div_euclid(SECS_PER_DAY);
    let mut y: i64 = 1970;
    loop {
        let yd = if is_leap(y) { 366 } else { 365 };
        if days < yd { break; }
        days -= yd;
        y += 1;
    }
    let dom = [31i64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m: i64 = 1;
    for mm in 1..=12 {
        let dim = if mm == 2 && is_leap(y) { 29 } else { dom[(mm - 1) as usize] };
        if days < dim { m = mm; break; }
        days -= dim;
    }
    let d = days + 1;
    let h = secs_in_day / 3600;
    let mi = (secs_in_day % 3600) / 60;
    let s = secs_in_day % 60;
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", y, m, d, h, mi, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_unix_epoch() {
        assert_eq!(format_unix(0), "19700101-000000");
    }

    #[test]
    fn format_unix_known() {
        // 2024-01-01 00:00:00 UTC = 1_704_067_200
        assert_eq!(format_unix(1_704_067_200), "20240101-000000");
    }
}
