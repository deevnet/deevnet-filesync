//! The sync engine: a three-way comparison of each path between
//! - **base**: what this device last agreed with the hub (the last synced snapshot),
//! - **local**: what is on disk now,
//! - **remote**: the hub's newest snapshot.
//!
//! A path changed on only one side takes that side's version. A path changed on both sides to
//! different content is a conflict: the remote version takes the name, and the local version is
//! kept beside it as a conflict copy (CONF-1). An edit beats a delete (CONF-4).
//!
//! Pushing claims the next snapshot number with a conditional write. If another device got
//! there first, the engine starts over from a fresh scan (HUB-4).

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::hub::Hub;
use crate::manifest::{same, Entries, Entry, Snapshot};
use crate::scan::{self, Index, IndexEntry};
use crate::util;

/// What a device remembers about one mounted root, between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MountState {
    pub base_seq: Option<u64>,
    pub base: Entries,
    pub index: Index,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub device: String,
    /// Upload local changes. `false` for pull-only and receive-only mounts (ROOT-9).
    pub push: bool,
    /// Plan only; change nothing locally or in the hub.
    pub dry_run: bool,
    /// Allow a run that would delete everything on one side (see `guard`).
    pub allow_mass_delete: bool,
    pub max_file_bytes: u64,
}

