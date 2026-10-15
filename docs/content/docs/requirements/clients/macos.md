---
title: "macOS"
weight: 30
---

# macOS Client

**Status:** Draft. **Phase:** 5. Common client requirements are on the [Clients](../) page.

## Versions and hardware
- **MAC-1 (Phase 5, proposed):** The two most recent major macOS releases, on Apple Silicon. Intel
  Macs are not supported.
- **MAC-2 (Phase 5):** Roots on APFS. Both the default case-insensitive variant and the
  case-sensitive variant are supported.

## Agent
- **MAC-3 (Phase 5):** The agent runs as a per-user **launchd agent**, started at login.

## Change detection
- **MAC-4 (Phase 5):** FSEvents for live changes, plus periodic rescans.

## Privacy permissions
- **MAC-5 (Phase 5):** macOS requires the user's explicit consent before an app can read
  Desktop, Documents, Downloads, iCloud Drive, or removable and network volumes. The client
  asks for access when a root is mounted in one of these places, explains why, and reports
  clearly when access is denied rather than treating the folder as empty. *An empty-looking root
  must never be synced as "everything was deleted".*

## Overlap with iCloud
- **MAC-6 (Phase 5):** With "Desktop & Documents Folders" in iCloud Drive, those folders are
  managed by iCloud, and "Optimize Mac Storage" can replace files with placeholders. The client
  detects a root inside iCloud-managed storage and warns (ROOT-8). It skips placeholders rather
  than triggering downloads.

## File system behavior
- **MAC-7 (Phase 5):** Names follow the [Metadata Matrix](../metadata/) name policy. In
  particular, NFC and NFD forms of the same name are the same file on APFS.
- **MAC-8 (Phase 5):** Package/bundle directories are synced as directories (see the Metadata
  Matrix).

## Keys and secrets
- **MAC-9 (Phase 5, proposed):** The device private key is held in the Secure Enclave where the
  key algorithm allows it (P-256 only), or in the Keychain otherwise.

## Distribution and UI
- **MAC-10 (Phase 5):** A menu bar app with status, push now, pull now, and notifications (UI-1).

## Open Questions
1. **Signing and notarization:** distributing a macOS app without Gatekeeper warnings needs a
   paid Apple Developer membership and Apple's notarization service. That is a dependency on
   Apple. For a personal tool, is building locally (or approving an unsigned app once) acceptable
   instead?
