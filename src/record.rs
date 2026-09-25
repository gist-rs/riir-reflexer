//! The trajectory submission client — free, opt-in, OFF by default.
//!
//! When the bin runs with `--record <path.jsonl>`, every answered request
//! appends one replayable row (the exact request JSON + the engine's
//! outcomes) to `<path>`; on clean EOF the bin writes
//! `<path>.manifest.json` — row count, the BLAKE3 of the rows file, the
//! genome digest, and an Ed25519 signature over
//! `reflexer-trajectory-v1\n{rows}\n{rows_blake3}\n{genome}\n` made with
//! the PER-MACHINE submission key. The private improvement loop
//! replay-verifies rows against its own engine before crediting anything
//! (the anti-poisoning gate lives there, not here).
//!
//! Key handling: the 32-byte seed lives at `$REFLEXER_SUBMISSION_KEY` or
//! `~/.config/reflexer/submission.ed25519` (0600, created on first use).
//! The PUBLIC half rides the manifest so anyone can verify; the private
//! half never leaves the machine. NO billing code, NO tokenomics — ever.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use katgpt_core::decision_wire::{Answer, DecisionRequest, Outcome};
use rand_core::RngCore;
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The signed message's domain tag (version-pinned).
pub const TRAJECTORY_DOMAIN: &str = "reflexer-trajectory-v1";

/// One recorded decision — the exact request + the engine's outcomes, so
/// replay-verification is a pure function of the row.
#[derive(serde::Serialize)]
pub struct Row {
    pub seq: u64,
    pub request: DecisionRequest,
    pub outcomes: Vec<RowOutcome>,
}

#[derive(serde::Serialize)]
pub struct RowOutcome {
    pub question_id: String,
    pub outcome: Option<Outcome>,
}

/// The manifest written at clean EOF.
#[derive(serde::Serialize, Deserialize)]
pub struct Manifest {
    pub proto: u32,
    pub rows: u64,
    /// BLAKE3 of the exact rows-file bytes.
    pub rows_blake3: String,
    pub genome: String,
    /// The verifying (public) key, hex — the signer's identity.
    pub verifying_key: String,
    /// Ed25519 signature over `{TRAJECTORY_DOMAIN}\n{rows}\n{rows_blake3}\n{genome}\n`, hex.
    pub signature: String,
}

pub struct TrajectoryRecorder {
    path: PathBuf,
    file: std::fs::File,
    hasher: blake3::Hasher,
    rows: u64,
    signer: SigningKey,
}

/// Where the per-machine submission key lives.
pub fn submission_key_path() -> PathBuf {
    if let Some(p) = std::env::var_os("REFLEXER_SUBMISSION_KEY") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config/reflexer/submission.ed25519")
}

fn load_or_create_key(path: &Path) -> std::io::Result<SigningKey> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        let seed: [u8; 32] = bytes.try_into().map_err(|b: Vec<u8>| {
            std::io::Error::other(format!("key file must be 32 bytes, got {}", b.len()))
        })?;
        return Ok(SigningKey::from_bytes(&seed));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut seed = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);
    std::fs::write(path, key.to_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(key)
}

impl TrajectoryRecorder {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let signer = load_or_create_key(&submission_key_path())?;
        let file = std::fs::File::create(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            hasher: blake3::Hasher::new(),
            rows: 0,
            signer,
        })
    }

    /// Append one row. `request_line` is the exact stdin line (the row
    /// embeds it parsed; the hash covers the ROW bytes, not the request).
    pub fn record(&mut self, request: DecisionRequest, answers: &[Answer]) -> std::io::Result<()> {
        let row = Row {
            seq: self.rows,
            request,
            outcomes: answers
                .iter()
                .map(|a| RowOutcome {
                    question_id: a.question_id.clone(),
                    outcome: a.outcome,
                })
                .collect(),
        };
        let mut line = serde_json::to_vec(&row)?;
        line.push(b'\n');
        self.file.write_all(&line)?;
        self.hasher.update(&line);
        self.rows += 1;
        Ok(())
    }

    /// Flush + write the signed manifest beside the rows file. Returns
    /// the manifest path.
    pub fn finish(mut self, genome: &str) -> std::io::Result<PathBuf> {
        self.file.flush()?;
        let digest = self.hasher.finalize();
        let manifest = Manifest {
            proto: crate::proto::PROTO,
            rows: self.rows,
            rows_blake3: digest.to_string(),
            genome: genome.to_string(),
            verifying_key: hex(self.signer.verifying_key().as_bytes()),
            signature: hex(&self
                .signer
                .sign(signed_message(self.rows, &digest.to_string(), genome).as_bytes())
                .to_bytes()),
        };
        let path = self.path.with_extension("manifest.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&manifest)?)?;
        Ok(path)
    }
}

/// The exact bytes the signature covers.
pub fn signed_message(rows: u64, rows_blake3: &str, genome: &str) -> String {
    format!("{TRAJECTORY_DOMAIN}\n{rows}\n{rows_blake3}\n{genome}\n")
}

/// Verify a manifest against rows-file bytes — the public half anyone can
/// run (and what the private loop runs before replay).
pub fn verify_manifest(manifest: &Manifest, rows_bytes: &[u8]) -> Result<(), &'static str> {
    let digest = blake3::hash(rows_bytes).to_string();
    if digest != manifest.rows_blake3 {
        return Err("rows blake3 mismatch");
    }
    let key_bytes = unhex(&manifest.verifying_key).ok_or("bad verifying key hex")?;
    let key_arr: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "verifying key must be 32 bytes")?;
    let key = VerifyingKey::from_bytes(&key_arr).map_err(|_| "bad verifying key")?;
    let sig_bytes = unhex(&manifest.signature).ok_or("bad signature hex")?;
    let sig = ed25519_dalek::Signature::from_bytes(
        &sig_bytes
            .try_into()
            .map_err(|_| "signature must be 64 bytes")?,
    );
    key.verify(
        signed_message(manifest.rows, &manifest.rows_blake3, &manifest.genome).as_bytes(),
        &sig,
    )
    .map_err(|_| "signature does not verify")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip_verifies_and_tamper_fails() {
        let dir = std::env::temp_dir().join(format!("reflexer-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        unsafe { std::env::set_var("REFLEXER_SUBMISSION_KEY", dir.join("k.seed")) };
        let rows_path = dir.join("rows.jsonl");
        let mut rec = TrajectoryRecorder::open(&rows_path).unwrap();
        rec.record(
            serde_json::from_str(
                r#"{"state":"{}","questions":[{"id":"q","kind":"noul","prompt":"p","options":[]}]}"#,
            )
            .unwrap(),
            &[],
        )
        .unwrap();
        let manifest_path = rec.finish("68cae9d382014662").unwrap();
        let manifest: Manifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        let rows_bytes = std::fs::read(&rows_path).unwrap();
        assert_eq!(manifest.rows, 1);
        verify_manifest(&manifest, &rows_bytes).unwrap();
        // Tamper: one flipped row byte must break verification.
        let mut tampered = rows_bytes.clone();
        tampered[0] ^= 1;
        assert!(verify_manifest(&manifest, &tampered).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
