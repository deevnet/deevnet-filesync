---
title: "Roadmap"
weight: 20
bookCollapseSection: false
---

# Roadmap

**Status:** Draft. The phases follow from the [Requirements](../requirements/). Requirement IDs
in each phase trace back to that page.

**Legend:** ✅ Complete | 🔄 In Progress | ⏳ Planned

| Phase | Name | Outcome | Status |
|---|---|---|---|
| 0 | Foundations | Requirements and design are settled; the repo is ready for code | 🔄 |
| P | Prototype | A working end-to-end prototype on Linux against S3-compatible storage, to inform the Phase 0 design | 🔄 |
| 1 | Encrypted hub on Linux | One Linux device round-trips encrypted roots to S3 | ⏳ |
| 2 | Multi-device sync on Linux | Two or more Linux devices stay in sync unattended | ⏳ |
| 3 | Data safety and v1.0 | Versions, trash, retention, restore. **Linux MVP release** | ⏳ |
| 4 | Windows | Windows client and tray app | ⏳ |
| 5 | macOS | macOS client and tray app | ⏳ |
| 6 | Android | Read-only Android app | ⏳ |
| — | Backlog | Later items, not yet assigned to a phase | ⏳ |

Platform order is deliberate: **Linux → Windows → macOS → Android.** The Rust core stays
cross-platform from Phase 1 onwards, so each new platform adds a shell around the core, not a
rewrite.

## Phase 0: Foundations

Settle what to build and how, before writing code.

- Requirements: close the open questions (key design, hub access).
- Architecture: components, how they fit, and the core / platform-shell boundary.
- **Hub data format:** the on-hub layout, object naming, encryption envelope, and how devices
  coordinate through plain object storage. This is documented as a specification (IND-2).
- **Identity and key design:** the PKI-based device identity, enrollment, revocation, and
  recovery (SEC-3, SEC-5, SEC-7, SEC-9..11).
- **Metadata deep dive:** a decision per metadata type and per platform (META-*). *(Done; see the
  Metadata Matrix.)*
- **Backend interface:** the narrow storage contract (BE-2), and confirmation that AWS S3 meets
  its concurrency needs (HUB-4).
- **Hub access model:** device credentials and protection against a compromised device
  (HUB-6, HUB-7).
- **Dependency policy:** what counts as a "primitive", and the initial list of crates (IND-1).
- Repo layout, Rust workspace, CI (build, lint, test, secret scan), docs publishing. *(The docs
  publishing piece is done.)*

**Exit:** the design documents are reviewed, and the Rust workspace builds in CI.

## Phase P: Prototype

Runs alongside Phase 0. Build the simplest thing that syncs, to learn what the design documents
need to say. Prototype code may be reworked or replaced in Phase 1.

- Rust workspace: a core library and a CLI.
- Backend interface (BE-2) with two implementations: a local directory (for tests) and an
  S3-compatible client, tested against MinIO, including Deevnet's MinIO.
- End-to-end encryption of contents, names, and structure (SEC-1, SEC-2), with a single shared
  hub key. **Prototype shortcut:** no PKI; the key file is copied between devices by hand.
- Coordination through conditional writes (HUB-4): each change to a root is a new, numbered,
  encrypted snapshot, created only if that number is still free.
- Three-way sync with conflict copies (CONF-1, CONF-4), receive-only mounts (ROOT-9), portable
  name checks (META-6), and a guard against mass deletion.
- CLI: init, roots, mounts, status, sync (push), pull, history, restore (CLI-1, SYNC-4, SYNC-5,
  VER-4).

**Exit:** two devices (two config directories on one machine, then two machines) stay in sync
through MinIO, and the findings are written up for the Phase 0 design.

## Phase 1: Encrypted hub on Linux

Prove the core: encryption, the hub format, and the backend boundary, with one device.

- Rust core: local state database, filesystem scanner, hashing, and the encryption envelope.
- AWS S3 backend behind the backend interface (BE-1, BE-2, BE-4), with a versioned hub format
  (HUB-1, HUB-2) and scriptable hub setup (HUB-9).
