---
title: "Clients"
weight: 20
bookCollapseSection: false
---

# Clients

**Status:** Draft. Requirements common to every client. Platform-specific requirements are on
each platform's page. Core requirements are on the [Requirements](../) page.

| Platform | Phase | Background agent | Change detection | Device key storage | UI |
|---|---|---|---|---|---|
| [Linux](linux/) | 1–3 (MVP) | systemd user service | inotify + rescan | Secret Service or TPM 2.0 | CLI; tray in Phase 4 |
| [Windows](windows/) | 4 | Per-user background process | `ReadDirectoryChangesW` + rescan | TPM 2.0 / DPAPI | CLI + tray |
| [macOS](macos/) | 5 | launchd user agent | FSEvents + rescan | Keychain / Secure Enclave | CLI + menu bar |
| [Android](android/) (read-only) | 6 | WorkManager scheduled pulls | Not needed: nothing originates on Android | Android Keystore | App |

## Interfaces
- **CLI-1 (MVP):** A command-line interface exposes every capability: setup, enrollment, mounts,
  status, push now, pull now, conflicts, versions, restore. On desktop platforms, every other UI
  is built on what the CLI can do.
- **CLI-2 (MVP):** A background sync agent runs on each desktop platform. It starts
  automatically and keeps running without a terminal open.
- **CLI-3 (MVP, proposed):** The CLI and the agent share one local control channel. "Push now" or
  "pull now" from the CLI acts on the running agent, never on a second, competing copy of the
  engine.
- **UI-1 (Phase 4):** A tray / menu bar app on desktop platforms: status, push now, pull now,
  pause, conflicts, and notifications. Linux gets it at the same time as Windows.
- **UI-2 (Phase 6):** A read-only Android app (AND-10).

## Common behavior
- **CLNT-1 (MVP, proposed):** Configuration and local state live in the platform's standard
  per-user locations, never inside a sync root (ROOT-6).
- **CLNT-2 (MVP, proposed):** The device private key is never stored in plaintext. It is held in
  the platform's key store or hardware where available, or else encrypted with a passphrase.
- **CLNT-3 (MVP, proposed):** Logs are local only, rotated, and never contain file contents or
  key material.
- **CLNT-4 (MVP, proposed):** No auto-update mechanism that phones home (IND-3). Updates come
  through the platform's package mechanism, or are installed manually.
- **CLNT-5 (MVP, proposed):** Status is always explainable: for any file, the client can say
  whether it is synced, pending, ignored, skipped (and why), or in conflict.
- **CLNT-6 (MVP, proposed):** Sync can be paused and resumed per device, and per root.
