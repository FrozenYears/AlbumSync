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

/// 把 MLSD 行里 `Modify=YYYYMMDDHHMMSS.fff;` 的小数秒去掉，
/// 保留 `Modify=YYYYMMDDHHMMSS;`。其他字段不动。
fn strip_modify_fraction(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    let Some(p) = lower.find("modify=") else { return raw.to_string() };
    let value_start = p + 7;
    let rest = &raw[value_start..];
    let value_end = rest.find(';').map(|e| value_start + e).unwrap_or(raw.len());
    let value = &raw[value_start..value_end];
    let Some(dot_off) = value.find('.') else { return raw.to_string() };
    let mut out = String::with_capacity(raw.len());
    out.push_str(&raw[..value_start + dot_off]);
    out.push_str(&raw[value_end..]);
    out
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

        // Primitive FTPd（以及不少其它 FTP 服务器）不支持 MLSD <abs-path>。
        // 这里改成先 CWD 再 MLSD(None)，对所有 RFC 3659 合规的服务器都通用。
        let cwd_target = if dir.is_empty() { "/" } else { dir.as_str() };
        if let Err(e) = ftp.cwd(cwd_target).await {
            tracing::warn!(dir = %dir, err = %e, "CWD failed, skip directory");
            continue;
        }
        let listing: Vec<String> = ftp.mlsd(None).await?;
        let entries_total = listing.len();

        let mut dir_total = 0usize;
        let mut file_total = 0usize;
        let mut accepted = 0usize;
        let mut skipped_ext = 0usize;
        let mut skipped_filter = 0usize;

        for raw in listing {
            // Primitive FTPd 在 Modify 字段加毫秒小数（RFC 3659 允许，但 suppaftp 8 不接受）
            let cleaned = strip_modify_fraction(&raw);
            let file: ListFile = match ListParser::parse_mlsd(&cleaned) {
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
                dir_total += 1;
                if !excluded(&rel, cfg) {
                    stack.push((abs_path, depth + 1));
                } else {
                    tracing::debug!(rel = %rel, "dir excluded");
                }
            } else if file.is_file() {
                file_total += 1;
                let ext_ok = is_media(&rel, &cfg.extensions);
                let inc_ok = included(&rel, cfg);
                let exc_hit = excluded(&rel, cfg);
                if !ext_ok {
                    skipped_ext += 1;
                    tracing::debug!(rel = %rel, "file skipped: not media ext");
                } else if !inc_ok || exc_hit {
                    skipped_filter += 1;
                    tracing::debug!(rel = %rel, inc_ok, exc_hit, "file skipped: glob filter");
                } else {
                    accepted += 1;
                    let size = file.size() as u64;
                    let mtime = systemtime_to_unix(file.modified());
                    out.push(FileEntry { rel_path: rel, size, mtime });
                }
            }
        }

        tracing::info!(
            dir = %dir,
            depth,
            entries = entries_total,
            dirs = dir_total,
            files = file_total,
            accepted,
            skipped_ext,
            skipped_filter,
            "scanned directory"
        );
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
    fn strip_fraction_basic() {
        let inp = "Size=3452;Modify=20241225213441.564;Type=dir; 音乐";
        assert_eq!(
            strip_modify_fraction(inp),
            "Size=3452;Modify=20241225213441;Type=dir; 音乐"
        );
    }

    #[test]
    fn strip_fraction_no_dot() {
        let inp = "Size=10;Modify=20240101000000;Type=file; a.jpg";
        assert_eq!(strip_modify_fraction(inp), inp);
    }

    #[test]
    fn strip_fraction_modify_at_end() {
        let inp = "Type=file;Size=10;Modify=20240101000000.5 a.jpg";
        // 没有分号 → 把 value_end 视为 raw.len()，截到 .
        assert_eq!(strip_modify_fraction(inp), "Type=file;Size=10;Modify=20240101000000");
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
