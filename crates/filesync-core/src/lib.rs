//! Deevnet FileSync core (prototype).
//!
//! The pieces, from the bottom up:
//! - [`backend`]: the narrow storage contract (BE-2) and its implementations.
//! - [`crypto`]: keys, encryption envelope, and content identifiers.
//! - [`hub`]: the hub layout: roots, snapshots, and blobs, all encrypted.
//! - [`scan`] and [`names`]: reading a local root, with portable-name checks.
//! - [`engine`]: the three-way sync between the last synced state, local, and remote.

pub mod backend;
pub mod crypto;
pub mod engine;
pub mod hub;
pub mod manifest;
pub mod names;
pub mod s3;
pub mod scan;
pub mod util;
