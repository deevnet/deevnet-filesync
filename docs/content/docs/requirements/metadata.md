---
title: "Metadata Matrix"
weight: 10
---

# Metadata Matrix

**Status:** Draft, for discussion. This page backs requirements META-1 to META-6.

A file is more than its bytes. Each platform keeps a different set of names, timestamps,
permissions, and extra data alongside the bytes. This page sets out what each platform can
represent, so that we can choose, deliberately, for each kind of metadata:

- what FileSync **syncs**,
- what it **maps** to an approximation on platforms that can't represent it exactly,
- what stays **local-only** (never leaves the device), and
- what it **skips** (refuses to sync, and tells the user).

The hub itself is not a constraint. Metadata is stored inside the encrypted file records, so the
hub can carry whatever we decide. The limits come from the *receiving* platform.

**Legend:** ✅ supported | ⚠️ partial / with caveats | ❌ not supported | 🔍 to verify in Phase 0
testing

**Platforms assumed:** Linux on ext4/btrfs/xfs; Windows on NTFS; macOS on APFS (default
case-insensitive); Android shared storage (`/sdcard`, reached through the Storage Access Framework
or the FUSE layer). Removable media (FAT32/exFAT) are out of scope for sync roots.

## 1. File names and paths

| Property | Linux | Windows | macOS | Android (shared) |
|---|---|---|---|---|
| Name encoding | Arbitrary bytes (usually UTF-8; invalid UTF-8 is possible) | UTF-16 (unpaired surrogates possible) | UTF-8 | UTF-8 |
| Max name length | 255 bytes | 255 UTF-16 units | 255 UTF-8 chars | 255 bytes |
| Max path length | 4096 bytes | 260 chars by default; ~32k with long paths enabled | 1024 bytes | 4096 bytes 🔍 |
| Case sensitivity | Sensitive | Insensitive, preserving (per-directory opt-in for sensitive) | Insensitive, preserving (default volume) | Insensitive 🔍 |
| Unicode normalization | None: NFC and NFD are different names | None: different names | Preserving, but NFC/NFD of the same name collide | None 🔍 |
| Forbidden characters | `/`, NUL | `< > : " / \ \| ? *`, control chars 0–31 | `/`, NUL (`:` shows as `/` in Finder) | FAT-style set `" * : < > ? \ \|` through SAF 🔍 |
| Reserved names | None | `CON PRN AUX NUL COM1–9 LPT1–9`, with or without an extension | None | None |
| Trailing space or dot | Allowed | Stripped or refused | Allowed | Allowed 🔍 |

### What this means

Linux is the most permissive. Every other platform can be handed a name it cannot store:
- `Report.pdf` and `report.pdf` side by side: valid on Linux; they collide on Windows, macOS, and
  Android.
- `Q3: results.xlsx`: valid on Linux and macOS; invalid on Windows and Android.
- `café.txt` in NFC and in NFD: two files on Linux; one file on macOS.
- `aux.txt`: valid everywhere except Windows.

**Proposal: the "portable names" rule.** Every root has a name policy:
- **portable** *(default)*: names must be valid on all four platforms. A non-portable name is
  flagged as soon as it appears on any device, so it can be renamed while the user still
  remembers it. It is not uploaded until fixed.
- **permissive**: any name is accepted. On a device that can't represent a name, that one file is
  **not materialized there** and is shown as a sync issue. It is never renamed silently, never
  mangled, and never merged with its look-alike.

Either way, invalid UTF-8 names are refused at the source, and names are compared in a
normalized, case-folded form to detect collisions before they reach a device.

## 2. Timestamps

| Property | Linux | Windows | macOS | Android (shared) |
|---|---|---|---|---|
| Modification time (mtime) | ✅ ns, settable | ✅ 100 ns, settable | ✅ ns, settable | ⚠️ readable; **setting it is unreliable or impossible through SAF** 🔍 |
| Creation / birth time | ⚠️ readable via `statx` on most filesystems; **not settable** | ✅ settable | ✅ settable | ❌ |
| Change time (ctime) | ✅ not settable | ✅ not settable | ✅ not settable | ⚠️ |
| Access time (atime) | ⚠️ often disabled (`relatime`/`noatime`) | ⚠️ often disabled | ✅ | ⚠️ |

**Proposal:**
- **mtime: synced** (META-1), stored with nanosecond precision and applied at whatever precision
  the destination has.
- **birth time: local-only.** Linux can't set it, so syncing it would be one-way at best.
- **ctime and atime: never synced.** They describe local activity, not the document.
- Sync decisions use **content hashes**, not timestamps. mtime is preserved for the user's
  benefit, not trusted for correctness. On Android, where mtime may not be settable, the
  client remembers the true mtime from the hub rather than assuming the filesystem has it.
  **Fidelity loss on Android: files show the download time in other apps.**

## 3. Permissions and ownership

| Property | Linux | Windows | macOS | Android (shared) |
|---|---|---|---|---|
| POSIX mode bits (rwx) | ✅ | ❌ | ✅ | ❌ (fixed by the platform) |
| Executable bit | ✅ | ❌ (decided by file extension) | ✅ | ❌ |
| Read-only | Via mode bits | ✅ attribute | Via mode bits, or `uchg` flag | ❌ |
| setuid / setgid / sticky | ✅ | ❌ | ✅ | ❌ |
| Owner / group (uid, gid, SID) | ✅ | ✅ | ✅ | ❌ |
| ACLs | ⚠️ POSIX ACLs, optional | ✅ NTFS ACLs, rich | ✅ NFSv4-style | ❌ |

**Proposal:**
- **Executable bit: synced** between Linux and macOS. Ignored on Windows and Android, and preserved
  in the hub so that it survives a round-trip through them.
