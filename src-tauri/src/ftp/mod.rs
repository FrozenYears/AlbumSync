// FTP 客户端封装（基于 suppaftp 8 + tokio）

use std::time::Duration;

use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;

use crate::error::{AlbumError, Result};

pub mod downloader;
pub mod walker;

/// 创建一条已登录的 FTP 控制连接（二进制模式）
pub async fn connect_login(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    timeout: Duration,
) -> Result<AsyncFtpStream> {
    let addr = format!("{host}:{port}");
    let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&addr))
        .await
        .map_err(|_| AlbumError::Ftp(format!("连接 {addr} 超时")))??;
    let mut ftp = ftp;
    ftp.login(username, password).await?;
    ftp.transfer_type(FileType::Binary).await?;
    Ok(ftp)
}

/// 简易拨测：连接 + 登录 + 取服务器 banner
pub async fn test_connection(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> Result<String> {
    let mut ftp = connect_login(host, port, username, password, Duration::from_secs(8)).await?;
    let banner = ftp
        .get_welcome_msg()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(no banner)".into());
    let _ = ftp.quit().await;
    Ok(banner)
}
