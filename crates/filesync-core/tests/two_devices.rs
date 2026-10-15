//! Two devices syncing one root through a directory hub.

use std::fs;
use std::path::{Path, PathBuf};

use filesync_core::backend::DirBackend;
use filesync_core::crypto::Keys;
use filesync_core::engine::{self, MountState, Options, Report};
use filesync_core::hub::Hub;

const MASTER: [u8; 32] = [42u8; 32];

struct Device {
    name: &'static str,
    dir: PathBuf,
    state: MountState,
    push: bool,
}

struct World {
    _tmp: tempfile::TempDir,
    hub_dir: PathBuf,
    root: String,
}

impl World {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let hub_dir = tmp.path().join("hub");
        let hub = hub_at(&hub_dir);
        hub.init().unwrap();
        let root = hub.create_root("Finance").unwrap();
        World {
            hub_dir,
            root,
            _tmp: tmp,
        }
    }

    fn device(&self, name: &'static str) -> Device {
        let dir = self.hub_dir.parent().unwrap().join(name);
        fs::create_dir_all(&dir).unwrap();
        Device {
            name,
            dir,
            state: MountState::default(),
            push: true,
        }
    }

    fn sync(&self, d: &mut Device) -> Report {
        self.try_sync(d, false).unwrap()
    }

    fn try_sync(&self, d: &mut Device, allow_mass_delete: bool) -> anyhow::Result<Report> {
        let opt = Options {
            device: d.name.into(),
            push: d.push,
            dry_run: false,
            allow_mass_delete,
            max_file_bytes: 10 * 1024 * 1024,
        };
        engine::sync(
            &hub_at(&self.hub_dir),
            &self.root,
            &d.dir,
            &mut d.state,
            &opt,
        )
    }
}

fn hub_at(dir: &Path) -> Hub {
    Hub::new(
        Box::new(DirBackend::new(dir).unwrap()),
        Keys::from_master(&MASTER),
    )
}

fn write(d: &Device, rel: &str, content: &str) {
    let p = d.dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

fn read(d: &Device, rel: &str) -> Option<String> {
    fs::read_to_string(d.dir.join(rel)).ok()
}

fn files(d: &Device) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.push(p.strip_prefix(base).unwrap().to_string_lossy().into());
            }
        }
    }
    walk(&d.dir, &d.dir, &mut out);
    out.sort();
    out
}

/// Distinct mtimes, so the scan's (size, mtime) index notices same-size rewrites.
fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(20));
}

#[test]
fn changes_propagate_both_ways() {
    let w = World::new();
    let (mut a, mut b) = (w.device("laptop"), w.device("desktop"));

    write(&a, "taxes/2025.txt", "draft");
    write(&a, "budget.csv", "1,2,3");
    fs::create_dir_all(a.dir.join("empty")).unwrap();
    assert_eq!(w.sync(&mut a).committed_seq, Some(0));

    w.sync(&mut b);
    assert_eq!(read(&b, "taxes/2025.txt").as_deref(), Some("draft"));
    assert!(
        b.dir.join("empty").is_dir(),
        "empty directories sync (META-2)"
    );

    tick();
    write(&b, "taxes/2025.txt", "final");
    fs::remove_file(b.dir.join("budget.csv")).unwrap();
    assert_eq!(w.sync(&mut b).committed_seq, Some(1));

    let r = w.sync(&mut a);
    assert_eq!(read(&a, "taxes/2025.txt").as_deref(), Some("final"));
    assert_eq!(read(&a, "budget.csv"), None);
    assert_eq!(r.removed_local, vec!["budget.csv"]);

    // Nothing left to do on either side.
    assert_eq!(w.sync(&mut a).committed_seq, None);
    assert_eq!(w.sync(&mut b).committed_seq, None);
}

#[test]
fn concurrent_edits_keep_both_versions() {
    let w = World::new();
    let (mut a, mut b) = (w.device("laptop"), w.device("desktop"));
    write(&a, "notes.txt", "v1");
    w.sync(&mut a);
    w.sync(&mut b);

    tick();
    write(&a, "notes.txt", "laptop edit");
    write(&b, "notes.txt", "desktop edit");
    w.sync(&mut a);
    let r = w.sync(&mut b);
    assert_eq!(r.conflicts.len(), 1, "{r:?}");
    let copy = &r.conflicts[0];
    assert!(copy.starts_with("notes.sync-conflict-desktop-") && copy.ends_with(".txt"));
    assert_eq!(read(&b, "notes.txt").as_deref(), Some("laptop edit"));
    assert_eq!(read(&b, copy).as_deref(), Some("desktop edit"));

    // The conflict copy reaches the other device too: nothing is lost anywhere (CONF-1).
    w.sync(&mut a);
    assert_eq!(read(&a, copy).as_deref(), Some("desktop edit"));
    assert_eq!(files(&a), files(&b));
}

