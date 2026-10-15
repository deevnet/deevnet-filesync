//! Reads a local root into manifest entries.
//!
//! Hashing every file on every scan would be slow, so the scan keeps an index of
//! (size, mtime) → content id from the last scan and only rehashes files whose size or
//! modification time changed.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::crypto::Keys;
use crate::manifest::{Entries, Entry};
use crate::names;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub size: u64,
    pub mtime_ns: i64,
    pub blob: String,
}

pub type Index = BTreeMap<String, IndexEntry>;

pub struct ScanResult {
    pub entries: Entries,
    pub index: Index,
    /// Things the user should know about: skipped files and why.
    pub warnings: Vec<String>,
}

pub fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(unix)]
pub fn is_exec(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub fn is_exec(_meta: &fs::Metadata) -> bool {
    false
}

pub fn hash_file(keys: &Keys, path: &Path) -> Result<String> {
    let mut f = fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut h = keys.id_hasher();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex().to_string())
}

pub fn scan(root: &Path, keys: &Keys, index: &Index, max_file_bytes: u64) -> Result<ScanResult> {
    if !root.is_dir() {
        bail!(
            "{} is not a directory (missing, unmounted, or not accessible); refusing to sync it",
            root.display()
        );
    }
    let mut out = ScanResult {
        entries: Entries::new(),
        index: Index::new(),
        warnings: Vec::new(),
    };
    let mut stack = vec![String::new()];
    while let Some(rel_dir) = stack.pop() {
        let dir = if rel_dir.is_empty() {
            root.to_path_buf()
        } else {
            root.join(&rel_dir)
        };
        let mut children: Vec<_> = fs::read_dir(&dir)
            .with_context(|| format!("listing {}", dir.display()))?
            .collect::<std::io::Result<_>>()?;
        children.sort_by_key(|e| e.file_name());
        for child in children {
            let os_name = child.file_name();
            let Some(name) = os_name.to_str() else {
                out.warnings.push(format!(
                    "skipped {}: name is not valid UTF-8",
                    dir.join(&os_name).display()
                ));
                continue;
            };
            if names::ignored(name) {
                continue;
            }
            let rel = if rel_dir.is_empty() {
                name.to_string()
            } else {
                format!("{rel_dir}/{name}")
            };
            if let Some(problem) = names::component_problem(name) {
                out.warnings
                    .push(format!("skipped {rel}: name is not portable ({problem})"));
                continue;
            }
            let meta = fs::symlink_metadata(child.path())?;
            let ft = meta.file_type();
            if ft.is_symlink() {
                out.warnings
                    .push(format!("skipped {rel}: symlinks are not synced"));
            } else if ft.is_dir() {
                out.entries.insert(rel.clone(), Entry::Dir);
                stack.push(rel);
            } else if ft.is_file() {
                let size = meta.len();
                if size > max_file_bytes {
                    out.warnings.push(format!(
                        "skipped {rel}: {size} bytes is over the {max_file_bytes}-byte file limit"
                    ));
                    continue;
                }
                let mtime = mtime_ns(&meta);
                let blob = match index.get(&rel) {
                    Some(ix) if ix.size == size && ix.mtime_ns == mtime => ix.blob.clone(),
                    _ => hash_file(keys, &child.path())?,
                };
                out.index.insert(
                    rel.clone(),
                    IndexEntry {
                        size,
                        mtime_ns: mtime,
                        blob: blob.clone(),
                    },
                );
                out.entries.insert(
                    rel,
                    Entry::File {
                        blob,
                        size,
                        mtime_ns: mtime,
                        exec: is_exec(&meta),
                    },
                );
            } else {
                out.warnings
                    .push(format!("skipped {rel}: not a regular file or directory"));
            }
        }
    }
    drop_collisions(&mut out);
    Ok(out)
}

/// Keeps the first (in sorted order) of any names that collide on case-insensitive or
/// normalization-insensitive platforms, and skips the rest, with everything beneath them.
fn drop_collisions(out: &mut ScanResult) {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut dropped: Vec<String> = Vec::new();
    for path in out.entries.keys() {
        if dropped.iter().any(|d| path.starts_with(&format!("{d}/"))) {
            continue;
        }
        let key = names::collision_key(path);
        if let Some(first) = seen.get(&key) {
            out.warnings.push(format!(
                "skipped {path}: its name collides with {first} on Windows and macOS"
            ));
            dropped.push(path.clone());
        } else {
            seen.insert(key, path.clone());
        }
    }
    for d in &dropped {
        let under = format!("{d}/");
        out.entries.retain(|p, _| p != d && !p.starts_with(&under));
        out.index.retain(|p, _| p != d && !p.starts_with(&under));
    }
}
