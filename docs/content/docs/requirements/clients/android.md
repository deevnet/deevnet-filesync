---
title: "Android"
weight: 40
---

# Android Client

**Status:** Draft. **Phase:** 6. Common client requirements are on the [Clients](../) page.

The Android client is an app built on the same Rust core, called from Kotlin. Android shapes the
design more than the desktop platforms do: it restricts filesystem access, background work, and
app distribution.

## Versions
- **AND-3 (Phase 6, proposed):** Android 13 and later.

## Storage
- **AND-2 (Phase 6, open):** How sync roots appear on the device. Options:
  - **Storage Access Framework (SAF):** the user picks any folder (e.g. `Documents/Finance`) and
    grants the app lasting access to it. This honors "no forced layout". Access is through a
    slower, URI-based API with no change notifications, and setting modification times is
    unreliable (see the [Metadata Matrix](../metadata/)).
  - **App-specific storage:** fast and simple, but the files live in the app's own folder, other
    apps reach them only through share/open pickers, and they are deleted if the app is
    uninstalled.
  - **"All files access" permission:** normal file access everywhere, but Google Play allows it
    only for file-manager-type apps. It works only for apps installed outside Google Play.

  *Lean: SAF.*

## Background sync
- **AND-1 (Phase 6):** Sync runs only on Wi-Fi and/or while charging, as the user configures.
- **AND-4 (Phase 6):** Background sync is scheduled through Android's job scheduler (WorkManager).
  Its minimum period is about 15 minutes, and Android may defer it further. Changes are found by
  rescanning, not by live watching.
- **AND-5 (Phase 6):** "Push now" and "pull now" run immediately while the app is open, with
  visible progress.
- **AND-6 (Phase 6, proposed):** The app does not ask to be exempted from battery optimization.

## Keys and secrets
- **AND-7 (Phase 6):** The device private key is generated and held in the Android Keystore,
  hardware-backed where the device supports it.

## UI
- **AND-8 (Phase 6):** The app shows roots and their status, conflicts, and sync settings, and
  offers push now and pull now. It sends notifications for conflicts and problems.

## Distribution
- **AND-9 (Phase 6, open):** Sideloaded APK, F-Droid, or Google Play. *Note:* Google has announced
  that, from 2026, Android will require apps on certified devices to come from verified developers,
  even when sideloaded. 🔍 Check the status of that requirement before Phase 6, as it bears
  directly on the independence principle.

## Open Questions
1. **Storage model (AND-2).**
2. **Distribution channel (AND-9).**
3. **Camera roll:** out of scope under WKL-1 (media is a non-goal). Confirm.
