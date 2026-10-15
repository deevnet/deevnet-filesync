---
title: "Metadata Matrix"
weight: 10
---

# Metadata Matrix

**Status:** Decided (2026-10-15). This page backs requirements META-1 to META-7.

A file is more than its bytes. Each platform keeps a different set of names, timestamps,
permissions, and extra data alongside the bytes. This page sets out what each platform can
represent, and records, for each kind of metadata, whether FileSync:

- **syncs** it,
- **maps** it to an approximation on platforms that can't represent it exactly,
- keeps it **local-only** (it never leaves the device), or
- **skips** the file (refuses to sync it, and tells the user).

The hub itself is not a constraint. Metadata is stored inside the encrypted file records, so the
hub can carry whatever we decide. The limits come from the *receiving* platform.

**Legend:** ✅ supported | ⚠️ partial / with caveats | ❌ not supported | n/a not relevant |
🔍 to verify in Phase 0 testing

**Platforms assumed:** Linux on ext4/btrfs/xfs; Windows on NTFS; macOS on APFS (default
case-insensitive); Android as a **read-only** device (AND-10), storing files in the app's own
storage and showing them to other apps through a read-only document provider (AND-2). Because
nothing originates on Android, most metadata questions don't apply there. Removable media
(FAT32/exFAT) are out of scope for sync roots.

## 1. File names and paths

| Property | Linux | Windows | macOS | Android (read-only) |
|---|---|---|---|---|
| Name encoding | Arbitrary bytes (usually UTF-8; invalid UTF-8 is possible) | UTF-16 (unpaired surrogates possible) | UTF-8 | UTF-8 |
| Max name length | 255 bytes | 255 UTF-16 units | 255 UTF-8 chars | 255 bytes |
| Max path length | 4096 bytes | 260 chars by default; ~32k with long paths enabled | 1024 bytes | 4096 bytes |
| Case sensitivity | Sensitive | Insensitive, preserving (per-directory opt-in for sensitive) | Insensitive, preserving (default volume) | Sensitive (app storage) |
| Unicode normalization | None: NFC and NFD are different names | None: different names | Preserving, but NFC/NFD of the same name collide | None |
| Forbidden characters | `/`, NUL | `< > : " / \ \| ? *`, control chars 0–31 | `/`, NUL (`:` shows as `/` in Finder) | `/`, NUL |
| Reserved names | None | `CON PRN AUX NUL COM1–9 LPT1–9`, with or without an extension | None | None |
| Trailing space or dot | Allowed | Stripped or refused | Allowed | Allowed |

### What this means

Linux is the most permissive, and **Windows is the strictest**. macOS adds case and normalization
collisions. Android's app storage accepts almost anything, so it adds no constraints. Examples:
- `Report.pdf` and `report.pdf` side by side: valid on Linux; they collide on Windows and macOS.
- `Q3: results.xlsx`: valid on Linux and macOS; invalid on Windows.
- `café.txt` in NFC and in NFD: two files on Linux; one file on macOS.
- `aux.txt`: valid everywhere except Windows.

**Decision: every root uses portable names** (META-6).
- A name must be valid on every platform: in practice, valid on Windows and unique after
  case-folding and Unicode normalization.
- A non-portable name is flagged as soon as it appears on any device, so it can be renamed while
  the user still remembers it. That file is **not uploaded** until it is renamed. It is never
  renamed silently, never mangled, and never merged with its look-alike.
- Invalid UTF-8 names are refused at the source.
- There is no per-root "permissive" option. It could be added later without a format change.

## 2. Timestamps

| Property | Linux | Windows | macOS | Android (read-only) |
|---|---|---|---|---|
| Modification time (mtime) | ✅ ns, settable | ✅ 100 ns, settable | ✅ ns, settable | ✅ settable in app storage; reported to other apps by the document provider |
| Creation / birth time | ⚠️ readable via `statx` on most filesystems; **not settable** | ✅ settable | ✅ settable | n/a |
| Change time (ctime) | ✅ not settable | ✅ not settable | ✅ not settable | n/a |
| Access time (atime) | ⚠️ often disabled (`relatime`/`noatime`) | ⚠️ often disabled | ✅ | n/a |

**Decision:**
- **mtime: synced** (META-1), stored with nanosecond precision and applied at whatever precision
  the destination has.
- **Birth time: local-only.** Linux can't set it, so syncing it would be one-way at best.
- **ctime and atime: never synced.** They describe local activity, not the document.
- Sync decisions use **content hashes**, not timestamps. mtime is preserved for the user's
  benefit, not trusted for correctness.

## 3. Permissions and ownership

| Property | Linux | Windows | macOS | Android (read-only) |
|---|---|---|---|---|
| POSIX mode bits (rwx) | ✅ | ❌ | ✅ | n/a (app sandbox) |
| Executable bit | ✅ | ❌ (decided by file extension) | ✅ | n/a |
| Read-only | Via mode bits | ✅ attribute | Via mode bits, or `uchg` flag | Always (read-only device) |
| setuid / setgid / sticky | ✅ | ❌ | ✅ | n/a |
| Owner / group (uid, gid, SID) | ✅ | ✅ | ✅ | n/a |
| ACLs | ⚠️ POSIX ACLs, optional | ✅ NTFS ACLs, rich | ✅ NFSv4-style | n/a |

**Decision:**
- **Executable bit: synced** between Linux and macOS (META-3). It is ignored on Windows and
  Android, and preserved in the hub so that it survives an edit made on Windows.
- **Read-only: local-only.** It is rarely set deliberately, and syncing it causes surprises (a
  file that can't be saved on another device, or an incoming change that can't be applied).