- Linux platform basics: filesystems, key protection, packaging (LNX-1..4, LNX-9..11).
- User CA and device identity, enough for one device (SEC-9).
- Sync roots and mounts at arbitrary paths, with no client files inside roots, no overlapping
  roots, and warnings for roots inside other sync products (ROOT-1..3, ROOT-5..8).
- Ignore rules with built-in defaults (IGN-1, IGN-2).
- Size limits and workload guardrails (WKL-2).
- CLI: init, mount and unmount a root, status, push now, pull now (CLI-1, SYNC-4, SYNC-5).
- Resumable, crash-safe transfers (SYNC-7, SYNC-8, REL-2).
- Metadata handling as decided in the Metadata Matrix (META-1..6), with the META-7 field
  reserved in the format.

**Exit:** a root pushed from one Linux device restores byte-for-byte into a fresh location, and
the hub reveals no file names, contents, or structure.

## Phase 2: Multi-device sync on Linux

Turn it into sync.

- Enroll a second device from an existing one; revoke a device (SEC-5, SEC-10).
- Receive-only mounts with read-only hub credentials (ROOT-9).
- Background agent as a systemd user service, controlled by the CLI (CLI-2, CLI-3, LNX-5, LNX-6).
- Filesystem watching plus periodic rescan (PERF-1, LNX-7, LNX-8). Changes propagate within
  minutes (SYNC-3).
- Conditional-write coordination and per-device hub credentials (HUB-4, HUB-6).
- Offline queueing (SYNC-6).
- Conflict detection and conflict copies; conflicts shown in status (CONF-1, CONF-2, CONF-4).
- Cost-aware polling of the hub (PERF-2).
- Hub integrity checks (SEC-4).

**Exit:** two Linux devices edit the same roots for a few weeks of daily use with no data loss,
and every conflict produces a conflict copy.

## Phase 3: Data safety and v1.0 (Linux MVP)

Make it safe to trust with real files.

- Version history and per-root retention policies (VER-1, VER-2).
- Trash for deletions (VER-3).
- List and restore file versions (VER-4).
- Garbage collection of expired data, using the privileged credential (VER-6, HUB-7).
- Verify / scrub of local state against the hub (REL-3).
- Recovery from the user's offline root when every device is lost (SEC-11), exercised end to end.
- The hub format specification is finalized for v1.0 (IND-2).

**Exit:** a recovery drill (every device "lost", rebuilt from the offline root plus the hub)
succeeds. **v1.0 is released for Linux.**

## Phase 4: Windows

- Windows agent and client built on the shared core (PLT-2, WIN-*).
- Cross-platform naming: reserved names and characters, case-insensitivity, Unicode
  normalization (META-6).
- System tray app on Windows, and on Linux as well (UI-1, LNX-12, LNX-13).

## Phase 5: macOS

- macOS agent (launchd) and client (PLT-3, MAC-*).
- macOS-specific metadata decisions applied (META-5).
- Tray / menu bar app (UI-1).

## Phase 6: Android

- Read-only Android app built on the shared core via FFI (PLT-4, UI-2, AND-10, AND-*).
- App storage exposed to other apps through a read-only document provider (AND-2).
- Pulls only on Wi-Fi and/or while charging (AND-1).

## Backlog

Not yet assigned to a phase.

- Point-in-time recovery of a root or folder (VER-5).
- "Came from the internet" flag (META-7).
- Per-file-type conflict strategies (CONF-3).
- Subfolder-level selective sync (ROOT-4).
- Data migration process between backends (BE-6).
- Self-hosted backends (BE-3), and other S3-compatible providers.
- Content-defined chunking / deduplication (PERF-3).
- Key rotation and re-encryption after revocation (SEC-6).
- LAN device-to-device sync (SYNC-9).
- Consumer cloud drives as dumb storage (BE-5).
- iOS / iPadOS (PLT-5).
