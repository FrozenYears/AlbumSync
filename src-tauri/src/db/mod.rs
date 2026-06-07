// 数据库初始化 + 连接池
//
// 路径：<app_data_dir>/albumsync.db
// 启动期自动执行 migrations/ 下的 SQL，启用 WAL + busy_timeout。

use std::path::PathBuf;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use tauri::Manager;

use crate::error::{AlbumError, Result};

pub mod models;
pub mod queries;

/// 数据库句柄（克隆即增加 Arc）
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 在指定的应用数据目录初始化数据库
    pub async fn init(app: &tauri::AppHandle) -> Result<Self> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| AlbumError::Config(format!("无法获取 app_data_dir: {e}")))?;
        std::fs::create_dir_all(&data_dir)?;

        let db_path: PathBuf = data_dir.join("albumsync.db");
        tracing::info!(path = %db_path.display(), "opening sqlite database");

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(10))
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await?;

        // 执行迁移
        sqlx::migrate!("./migrations").run(&pool).await?;
        tracing::info!("database migrations applied");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
