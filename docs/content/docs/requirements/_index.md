---
title: "Requirements"
weight: 10
bookCollapseSection: false
---

# Requirements

**Status:** Draft. Requirements gathering in progress.

This page holds the **core** requirements: the ones that apply everywhere. Detail lives on
sub-pages:

| Page | Covers |
|---|---|
| [Metadata Matrix](metadata/) | Names, timestamps, permissions, and file types across platforms |
| [Clients](clients/) | Requirements common to every client, plus one page per platform: [Linux](clients/linux/), [Windows](clients/windows/), [macOS](clients/macos/), [Android](clients/android/) |
| [Cloud Hub](hub/) | The storage backend: AWS S3 for the MVP, the backend interface, access control, cost |

Requirements carry a short ID (e.g. `SYNC-3`) so the [Roadmap](../roadmap/) can trace phases back
to them, and a priority:

- **MVP**: required for the first usable release (Linux).
- **Phase N**: required when that platform's phase ships.
- **Later**: planned, but not yet assigned to a phase.
- **Non-goal**: deliberately out of scope.

Items marked *proposed* were suggested during drafting and are awaiting confirmation.

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
Platforms arrive in this order: **Linux → Windows → macOS → Android** (see [Roadmap](../roadmap/)).

- **PLT-1 (MVP):** [Linux](clients/linux/).
- **PLT-2 (Phase 4):** [Windows](clients/windows/).
- **PLT-3 (Phase 5):** [macOS](clients/macos/).
- **PLT-4 (Phase 6):** [Android](clients/android/), as a **read-only** device: files never
  originate on Android (AND-10).
- **PLT-5 (Later, maybe):** iOS / iPadOS. The design should not rule this out.
- **PLT-6 (MVP):** The core is cross-platform from the start. Platform-specific code lives in
  thin shells, so a new platform does not need a rewrite.

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
- **ROOT-6 (MVP, proposed):** The client puts nothing of its own inside a mounted root: no hidden
  `.filesync` folders, no marker files. All configuration and state live outside the roots.
  (Conflict copies are user files, so they are the exception.)
- **ROOT-7 (MVP, proposed):** Roots on a device cannot be nested or overlap each other.
- **ROOT-8 (MVP, proposed):** The client warns when a root is inside a folder that another sync
  product manages (e.g. OneDrive, iCloud Drive, Dropbox), since two sync engines on the same
  files fight each other.
- **ROOT-9 (MVP, proposed):** A mount can be **receive-only**: the device downloads changes from
  the hub but never uploads. A device whose mounts are all receive-only gets a read-only hub
  credential (HUB-6). Android
  is always receive-only (AND-10); on desktops it is an option per mount.

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
Summary only. The per-platform capability matrix and the reasoning behind these requirements are on
the [Metadata Matrix](metadata/) page.

- **META-1 (MVP):** File contents (byte-exact) and modification time are preserved.
- **META-2 (MVP):** Empty directories are synced.
- **META-3 (MVP, proposed):** The executable bit and a read-only flag are synced and mapped per
  platform. Other permissions, ownership, and ACLs are not synced.
- **META-4 (MVP, proposed):** Symlinks, special files, and other products' cloud placeholders
  are skipped with a warning.
- **META-5 (MVP, proposed):** Extended attributes, alternate data streams, and resource forks are
  not synced in v1. The format leaves room for an allow-listed set later.
- **META-6 (MVP):** Names that are invalid or ambiguous on another platform (Windows reserved
  characters and names, case-only differences, Unicode normalization differences) are detected
  and handled predictably, never silently lost or overwritten.

### Storage backend
Summary only. Details are on the [Cloud Hub](hub/) page.

- **BE-0 (MVP):** Only one backend is active at a time. That one backend is the hub.
- **BE-1 (MVP):** AWS S3 is the MVP backend.
- **BE-4 (MVP):** The sync engine, crypto, and hub format do not depend on anything AWS-specific.

### Clients
Summary only. Details are on the [Clients](clients/) page and its per-platform pages.

- **CLI-1 (MVP):** A CLI exposes every capability.
- **CLI-2 (MVP):** A background sync agent runs on each desktop platform.
- **UI-1 (Phase 4):** A tray app on desktop platforms. **UI-2 (Phase 6):** A read-only Android app.

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
- **SEC-7 (MVP):** Identity is PKI-based and owned by the user. *Direction agreed; detailed
  design in Phase 0.*
- **SEC-9 (MVP):** Each device has its own key pair, generated on the device, and a certificate
  issued by a CA that the user controls. Devices prove who they are with certificates, not with a
  shared passphrase.
- **SEC-10 (MVP):** Enrolling a new device needs more than a passphrase. It needs approval from
  an existing device, or the user's offline CA.
- **SEC-11 (MVP):** Recovery when every device is lost uses the user's offline root (modeled on
  the Deevnet Root of Trust: passphrase-encrypted keys on offline media). The root can issue
  a new device certificate *and* recover access to the data keys.
- **SEC-12 (MVP):** FileSync has its **own** root CA, held offline by the user. It does not depend
  on the Deevnet PKI or any Deevnet infrastructure. *(It may later be made subordinate to another
  root, if the user wants a single trust anchor.)*
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
  are kept modest and configurable (see [Cloud Hub](hub/)).
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

Topics that still need a deliberate discussion. Detail lives on the page linked.

1. **Metadata:** the "Decisions needed" list on the [Metadata Matrix](metadata/).
2. **PKI design (SEC-7, SEC-9..12):**
   - How certificates bind to data encryption. Certificates prove identity and sign changes; the
     data keys are separately wrapped to each enrolled device's public key and to the offline
     recovery key.
   - Device revocation: a signed device list in the hub, a CRL, or both.
   - **Key algorithm versus hardware key storage.** To keep device keys in hardware (TPM 2.0,
     Apple Secure Enclave, Android Keystore), the algorithm must be one that all of them support.
     ECDSA P-256 is the common denominator; Ed25519 is not supported everywhere. This needs to be
     decided before any device key is generated.
3. **Hub access and concurrency:** on the [Cloud Hub](hub/) page.
4. **Per-platform questions:** on each [client platform page](clients/).
