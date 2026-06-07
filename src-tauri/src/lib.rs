// 应用入口：仅装配 Tauri Builder，业务逻辑写在后续模块中

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
