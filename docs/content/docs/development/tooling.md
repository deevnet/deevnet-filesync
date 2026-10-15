---
title: "Build Tooling"
weight: 10
---

# Build Tooling

**Status:** Tracking list. Development so far happens on the Deevnet build server. This page
records every tool added there **beyond what the Ansible-built builder provides**, so that it can
later become a `filesync` builder role, or be handled in another way.

| Tool | Version installed | Installed how | Needed for | Since |
|---|---|---|---|---|
| `rust`, `cargo` | 1.99.0 (Fedora 43 packages) | `dnf install rust cargo` | Building the Rust workspace | Prototype |
| `clippy`, `rustfmt` | 1.99.0 (Fedora 43 packages) | `dnf install clippy rustfmt` | Linting and formatting | Prototype |
| MinIO container image | `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z` | Already present in local podman storage; public pulls of MinIO images now fail | A disposable local S3 endpoint for end-to-end tests | Prototype |
| `hugo` (extended) | 0.140.1 | Already present on the build server | Building the docs site | Docs |

Rust crates are fetched by Cargo from crates.io at build time and pinned in `Cargo.lock`. They are
project dependencies, not host tooling, so they are not listed here.

## Build host notes

- **`/home` on the build server is full** (20 GB volume, 62 MB free on 2026-10-15). Most of it
  predates FileSync: podman storage (8.4 GB), the Go module cache (2.8 GB), VS Code server (1.7 GB).
  FileSync adds the Cargo registry cache in `~/.cargo` (about 100 MB). Build output goes to
  `target/` in the repo, on `/srv`, which has room. A builder role should put `CARGO_HOME` and
  container storage on a volume sized for them.
- The end-to-end test keeps MinIO's data in `/tmp` (tmpfs) for the same reason.

## Open Questions
1. A `filesync` builder role in `ansible-collection-deevnet.builder`, or a build container?
2. Fedora's packaged Rust, or `rustup` with a pinned toolchain (`rust-toolchain.toml`)?
3. Where does a local S3 test server come from, now that MinIO's public images are gone: a
   mirrored image on the Deevnet artifact server, or a different S3-compatible server?
