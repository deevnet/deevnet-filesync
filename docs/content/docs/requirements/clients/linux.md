---
title: "Linux"
weight: 10
---

# Linux Client

**Status:** Draft. **Phase:** 1–3 (MVP). Linux is the first platform and the reference platform for
the sync engine. Common client requirements are on the [Clients](../) page.

## Distributions and filesystems
- **LNX-1 (MVP, proposed):** Fedora (the current release) is the primary, tested distribution.
  Other distributions are expected to work but are not tested in the MVP.
- **LNX-2 (MVP):** Roots on ext4, btrfs, and xfs are supported.
- **LNX-3 (MVP, proposed):** Roots on network filesystems (NFS, SMB) are allowed, but get rescan
  only: inotify does not report changes made by other machines. The client says so when the
  root is mounted.
- **LNX-4 (MVP, proposed):** Works with SELinux in enforcing mode, without policy changes.

## Agent
- **LNX-5 (MVP):** The background agent runs as a **systemd user service**, starting at login.
- **LNX-6 (MVP, proposed):** Optional "keep running while logged out", via systemd lingering,
  for always-on machines.

## Change detection
- **LNX-7 (MVP):** inotify for live changes, plus periodic full rescans as a safety net (PERF-1).
- **LNX-8 (MVP, proposed):** When the inotify watch limit (`max_user_watches`) is too low for a
  root, the client says so, explains the fix, and falls back to rescans. It never silently misses
  changes.

## Keys and secrets
- **LNX-9 (MVP, proposed):** The device private key is protected by one of: the Secret Service
  (GNOME Keyring / KWallet), a TPM 2.0, or a passphrase-encrypted key file for headless machines.

## Packaging
- **LNX-10 (MVP, proposed):** Distributed as an RPM for Fedora, plus a standalone binary tarball.
- **LNX-11 (Non-goal, proposed):** Flatpak and Snap. Their sandboxes restrict filesystem access,
  which conflicts with "any folder can be a root".

## Desktop integration (Phase 4)
- **LNX-12 (Phase 4):** The tray app uses the StatusNotifierItem standard. On GNOME this needs
  the AppIndicator extension, which should be documented.
- **LNX-13 (Phase 4):** Notifications use the freedesktop notification standard.

## Open Questions
1. **Other distributions:** is Fedora-only testing right for the MVP, or does Debian/Ubuntu also
   matter early?
2. **Headless/server machines:** should a machine without a desktop session (e.g. a Deevnet VM)
   be a supported device?
