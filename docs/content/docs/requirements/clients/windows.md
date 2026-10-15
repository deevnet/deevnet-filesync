---
title: "Windows"
weight: 20
---

# Windows Client

**Status:** Draft. **Phase:** 4. Common client requirements are on the [Clients](../) page.

## Versions and filesystems
- **WIN-1 (Phase 4, proposed):** Windows 11. Windows 10 reached end of support in October 2025.
- **WIN-2 (Phase 4):** Roots on NTFS. ReFS, FAT32, and exFAT are not supported for roots.

## Agent
- **WIN-3 (Phase 4, proposed):** The agent runs **per user**, started at sign-in, not as a
  Windows Service. A service runs as a different account, which complicates access to the user's
  files and the user's key store.

## Change detection
- **WIN-4 (Phase 4):** `ReadDirectoryChangesW` for live changes, plus periodic rescans. The NTFS
  change journal (USN) is not used, because it needs administrator rights.

## File system behavior
- **WIN-5 (Phase 4):** Long paths (over 260 characters) are handled whether or not the system's
  long-path setting is enabled.
- **WIN-6 (Phase 4):** **Locked files.** Office and other applications hold files open with
  exclusive locks. The client reads with shared access, retries files it cannot read, and
  delays applying an incoming change to a locked file. It never fails the whole sync because of
  one locked file.
- **WIN-7 (Phase 4):** Incoming changes are written to a temporary file and swapped into place
  atomically, so applications never see a half-written file.
- **WIN-8 (Phase 4):** Names follow the [Metadata Matrix](../metadata/) name policy: reserved
  names, forbidden characters, trailing dots and spaces, and case-only collisions.
- **WIN-9 (Phase 4, proposed):** Tolerates antivirus scanning (Microsoft Defender) briefly
  locking newly written files.

## Overlap with OneDrive
- **WIN-10 (Phase 4):** Windows 11 often redirects Documents, Desktop, and Pictures into OneDrive
  ("Known Folder Move"). The client detects a root inside a OneDrive-managed folder and warns
  (ROOT-8).

## Keys and secrets
- **WIN-11 (Phase 4, proposed):** The device private key is held by the TPM (through the Windows
  platform crypto provider) where available, or else protected with DPAPI.

## Packaging and UI
- **WIN-12 (Phase 4, proposed):** A per-user installer that does not need administrator rights.
- **WIN-13 (Phase 4):** A notification-area (tray) icon and Windows toast notifications (UI-1).

## Open Questions
1. **Code signing:** unsigned binaries trigger SmartScreen warnings. Is a code-signing
   certificate worth buying for a personal tool, or are the warnings acceptable?
