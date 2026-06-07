// 前后端事件契约
//
// - 高频同步进度走 Tauri Channel：在 #[tauri::command] 里以 `Channel<SyncEvent>` 参数接收
// - 低频生命周期事件走 AppHandle::emit("event-name", payload)

use serde::Serialize;

/// 同步任务通过 Channel 推送的事件（高频）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncEvent {
    /// 开始：远端清单已完成，准备开始下载
    Started {
        run_id: i64,
        total_files: u64,
        total_bytes: u64,
        deletes: u32,
    },
    /// 进度（节流后聚合，约 200ms 一次）
    Progress {
        done_files: u64,
        done_bytes: u64,
        current_file: String,
        speed_bps: u64,
    },
    /// 单个文件完成（用于实时列表更新）
    FileDone {
        rel_path: String,
        action: FileAction,
    },
    /// 单个文件失败（不中断整体）
    Failed {
        rel_path: String,
        reason: String,
    },
    /// 全部完成
    Finished {
        run_id: i64,
        added: u32,
        updated: u32,
        deleted: u32,
        failed: u32,
    },
    /// 被用户中止
    Aborted { run_id: i64 },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileAction {
    Added,
    Updated,
    Deleted,
    Skipped,
}

// ---------- 低频广播事件名常量 ----------

pub const EVT_DEVICE_STATUS_CHANGED: &str = "device-status-changed";
pub const EVT_TRASH_GC_FINISHED: &str = "trash-gc-finished";
pub const EVT_APP_ERROR: &str = "app-error";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatusChangedPayload {
    pub device_id: i64,
    pub online: bool,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashGcFinishedPayload {
    pub purged: u32,
    pub freed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorPayload {
    pub r#where: String,
    pub message: String,
}
