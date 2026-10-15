---
title: "Requirements"
weight: 10
bookCollapseSection: false
---

# Requirements

**Status:** Draft. Requirements gathering in progress.

This page captures what Deevnet FileSync must do. Requirements carry a short ID (e.g. `SYNC-3`) so
the project plan can trace phases back to them, and a priority:

- **MVP**: required for the first usable release.
- **Later**: planned, but not in the first release.
- **Non-goal**: deliberately out of scope.

## Vision

Deevnet FileSync extends the *personal computer* across all of one person's devices. A person's
working files (tax returns, spreadsheets, documents) are present, current, and usable on every
device they own, even offline. Storage is rented from a cloud provider the user picks, and that
provider never sees file contents, file names, or the user's identity.

## Guiding Principles

### Local first
- The local copy is the primary copy. Every device holds real files on a real filesystem.
- All read and write operations work offline. Sync is something that happens *afterwards*.
- The cloud is a rendezvous point and durable store, not the system of record for "what is true".

### Independence and control
- The user owns their files, their keys, and their identity. No account with this project, and
  no identity tied to the storage provider beyond a storage credential.
- **Building blocks, not solutions.** External software may be used only as *primitives*:
  crypto libraries, an S3 client, filesystem-watch APIs, hashing, compression, the Hugo docs
  toolchain. No third-party sync engines, sync frameworks, or products whose direction we would
  have to track.
- Storage backends sit behind a small, deliberately narrow interface so that a provider can
  be replaced, and so that self-hosted storage can be added later.

### No forced layout
- No mandatory "sync folder". Any directory on a device can be a sync root, and the same logical
  root can live at a different path on each device.

### Simple first, optimize later
- Prefer the simplest design that satisfies the principles: cloud as hub, full copies, simple
  conflict handling. Optimizations (LAN peer sync, smarter merges) come later.

## Scope

### Users
- **USR-1 (MVP):** One user. The system is an extension of a single person's personal computer.
- **USR-2 (Non-goal):** Multiple users, households, or families.
- **USR-3 (Non-goal):** Sharing files or folders with other people; public or share links.

