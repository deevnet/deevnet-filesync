//! Keys and the encryption envelope.
//!
//! **Prototype shortcut:** one 256-bit hub key, shared by every device as a key file. The
//! PKI-based design (SEC-7, SEC-9..12) replaces this in Phase 1.
//!
//! From the hub key, two subkeys are derived with BLAKE3's key derivation:
//! - an encryption key, for XChaCha20-Poly1305;
//! - an identifier key, for keyed BLAKE3 content identifiers. Keyed, so that the hub cannot
//!   confirm whether a known file is stored there by hashing it.
//!
//! Envelope: `MAGIC (4) | nonce (24) | ciphertext+tag`. The object's hub key is bound in as
//! associated data, so an object cannot be swapped for another one undetected (SEC-4).

use anyhow::{anyhow, bail, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::util;

const MAGIC: &[u8; 4] = b"DFS1";
const NONCE_LEN: usize = 24;

pub struct Keys {
    cipher: XChaCha20Poly1305,
    id_key: [u8; 32],
}

impl Keys {
    pub fn generate_master() -> [u8; 32] {
        util::random_bytes::<32>()
    }

    pub fn from_master(master: &[u8; 32]) -> Self {
        let enc = blake3::derive_key("deevnet-filesync prototype-1 encryption", master);
        let id_key = blake3::derive_key("deevnet-filesync prototype-1 content-id", master);
        Self {
            cipher: XChaCha20Poly1305::new(&enc.into()),
            id_key,
        }
    }

    /// A streaming hasher for content identifiers.
    pub fn id_hasher(&self) -> blake3::Hasher {
        blake3::Hasher::new_keyed(&self.id_key)
    }

    pub fn content_id(&self, data: &[u8]) -> String {
        self.id_hasher()
            .update(data)
            .finalize()
            .to_hex()
            .to_string()
    }

    pub fn seal(&self, object_key: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce = util::random_bytes::<NONCE_LEN>();
        let ct = self
            .cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: object_key.as_bytes(),
                },
            )
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn open(&self, object_key: &str, sealed: &[u8]) -> Result<Vec<u8>> {
        if sealed.len() < MAGIC.len() + NONCE_LEN || &sealed[..4] != MAGIC {
            bail!("{object_key}: not a FileSync envelope");
        }
        let (nonce, ct) = sealed[4..].split_at(NONCE_LEN);
        self.cipher
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: object_key.as_bytes(),
                },
            )
            .map_err(|_| {
                anyhow!("{object_key}: decryption failed (wrong key, or the object was altered)")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_binding() {
        let k = Keys::from_master(&[7u8; 32]);
        let sealed = k.seal("a/b", b"hello").unwrap();
        assert_eq!(k.open("a/b", &sealed).unwrap(), b"hello");
        assert!(k.open("a/c", &sealed).is_err(), "moved object must fail");
        let other = Keys::from_master(&[8u8; 32]);
        assert!(other.open("a/b", &sealed).is_err(), "wrong key must fail");
    }
}