#[test]
fn edit_beats_delete() {
    let w = World::new();
    let (mut a, mut b) = (w.device("laptop"), w.device("desktop"));
    write(&a, "plan.md", "v1");
    w.sync(&mut a);
    w.sync(&mut b);

    tick();
    fs::remove_file(a.dir.join("plan.md")).unwrap();
    write(&a, "keep.md", "so the folder is not empty");
    write(&b, "plan.md", "v2");
    w.sync(&mut a);
    w.sync(&mut b);
    w.sync(&mut a);
    assert_eq!(
        read(&a, "plan.md").as_deref(),
        Some("v2"),
        "the edit wins (CONF-4)"
    );
    assert_eq!(read(&b, "plan.md").as_deref(), Some("v2"));
}

#[test]
fn empty_folder_does_not_wipe_the_hub() {
    let w = World::new();
    let mut a = w.device("laptop");
    write(&a, "a.txt", "1");
    write(&a, "b.txt", "2");
    w.sync(&mut a);

    fs::remove_file(a.dir.join("a.txt")).unwrap();
    fs::remove_file(a.dir.join("b.txt")).unwrap();
    assert!(
        w.try_sync(&mut a, false).is_err(),
        "mass delete must be refused"
    );
    assert_eq!(w.try_sync(&mut a, true).unwrap().removed_remote.len(), 2);
}

#[test]
fn receive_only_never_uploads() {
    let w = World::new();
    let (mut a, mut phone) = (w.device("laptop"), w.device("phone"));
    phone.push = false;
    write(&a, "doc.txt", "from laptop");
    w.sync(&mut a);
    w.sync(&mut phone);
    assert_eq!(read(&phone, "doc.txt").as_deref(), Some("from laptop"));

    write(&phone, "local-only.txt", "stays here");
    let r = w.sync(&mut phone);
    assert_eq!(r.committed_seq, None);
    assert_eq!(r.pending, vec!["added    local-only.txt"]);
    w.sync(&mut a);
    assert_eq!(read(&a, "local-only.txt"), None);
}

#[test]
fn non_portable_names_are_skipped_not_uploaded() {
    let w = World::new();
    let (mut a, mut b) = (w.device("laptop"), w.device("desktop"));
    write(&a, "ok.txt", "fine");
    write(&a, "Q3: results.txt", "colon");
    write(&a, "Report.txt", "upper");
    write(&a, "report.txt", "lower");
    let r = w.sync(&mut a);
    assert_eq!(r.warnings.len(), 2, "{:?}", r.warnings);
    w.sync(&mut b);
    assert_eq!(files(&b), vec!["Report.txt", "ok.txt"]);
}

#[test]
fn history_and_restore() {
    let w = World::new();
    let mut a = w.device("laptop");
    write(&a, "letter.txt", "first");
    w.sync(&mut a);
    tick();
    write(&a, "letter.txt", "second");
    w.sync(&mut a);

    let hub = hub_at(&w.hub_dir);
    assert_eq!(hub.snapshot_seqs(&w.root, None).unwrap(), vec![0, 1]);
    let dest = w.hub_dir.parent().unwrap().join("restored");
    engine::restore(&hub, &w.root, 0, Some("letter.txt"), &dest).unwrap();
    assert_eq!(
        fs::read_to_string(dest.join("letter.txt")).unwrap(),
        "first"
    );
}

#[test]
fn racing_writers_both_land() {
    // Both devices change the root from the same base. B picks up A's snapshot before
    // claiming the next number, so both changes land.
    let w = World::new();
    let (mut a, mut b) = (w.device("laptop"), w.device("desktop"));
    write(&a, "seed.txt", "x");
    w.sync(&mut a);
    w.sync(&mut b);
    write(&a, "from-a.txt", "a");
    write(&b, "from-b.txt", "b");
    assert_eq!(w.sync(&mut a).committed_seq, Some(1));
    assert_eq!(w.sync(&mut b).committed_seq, Some(2));
    w.sync(&mut a);
    assert_eq!(files(&a), vec!["from-a.txt", "from-b.txt", "seed.txt"]);
    assert_eq!(files(&a), files(&b));
}

#[test]
fn hub_holds_no_plaintext() {
    let w = World::new();
    let mut a = w.device("laptop");
    write(&a, "secret-name.txt", "secret-content");
    w.sync(&mut a);
    let mut stack = vec![w.hub_dir.clone()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            let s = p.to_string_lossy().to_string();
            assert!(!s.contains("secret") && !s.contains("Finance"), "{s}");
            if p.is_dir() {
                stack.push(p);
            } else {
                let data = fs::read(&p).unwrap();
                let text = String::from_utf8_lossy(&data);
                assert!(!text.contains("secret") && !text.contains("Finance"), "{s}");
            }
        }
    }
}

#[test]
fn a_snapshot_number_has_exactly_one_writer() {
    use filesync_core::manifest::{Entries, Snapshot};
    let w = World::new();
    let hub = hub_at(&w.hub_dir);
    let snap = |device: &str| Snapshot {
        seq: 0,
        parent: None,
        device: device.into(),
        created_unix: 0,
        entries: Entries::new(),
    };
    assert!(hub.put_snapshot(&w.root, &snap("laptop")).unwrap());
    assert!(
        !hub.put_snapshot(&w.root, &snap("desktop")).unwrap(),
        "second claim must lose"
    );
    assert_eq!(hub.get_snapshot(&w.root, 0).unwrap().device, "laptop");
}