- **Read-only: mapped.** It is stored as one "read-only" flag. On Linux/macOS it applies as
  clearing the write bits, and on Windows as the read-only attribute.
- **Other mode bits: local-only.** New files get the device's normal defaults (umask).
- **setuid/setgid/sticky: never synced.** This is a security boundary.
- **Owner, group, ACLs: never synced.** There is one user, and on each device the files belong to
  whoever runs the client.

## 4. File types

| Type | Linux | Windows | macOS | Android (shared) |
|---|---|---|---|---|
| Regular file | ✅ | ✅ | ✅ | ✅ |
| Directory, including empty | ✅ | ✅ | ✅ | ✅ |
| Symlink | ✅ | ⚠️ needs admin rights or Developer Mode; file vs. directory links | ✅ | ❌ |
| Hard link | ✅ | ✅ | ✅ | ❌ |
| Junction / reparse point | ❌ | ✅ | ❌ | ❌ |
| macOS alias | ❌ (regular file) | ❌ (regular file) | ✅ (file + Finder flags) | ❌ |
| Windows `.lnk` shortcut | Regular file | ✅ | Regular file | Regular file |
| Sparse file | ✅ | ✅ | ✅ | 🔍 |
| FIFO, socket, device node | ✅ | ❌ | ✅ | ❌ |
| macOS package/bundle (directory shown as one file) | Plain directory | Plain directory | ✅ | Plain directory |
| Cloud placeholders (OneDrive, iCloud) | ❌ | ✅ | ✅ | ❌ |

**Proposal:**
- **Regular files and directories (including empty ones): synced** (META-2).
- **Symlinks: skipped with a warning in v1.** Following them can escape the root or loop, and
  Windows and Android can't create them reliably. *Later option:* sync relative links that stay
  inside the root, as links, on platforms that support them.
- **Hard links: synced as independent files.** Contents are preserved; the link is not.
  If the hub stores contents by hash, it keeps only one copy anyway.
- **Sparse files: synced as normal files.** Contents are preserved; holes are filled in.
- **FIFOs, sockets, devices, junctions/reparse points: skipped with a warning.**
- **Cloud placeholders** from other sync products: **skipped with a warning.** Reading one would
  trigger a download by the other product.
- **Bundles/packages:** synced as directories. Note: documents saved as bundles (e.g. some older
  macOS app formats) are synced file by file, so a half-synced bundle is possible during
  propagation.
- **Aliases and `.lnk` files** are regular files and sync as such. They may not resolve on another
  device, and that is expected.

## 5. Extended attributes and alternate streams

| Property | Linux | Windows | macOS | Android (shared) |
|---|---|---|---|---|
| User extended attributes | ✅ `user.*` | ⚠️ NTFS EAs exist but are rarely used | ✅ | ❌ |
| Alternate data streams | ❌ | ✅ | ❌ (resource fork lives in an xattr) | ❌ |
| Resource fork | ❌ | ❌ | ⚠️ legacy, `com.apple.ResourceFork` | ❌ |
| User tags | ⚠️ `user.xdg.tags` convention, few apps use it | ❌ | ✅ Finder tags | ❌ |
| Download origin marker | ❌ | ✅ Mark of the Web (`Zone.Identifier` stream) | ✅ quarantine (`com.apple.quarantine`) | ❌ |
| Hidden flag | ❌ (dotfile convention only) | ✅ attribute | ✅ `UF_HIDDEN` flag, plus dotfiles | ❌ (dotfile convention) |
| Immutable / append-only flags | ✅ `chattr` | ⚠️ | ✅ `uchg`/`schg` | ❌ |

**Proposal:**
- **xattrs, ADS, resource forks: not synced in v1** (local-only). They mostly hold application
  caches and OS bookkeeping. The format should still leave room for an optional,
  allow-listed set of attributes later.
- **Tags: Later.** A possible mapping is Finder tags ↔ `user.xdg.tags`, with Windows and Android
  not able to show them. It needs a Later decision.
- **Download origin markers: not synced.** *But see decision 4 below.*
- **Hidden flag: not synced.** Dotfiles stay hidden on Linux/macOS/Android by convention. On
  Windows, dotfiles are visible.
- **Immutable/append-only flags: never synced.**

## 6. Content

These are listed for completeness; they are not metadata decisions.
- **Bytes are never altered.** No line-ending translation, no encoding conversion, and no BOM
  changes.
- **Large files:** limited by WKL-2, not by the platforms in scope.

## Summary: canonical metadata record

What one file's record in the hub carries under this proposal:

| Field | Synced | Notes |
|---|---|---|
| Path within root (UTF-8, as created) | ✅ | Collision-checked in normalized, case-folded form |
| Type (file / directory) | ✅ | Symlinks reserved for later |
| Content hash and size | ✅ | Encrypted, like everything else in the record |
| mtime (ns) | ✅ | Applied at destination precision; best effort on Android |
| Executable | ✅ | Applied on Linux/macOS only |
| Read-only | ✅ | Mapped per platform |
| Everything else | ❌ | Local-only, or skipped with a warning |

## Decisions needed

1. **Name policy:** is the per-root **portable** (default) / **permissive** choice right, or
   should every root simply be portable?
2. **Symlinks:** skip-and-warn in v1. Agreed?
3. **Read-only mapping:** sync it, or leave it local-only too?
4. **Download origin markers:** files that arrive through sync come from your own devices.
   Should the client *add* Mark of the Web / quarantine on arrival, so that a file which had the
   marker on the originating device keeps that protection on the others? That would mean
   syncing the marker's *presence* as a flag. *Lean: yes, as a single "came from the internet"
   flag, Later.*
5. **Android mtime loss:** acceptable, given that the app keeps the true value?
