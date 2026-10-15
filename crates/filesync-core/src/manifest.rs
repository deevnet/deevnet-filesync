//! What a snapshot of a root records about each path (see the Metadata Matrix).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    Dir,
    File {
        /// Keyed content identifier; also the blob's name in the hub.
        blob: String,
        size: u64,
        mtime_ns: i64,
        #[serde(default)]
        exec: bool,
    },
}

impl Entry {
    /// Whether two entries hold the same content. Modification time alone is not a change:
    /// content hashes decide.
    pub fn same_content(&self, other: &Entry) -> bool {
        match (self, other) {
            (Entry::Dir, Entry::Dir) => true,
            (
                Entry::File {
                    blob: a, exec: ea, ..
                },
                Entry::File {
                    blob: b, exec: eb, ..
                },
            ) => a == b && ea == eb,
            _ => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Entry::Dir)
    }
}

pub fn same(a: Option<&Entry>, b: Option<&Entry>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_content(b),
        _ => false,
    }
}

pub type Entries = BTreeMap<String, Entry>;

/// One numbered state of a root. Stored encrypted at `roots/<id>/snap/<seq>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub seq: u64,
    pub parent: Option<u64>,
    pub device: String,
    pub created_unix: i64,
    pub entries: Entries,
}

/// A root's descriptive record. Stored encrypted at `roots/<id>/meta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootMeta {
    pub name: String,
    pub created_unix: i64,
}
