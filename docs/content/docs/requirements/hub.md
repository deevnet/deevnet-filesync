---
title: "Cloud Hub"
weight: 30
---

# Cloud Hub

**Status:** Draft. Requirements for the storage side of the system: the hub that every device
syncs through. Core requirements are on the [Requirements](../) page.

The hub is **storage only**. It holds encrypted objects and runs no FileSync code. All
intelligence (sync decisions, encryption, conflict handling) lives in the clients. That keeps the
provider replaceable and keeps it from ever needing to understand the data.

## Backend model
- **BE-0 (MVP):** Only one backend is active at a time. That one backend is the hub.
- **BE-1 (MVP):** AWS S3 is the MVP backend.
- **BE-2 (MVP):** The backend interface is narrow and primitive: put, get, list, delete, and a
  conditional write (compare-and-swap) for coordination. New backends implement only that
  interface.
- **BE-4 (MVP):** The sync engine, crypto, and hub format do not depend on anything AWS-specific.
  Anything AWS-specific (authentication, request signing, provider quirks) stays inside the S3
  backend adapter.
- **BE-3 (Later):** Self-hosted storage, e.g. MinIO on Deevnet infrastructure, SFTP, or a local or
  NAS path.
- **BE-5 (Later):** Other S3-compatible providers (Backblaze B2, Cloudflare R2, Wasabi), and
  consumer cloud drives (Google Drive, OneDrive, Dropbox) used as dumb storage.
- **BE-6 (Later):** A **data migration process** moves the hub from one backend to another (e.g.
  AWS S3 → self-hosted) without re-uploading from every device.
- **BE-7 (Non-goal):** Mirroring to multiple backends at the same time.

## What the hub holds
- **HUB-1 (MVP):** Everything stored in the hub is encrypted before it leaves a device. Object
  names are opaque and reveal nothing about file names, paths, or roots (SEC-1, SEC-2).
- **HUB-2 (MVP):** The hub layout and object formats are a versioned, documented specification
  (IND-2). The hub records its format version, and a client refuses to write to a hub whose
  format version it does not understand.
- **HUB-3 (MVP, proposed):** The hub stores no plaintext metadata about the user beyond what the
  provider inevitably sees: object count, object sizes, and access times and IP addresses.

## Coordination
- **HUB-4 (MVP):** Devices coordinate safely through plain object storage using conditional
  writes. Two devices updating at the same time never silently lose one update. (AWS S3 supports
  conditional writes; support among other S3-compatible providers varies, which BE-5 has to
  check.)
- **HUB-5 (MVP, proposed):** No server-side compute in the MVP: no Lambda functions, no event
  notifications, no provider-specific services. Change detection is client polling (SYNC-3).

## Access control and protection
- **HUB-6 (MVP, proposed):** Every device has its own credential, scoped to only the FileSync
  bucket (or prefix). Revoking a device (SEC-5) also removes its storage access.
- **HUB-7 (MVP, proposed):** **A compromised device cannot destroy the hub.** Day-to-day device
  credentials can add data but cannot permanently delete or overwrite history. Permanent
  deletion (garbage collection, VER-6) uses a separate, more privileged credential. This
  protects against ransomware or a stolen laptop wiping out every version.
- **HUB-8 (MVP, proposed):** Provider-side encryption at rest (e.g. SSE-S3) is enabled as
  defense in depth. It is *in addition to* FileSync's end-to-end encryption, never instead of it.

## Setup and operation
- **HUB-9 (MVP, proposed):** Setting up the hub (bucket, access policies, device credentials) is
  documented and scriptable. The CLI can generate the policy documents; clicking through a
  provider console is never required.
- **HUB-10 (MVP):** The user chooses the provider account, region, and bucket. FileSync never
  creates or requires an account with anyone else.

## Cost
- **PERF-2 (MVP):** Request counts and polling frequency are kept modest and configurable,
  because cloud object storage bills per request as well as per GB.
- **HUB-11 (MVP, proposed):** The docs include a cost model: expected monthly cost for a typical
  workload (e.g. 20 GB, 3 devices, polling every few minutes).
- **HUB-12 (Later, proposed):** The CLI reports hub usage (stored bytes, retained versions, trash),
  so that retention policies can be tuned against cost.

## Open Questions

1. **Device credentials for AWS (HUB-6):** short-lived credentials via **AWS IAM Roles
   Anywhere** (the device certificate from the FileSync CA is exchanged for temporary AWS
   credentials, with no long-lived keys on devices), or a scoped static access key per device?
   Roles Anywhere fits the PKI direction well, and is AWS-only, so it would live inside the S3
   adapter.
2. **Protection mechanism (HUB-7):** S3 bucket versioning plus a no-delete policy for devices,
   S3 Object Lock, or both? What is the privileged credential for garbage collection: the
   offline CA, or one designated device?
3. **Provisioning tool (HUB-9):** CLI-generated policy documents applied by hand, or an
   infrastructure-as-code definition (Terraform or Ansible, matching Deevnet practice)?
