-- AlbumSync 初始 schema（v0.1）
-- 通过 sqlx::migrate!("./migrations") 在应用启动时自动执行

PRAGMA foreign_keys = ON;

-- ========== 1) 设备配置 ==========
CREATE TABLE devices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    host            TEXT    NOT NULL,
    port            INTEGER NOT NULL DEFAULT 1024,
    username        TEXT    NOT NULL,
    -- 密码不在表里，存 Windows Credential Manager
    is_active       INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    backup_root     TEXT    NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- ========== 2) 本地文件清单（每条 = 备份根下一个文件）==========
CREATE TABLE file_index (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id       INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    rel_path        TEXT    NOT NULL,
    size            INTEGER NOT NULL,
    mtime_unix      INTEGER NOT NULL,
    local_status    TEXT    NOT NULL CHECK (local_status IN ('present', 'partial', 'missing')),
    last_synced_at  INTEGER NOT NULL,
    UNIQUE (device_id, rel_path)
);
CREATE INDEX idx_file_device ON file_index(device_id);

-- ========== 3) 同步运行记录 ==========
CREATE TABLE sync_runs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id        INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    started_at       INTEGER NOT NULL,
    ended_at         INTEGER,
    status           TEXT    NOT NULL CHECK (status IN ('running', 'succeeded', 'partial', 'failed', 'aborted')),
    added            INTEGER NOT NULL DEFAULT 0,
    updated          INTEGER NOT NULL DEFAULT 0,
    deleted          INTEGER NOT NULL DEFAULT 0,
    failed           INTEGER NOT NULL DEFAULT 0,
    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
    error_summary    TEXT
);
CREATE INDEX idx_runs_device ON sync_runs(device_id, started_at DESC);

-- ========== 4) 单次运行的失败明细 ==========
CREATE TABLE sync_failures (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id    INTEGER NOT NULL REFERENCES sync_runs(id) ON DELETE CASCADE,
    rel_path  TEXT    NOT NULL,
    reason    TEXT    NOT NULL
);
CREATE INDEX idx_failures_run ON sync_failures(run_id);

-- ========== 5) 软删除回收站 ==========
CREATE TABLE trash (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id    INTEGER NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    original_rel TEXT    NOT NULL,
    trash_rel    TEXT    NOT NULL,
    size         INTEGER NOT NULL,
    deleted_at   INTEGER NOT NULL,
    expire_at    INTEGER NOT NULL
);
CREATE INDEX idx_trash_expire ON trash(expire_at);
CREATE INDEX idx_trash_device ON trash(device_id);

-- ========== 6) 应用配置（KV）==========
CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 默认值
INSERT INTO app_settings(key, value) VALUES
    ('retention_days', '30'),
    ('auto_start',     'false'),
    ('concurrency',    '4'),
    ('include_globs',  '["DCIM/**/*","Pictures/**/*","Tencent/MicroMsg/WeiXin/**/*","Tencent/QQ_Images/**/*","Movies/**/*"]'),
    ('exclude_globs',  '["**/.thumbnails/**","**/cache/**","**/*.tmp"]');