### Platforms
- **PLT-1 (MVP):** Linux.
- **PLT-2 (MVP?):** Windows. *(See [Open Questions](#open-questions): phasing.)*
- **PLT-3 (MVP?):** macOS.
- **PLT-4 (Later):** Android.
- **PLT-5 (Later, maybe):** iOS / iPadOS. The design should not rule this out.

### Workload
- Intended for "normal user files": documents, spreadsheets, PDFs, tax software data files,
  configuration files, and so on.
- Images (e.g. `.jpg`) are allowed, but they are the exception, not the rule.
- **WKL-1 (Non-goal):** Media libraries (music, video, large photo collections). Those belong in
  a different, purpose-built solution.
- **WKL-2 (MVP):** Configurable limits, with sensible defaults, on individual file size and on
  total root size. The client warns about or refuses content that falls outside the intended
  workload.

## Concepts

- **Device:** A machine running a FileSync client, enrolled with the user's keys.
- **Sync root:** A logical, named collection of files (e.g. "Finance", "Documents"). It exists
  independently of any device.
- **Mount:** The binding of a sync root to a local path on a specific device. A device may mount
  some roots and not others.
- **Hub:** The cloud storage backend that all devices sync through.
- **Version:** A recorded state of a file at a point in time, kept according to the root's
  retention policy.

## Functional Requirements

### Sync roots and mounts
- **ROOT-1 (MVP):** A user can create any number of sync roots.
- **ROOT-2 (MVP):** A device can mount any root at any local path the user chooses. Paths differ
  freely between devices (e.g. `~/Documents/Taxes` on Linux, `D:\Finance\Taxes` on Windows).
- **ROOT-3 (MVP):** Each device chooses which roots it mounts (selective sync at the root level).
- **ROOT-4 (Later):** Selective sync of subfolders within a root.
- **ROOT-5 (MVP):** Unmounting a root on a device stops syncing it and leaves the local files in
  place. Local files are never deleted as a side effect of unmounting.

### Sync behavior
- **SYNC-1 (MVP):** Every mounting device keeps a full local copy of the root. No placeholder,
  virtual, or on-demand files.
- **SYNC-2 (MVP):** The cloud hub is the only sync path. Devices never talk to each other
  directly.
- **SYNC-3 (MVP):** A local change reaches the hub, and other online devices see it, within a few
  minutes under normal conditions.
- **SYNC-4 (MVP):** **Push now:** the user can force an immediate upload of pending changes from
  the originating device.
- **SYNC-5 (MVP):** **Pull now:** the user can force an immediate check for, and download of,
  remote changes on a device.
- **SYNC-6 (MVP):** Changes made while offline are queued and synced when connectivity returns.
- **SYNC-7 (MVP):** Files that are partially written or open for write are not uploaded mid-write.
  The client waits for the file to settle.
- **SYNC-8 (MVP):** Sync is resumable. An interrupted upload or download does not corrupt either
  side and resumes without starting over where practical.
- **SYNC-9 (Later):** Direct device-to-device sync over the LAN.

### Conflicts
- **CONF-1 (MVP):** When the same file has been changed on two devices since they last synced, no
  data is lost. Both versions are kept: one keeps the original name, the other is renamed to a
  conflict copy that identifies the device and time.
- **CONF-2 (MVP):** Conflicts are surfaced to the user (CLI status, tray notification).
- **CONF-3 (Later):** Conflict strategies per file type (e.g. merge for text, newest-wins for
  certain files).
- **CONF-4 (MVP):** Edit-versus-delete conflicts resolve in favor of keeping the data.

### Versioning, deletion, and recovery
- **VER-1 (MVP):** Previous versions of files are retained in the hub.
- **VER-2 (MVP):** Retention policy is set per sync root (e.g. keep 30 days, keep N versions,
  keep forever).
- **VER-3 (MVP):** Deleted files go to a recoverable trash area for the root's retention period.
- **VER-4 (MVP):** The user can list and restore previous versions of a single file.
- **VER-5 (Later):** Point-in-time recovery: restore an entire root, or a folder within it, as
  it existed at a given time.
- **VER-6 (MVP):** Space used by expired versions and trash is reclaimed by a maintenance
  (garbage collection) process.

### Ignore rules
- **IGN-1 (MVP):** Each root supports ignore patterns using gitignore-style syntax.
- **IGN-2 (MVP):** Built-in default ignores for OS and application junk (`.DS_Store`,
  `Thumbs.db`, `desktop.ini`, `~$*` Office lock files, editor swap files, etc.), which the user
  can override.

### File metadata
*Deep dive pending; see [Open Questions](#open-questions).*

- **META-1 (MVP):** File contents and modification time are preserved.
- **META-2 (MVP):** Empty directories are synced.
- **META-3 (TBD):** POSIX permissions / executable bit.
- **META-4 (TBD):** Symlinks.
- **META-5 (TBD):** Extended attributes, ACLs, macOS resource forks, Windows alternate data
  streams.
- **META-6 (MVP):** Names that are invalid or ambiguous on another platform (Windows reserved
  characters and names, case-only differences, Unicode normalization differences) are detected
  and handled predictably, never silently lost or overwritten.

### Backends
- **BE-1 (MVP):** An S3-compatible object storage backend (AWS S3 first; others such as Backblaze
  B2, Cloudflare R2, Wasabi, and MinIO are expected to work through the same interface).
- **BE-2 (MVP):** The backend interface is narrow and primitive (put, get, list, delete, plus
  whatever concurrency primitive the design needs) so that new backends are cheap to add.
- **BE-3 (Later):** Self-hosted storage (e.g. MinIO on Deevnet infrastructure, SFTP, a local or
  NAS path).
- **BE-4 (TBD):** Multiple backends at once (mirroring), and migrating between backends.
  *Discussion pending.*
- **BE-5 (Later):** Consumer cloud drives (Google Drive, OneDrive, Dropbox) as dumb storage.

### Clients and interfaces
- **CLI-1 (MVP):** A command-line interface that exposes every capability: setup, mounts, status,
  push now, pull now, conflicts, versions, restore.
- **CLI-2 (MVP):** A background sync agent (daemon/service) per desktop platform that runs
  without a user session where the OS allows it.
- **UI-1 (Later):** A system tray app on desktop platforms that shows status and offers push now,
  pull now, pause, and notifications.
- **UI-2 (Later):** An Android app.
- **AND-1 (Later):** Android sync runs only on Wi-Fi and/or while charging (user-configurable).
- **AND-2 (TBD):** How Android exposes sync roots on the filesystem (Storage Access Framework
  versus app-specific storage). *Discussion pending.*

## Non-Functional Requirements

### Security and privacy
- **SEC-1 (MVP):** End-to-end encryption. All data leaves the device encrypted. The backend
  never sees plaintext file contents.
- **SEC-2 (MVP):** File names, directory structure, file sizes (as far as practical), and root
  names are hidden from the backend.
- **SEC-3 (MVP):** Encryption keys are generated and held by the user's devices. Neither the
  storage provider nor any service run by this project ever has them.
- **SEC-4 (MVP):** Integrity: tampering with or corruption of hub data is detected, never
  silently accepted.
- **SEC-5 (MVP):** A lost or compromised device can be revoked so that it can no longer
  access future data.
- **SEC-6 (Later):** Key rotation and re-encryption after revocation.
- **SEC-7 (TBD):** Key hierarchy, device enrollment, and recovery when every device is lost.
  *Discussion pending.*
- **SEC-8 (MVP):** Only well-established, audited cryptographic primitives and libraries are
  used. No home-grown cryptography.

### Reliability and data safety
- **REL-1 (MVP):** The system never deletes or overwrites user data without a recoverable copy
  existing somewhere (version history, trash, or a conflict copy).
- **REL-2 (MVP):** A crash or power loss at any point leaves local state and hub state
  consistent, or recoverable to consistency.
- **REL-3 (MVP):** A verify or scrub operation can check local state against the hub.

### Independence
- **IND-1 (MVP):** Dependencies are limited to primitives (see Guiding Principles). Each
  significant dependency is documented with its purpose.
- **IND-2 (MVP):** The on-hub data format is documented in this repository, so data stays
  recoverable even without the FileSync software.
- **IND-3 (MVP):** No telemetry and no phone-home behavior.

### Performance and footprint
- **PERF-1 (MVP):** An idle client uses little CPU and battery. Change detection uses OS
  filesystem notifications where available, with periodic rescans as a safety net.
- **PERF-2 (MVP):** Cost-aware against metered cloud APIs: polling frequency and request counts
  are kept modest and configurable.
- **PERF-3 (Later):** Content-defined chunking / deduplication so that small edits to large files
  don't re-upload the whole file.

### Technology
- **TECH-1 (MVP):** A shared core in Rust (sync engine, crypto, backend adapters, local state),
  with thin platform-specific shells.
- **TECH-2 (MVP):** A monorepo holding all clients, the core, any server-side components, and
  these docs.
- **TECH-3:** License to be decided as part of a Deevnet-wide licensing decision.

### Documentation
- **DOC-1 (MVP):** A standalone Hugo site (`hugo-book` theme) in `docs/`, published via GitHub
  Pages.

## Open Questions

Topics that still need a deliberate discussion before they become requirements.

1. **Multiple backends / migration (BE-4):** Do we mirror to more than one hub? Migrate between
   providers? Treat a backend as a replaceable hub with a documented export/import?
2. **Key model and recovery (SEC-7):** Should there be one master key with per-device keys? How
   is a new device enrolled (via an existing device, or passphrase only)? What is the recovery
   path if every device is lost (printed recovery key, passphrase)?
3. **Metadata (META-3..6):** A deliberate, cross-platform decision for each kind of metadata.
4. **Android storage model (AND-2):** Storage Access Framework versus app-specific directories,
   and what that means for the "no forced layout" principle.
5. **Platform phasing:** Which desktop platforms are in the first release: Linux only, or
   Linux, Windows and macOS together?
6. **Concurrency on a dumb hub:** How devices safely coordinate through plain S3 (conditional
   writes / compare-and-swap support varies between S3-compatible providers). This is mostly a
   design question, but it may constrain which providers count as "S3-compatible enough".
