---
title: "Prototype"
weight: 20
---

# Prototype

**Status:** Working (2026-10-15). This is the [Phase P](../roadmap/#phase-p-prototype) prototype:
the `filesync` CLI and the `filesync-core` library, in `crates/`.

## What it does

- Encrypted hub on any S3-compatible store (tested against MinIO), or on a local directory.
- Roots, mounted at any path, with a different path on each device.
- `status`, `sync` (push now), and `pull` (pull now), plus `run --interval N` for unattended
  polling.
- Three-way sync: changes flow both ways, conflicting edits keep both versions as a conflict copy,
  and an edit beats a delete.
- Receive-only mounts (`mount --receive-only`).
- Portable-name checks (Windows-invalid names and case/normalization collisions are skipped, with
  a warning), symlinks skipped, a per-file size limit (100 MiB by default).
- Version history (`history`) and restore of a whole root or one path from any snapshot
  (`restore`).
- Refuses to sync a root whose folder is missing, or to propagate "everything deleted" without
  `--allow-mass-delete`.

## What it deliberately does not do (yet)

| Shortcut | Final design |
|---|---|
| One shared hub key, copied to each device by hand | PKI device identity, enrollment, revocation (SEC-7, SEC-9..12) |
| Whole-file blobs, no chunking | Content-defined chunking (PERF-3) |
| Blob sizes visible to the hub | Padding or chunking to hide sizes (SEC-2) |
| Every snapshot lists every path | Incremental snapshots or a compacted index |
| No garbage collection or retention | VER-2, VER-6, HUB-7 |
| Polling only (`run --interval`) | Filesystem watching plus rescans (LNX-7) |
| One static S3 credential, with delete rights | Per-device, least-privilege credentials (HUB-6, HUB-7) |

## Build and test

```bash
cargo build --release           # target/release/filesync
cargo test                      # unit tests, plus two-device tests on a directory hub
```

## Try it with two "devices" on one machine

Each device is a config directory plus a state directory. A directory hub needs no credentials:

```bash
F=target/release/filesync
export FILESYNC_CONFIG_DIR=/tmp/fs/a/cfg FILESYNC_STATE_DIR=/tmp/fs/a/state
$F init --device laptop --hub-dir /tmp/fs/hub
$F root create Finance
$F mount Finance ~/Finance
$F sync

export FILESYNC_CONFIG_DIR=/tmp/fs/b/cfg FILESYNC_STATE_DIR=/tmp/fs/b/state
$F init --device desktop --hub-dir /tmp/fs/hub --join /tmp/fs/a/cfg/hub.key
$F mount Finance /tmp/fs/b/Money
$F sync
```

## Using Deevnet's MinIO

The tenant state store, `https://tfstate.mobile.deevnet.net`, works as a hub: the client trusts the
Deevnet Root CA through the system trust store, and requests reach MinIO and are authenticated.
What's needed is a **scoped user** for a dedicated bucket. The prototype should never use MinIO's
root credentials.

With MinIO's client (`mc`), authenticated as the root user, wherever you normally administer the
state store:

```bash
mc mb deevnet/filesync-proto

cat > filesync-proto.json <<'POLICY'
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow",
      "Action": ["s3:ListBucket", "s3:GetBucketLocation"],
      "Resource": ["arn:aws:s3:::filesync-proto"] },
    { "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"],
      "Resource": ["arn:aws:s3:::filesync-proto/*"] }
  ]
}
POLICY
mc admin policy create deevnet filesync-proto filesync-proto.json
mc admin user add deevnet filesync-proto '<a new secret>'
mc admin policy attach deevnet filesync-proto --user filesync-proto
```

Then, on each device, put the credentials where only that user can read them:

```bash
mkdir -p ~/.config/deevnet-filesync && chmod 700 ~/.config/deevnet-filesync
cat > ~/.config/deevnet-filesync/credentials.toml <<'CREDS'
access_key_id = "filesync-proto"
secret_access_key = "<the secret>"
CREDS
chmod 600 ~/.config/deevnet-filesync/credentials.toml

filesync init --device <name> --s3-endpoint https://tfstate.mobile.deevnet.net --bucket filesync-proto
```

Environment variables `FILESYNC_S3_ACCESS_KEY_ID` and `FILESYNC_S3_SECRET_ACCESS_KEY` override the
file.

## Findings for Phase 0

What building it taught, to carry into the design documents:

1. **Conditional writes are enough to coordinate.** "Create snapshot N+1 only if absent" makes
   each change to a root single-writer, with no server-side code (HUB-4, HUB-5). MinIO honors
   `If-None-Match: *` on PUT.
2. **Snapshots double as version history.** Every snapshot is a complete, restorable state of a
   root, so point-in-time restore (VER-5) comes almost for free. The cost is that the snapshot
   grows with the root, and is rewritten on every change. Phase 0 should design incremental
   snapshots and decide how retention prunes them.
3. **`start-after` listing keeps polling cheap.** A device only lists snapshots newer than the one
   it has, so an idle poll is one small LIST request (PERF-2).
4. **A hand-written SigV4 signer is small.** It's about 150 lines, with no AWS SDK, which supports
   the independence principle (IND-1). It works against MinIO; it still needs testing against AWS
   S3 itself.
5. **Keyed content identifiers** stop the hub from confirming whether it holds a known file, and
   cost nothing extra.
6. **Mass-deletion guard.** An empty root folder (unmounted disk, denied permission) looks exactly
   like "the user deleted everything". The guard is necessary; it should become a requirement.
7. **Deletes need care against races.** The client deletes a local file only if it still matches
   what the scan saw; otherwise the edit wins on the next run.
8. **Blob uploads always send the body**, even when the blob already exists. With whole-file
   blobs this is wasteful on large files; chunking (PERF-3) or a cheap existence check should
   fix it.
9. **MinIO's community container images are gone from public registries**, so pinned tags no
   longer pull. Local S3 testing needs either a locally held image or another S3-compatible
   server (see [Build Tooling](tooling/)).