#[derive(Debug, Default)]
pub struct Report {
    pub remote_seq: Option<u64>,
    pub committed_seq: Option<u64>,
    pub downloaded: Vec<String>,
    pub removed_local: Vec<String>,
    pub uploaded: Vec<String>,
    pub removed_remote: Vec<String>,
    pub conflicts: Vec<String>,
    /// Local changes not uploaded (pull-only, receive-only, or dry run).
    pub pending: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
enum Op {
    /// Move a conflicting local item aside, to its conflict-copy name.
    Aside {
        from: String,
        to: String,
    },
    MkDir(String),
    /// Download `Entry` into place.
    Write(String, Entry),
    /// Delete a local item, which the scan saw as `Entry`.
    Remove(String, Entry),
}

struct Plan {
    ops: Vec<Op>,
    merged: Entries,
    conflicts: Vec<String>,
    local_changes: Vec<String>,
}

const MAX_ATTEMPTS: usize = 5;

pub fn sync(
    hub: &Hub,
    root_id: &str,
    dir: &Path,
    state: &mut MountState,
    opt: &Options,
) -> Result<Report> {
    for _ in 0..MAX_ATTEMPTS {
        if let Some(report) = attempt(hub, root_id, dir, state, opt)? {
            return Ok(report);
        }
    }
    bail!("another device kept updating this root; gave up after {MAX_ATTEMPTS} attempts")
}

/// One pass. `Ok(None)` means another device won the snapshot number; try again.
fn attempt(
    hub: &Hub,
    root_id: &str,
    dir: &Path,
    state: &mut MountState,
    opt: &Options,
) -> Result<Option<Report>> {
    let scanned = scan::scan(dir, hub.keys(), &state.index, opt.max_file_bytes)?;
    let local = scanned.entries;

    let remote_seq = hub.latest_seq(root_id, state.base_seq)?;
    let remote = if remote_seq != state.base_seq {
        hub.get_snapshot(root_id, remote_seq.expect("newer seq exists"))?
            .entries
    } else {
        state.base.clone()
    };

    guard(&state.base, &local, &remote, opt)?;
    let plan = plan(&state.base, &local, &remote, opt);

    let mut report = Report {
        remote_seq,
        warnings: scanned.warnings,
        conflicts: plan.conflicts.clone(),
        ..Default::default()
    };
    for op in &plan.ops {
        match op {
            Op::Write(p, _) | Op::MkDir(p) => report.downloaded.push(p.clone()),
            Op::Remove(p, _) => report.removed_local.push(p.clone()),
            Op::Aside { .. } => {}
        }
    }
    if !opt.push || opt.dry_run {
        report.pending = plan.local_changes.clone();
    }
    if opt.dry_run {
        if opt.push {
            diff_remote(&plan.merged, &remote, &mut report);
        }
        return Ok(Some(report));
    }

    state.index = scanned.index;
    execute(hub, dir, &plan.ops, state, &mut report.warnings)?;
    state.base = remote.clone();
    state.base_seq = remote_seq;

    if !opt.push || unchanged(&plan.merged, &remote) {
        return Ok(Some(report));
    }

    // Upload new content, then claim the next snapshot number.
    let known: HashSet<&str> = remote
        .values()
        .chain(state.base.values())
        .filter_map(blob_of)
        .collect();
    for (path, entry) in &plan.merged {
        if let Entry::File { blob, .. } = entry {
            if !known.contains(blob.as_str()) {
                let data = fs::read(dir.join(path)).with_context(|| format!("reading {path}"))?;
                if hub.keys().content_id(&data) != *blob {
                    // Changed since the scan: start over and pick up the newer content.
                    return Ok(None);
                }
                hub.put_blob(blob, &data)?;
            }
        }
    }
    let seq = remote_seq.map_or(0, |s| s + 1);
    let snap = Snapshot {
        seq,
        parent: remote_seq,
        device: opt.device.clone(),
        created_unix: util::now_unix(),
        entries: plan.merged.clone(),
    };
    if !hub.put_snapshot(root_id, &snap)? {
        return Ok(None);
    }
    diff_remote(&plan.merged, &remote, &mut report);
    state.base = plan.merged;
    state.base_seq = Some(seq);
    report.committed_seq = Some(seq);
    Ok(Some(report))
}

/// Refuses runs that would wipe out one side wholesale. An empty-looking root usually means
/// an unmounted disk or denied permissions, not that the user deleted everything (MAC-5).
fn guard(base: &Entries, local: &Entries, remote: &Entries, opt: &Options) -> Result<()> {
    if opt.allow_mass_delete || base.is_empty() {
        return Ok(());
    }
    if local.is_empty() && opt.push {
        bail!(
            "the local folder is empty but this root last held {} items; refusing to delete them \
             everywhere. If that is really intended, re-run with --allow-mass-delete",
            base.len()
        );
    }
    if remote.is_empty() {
        bail!(
            "the hub's newest snapshot of this root is empty but it last held {} items; refusing \
             to delete them locally. If that is really intended, re-run with --allow-mass-delete",
            base.len()
        );
    }
    Ok(())
}

fn plan(base: &Entries, local: &Entries, remote: &Entries, opt: &Options) -> Plan {
    let mut p = Plan {
        ops: Vec::new(),
        merged: Entries::new(),
        conflicts: Vec::new(),
        local_changes: Vec::new(),
    };
    let paths: BTreeSet<&String> = base
        .keys()
        .chain(local.keys())
        .chain(remote.keys())
        .collect();
    for path in paths {
        let (b, l, r) = (base.get(path), local.get(path), remote.get(path));
        let local_changed = !same(l, b);
        let remote_changed = !same(r, b);
        match (local_changed, remote_changed) {
            (false, false) => keep(&mut p.merged, path, r),
            (true, false) => {
                p.local_changes.push(describe_local(path, b, l));
                keep(&mut p.merged, path, if opt.push { l } else { r });
            }
            (false, true) => {
                take_remote(&mut p.ops, path, l, r);
                keep(&mut p.merged, path, r);
            }
            (true, true) if same(l, r) => keep(&mut p.merged, path, r),
            (true, true) => match (l, r) {
                // Deleted here, edited there: the edit wins (CONF-4).
                (None, Some(_)) => {
                    take_remote(&mut p.ops, path, l, r);
                    keep(&mut p.merged, path, r);
                }
                // Edited here, deleted there: the edit wins (CONF-4).
                (Some(_), None) => {
                    p.local_changes.push(describe_local(path, b, l));
                    keep(&mut p.merged, path, if opt.push { l } else { None });
                }
                (Some(le), Some(re)) if le.is_dir() && re.is_dir() => keep(&mut p.merged, path, r),
                (Some(le), Some(_)) => {
                    let aside = conflict_name(path, &opt.device);
                    p.ops.push(Op::Aside {
                        from: path.clone(),
                        to: aside.clone(),
                    });
                    take_remote(&mut p.ops, path, None, r);
                    keep(&mut p.merged, path, r);
                    if opt.push {
                        if let Entry::File { .. } = le {
                            p.merged.insert(aside.clone(), le.clone());
                        }
                    }
                    p.conflicts.push(aside);
                }
                (None, None) => unreachable!("both absent is never a change on both sides"),
            },
        }
    }
    p
}

fn keep(merged: &mut Entries, path: &str, e: Option<&Entry>) {
    if let Some(e) = e {
        merged.insert(path.to_string(), e.clone());
    }
}

fn take_remote(ops: &mut Vec<Op>, path: &str, l: Option<&Entry>, r: Option<&Entry>) {
    match (l, r) {
        (Some(le), None) => ops.push(Op::Remove(path.into(), le.clone())),
        (l, Some(Entry::Dir)) => {
            if let Some(le @ Entry::File { .. }) = l {
                ops.push(Op::Remove(path.into(), le.clone()));
            }
            ops.push(Op::MkDir(path.into()));
        }
        (l, Some(re)) => {
            if let Some(Entry::Dir) = l {
                ops.push(Op::Remove(path.into(), Entry::Dir));
            }
            ops.push(Op::Write(path.into(), re.clone()));
        }
        (None, None) => {}
    }
}

fn describe_local(path: &str, b: Option<&Entry>, l: Option<&Entry>) -> String {
    match (b, l) {
        (None, Some(_)) => format!("added    {path}"),
        (Some(_), None) => format!("deleted  {path}"),
        _ => format!("modified {path}"),
    }
}

/// `dir/report.pdf` → `dir/report.sync-conflict-<device>-<UTC time>.pdf`
fn conflict_name(path: &str, device: &str) -> String {
    let device: String = device
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let tag = format!(
        "sync-conflict-{device}-{}",
        util::compact_utc(util::now_unix())
    );
    let (dir, name) = match path.rsplit_once('/') {
        Some((d, n)) => (format!("{d}/"), n),
        None => (String::new(), path),
    };
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{dir}{stem}.{tag}.{ext}"),
        _ => format!("{dir}{name}.{tag}"),
    }
}

