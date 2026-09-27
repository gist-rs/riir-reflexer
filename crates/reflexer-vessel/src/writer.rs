//! The PUBLIC-RELEASE writer — the FIRST writer in the format repo.
//!
//! Instinct Proposal 001 T4 (the owner's design decision, 2026-09-27):
//! the public FORMAT repo owns the public-class writer. This module is
//! it. No path in this repo (or any) writes class 1 — HOSTED-ONLY
//! minting stays riir-train's `vessel_mint`, and `encode_raw` stays
//! `pub(crate)` for this crate's own hostile-forgery tests.
//!
//! What the writer adds over [`crate::encode_public`]:
//! - **Result, never a panic** — the 1 MiB public cap is enforced AT
//!   WRITE TIME as [`VesselError::PayloadTooLarge`], so a minter gets a
//!   refusal it can print, not a broken process.
//! - **The commitment out** — a mint returns the vessel's blake3
//!   commitment ([`MintedVessel`]), the pin every consumer records (the
//!   measurement law's per-row artifact pin; reflex's head lanes pin
//!   these beside the head digests).
//! - **Key handling** — strict 64-hex seed parsing and a key-file
//!   reader (64-hex text or 32 raw bytes), the fail-closed posture the
//!   `reflexer sign` subcommand and reflex's `mint-heads` share: no
//!   key, no signature, no output.
//!
//! Determinism: ed25519 signing is deterministic, so the same (key,
//! key_id, artifact_version, parent, payload) mints byte-identical
//! vessels — re-running a mint refreshes nothing and moves no pin.

use crate::{Class, Header, MAX_PAYLOAD, VesselError, commitment_of, encode_raw};
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::path::Path;

/// A freshly minted PUBLIC-RELEASE vessel: the bytes to write and the
/// blake3 commitment over the signed region — the pin consumers record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MintedVessel {
    pub bytes: Vec<u8>,
    pub commitment: [u8; 32],
}

/// Mint a PUBLIC-RELEASE vessel. `parent` is the parent's commitment
/// (zeros = genesis); lineage discipline (strictly-increasing
/// `artifact_version`) is the minter's job, enforced at the reader by
/// the monotonic gate.
pub fn sign_public(
    key: &SigningKey,
    key_id: u32,
    artifact_version: u64,
    parent: [u8; 32],
    payload: &[u8],
) -> Result<MintedVessel, VesselError> {
    // The 1 MiB public cap, AT WRITE TIME (the reader refuses the same
    // bound before allocation — both sides of the door are bounded).
    if payload.len() > MAX_PAYLOAD {
        return Err(VesselError::PayloadTooLarge {
            len: payload.len() as u64,
            cap: MAX_PAYLOAD,
        });
    }
    let header = Header {
        format_version: crate::FORMAT_VERSION,
        class: Class::PublicRelease,
        key_id,
        artifact_version,
        parent_commitment: parent,
        payload_len: payload.len() as u64,
    };
    let bytes = encode_raw(key, header.clone(), payload);
    let commitment = commitment_of(&header, payload);
    Ok(MintedVessel { bytes, commitment })
}

/// Parse a signing key from its 64-hex seed (the `--key` flag / the
/// `REFLEXER_SIGN_KEY` env form). Strict: exactly 64 ASCII hex chars —
/// no `0x` prefix, no whitespace tolerance at this layer (callers trim
/// env/file input themselves before handing it here).
pub fn signing_key_from_seed_hex(hex: &str) -> Result<SigningKey, String> {
    let seed = hex_decode32(hex)?;
    // ed25519-dalek v2's from_bytes is infallible (it clamps); any 32
    // bytes are a usable key — the STRICT gate is the hex parse above.
    Ok(SigningKey::from_bytes(&seed))
}

/// Parse a signing key from a FILE: either 64-hex text (any surrounding
/// whitespace, optional trailing newline) or 32 raw key bytes. The
/// file form exists so a release script can pass a key by path without
/// it entering a shell history or a process listing.
pub fn signing_key_from_file(path: &Path) -> Result<SigningKey, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() == 32 {
        return Ok(SigningKey::from_bytes(&bytes.try_into().expect("len")));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        format!(
            "{}: not 32 raw bytes and not UTF-8 hex text",
            path.display()
        )
    })?;
    signing_key_from_seed_hex(text.trim())
}

/// The verifying key of a signing key, hex-encoded — the trust anchor
/// the consumer side pins (`RIIR_REFLEX_HEADS_PUBKEY`, the operator
/// `--vessel-pubkey` posture). Printed by every mint so the loop
/// closes with zero extra crypto tooling.
pub fn verifying_key_hex(key: &SigningKey) -> String {
    let vk: VerifyingKey = key.verifying_key();
    hex32(&vk.to_bytes())
}

/// Re-verify a minted vessel against pins — the mint-side round-trip
/// arm. A minter that cannot open its own output has no business
/// shipping it; the `reflexer sign` subcommand runs this before writing.
pub fn verify_roundtrip(
    bytes: &[u8],
    pins: &crate::PinTable,
) -> Result<crate::VerifiedVessel, VesselError> {
    crate::decode(bytes, pins)
}

fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        use std::fmt::Write as _;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn hex_decode32(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "expected 64 hex chars (32 bytes), got {} chars",
            s.len()
        ));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).expect("hex");
        let lo = (chunk[1] as char).to_digit(16).expect("hex");
        out[i] = ((hi << 4) | lo) as u8;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "vessel_public_read")]
    use crate::PinTable;
    #[cfg(feature = "vessel_public_read")]
    use crate::{Class, decode, peek};

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[9u8; 32])
    }

    fn payload() -> Vec<u8> {
        b"genome-line-v1 demo payload for the writer tests".to_vec()
    }

    #[cfg(feature = "vessel_public_read")]
    fn pins_for(key: &SigningKey) -> PinTable {
        PinTable::with_key(4, key.verifying_key())
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn sign_public_roundtrips_through_the_reader() {
        let key = test_key();
        let pins = pins_for(&key);
        let minted = sign_public(&key, 4, 1, [0u8; 32], &payload()).expect("mint");
        let verified = decode(&minted.bytes, &pins).expect("the mint must open");
        assert_eq!(verified.payload(), payload().as_slice());
        assert_eq!(verified.header().class, Class::PublicRelease);
        assert_eq!(verified.header().key_id, 4);
        assert_eq!(verified.commitment(), minted.commitment);
        // The commitment is over the SIGNED region — equals a recompute.
        let (header, _) = peek(&minted.bytes).expect("peek");
        assert_eq!(commitment_of(&header, &payload()), minted.commitment);
    }

    #[test]
    fn minting_is_deterministic_same_inputs_same_bytes() {
        let key = test_key();
        let a = sign_public(&key, 4, 1, [0u8; 32], &payload()).expect("mint");
        let b = sign_public(&key, 4, 1, [0u8; 32], &payload()).expect("mint");
        assert_eq!(
            a.bytes, b.bytes,
            "ed25519 is deterministic: identical inputs mint identical bytes"
        );
        assert_eq!(a.commitment, b.commitment);
        // A different key-id or version is a different vessel.
        let c = sign_public(&key, 5, 1, [0u8; 32], &payload()).expect("mint");
        assert_ne!(a.bytes, c.bytes);
        let d = sign_public(&key, 4, 2, [0u8; 32], &payload()).expect("mint");
        assert_ne!(a.commitment, d.commitment);
    }

    #[test]
    fn the_public_cap_is_enforced_at_write_time_with_a_result() {
        let key = test_key();
        let big = vec![0u8; MAX_PAYLOAD + 1];
        let err = sign_public(&key, 4, 1, [0u8; 32], &big).expect_err("over-cap refused");
        assert_eq!(
            err,
            VesselError::PayloadTooLarge {
                len: big.len() as u64,
                cap: MAX_PAYLOAD
            }
        );
        // Exactly at the cap is legal.
        let at_cap = vec![0u8; MAX_PAYLOAD];
        assert!(sign_public(&key, 4, 1, [0u8; 32], &at_cap).is_ok());
    }

    #[test]
    fn seed_hex_parsing_is_strict() {
        assert!(signing_key_from_seed_hex(&"ab".repeat(32)).is_ok());
        assert!(signing_key_from_seed_hex(&"AB".repeat(32)).is_ok());
        // Wrong length, non-hex, 0x prefix, whitespace — all refuse.
        assert!(signing_key_from_seed_hex("ab").is_err());
        assert!(signing_key_from_seed_hex(&"zz".repeat(32)).is_err());
        assert!(signing_key_from_seed_hex(&format!("0x{}", "ab".repeat(32))).is_err());
        assert!(signing_key_from_seed_hex(&format!(" {}\n", "ab".repeat(32))).is_err());
    }

    #[test]
    fn key_file_reads_hex_text_and_raw_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "reflexer_vessel_writer_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        // 64-hex text with a trailing newline (the common editor shape).
        let hex_path = dir.join("key.hex");
        std::fs::write(&hex_path, format!("{}\n", "11".repeat(32))).expect("write");
        let a = signing_key_from_file(&hex_path).expect("hex file");
        assert_eq!(
            verifying_key_hex(&a),
            verifying_key_hex(&signing_key_from_seed_hex(&"11".repeat(32)).unwrap())
        );
        // 32 raw bytes.
        let raw_path = dir.join("key.bin");
        std::fs::write(&raw_path, [9u8; 32]).expect("write");
        let b = signing_key_from_file(&raw_path).expect("raw file");
        assert_eq!(verifying_key_hex(&b), verifying_key_hex(&test_key()));
        // A minter's key opens the vessel it minted (needs the public
        // reader compiled — the same crate's capability gate).
        #[cfg(feature = "vessel_public_read")]
        {
            let pins = PinTable::with_key(1, b.verifying_key());
            let minted = sign_public(&b, 1, 1, [0u8; 32], &payload()).expect("mint");
            assert!(decode(&minted.bytes, &pins).is_ok());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "vessel_public_read")]
    #[test]
    fn every_byte_tamper_of_a_minted_vessel_closes_the_door() {
        let key = test_key();
        let pins = pins_for(&key);
        let minted = sign_public(&key, 4, 1, [0u8; 32], &payload()).expect("mint");
        for i in 0..minted.bytes.len() {
            let mut tampered = minted.bytes.clone();
            tampered[i] ^= 0x01;
            assert!(
                decode(&tampered, &pins).is_err(),
                "byte {i} flipped and the vessel still opened"
            );
        }
    }

    #[test]
    fn the_writer_only_produces_the_public_class() {
        let key = test_key();
        let minted = sign_public(&key, 4, 1, [0u8; 32], &payload()).expect("mint");
        assert_eq!(
            minted.bytes[12..16],
            0u32.to_le_bytes(),
            "flags bit0 (class) is 0 = public"
        );
    }
}
