// MLSD 递归扫描器
//
// Primitive FTPd 支持 MLSD（RFC 3659），返回机器可读的 size + mtime。
// 这里递归遍历指定目录，按 include/exclude glob + 扩展名过滤，产出 FileEntry。

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use glob::Pattern;
use suppaftp::list::{File as ListFile, ListParser};
use suppaftp::tokio::AsyncFtpStream;

use crate::error::{AlbumError, Result};
use crate::sync::diff::FileEntry;

#[derive(Debug, Clone)]
pub struct WalkConfig {
    pub includes: Vec<Pattern>,
    pub excludes: Vec<Pattern>,
    pub extensions: HashSet<String>,
    pub max_depth: u32,
}

impl WalkConfig {
    pub fn from_settings(includes: &[String], excludes: &[String]) -> Result<Self> {
        let parse = |v: &[String]| -> Result<Vec<Pattern>> {
            v.iter()
                .map(|s| {
                    Pattern::new(s)
                        .map_err(|e| AlbumError::Config(format!("无效 glob `{s}`: {e}")))
                })
                .collect()
        };
        Ok(Self {
            includes: parse(includes)?,
            excludes: parse(excludes)?,
            extensions: media_extensions(),
            max_depth: 16,
        })
    }
}

fn media_extensions() -> HashSet<String> {
    [
        "jpg", "jpeg", "png", "heic", "heif", "webp", "gif", "bmp", "tiff", "tif",
        "raw", "dng", "arw", "cr2", "cr3", "nef", "orf", "rw2",
        "mp4", "mov", "3gp", "mkv", "avi", "m4v", "webm", "mts", "m2ts",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn is_media(rel_path: &str, exts: &HashSet<String>) -> bool {
    rel_path
        .rsplit('.')
        .next()
        .map(|e| exts.contains(&e.to_ascii_lowercase()))
        .unwrap_or(false)
}

fn included(rel_path: &str, cfg: &WalkConfig) -> bool {
    if cfg.includes.is_empty() {
        return true;
    }
    cfg.includes.iter().any(|p| p.matches(rel_path))
}

fn excluded(rel_path: &str, cfg: &WalkConfig) -> bool {
    cfg.excludes.iter().any(|p| p.matches(rel_path))
}

fn systemtime_to_unix(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 递归扫描远端目录。
/// `base_dir` 是相对 FTP chroot 的起始路径（如 "/" 或 "/sdcard"）。
/// rel_path 以 base_dir 为基准（不含开头 /）。
pub async fn walk(
    ftp: &mut AsyncFtpStream,
    base_dir: &str,
    cfg: &WalkConfig,
) -> Result<Vec<FileEntry>> {
    let base = base_dir.trim_end_matches('/').to_string();
    let mut out = Vec::new();
    let mut stack: Vec<(String, u32)> = vec![(base.clone(), 0)];

    while let Some((dir, depth)) = stack.pop() {
        if depth > cfg.max_depth { continue; }

        let listing_arg = if dir.is_empty() { None } else { Some(dir.as_str()) };
        let listing: Vec<String> = ftp.mlsd(listing_arg).await?;

        for raw in listing {
            let file: ListFile = match ListParser::parse_mlsd(&raw) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!(line = %raw, err = %e, "skip unparseable MLSD line");
                    continue;
                }
            };
            let name = file.name();
            if name == "." || name == ".." { continue; }

            let abs_path = if dir.is_empty() {
                format!("/{name}")
            } else {
                format!("{dir}/{name}")
            };

            let rel = abs_path
                .strip_prefix(&format!("{base}/"))
                .map(|s| s.to_string())
                .unwrap_or_else(|| abs_path.trim_start_matches('/').to_string());

            if file.is_directory() {
                if !excluded(&rel, cfg) {
                    stack.push((abs_path, depth + 1));
                }
            } else if file.is_file()
                && is_media(&rel, &cfg.extensions)
                && included(&rel, cfg)
                && !excluded(&rel, cfg)
            {
                let size = file.size() as u64;
                let mtime = systemtime_to_unix(file.modified());
                out.push(FileEntry { rel_path: rel, size, mtime });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_media_check() {
        let ext = media_extensions();
        assert!(is_media("DCIM/Camera/IMG_001.JPG", &ext));
        assert!(is_media("a.heic", &ext));
        assert!(!is_media("a.txt", &ext));
        assert!(!is_media("noext", &ext));
    }

    #[test]
    fn include_exclude() {
        let cfg = WalkConfig::from_settings(
            &["DCIM/**/*".into(), "Pictures/**/*".into()],
            &["**/.thumbnails/**".into(), "**/*.tmp".into()],
        )
        .unwrap();
        assert!(included("DCIM/Camera/x.jpg", &cfg));
        assert!(included("Pictures/Screenshots/y.png", &cfg));
        assert!(!included("Other/a.jpg", &cfg));
        assert!(excluded("DCIM/.thumbnails/x.jpg", &cfg));
        assert!(excluded("DCIM/x.tmp", &cfg));
    }
}
