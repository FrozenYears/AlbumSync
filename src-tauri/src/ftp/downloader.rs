// 单文件流式下载（含 .part 临时文件 + 断点续传）

use std::path::{Path, PathBuf};

use suppaftp::tokio::AsyncFtpStream;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{AlbumError, Result};

#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub bytes_downloaded: u64,
    pub resumed_from: u64,
}

/// 下载远端文件到本地路径。
///
/// 流程：
///   1) 确保父目录存在
///   2) 若 `<dst>.part` 存在 → 用其大小作为续传起点，REST + offset
///   3) 否则从 0 开始
///   4) 流式写入 .part
///   5) 完成后原子重命名为 dst
pub async fn download_one<F>(
    ftp: &mut AsyncFtpStream,
    remote_path: &str,
    dst: &Path,
    expected_size: u64,
    mut on_progress: F,
) -> Result<DownloadOutcome>
where
    F: FnMut(u64),
{
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).await?;
    }
    let part_path: PathBuf = dst.with_extension(format!(
        "{}.part",
        dst.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));

    let resume_from = match fs::metadata(&part_path).await {
        Ok(m) if expected_size > 0 && m.len() == expected_size => {
            if dst.exists() {
                fs::remove_file(dst).await?;
            }
            fs::rename(&part_path, dst).await?;
            return Ok(DownloadOutcome {
                bytes_downloaded: 0,
                resumed_from: m.len(),
            });
        }
        Ok(m) if expected_size > 0 && m.len() < expected_size => m.len(),
        Ok(_) => {
            let _ = fs::remove_file(&part_path).await;
            0
        }
        Err(_) => 0,
    };

    if resume_from > 0 {
        ftp.resume_transfer(resume_from as usize).await?;
    }

    let mut stream = ftp.retr_as_stream(remote_path).await?;

    let mut file = if resume_from > 0 {
        fs::OpenOptions::new().append(true).open(&part_path).await?
    } else {
        fs::File::create(&part_path).await?
    };

    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).await?;
        total += n as u64;
        on_progress(n as u64);
    }
    file.flush().await?;
    drop(file);

    ftp.finalize_retr_stream(stream).await?;

    let written = fs::metadata(&part_path).await?.len();
    if expected_size > 0 && written != expected_size {
        return Err(AlbumError::Ftp(format!(
            "下载尺寸不一致: 期望 {expected_size}，实得 {written}"
        )));
    }

    if dst.exists() {
        fs::remove_file(dst).await?;
    }
    fs::rename(&part_path, dst).await?;

    Ok(DownloadOutcome {
        bytes_downloaded: total,
        resumed_from: resume_from,
    })
}