fn depth(p: &str) -> usize {
    p.matches('/').count()
}

fn execute(
    hub: &Hub,
    dir: &Path,
    ops: &[Op],
    state: &mut MountState,
    warnings: &mut Vec<String>,
) -> Result<()> {
    // Order: set conflicting items aside, create directories top-down, write files, delete
    // files, then delete directories bottom-up.
    for op in ops {
        if let Op::Aside { from, to } = op {
            fs::rename(dir.join(from), dir.join(to))
                .with_context(|| format!("moving {from} aside to {to}"))?;
            if let Some(ix) = state.index.remove(from) {
                state.index.insert(to.clone(), ix);
            }
        }
    }
    let mut mkdirs: Vec<&String> = ops
        .iter()
        .filter_map(|o| if let Op::MkDir(p) = o { Some(p) } else { None })
        .collect();
    mkdirs.sort_by_key(|p| depth(p));
    let mut removes: Vec<(&String, &Entry)> = ops
        .iter()
        .filter_map(|o| {
            if let Op::Remove(p, e) = o {
                Some((p, e))
            } else {
                None
            }
        })
        .collect();
    removes.sort_by_key(|(p, e)| (e.is_dir(), std::cmp::Reverse(depth(p))));

    // A file being replaced by a directory, or the reverse, must go first.
    for (path, e) in &removes {
        if mkdirs.contains(path)
            || ops
                .iter()
                .any(|o| matches!(o, Op::Write(p, _) if p == *path))
        {
            remove_local(dir, path, e, state, warnings)?;
        }
    }
    for path in &mkdirs {
        fs::create_dir_all(dir.join(path)).with_context(|| format!("creating {path}"))?;
    }
    for op in ops {
        if let Op::Write(
            path,
            Entry::File {
                blob,
                mtime_ns,
                exec,
                ..
            },
        ) = op
        {
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let data = hub.get_blob(blob)?;
            util::write_atomic(&target, &data)?;
            set_exec(&target, *exec)?;
            let ft = filetime::FileTime::from_unix_time(
                mtime_ns.div_euclid(1_000_000_000),
                mtime_ns.rem_euclid(1_000_000_000) as u32,
            );
            filetime::set_file_mtime(&target, ft)?;
            let meta = fs::metadata(&target)?;
            state.index.insert(
                path.clone(),
                IndexEntry {
                    size: meta.len(),
                    mtime_ns: scan::mtime_ns(&meta),
                    blob: blob.clone(),
                },
            );
        }
    }
    for (path, e) in &removes {
        if !(mkdirs.contains(path)
            || ops
                .iter()
                .any(|o| matches!(o, Op::Write(p, _) if p == *path)))
        {
            remove_local(dir, path, e, state, warnings)?;
        }
    }
    Ok(())
}

