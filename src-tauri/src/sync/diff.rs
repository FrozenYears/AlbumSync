// 远端 vs 本地清单差异计算
//
// 输入：两组 FileEntry，按相对路径分组
// 输出：DiffItem 列表（Add / Update / DeleteLocal）

use std::collections::HashMap;

/// 远端或本地的一条文件条目
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub rel_path: String,
    pub size: u64,
    pub mtime: i64,
}

#[derive(Debug, Clone)]
pub enum DiffItem {
    /// 远端有、本地无 → 下载
    Add(FileEntry),
    /// 路径相同但 size 或 mtime 不一致 → 重下
    Update(FileEntry),
    /// 远端无、本地有 → 入 trash
    DeleteLocal { rel_path: String, size: u64 },
}

/// 计算 diff
///
/// - mtime 允许 ±1 秒误差（FTP 不同实现的精度不同）
/// - size 必须严格相等
pub fn compute_diff(local: &[FileEntry], remote: &[FileEntry]) -> Vec<DiffItem> {
    let mut local_map: HashMap<&str, &FileEntry> =
        local.iter().map(|e| (e.rel_path.as_str(), e)).collect();
    let mut out = Vec::with_capacity(remote.len());

    for r in remote {
        match local_map.remove(r.rel_path.as_str()) {
            Some(l) => {
                if l.size != r.size || (l.mtime - r.mtime).abs() > 1 {
                    out.push(DiffItem::Update(r.clone()));
                }
            }
            None => out.push(DiffItem::Add(r.clone())),
        }
    }

    // local_map 里剩下的就是远端不再存在的
    for (rel_path, l) in local_map {
        out.push(DiffItem::DeleteLocal {
            rel_path: rel_path.to_string(),
            size: l.size,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fe(p: &str, s: u64, m: i64) -> FileEntry {
        FileEntry {
            rel_path: p.into(),
            size: s,
            mtime: m,
        }
    }

    #[test]
    fn empty_local_all_add() {
        let r = vec![fe("a", 1, 100), fe("b", 2, 200)];
        let d = compute_diff(&[], &r);
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0], DiffItem::Add(_)));
        assert!(matches!(d[1], DiffItem::Add(_)));
    }

    #[test]
    fn empty_remote_all_delete() {
        let l = vec![fe("a", 1, 100), fe("b", 2, 200)];
        let d = compute_diff(&l, &[]);
        assert_eq!(d.len(), 2);
        assert!(matches!(d[0], DiffItem::DeleteLocal { .. }));
    }

    #[test]
    fn unchanged_no_diff() {
        let same = vec![fe("a", 1, 100)];
        assert_eq!(compute_diff(&same, &same).len(), 0);
    }

    #[test]
    fn mtime_tolerance_one_second() {
        let l = vec![fe("a", 1, 100)];
        let r = vec![fe("a", 1, 101)];
        assert_eq!(compute_diff(&l, &r).len(), 0);
        let r2 = vec![fe("a", 1, 102)];
        assert_eq!(compute_diff(&l, &r2).len(), 1);
    }

    #[test]
    fn size_change_is_update() {
        let l = vec![fe("a", 1, 100)];
        let r = vec![fe("a", 2, 100)];
        let d = compute_diff(&l, &r);
        assert!(matches!(d[0], DiffItem::Update(_)));
    }

    #[test]
    fn mixed_scenarios() {
        let l = vec![
            fe("keep", 10, 1000),
            fe("update", 20, 2000),
            fe("gone", 30, 3000),
        ];
        let r = vec![
            fe("keep", 10, 1000),
            fe("update", 25, 2000),
            fe("new", 40, 4000),
        ];
        let d = compute_diff(&l, &r);
        assert_eq!(d.len(), 3);
        let kinds: Vec<&'static str> = d
            .iter()
            .map(|x| match x {
                DiffItem::Add(_) => "add",
                DiffItem::Update(_) => "update",
                DiffItem::DeleteLocal { .. } => "del",
            })
            .collect();
        assert!(kinds.contains(&"add"));
        assert!(kinds.contains(&"update"));
        assert!(kinds.contains(&"del"));
    }
}
