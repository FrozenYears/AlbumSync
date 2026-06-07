// AlbumSync 主入口装配

use std::sync::Arc;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, RunEvent, WindowEvent,
};
use tauri_plugin_single_instance::init as single_instance_init;

pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod ftp;
pub mod sync;
pub mod trash;

use db::Database;
use sync::SyncState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_filter = std::env::var("ALBUMSYNC_LOG")
        .unwrap_or_else(|_| "info,albumsync=debug".to_string());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(log_filter))
        .with_target(true)
        .try_init();

    let app = tauri::Builder::default()
        .plugin(single_instance_init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = init_async(handle).await {
                    tracing::error!(error = %e, "init_async failed");
                }
            });

            // Ctrl+C / 终端关闭 → 主动 exit(0)，避免 STATUS_CONTROL_C_EXIT (0xC000013A)
            let ctrl_c_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    tracing::info!("Ctrl+C received, shutting down cleanly");
                    ctrl_c_handle.exit(0);
                }
            });

            // 托盘
            if let Err(e) = setup_tray(app.handle()) {
                tracing::error!(error = %e, "setup_tray failed");
            }

            // 关闭按钮 → 隐藏到托盘
            if let Some(win) = app.get_webview_window("main") {
                let win2 = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let _ = win2.hide();
                        api.prevent_close();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::device::get_active_device,
            commands::device::save_device,
            commands::device::delete_device,
            commands::device::test_connection,
            commands::device::device_status,
            commands::sync::sync_start,
            commands::sync::sync_abort,
            commands::history::list_sync_runs,
            commands::trash::list_trash,
            commands::trash::restore_trash,
            commands::trash::purge_trash,
            commands::settings::get_settings,
            commands::settings::update_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building AlbumSync");

    // 即使所有窗口关闭也保持进程运行（托盘驻留）
    app.run(|_app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();
        }
    });
}

async fn init_async(app: tauri::AppHandle) -> error::Result<()> {
    let db = Database::init(&app).await?;
    let sync_state = Arc::new(SyncState::default());

    // 启动 trash GC（每 6 小时一次，启动时立即跑一次）
    let pool = db.pool().clone();
    let app2 = app.clone();
    let pool_for_factory = pool.clone();
    let lock = sync_state.sync_lock.clone();
    trash::gc::spawn_gc_task(
        app2,
        pool,
        move || backup_root_blocking(&pool_for_factory),
        lock,
    );

    app.manage(db);
    app.manage(sync_state);
    let _ = app.emit("app-ready", ());
    Ok(())
}

/// 阻塞读出活跃设备的 backup_root，用于 GC 后台任务里同步调用
fn backup_root_blocking(pool: &sqlx::SqlitePool) -> Option<std::path::PathBuf> {
    let res = tauri::async_runtime::block_on(async {
        db::queries::list_active_devices(pool).await
    });
    match res {
        Ok(mut v) => v.pop().map(|r| std::path::PathBuf::from(r.backup_root)),
        Err(_) => None,
    }
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "显示主窗口").build(app)?;
    let sync = MenuItemBuilder::with_id("sync", "立即同步").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出 AlbumSync").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &sync, &quit]).build()?;

    TrayIconBuilder::new()
        .tooltip("AlbumSync")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "sync" => {
                // 通知前端，由前端调 sync_start（前端会带 Channel）
                let _ = app.emit("tray-sync-clicked", ());
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick { .. } => {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// 健康检查命令
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}
