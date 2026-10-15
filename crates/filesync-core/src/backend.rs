//! The storage contract (BE-2): everything the hub needs from a storage provider.
//!
//! Deliberately primitive: whole-object put/get/delete, a prefix listing, and one
//! concurrency primitive, "create only if absent", which the engine uses to make each
//! snapshot number single-writer (HUB-4).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::util;

pub trait Backend: Send + Sync {
    /// Stores an object, replacing any existing one.
    fn put(&self, key: &str, data: &[u8]) -> Result<()>;
    /// Stores an object only if no object with that key exists. Returns `false` if one did.
    fn put_if_absent(&self, key: &str, data: &[u8]) -> Result<bool>;
    /// Fetches an object, or `None` if it does not exist.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn delete(&self, key: &str) -> Result<()>;
    /// Lists keys beginning with `prefix`, in ascending order, optionally only those
    /// sorting after `start_after`.
    fn list(&self, prefix: &str, start_after: Option<&str>) -> Result<Vec<String>>;
    /// One line describing where the hub is, for status output.
    fn describe(&self) -> String;
}

/// Checks that a key uses only the characters the hub layout produces.
pub fn check_key(key: &str) -> Result<()> {
    if key.is_empty()
        || key.starts_with('/')
        || key.contains("..")
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'-' || b == b'_' || b == b'.')
    {
        bail!("invalid hub object key: {key:?}");
    }
    Ok(())
}

/// A hub in a local directory. Used for tests, and usable for a hub on a mounted NAS (BE-3).
pub struct DirBackend {
    root: PathBuf,
}

impl DirBackend {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
        Ok(Self { root })
    }

    fn path(&self, key: &str) -> Result<PathBuf> {
        check_key(key)?;
        Ok(self.root.join(key))
    }
}

impl Backend for DirBackend {
    fn put(&self, key: &str, data: &[u8]) -> Result<()> {
        let p = self.path(key)?;
        fs::create_dir_all(p.parent().unwrap())?;
        util::write_atomic(&p, data)
    }

    fn put_if_absent(&self, key: &str, data: &[u8]) -> Result<bool> {
        let p = self.path(key)?;
        let dir = p.parent().unwrap();
        fs::create_dir_all(dir)?;
        // Write a temporary file, then hard-link it into place: link() fails atomically if the
        // target exists, and readers never see a partial object.
        let tmp = dir.join(format!(
            "{}{}.tmp",
            util::TEMP_PREFIX,
            hex::encode(util::random_bytes::<6>())
        ));
        fs::write(&tmp, data)?;
        let linked = fs::hard_link(&tmp, &p);
        let _ = fs::remove_file(&tmp);
        match linked {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e).with_context(|| format!("creating {}", p.display())),
        }
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(self.path(key)?) {
            Ok(d) => Ok(Some(d)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        match fs::remove_file(self.path(key)?) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    fn list(&self, prefix: &str, start_after: Option<&str>) -> Result<Vec<String>> {
        let mut out = Vec::new();
        walk(&self.root, &self.root, &mut out)?;
        out.retain(|k| k.starts_with(prefix) && start_after.is_none_or(|s| k.as_str() > s));
        out.sort();
        Ok(out)
    }

    fn describe(&self) -> String {
        format!("directory {}", self.root.display())
    }
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(util::TEMP_PREFIX) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            walk(base, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
