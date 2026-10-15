//! The hub layout. Every object except `format` is encrypted, and object names reveal nothing
//! about roots, paths, or file names (SEC-1, SEC-2, HUB-1).
//!
//! ```text
//! format                          plaintext format marker (HUB-2)
//! blobs/<content-id>              encrypted file contents, one object per distinct content
//! roots/<root-id>/meta            encrypted RootMeta (the root's name)
//! roots/<root-id>/snap/<seq>      encrypted Snapshot; seq is zero-padded so keys sort by number
//! ```
//!
//! A snapshot number is claimed with a conditional write, so exactly one device wins each
//! number and nobody's change is silently lost (HUB-4). Old snapshots are the version history.

use anyhow::{bail, Context, Result};

use crate::backend::Backend;
use crate::crypto::Keys;
use crate::manifest::{RootMeta, Snapshot};
use crate::util;

pub const FORMAT: &str = "deevnet-filesync prototype-1\n";

pub struct Hub {
    backend: Box<dyn Backend>,
    keys: Keys,
}

pub struct RootInfo {
    pub id: String,
    pub meta: RootMeta,
}

fn snap_key(root: &str, seq: u64) -> String {
    format!("roots/{root}/snap/{seq:020}")
}

impl Hub {
    pub fn new(backend: Box<dyn Backend>, keys: Keys) -> Self {
        Self { backend, keys }
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn describe(&self) -> String {
        self.backend.describe()
    }

    /// Writes the format marker if the hub is new; checks it otherwise.
    pub fn init(&self) -> Result<()> {
        self.backend.put_if_absent("format", FORMAT.as_bytes())?;
        self.check_format()
    }

    pub fn check_format(&self) -> Result<()> {
        match self.backend.get("format")? {
            Some(f) if f == FORMAT.as_bytes() => Ok(()),
            Some(f) => bail!(
                "hub format is {:?}; this client understands {:?}",
                String::from_utf8_lossy(&f).trim(),
                FORMAT.trim()
            ),
            None => bail!("no FileSync hub here (missing `format`); run `filesync init` first"),
        }
    }

    pub fn create_root(&self, name: &str) -> Result<String> {
        let id = hex::encode(util::random_bytes::<16>());
        let key = format!("roots/{id}/meta");
        let meta = RootMeta {
            name: name.to_string(),
            created_unix: util::now_unix(),
        };
        let sealed = self.keys.seal(&key, &serde_json::to_vec(&meta)?)?;
        if !self.backend.put_if_absent(&key, &sealed)? {
            bail!("root id collision; try again");
        }
        Ok(id)
    }

    pub fn list_roots(&self) -> Result<Vec<RootInfo>> {
        let mut out = Vec::new();
        for key in self.backend.list("roots/", None)? {
            let Some(id) = key
                .strip_prefix("roots/")
                .and_then(|k| k.strip_suffix("/meta"))
            else {
                continue;
            };
            let sealed = self
                .backend
                .get(&key)?
                .context("root vanished while listing")?;
            let meta: RootMeta = serde_json::from_slice(&self.keys.open(&key, &sealed)?)?;
            out.push(RootInfo {
                id: id.to_string(),
                meta,
            });
        }
        out.sort_by(|a, b| a.meta.name.cmp(&b.meta.name));
        Ok(out)
    }

    /// Snapshot numbers newer than `after` (all of them if `None`), ascending.
    pub fn snapshot_seqs(&self, root: &str, after: Option<u64>) -> Result<Vec<u64>> {
        let prefix = format!("roots/{root}/snap/");
        let start = after.map(|s| snap_key(root, s));
        Ok(self
            .backend
            .list(&prefix, start.as_deref())?
            .iter()
            .filter_map(|k| k.strip_prefix(&prefix)?.parse().ok())
            .collect())
    }

    pub fn latest_seq(&self, root: &str, after: Option<u64>) -> Result<Option<u64>> {
        Ok(self.snapshot_seqs(root, after)?.last().copied().or(after))
    }

    pub fn get_snapshot(&self, root: &str, seq: u64) -> Result<Snapshot> {
        let key = snap_key(root, seq);
        let sealed = self
            .backend
            .get(&key)?
            .with_context(|| format!("snapshot {seq} is missing"))?;
        let snap: Snapshot = serde_json::from_slice(&self.keys.open(&key, &sealed)?)?;
        if snap.seq != seq {
            bail!("snapshot {seq} claims to be {}", snap.seq);
        }
        Ok(snap)
    }

    /// Claims `snap.seq`. Returns `false` if another device claimed it first.
    pub fn put_snapshot(&self, root: &str, snap: &Snapshot) -> Result<bool> {
        let key = snap_key(root, snap.seq);
        let sealed = self.keys.seal(&key, &serde_json::to_vec(snap)?)?;
        self.backend.put_if_absent(&key, &sealed)
    }

    pub fn put_blob(&self, id: &str, data: &[u8]) -> Result<()> {
        let key = format!("blobs/{id}");
        let sealed = self.keys.seal(&key, data)?;
        // Same content, same id: if it already exists, it is already right.
        self.backend.put_if_absent(&key, &sealed)?;
        Ok(())
    }

    pub fn get_blob(&self, id: &str) -> Result<Vec<u8>> {
        let key = format!("blobs/{id}");
        let sealed = self
            .backend
            .get(&key)?
            .with_context(|| format!("blob {id} is missing from the hub"))?;
        let data = self.keys.open(&key, &sealed)?;
        if self.keys.content_id(&data) != id {
            bail!("blob {id} does not match its content id");
        }
        Ok(data)
    }
}
