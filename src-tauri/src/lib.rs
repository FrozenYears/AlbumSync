// AlbumSync 主入口装配

pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod ftp;

pub mod sync {
    pub mod diff;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running AlbumSync");
}

/// 健康检查命令：脚手架阶段用来验证前后端 IPC 通路
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}