fn remove_local(
    dir: &Path,
    path: &str,
    seen: &Entry,
    state: &mut MountState,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let target = dir.join(path);
    let meta = match fs::symlink_metadata(&target) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    match seen {
        Entry::Dir => {
            if let Err(e) = fs::remove_dir(&target) {
                warnings.push(format!("kept directory {path}: deleted elsewhere, but {e}"));
            }
        }
        Entry::File { size, mtime_ns, .. } => {
            // Only delete what the scan saw. If it changed since, keep it; the next run will
            // treat it as an edit, which beats the delete.
            if meta.len() != *size || scan::mtime_ns(&meta) != *mtime_ns {
                warnings.push(format!(
                    "kept {path}: deleted elsewhere, but changed here during sync"
                ));
                return Ok(());
            }
            fs::remove_file(&target).with_context(|| format!("deleting {path}"))?;
        }
    }
    state.index.remove(path);
    Ok(())
}

#[cfg(unix)]
fn set_exec(path: &Path, exec: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    let mode = perms.mode();
    let new = if exec {
        mode | ((mode & 0o444) >> 2)
    } else {
        mode & !0o111
    };
    if new != mode {
        perms.set_mode(new);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_exec(_path: &Path, _exec: bool) -> Result<()> {
    Ok(())
}

fn blob_of(e: &Entry) -> Option<&str> {
    match e {
        Entry::File { blob, .. } => Some(blob),
        Entry::Dir => None,
    }
}

fn unchanged(merged: &Entries, remote: &Entries) -> bool {
    merged.len() == remote.len() && merged.iter().all(|(p, e)| same(Some(e), remote.get(p)))
}

fn diff_remote(merged: &Entries, remote: &Entries, report: &mut Report) {
    for (p, e) in merged {
        if !same(Some(e), remote.get(p)) {
            report.uploaded.push(p.clone());
        }
    }
    for p in remote.keys() {
        if !merged.contains_key(p) {
            report.removed_remote.push(p.clone());
        }
    }
}

/// Writes the files of a snapshot (or the part of it under `prefix`) into `dest`, which must
/// not exist yet or be empty. Used for restoring versions (VER-4).
pub fn restore(
    hub: &Hub,
    root_id: &str,
    seq: u64,
    prefix: Option<&str>,
    dest: &Path,
) -> Result<usize> {
    if dest.exists() && fs::read_dir(dest)?.next().is_some() {
        bail!("{} is not empty; restore into a new folder", dest.display());
    }
    let snap = hub.get_snapshot(root_id, seq)?;
    let prefix = prefix.map(|p| p.trim_matches('/'));
    let mut count = 0;
    fs::create_dir_all(dest)?;
    for (path, entry) in &snap.entries {
        if let Some(pre) = prefix {
            if path != pre && !path.starts_with(&format!("{pre}/")) {
                continue;
            }
        }
        let target = dest.join(path);
        match entry {
            Entry::Dir => fs::create_dir_all(&target)?,
            Entry::File {
                blob,
                mtime_ns,
                exec,
                ..
            } => {
                fs::create_dir_all(target.parent().unwrap())?;
                util::write_atomic(&target, &hub.get_blob(blob)?)?;
                set_exec(&target, *exec)?;
                let ft = filetime::FileTime::from_unix_time(
                    mtime_ns.div_euclid(1_000_000_000),
                    mtime_ns.rem_euclid(1_000_000_000) as u32,
                );
                filetime::set_file_mtime(&target, ft)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_names() {
        let n = conflict_name("a/report.pdf", "lap top");
        assert!(n.starts_with("a/report.sync-conflict-lap_top-"), "{n}");
        assert!(n.ends_with(".pdf"));
        assert!(conflict_name(".bashrc", "d").starts_with(".bashrc.sync-conflict-d-"));
    }
}
