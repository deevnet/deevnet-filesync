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
| 1 | Encrypted hub on Linux | One Linux device round-trips encrypted roots to S3 | ⏳ |
| 2 | Multi-device sync on Linux | Two or more Linux devices stay in sync unattended | ⏳ |
| 3 | Data safety and v1.0 | Versions, trash, retention, restore. **Linux MVP release** | ⏳ |
| 4 | Windows | Windows client and tray app | ⏳ |
| 5 | macOS | macOS client and tray app | ⏳ |
| 6 | Android | Android app | ⏳ |
| — | Backlog | Later items, not yet assigned to a phase | ⏳ |

Platform order is deliberate: **Linux → Windows → macOS → Android.** The Rust core stays
cross-platform from Phase 1 onwards, so each new platform adds a shell around the core, not a
rewrite.

## Phase 0: Foundations

Settle what to build and how, before writing code.

- Requirements: close the open questions (metadata, Android storage, key design).
- Architecture: components, how they fit, and the core / platform-shell boundary.
- **Hub data format:** the on-hub layout, object naming, encryption envelope, and how devices
  coordinate through plain object storage. This is documented as a specification (IND-2).
- **Identity and key design:** the PKI-based device identity, enrollment, revocation, and
  recovery (SEC-3, SEC-5, SEC-7, SEC-9..11).
- **Metadata deep dive:** a decision per metadata type and per platform (META-*).
- **Backend interface:** the narrow storage contract (BE-2), and confirmation that AWS S3 meets
  its concurrency needs.
- **Dependency policy:** what counts as a "primitive", and the initial list of crates (IND-1).
- Repo layout, Rust workspace, CI (build, lint, test, secret scan), docs publishing. *(The docs
  publishing piece is done.)*

**Exit:** the design documents are reviewed, and the Rust workspace builds in CI.

## Phase 1: Encrypted hub on Linux

Prove the core: encryption, the hub format, and the backend boundary, with one device.

- Rust core: local state database, filesystem scanner, hashing, and the encryption envelope.
- AWS S3 backend behind the backend interface (BE-1, BE-2).
- User CA and device identity, enough for one device (SEC-9).
- Sync roots and mounts at arbitrary paths (ROOT-1, ROOT-2, ROOT-3, ROOT-5).
- Ignore rules with built-in defaults (IGN-1, IGN-2).
- Size limits and workload guardrails (WKL-2).
- CLI: init, mount and unmount a root, status, push now, pull now (CLI-1, SYNC-4, SYNC-5).
- Resumable, crash-safe transfers (SYNC-7, SYNC-8, REL-2).
- Metadata handling as decided in Phase 0 (META-1, META-2, META-6).

**Exit:** a root pushed from one Linux device restores byte-for-byte into a fresh location, and
the hub reveals no file names, contents, or structure.

## Phase 2: Multi-device sync on Linux

Turn it into sync.

- Enroll a second device from an existing one; revoke a device (SEC-5, SEC-10).
- Background agent as a systemd user service (CLI-2).
- Filesystem watching plus periodic rescan (PERF-1). Changes propagate within minutes (SYNC-3).
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
- Garbage collection of expired data (VER-6).
- Verify / scrub of local state against the hub (REL-3).
- Recovery from the user's offline root when every device is lost (SEC-11), exercised end to end.
- The hub format specification is finalized for v1.0 (IND-2).

**Exit:** a recovery drill (every device "lost", rebuilt from the offline root plus the hub)
succeeds. **v1.0 is released for Linux.**

## Phase 4: Windows

- Windows service and client built on the shared core (PLT-2).
- Cross-platform naming: reserved names and characters, case-insensitivity, Unicode
  normalization (META-6).
- System tray app on Windows, and on Linux as well (UI-1).

## Phase 5: macOS

- macOS agent (launchd) and client (PLT-3).
- macOS-specific metadata decisions applied (META-5).
- Tray / menu bar app (UI-1).

## Phase 6: Android

- Android app built on the shared core via FFI (PLT-4, UI-2).
- Storage model as decided for AND-2.
- Sync only on Wi-Fi and/or while charging (AND-1).

## Backlog

Not yet assigned to a phase.

- Point-in-time recovery of a root or folder (VER-5).
- Per-file-type conflict strategies (CONF-3).
- Subfolder-level selective sync (ROOT-4).
- Data migration process between backends (BE-6).
- Self-hosted backends (BE-3), and other S3-compatible providers.
- Content-defined chunking / deduplication (PERF-3).
- Key rotation and re-encryption after revocation (SEC-6).
- LAN device-to-device sync (SYNC-9).
- Consumer cloud drives as dumb storage (BE-5).
- iOS / iPadOS (PLT-5).
