// 同步引擎：编排 walker + diff + downloader + trash + progress + db

pub mod diff;
pub mod progress;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::ipc::Channel;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::config::DeviceConfig;
use crate::db::queries;
use crate::error::{AlbumError, Result};
use crate::events::{FileAction, SyncEvent};
use crate::ftp::walker::WalkConfig;
use crate::ftp::{connect_login, downloader, walker};
use crate::trash;

use self::diff::{compute_diff, DiffItem, FileEntry};
use self::progress::ProgressEmitter;

/// 同步引擎当前状态（一次仅一个 task 运行）
#[derive(Default)]
pub struct SyncState {
    pub inflight: Mutex<Option<RunningRun>>,
    /// 与 trash GC 共享：同步进行时 GC 跳过
    pub sync_lock: Arc<Mutex<()>>,
}

pub struct RunningRun {
    pub run_id: i64,
    pub device_id: i64,
    pub cancel: CancellationToken,
}

/// FTP 上扫描的基础目录（Primitive FTPd 默认以 /sdcard 为 root）
const REMOTE_BASE: &str = "/";

pub async fn run(
    pool: SqlitePool,
    state: Arc<SyncState>,
    device: DeviceConfig,
    channel: Channel<SyncEvent>,
) -> Result<()> {
    // 互斥：只允许一个同步任务运行
    let sync_guard = state.sync_lock.clone().lock_owned().await;

    let now = unix_now();
    let run_id = queries::start_sync_run(&pool, device.id, now).await?;

    let cancel = CancellationToken::new();
    {
        let mut inflight = state.inflight.lock().await;
        *inflight = Some(RunningRun {
            run_id,
            device_id: device.id,
            cancel: cancel.clone(),
        });
    }

    // 把同步业务包成 inner，让我们能在 finally 关闭 inflight
    let outcome = run_inner(&pool, &device, run_id, cancel.clone(), channel).await;

    // 复位 inflight
    {
        let mut inflight = state.inflight.lock().await;
        *inflight = None;
    }
    drop(sync_guard);

    outcome
}

async fn run_inner(
    pool: &SqlitePool,
    device: &DeviceConfig,
    run_id: i64,
    cancel: CancellationToken,
    channel: Channel<SyncEvent>,
) -> Result<()> {
    let mut emitter = ProgressEmitter::new(channel);

    // 读 settings 拿 include/exclude/concurrency
    let settings = crate::config::get_settings(pool).await?;
    let walk_cfg = WalkConfig::from_settings(&settings.include_globs, &settings.exclude_globs)?;
    let concurrency = settings.concurrency.clamp(1, 16) as usize;
    let retention_days = settings.retention_days;

    tracing::info!(run_id, device = %device.name, "sync starting");

    // 1) 远端扫描
    let mut ftp = connect_login(
        &device.host,
        device.port,
        &device.username,
        &device.password,
        Duration::from_secs(15),
    )
    .await?;
    let remote = walker::walk(&mut ftp, REMOTE_BASE, &walk_cfg).await?;
    let _ = ftp.quit().await;
    drop(ftp);

    if cancel.is_cancelled() {
        finish_aborted(pool, run_id, &emitter).await?;
        return Err(AlbumError::Cancelled);
    }

    // 2) 本地清单
    let local_rows = queries::list_file_index_for_device(pool, device.id).await?;
    let local: Vec<FileEntry> = local_rows
        .into_iter()
        .map(|r| FileEntry {
            rel_path: r.rel_path,
            size: r.size as u64,
            mtime: r.mtime_unix,
        })
        .collect();

    // 3) diff
    let diff = compute_diff(&local, &remote);
    let total_files: u64 = diff
        .iter()
        .filter(|d| matches!(d, DiffItem::Add(_) | DiffItem::Update(_)))
        .count() as u64;
    let total_bytes: u64 = diff
        .iter()
        .filter_map(|d| match d {
            DiffItem::Add(e) | DiffItem::Update(e) => Some(e.size),
            _ => None,
        })
        .sum();
    let deletes: u32 = diff
        .iter()
        .filter(|d| matches!(d, DiffItem::DeleteLocal { .. }))
        .count() as u32;

    emitter.started(run_id, total_files, total_bytes, deletes);
    tracing::info!(total_files, total_bytes, deletes, "diff computed");

    // 4) 处理 delete（先做，便于释放本地空间）
    let mut deleted = 0u32;
    let mut failed = 0u32;
    let mut bytes_downloaded: u64 = 0;

    for item in &diff {
        if let DiffItem::DeleteLocal { rel_path, .. } = item {
            if cancel.is_cancelled() { break; }
            let now = unix_now();
            match trash::move_to_trash(
                pool,
                device.id,
                &device.backup_root,
                rel_path,
                now,
                retention_days,
            )
            .await
            {
                Ok(_) => {
                    let _ = queries::delete_file_index(pool, device.id, rel_path).await;
                    emitter.file_done(rel_path, FileAction::Deleted);
                    deleted += 1;
                }
                Err(e) => {
                    tracing::warn!(rel = rel_path, err = %e, "move_to_trash failed");
                    emitter.failed(rel_path, &e.to_string());
                    let _ = queries::record_sync_failure(pool, run_id, rel_path, &e.to_string()).await;
                    failed += 1;
                }
            }
        }
    }

    // 5) 并发下载
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut tasks = Vec::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<DownloadMsg>(256);

    let mut added = 0u32;
    let mut updated = 0u32;

    // 为简单起见：每个并发任务自带一条 FTP 连接（生命周期同任务）。
    // 这样并发上限 = 同时活跃的 FTP 连接数，符合直觉。
    for item in &diff {
        let entry = match item {
            DiffItem::Add(e) => e.clone(),
            DiffItem::Update(e) => e.clone(),
            DiffItem::DeleteLocal { .. } => continue,
        };
        let is_update = matches!(item, DiffItem::Update(_));

        let sem = semaphore.clone();
        let host = device.host.clone();
        let port = device.port;
        let user = device.username.clone();
        let pass = device.password.clone();
        let backup_root = device.backup_root.clone();
        let cancel = cancel.clone();
        let tx = tx.clone();

        let handle = tokio::spawn(async move {
            let _permit = match sem.acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };
            if cancel.is_cancelled() { return; }
            let res = download_one_file(
                &host, port, &user, &pass, &backup_root, &entry, &tx, &cancel,
            )
            .await;
            let msg = match res {
                Ok(_bytes) => DownloadMsg::Done {
                    rel_path: entry.rel_path.clone(),
                    size: entry.size,
                    mtime: entry.mtime,
                    is_update,
                },
                Err(crate::error::AlbumError::Cancelled) => return,
                Err(e) => DownloadMsg::Failed {
                    rel_path: entry.rel_path.clone(),
                    reason: e.to_string(),
                },
            };
            let _ = tx.send(msg).await;
        });
        tasks.push(handle);
    }
    drop(tx);

    while let Some(msg) = rx.recv().await {
        match msg {
            DownloadMsg::Progress { current_file, bytes } => {
                bytes_downloaded += bytes;
                emitter.add_bytes(bytes, &current_file);
            }
            DownloadMsg::Done { rel_path, size, mtime, is_update } => {
                let now = unix_now();
                let _ = queries::upsert_file_index(
                    pool,
                    device.id,
                    &rel_path,
                    size as i64,
                    mtime,
                    "present",
                    now,
                )
                .await;
                if is_update {
                    updated += 1;
                    emitter.file_done(&rel_path, FileAction::Updated);
                } else {
                    added += 1;
                    emitter.file_done(&rel_path, FileAction::Added);
                }
            }
            DownloadMsg::Failed { rel_path, reason } => {
                emitter.failed(&rel_path, &reason);
                let _ = queries::record_sync_failure(pool, run_id, &rel_path, &reason).await;
                failed += 1;
            }
        }
    }
    for t in tasks { let _ = t.await; }
    emitter.flush_now();

    let now = unix_now();
    let status = if cancel.is_cancelled() {
        "aborted"
    } else if failed == 0 {
        "succeeded"
    } else {
        "partial"
    };
    queries::finish_sync_run(
        pool, run_id, status, added, updated, deleted, failed, bytes_downloaded, None, now,
    )
    .await?;

    if cancel.is_cancelled() {
        emitter.aborted(run_id);
        return Err(AlbumError::Cancelled);
    }
    emitter.finished(run_id, added, updated, deleted, failed);
    Ok(())
}