- **Other mode bits: local-only.** New files get the device's normal defaults (umask).
- **setuid/setgid/sticky: never synced.** This is a security boundary.
- **Owner, group, ACLs: never synced.** There is one user, and on each device the files belong to
  whoever runs the client.

## 4. File types

| Type | Linux | Windows | macOS | Android (read-only) |
|---|---|---|---|---|
| Regular file | ✅ | ✅ | ✅ | ✅ |
| Directory, including empty | ✅ | ✅ | ✅ | ✅ |
| Symlink | ✅ | ⚠️ needs admin rights or Developer Mode; file vs. directory links | ✅ | n/a |
| Hard link | ✅ | ✅ | ✅ | n/a |
| Junction / reparse point | ❌ | ✅ | ❌ | n/a |
| macOS alias | ❌ (regular file) | ❌ (regular file) | ✅ (file + Finder flags) | Regular file |
| Windows `.lnk` shortcut | Regular file | ✅ | Regular file | Regular file |
| Sparse file | ✅ | ✅ | ✅ | n/a |
| FIFO, socket, device node | ✅ | ❌ | ✅ | n/a |
| macOS package/bundle (directory shown as one file) | Plain directory | Plain directory | ✅ | Plain directory |
| Cloud placeholders (OneDrive, iCloud) | ❌ | ✅ | ✅ | n/a |

**Decision:**
- **Regular files and directories (including empty ones): synced** (META-2).
- **Symlinks: skipped with a warning in v1** (META-4). Following them can escape the root or
  loop, and Windows can't create them reliably. *Later option:* sync relative links that stay
  inside the root, as links, on platforms that support them.
- **Hard links: synced as independent files.** Contents are preserved; the link is not.
  If the hub stores contents by hash, it keeps only one copy anyway.
- **Sparse files: synced as normal files.** Contents are preserved; holes are filled in.
- **FIFOs, sockets, devices, junctions/reparse points: skipped with a warning** (META-4).
- **Cloud placeholders** from other sync products: **skipped with a warning** (META-4). Reading
  one would trigger a download by the other product.
- **Bundles/packages:** synced as directories. Note: documents saved as bundles (e.g. some older
  macOS app formats) are synced file by file, so a half-synced bundle is possible during
  propagation.
- **Aliases and `.lnk` files** are regular files and sync as such. They may not resolve on another
  device, and that is expected.

## 5. Extended attributes and alternate streams

| Property | Linux | Windows | macOS | Android (read-only) |
|---|---|---|---|---|
| User extended attributes | ✅ `user.*` | ⚠️ NTFS EAs exist but are rarely used | ✅ | n/a |
| Alternate data streams | ❌ | ✅ | ❌ (resource fork lives in an xattr) | n/a |
| Resource fork | ❌ | ❌ | ⚠️ legacy, `com.apple.ResourceFork` | n/a |
| User tags | ⚠️ `user.xdg.tags` convention, few apps use it | ❌ | ✅ Finder tags | n/a |
| Download origin marker | ❌ | ✅ Mark of the Web (`Zone.Identifier` stream) | ✅ quarantine (`com.apple.quarantine`) | ❌ |
| Hidden flag | ❌ (dotfile convention only) | ✅ attribute | ✅ `UF_HIDDEN` flag, plus dotfiles | Dotfile convention |
| Immutable / append-only flags | ✅ `chattr` | ⚠️ | ✅ `uchg`/`schg` | n/a |

**Decision:**
- **xattrs, ADS, resource forks: not synced in v1** (META-5). They mostly hold application
  caches and OS bookkeeping. The format leaves room for an optional, allow-listed set of
  attributes later.
- **Tags: Later.** A possible mapping is Finder tags ↔ `user.xdg.tags`, with Windows and Android
  not able to show them.
- **Download origin markers: synced as a single "came from the internet" flag, Later**
  (META-7). A file that carried Mark of the Web or quarantine on the device where it was
  created gets the same protection on Windows and macOS devices that receive it. The hub format
  reserves this flag from v1, so adding the behavior later needs no format change.
- **Hidden flag: not synced.** Dotfiles stay hidden on Linux, macOS, and Android by convention.
  On Windows, dotfiles are visible.
- **Immutable/append-only flags: never synced.**

## 6. Content

These are listed for completeness; they are not metadata decisions.
- **Bytes are never altered.** No line-ending translation, no encoding conversion, and no BOM
  changes.
- **Large files:** limited by WKL-2, not by the platforms in scope.

## Summary: canonical metadata record

What one file's record in the hub carries:

| Field | Synced | Notes |
|---|---|---|
| Path within root (UTF-8, as created) | ✅ | Must be portable; collision-checked in normalized, case-folded form |
| Type (file / directory) | ✅ | Symlinks reserved for later |
| Content hash and size | ✅ | Encrypted, like everything else in the record |
| mtime (ns) | ✅ | Applied at destination precision |
| Executable | ✅ | Applied on Linux/macOS only |
| Came from the internet | Reserved | Field exists from v1; behavior is Later (META-7) |
| Everything else | ❌ | Local-only, or skipped with a warning |

## Decision log

| # | Question | Decision |
|---|---|---|
| 1 | Name policy: per-root portable/permissive, or always portable? | **Always portable.** No permissive option for now. |
| 2 | Symlinks in v1? | **Skipped with a warning.** |
| 3 | Sync the read-only flag? | **No. It stays local-only.** |
| 4 | Download origin markers? | **Synced as one "came from the internet" flag, Later.** Field reserved in v1. |
| 5 | Android mtime loss acceptable? | **Moot.** Android is read-only and keeps the true mtime (AND-10). |
