//! Encryption at rest for variable values.
//!
//! Algorithm: XChaCha20-Poly1305 with a random 192-bit nonce per value.
//! The record id is the associated data, so a ciphertext copied to another
//! row fails to decrypt.
//!
//! Key source, in order:
//! 1. `TUFFIE_MASTER_KEY` (base64, 32 bytes). Use this on servers with a secret manager.
//! 2. `<data_dir>/master.key`, created on first start with mode 0600.

use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Context};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

const VERSION_PREFIX: &str = "v1:";

pub struct Crypto {
    cipher: XChaCha20Poly1305,
}

impl Crypto {
    pub fn load_or_create(data_dir: &Path) -> anyhow::Result<Self> {
        let key: Zeroizing<Vec<u8>> = if let Ok(k) = std::env::var("TUFFIE_MASTER_KEY") {
            Zeroizing::new(B64.decode(k.trim()).context("TUFFIE_MASTER_KEY is not valid base64")?)
        } else {
            let path = data_dir.join("master.key");
            if path.exists() {
                check_permissions(&path)?;
                let raw = Zeroizing::new(std::fs::read_to_string(&path)?);
                Zeroizing::new(B64.decode(raw.trim()).context("master.key is not valid base64")?)
            } else {
                let mut k = Zeroizing::new(vec![0u8; 32]);
                rand::rngs::OsRng.fill_bytes(&mut k);
                write_private(&path, B64.encode(&*k).as_bytes())?;
                tracing::info!(path = %path.display(), "created a new master key; back it up");
                k
            }
        };
        if key.len() != 32 {
            return Err(anyhow!("master key must be 32 bytes, got {}", key.len()));
        }
        Ok(Self { cipher: XChaCha20Poly1305::new_from_slice(&key).map_err(|e| anyhow!("{e}"))? })
    }

    pub fn encrypt(&self, plaintext: &str, aad: &str) -> anyhow::Result<String> {
        let mut nonce = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = self
            .cipher
            .encrypt(XNonce::from_slice(&nonce), Payload { msg: plaintext.as_bytes(), aad: aad.as_bytes() })
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ct);
        Ok(format!("{VERSION_PREFIX}{}", B64.encode(out)))
    }

    pub fn decrypt(&self, stored: &str, aad: &str) -> anyhow::Result<Zeroizing<String>> {
        let body = stored.strip_prefix(VERSION_PREFIX).ok_or_else(|| anyhow!("unknown cipher version"))?;
        let raw = B64.decode(body)?;
        if raw.len() < 24 {
            return Err(anyhow!("ciphertext too short"));
        }
        let (nonce, ct) = raw.split_at(24);
        let pt = self
            .cipher
            .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad: aad.as_bytes() })
            .map_err(|_| anyhow!("decryption failed (wrong master key?)"))?;
        Ok(Zeroizing::new(String::from_utf8(pt)?))
    }
}

impl Crypto {
    /// Encrypt a whole file (plan JSON, state snapshot). These files hold
    /// secrets in plain text when Terraform writes them, so Tuffie stores
    /// its own copies encrypted. `aad` binds the file to its run and name.
    pub fn write_encrypted(&self, path: &Path, data: &[u8], aad: &str) -> anyhow::Result<()> {
        let mut nonce = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = self
            .cipher
            .encrypt(XNonce::from_slice(&nonce), Payload { msg: data, aad: aad.as_bytes() })
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = b"TUFENC1".to_vec();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        write_private(path, &out)
    }

    pub fn read_encrypted(&self, path: &Path, aad: &str) -> anyhow::Result<Zeroizing<Vec<u8>>> {
        let raw = std::fs::read(path)?;
        let body = raw.strip_prefix(b"TUFENC1").ok_or_else(|| anyhow!("file is not encrypted by Tuffie"))?;
        if body.len() < 24 {
            return Err(anyhow!("file too short"));
        }
        let (nonce, ct) = body.split_at(24);
        let pt = self
            .cipher
            .decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad: aad.as_bytes() })
            .map_err(|_| anyhow!("decryption failed"))?;
        Ok(Zeroizing::new(pt))
    }
}

pub fn write_private(path: &Path, data: &[u8]) -> anyhow::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).with_context(|| format!("write {}", path.display()))?;
    f.write_all(data)?;
    Ok(())
}

fn check_permissions(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            tracing::warn!(path = %path.display(), "master.key can be read by other users; run chmod 600");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_aad_binding() {
        let dir = tempfile::tempdir().unwrap();
        let c = Crypto::load_or_create(dir.path()).unwrap();
        let enc = c.encrypt("s3cr3t", "var-1").unwrap();
        assert_eq!(&*c.decrypt(&enc, "var-1").unwrap(), "s3cr3t");
        assert!(c.decrypt(&enc, "var-2").is_err(), "aad must bind the ciphertext to the record");
        // Same key is loaded again from disk.
        let c2 = Crypto::load_or_create(dir.path()).unwrap();
        assert_eq!(&*c2.decrypt(&enc, "var-1").unwrap(), "s3cr3t");

        let f = dir.path().join("plan.json.enc");
        c.write_encrypted(&f, b"{\"pw\":\"x\"}", "run-1/plan").unwrap();
        assert!(!std::fs::read(&f).unwrap().windows(2).any(|w| w == b"pw"));
        assert_eq!(&*c.read_encrypted(&f, "run-1/plan").unwrap(), b"{\"pw\":\"x\"}");
        assert!(c.read_encrypted(&f, "run-2/plan").is_err());
    }
}
