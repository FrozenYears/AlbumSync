// 同步进度聚合 + Channel 推送
//
// 设计：
//   - 每完成一个字节回调累加 done_bytes
//   - 200ms 节流后通过 Channel 推 Progress
//   - FileDone / Failed 即时推送

use std::time::{Duration, Instant};

use tauri::ipc::Channel;

use crate::events::{FileAction, SyncEvent};

pub struct ProgressEmitter {
    channel: Channel<SyncEvent>,
    done_files: u64,
    done_bytes: u64,
    last_emit: Instant,
    last_bytes: u64,
    last_time: Instant,
    current_file: String,
    throttle: Duration,
}

impl ProgressEmitter {
    pub fn new(channel: Channel<SyncEvent>) -> Self {
        let now = Instant::now();
        Self {
            channel,
            done_files: 0,
            done_bytes: 0,
            last_emit: now,
            last_bytes: 0,
            last_time: now,
            current_file: String::new(),
            throttle: Duration::from_millis(200),
        }
    }

    pub fn started(&self, run_id: i64, total_files: u64, total_bytes: u64, deletes: u32) {
        let _ = self.channel.send(SyncEvent::Started {
            run_id,
            total_files,
            total_bytes,
            deletes,
        });
    }

    /// 累加 bytes 并按节流推 Progress
    pub fn add_bytes(&mut self, bytes: u64, current_file: &str) {
        self.done_bytes += bytes;
        if current_file != self.current_file {
            self.current_file = current_file.to_string();
        }
        let now = Instant::now();
        if now.duration_since(self.last_emit) >= self.throttle {
            self.flush_progress(now);
        }
    }

    pub fn file_done(&mut self, rel_path: &str, action: FileAction) {
        self.done_files += 1;
        let _ = self.channel.send(SyncEvent::FileDone {
            rel_path: rel_path.to_string(),
            action,
        });
    }

    pub fn failed(&self, rel_path: &str, reason: &str) {
        let _ = self.channel.send(SyncEvent::Failed {
            rel_path: rel_path.to_string(),
            reason: reason.to_string(),
        });
    }

    pub fn finished(&self, run_id: i64, added: u32, updated: u32, deleted: u32, failed: u32) {
        let _ = self.channel.send(SyncEvent::Finished {
            run_id,
            added,
            updated,
            deleted,
            failed,
        });
    }

    pub fn aborted(&self, run_id: i64) {
        let _ = self.channel.send(SyncEvent::Aborted { run_id });
    }

    pub fn flush_now(&mut self) {
        self.flush_progress(Instant::now());
    }

    fn flush_progress(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.last_time).as_secs_f64().max(0.001);
        let bytes_delta = self.done_bytes.saturating_sub(self.last_bytes);
        let speed = (bytes_delta as f64 / elapsed) as u64;
        let _ = self.channel.send(SyncEvent::Progress {
            done_files: self.done_files,
            done_bytes: self.done_bytes,
            current_file: self.current_file.clone(),
            speed_bps: speed,
        });
        self.last_emit = now;
        self.last_time = now;
        self.last_bytes = self.done_bytes;
    }
}
