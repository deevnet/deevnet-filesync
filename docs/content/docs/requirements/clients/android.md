---
title: "Android"
weight: 40
---

# Android Client

**Status:** Draft. **Phase:** 6. Common client requirements are on the [Clients](../) page.

The Android client is an app built on the same Rust core, called from Kotlin.

## Role: read-only access
- **AND-10 (Phase 6):** **Android is a read-only device** (a receive-only device, ROOT-9). It
  downloads and presents files from the hub; files never originate on, or are changed from,
  Android.

This removes most of what makes Android hard:
- no change detection on the phone, and so no reliance on background file watching;
- no conflicts originating on the phone;
- no uploads, so no "push now" on the phone;
- modification-time limitations don't matter, since the app keeps the true value from the hub;
- the phone's hub credential can be **read-only** (HUB-6). A lost phone can never modify or delete
  anything in the hub.

## Versions
- **AND-3 (Phase 6, proposed):** Android 13 and later.

## Storage and access
- **AND-2 (Phase 6, proposed):** Files are stored in the **app's own storage** and shown to the
  rest of the system through a **read-only document provider**. Synced roots then appear in the
  Android Files app and in every app's "open file" picker, as browsable folders, but other apps
  cannot change them.

  Being read-only, this avoids the trade-offs of the alternatives:
  - The Storage Access Framework (user-picked shared folders) is no longer needed. "No forced
    layout" exists to let *originating* devices choose where files live, and Android originates
    nothing.
  - The "all files access" permission is no longer needed.
- **AND-11 (Phase 6, proposed):** The phone chooses which roots it holds (ROOT-3), and keeps a
  full local copy of them, so they can be opened offline (SYNC-1).
- **AND-12 (Phase 6, proposed):** Opening a file hands a read-only copy to the viewing app (PDF
  viewer, spreadsheet app). If that app saves changes, it saves them as a new file elsewhere,
  never back into the synced root.

## Background sync
- **AND-1 (Phase 6):** Sync runs only on Wi-Fi and/or while charging, as the user configures.
- **AND-4 (Phase 6):** Background pulls are scheduled through Android's job scheduler
  (WorkManager). Its minimum period is about 15 minutes, and Android may defer it further.
- **AND-5 (Phase 6):** "Pull now" runs immediately while the app is open, with visible progress.
- **AND-6 (Phase 6, proposed):** The app does not ask to be exempted from battery optimization.

## Keys and secrets
- **AND-7 (Phase 6):** The device private key is generated and held in the Android Keystore,
  hardware-backed where the device supports it.

## UI
- **AND-8 (Phase 6):** The app shows the roots, their sync status and last-pulled time, and sync
  settings, and offers pull now. It lets the user browse and open files. It sends notifications
  for problems (e.g. hub unreachable, storage full).

## Distribution
- **AND-9 (Phase 6, open):** Sideloaded APK, F-Droid, or Google Play. *Note:* Google has announced
  that, from 2026, Android will require apps on certified devices to come from verified developers,
  even when sideloaded. 🔍 Check the status of that requirement before Phase 6, as it bears
  directly on the independence principle.

## Open Questions
1. **Full copy versus on demand (AND-11):** with a document provider, fetching a file only when it
   is opened is straightforward on Android. A full copy of the selected roots keeps the phone
   local-first and usable offline; on demand saves phone storage. *Lean: full copy, consistent
   with SYNC-1.*
2. **Distribution channel (AND-9).**