async fn finish_aborted(pool: &SqlitePool, run_id: i64, emitter: &ProgressEmitter) -> Result<()> {
    let now = unix_now();
    queries::finish_sync_run(pool, run_id, "aborted", 0, 0, 0, 0, 0, None, now).await?;
    emitter.aborted(run_id);
    Ok(())
}

enum DownloadMsg {
    Progress { current_file: String, bytes: u64 },
    Done {
        rel_path: String,
        size: u64,
        mtime: i64,
        is_update: bool,
    },
    Failed { rel_path: String, reason: String },
}

#[allow(clippy::too_many_arguments)]
async fn download_one_file(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    backup_root: &Path,
    entry: &FileEntry,
    tx: &tokio::sync::mpsc::Sender<DownloadMsg>,
    cancel: &CancellationToken,
) -> Result<u64> {
    if cancel.is_cancelled() { return Err(AlbumError::Cancelled); }

    // 连接也用 race：取消优先
    let connect_fut = connect_login(host, port, user, pass, Duration::from_secs(15));
    tokio::pin!(connect_fut);
    let mut ftp = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AlbumError::Cancelled),
        r = &mut connect_fut => r?,
    };

    let remote_path = format!("{REMOTE_BASE}{}{}", trailing_slash(REMOTE_BASE), entry.rel_path);
    let local_path = backup_root.join(&entry.rel_path);

    let outcome = {
        let rel = entry.rel_path.clone();
        let tx2 = tx.clone();
        let cancel2 = cancel.clone();
        let download_fut = downloader::download_one(
            &mut ftp,
            &remote_path,
            &local_path,
            entry.size,
            move |n| {
                if cancel2.is_cancelled() { return; }
                let _ = tx2.try_send(DownloadMsg::Progress {
                    current_file: rel.clone(),
                    bytes: n,
                });
            },
        );
        tokio::pin!(download_fut);
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AlbumError::Cancelled),
            r = &mut download_fut => r?,
        }
    };
    let _ = ftp.quit().await;
    Ok(outcome.bytes_downloaded)
}

fn trailing_slash(s: &str) -> &str {
    if s.ends_with('/') { "" } else { "/" }
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
