//! The reflexer bin — line-JSON over stdio.
//!
//! One request line in → one envelope line out. Malformed input answers a
//! typed error envelope and the pipe stays alive; EOF exits 0 (writing the
//! signed trajectory manifest first when `--record` is active).
//!
//! Usage:
//! ```text
//! reflexer [--record <path.jsonl>] [--vessel <path> [--vessel-pubkey <hex64>]
//!          [--vessel-force-downgrade]] [--vessel-print <path>]
//!          | sign --in <payload> --out <vessel> --key-id <u32>
//!                 [--artifact-version <u64>=1] [--parent <blake3-hex>]
//!                 [--key <hex64> | --key-file <path>]   (env REFLEXER_SIGN_KEY)
//!          | --version
//! ```
//!
//! `sign` (instinct Proposal 001 T4 — the public FORMAT repo owns the
//! public-class writer): mints one PUBLIC-RELEASE vessel from a payload
//! file. Fail-closed without a key — the key resolves from `--key`
//! (64-hex seed), then `--key-file` (64-hex text or 32 raw bytes), then
//! the `REFLEXER_SIGN_KEY` env. The 1 MiB public cap is enforced at
//! write time. On success it prints the vessel's blake3 commitment (the
//! pin consumers record) AND the verifying key hex (the trust anchor the
//! consumer side pins, e.g. `RIIR_REFLEX_HEADS_PUBKEY`) — the loop
//! closes with zero extra crypto tooling. The output is re-verified
//! against a wildcard pin of its own key before it is written: a minter
//! that cannot open its own output has no business shipping it.
//!
//! Vessels (Plan 002 / P3): `--vessel` boots the engine from a signed
//! decision artifact instead of the compiled substrate — verify (strict
//! ed25519 against the pin table), refuse HOSTED-ONLY fail-closed, check
//! the monotonic gate, construct the engine WHOLE, then serve. A vessel
//! failure is a boot failure (exit 1, loud) — never a silently degraded
//! engine. `--vessel-pubkey` is the operator trust anchor (the SEAL
//! `SEAL_VESSEL_PUBKEY` precedent; the compiled-in minting pin table is
//! empty until the first public artifact ships). `--vessel-print`
//! inspects a vessel's header without applying it.

use reflexer::engine::Engine;
use reflexer::proto::{GENOME_ID, PROTO};
use reflexer::serve::{error_envelope, handle_line, success_envelope};
use reflexer_vessel::{self as vessel, VesselError};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!(
            "reflexer {} proto={PROTO} genome={GENOME_ID}",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    if args.get(1).map(String::as_str) == Some("sign") {
        cmd_sign(&args[2..]);
        return;
    }
    let record_path = flag_value(&args, "--record");
    let vessel_path = flag_value(&args, "--vessel");
    let pubkey_hex = flag_value(&args, "--vessel-pubkey");
    let force_downgrade = args.iter().any(|a| a == "--vessel-force-downgrade");

    if let Some(path) = flag_value(&args, "--vessel-print") {
        print_vessel(&path, pubkey_hex.as_deref());
        return;
    }
    if pubkey_hex.is_some() && vessel_path.is_none() {
        eprintln!(
            "reflexer: --vessel-pubkey needs --vessel (it pins the key a vessel is verified against)"
        );
        std::process::exit(2);
    }

    let engine = match vessel_path {
        None => Engine::champion(),
        Some(path) => load_vessel_engine(&path, pubkey_hex.as_deref(), force_downgrade),
    };
    let mut recorder = record_path.map(|p| {
        reflexer::record::TrajectoryRecorder::open(std::path::Path::new(&p))
            .expect("open trajectory record file")
    });

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (at, line) in stdin.lock().lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("reflexer: stdin read error: {e}");
                std::process::exit(1);
            }
        };
        let request_index = at as u64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_line(&engine, trimmed) {
            Ok((request, response, ns)) => {
                if let Some(rec) = recorder.as_mut()
                    && let Err(e) = rec.record(request, &response.answers)
                {
                    eprintln!("reflexer: trajectory record failed: {e}");
                    std::process::exit(1);
                }
                write_line(&mut out, &success_envelope(&engine, response, ns));
            }
            Err(err) => write_line(&mut out, &error_envelope(&err, request_index)),
        }
    }

    if let Some(rec) = recorder {
        match rec.finish(&engine.genome_id()) {
            Ok(path) => eprintln!("reflexer: trajectory manifest written: {}", path.display()),
            Err(e) => {
                eprintln!("reflexer: trajectory manifest failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Boot the engine from a vessel: verify → class-refuse → monotonic →
/// construct → serve. Every failure is a LOUD boot failure (exit 1) — the
/// fallback-to-champion path does not exist, because a vessel that asked
/// to be applied and failed must never be papered over.
fn load_vessel_engine(path: &str, pubkey_hex: Option<&str>, force: bool) -> Engine {
    // The compiled-in minting pins FIRST, then the operator wildcard — a
    // pinned key verifies without any flag (the no-flag path must work
    // the day the first artifact ships), and the operator pin only ADDS
    // trust, never replaces it.
    let mut pins = vessel::default_pins();
    if let Some(hex) = pubkey_hex {
        pins = pins.with_wildcard(parse_pubkey(hex).unwrap_or_else(|e| {
            eprintln!("reflexer: --vessel-pubkey: {e}");
            std::process::exit(2);
        }));
    }
    let verified = vessel::open(std::path::Path::new(path), &pins).unwrap_or_else(|e| {
        eprintln!("reflexer: vessel refused: {e}");
        std::process::exit(1);
    });
    // BOTH downgrade gates: the compiled release floor (a validly-signed
    // OLD artifact cannot be handed to the operator once the floor moved)
    // and the substrate monotonic baseline (catches the v0 lineage fork).
    let floor_refusal = verified
        .check_floor(vessel::MIN_ARTIFACT_VERSION)
        .err()
        .map(|r| r.to_string());
    let mono_refusal = verified
        .check_monotonic(&vessel::SUBSTRATE)
        .err()
        .map(|r| r.to_string());
    // WHICH gate fired (the forced line is the audit trail — round-2 note)
    let (refusal, gate) = match (floor_refusal, mono_refusal) {
        (Some(r), _) => (r, "the release floor"),
        (None, Some(r)) => (r, "the substrate baseline"),
        (None, None) => (String::new(), ""),
    };
    if !refusal.is_empty() {
        if !force {
            eprintln!("reflexer: vessel refused: {refusal}");
            std::process::exit(1);
        }
        eprintln!(
            "reflexer: downgrade gate FORCED by operator flag — applied (logged): \
             artifact v{} bypassed {gate} [floor={}] — {refusal}",
            verified.header().artifact_version,
            vessel::MIN_ARTIFACT_VERSION
        );
    }
    let engine = Engine::from_vessel_payload(verified.payload()).unwrap_or_else(|| {
        eprintln!(
            "reflexer: vessel payload is not a genome line (the whole-snapshot wire) — refused"
        );
        std::process::exit(1);
    });
    eprintln!(
        "reflexer: vessel applied: digest={} version={} key-id={} class={} genome={}",
        verified.commitment_hex(),
        verified.header().artifact_version,
        verified.header().key_id,
        verified.header().class.as_str(),
        engine.genome_id()
    );
    engine
}

fn parse_pubkey(hex: &str) -> Result<ed25519_dalek::VerifyingKey, String> {
    let bytes = hex_decode32(hex.trim())?;
    ed25519_dalek::VerifyingKey::from_bytes(&bytes).map_err(|e| format!("bad verifying key: {e}"))
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

/// `--vessel-print`: inspect a vessel's header — structure always, the
/// signature verdict when a pin is available. Never applies anything.
fn print_vessel(path: &str, pubkey_hex: Option<&str>) {
    let buf = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("reflexer: read {path}: {e}");
        std::process::exit(1);
    });
    let (header, sig) = match vessel::peek(&buf) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("reflexer: vessel refused: {e}");
            std::process::exit(1);
        }
    };
    let commitment = {
        let mut h = blake3::Hasher::new();
        h.update(&buf[0..vessel::HEADER_LEN]);
        h.update(&buf[vessel::PREFIX_LEN..]);
        let mut c = [0u8; 32];
        c.copy_from_slice(h.finalize().as_bytes());
        hex32(&c)
    };
    // Same table the boot path uses (round-2 consistency note): compiled
    // pins first, so inspecting a real artifact works with no flag the
    // day the first artifact ships — the wildcard only ADDS.
    let mut pins = vessel::default_pins();
    if let Some(hex) = pubkey_hex {
        pins = pins.with_wildcard(parse_pubkey(hex).unwrap_or_else(|e| {
            eprintln!("reflexer: --vessel-pubkey: {e}");
            std::process::exit(2);
        }));
    }
    let sig_verdict: String = match pins.resolve_key(header.key_id) {
        Ok(key) => {
            let sig = ed25519_dalek::Signature::from_bytes(&sig);
            let mut msg = Vec::with_capacity(vessel::HEADER_LEN + (buf.len() - vessel::PREFIX_LEN));
            msg.extend_from_slice(&buf[0..vessel::HEADER_LEN]);
            msg.extend_from_slice(&buf[vessel::PREFIX_LEN..]);
            if key.verify_strict(&msg, &sig).is_ok() {
                "verified".to_string()
            } else {
                "BAD".to_string()
            }
        }
        Err(VesselError::UnknownKey(k)) => format!("unverified: no pin for key-id {k}"),
        Err(VesselError::RevokedKey(k)) => format!("REVOKED key-id {k}"),
        Err(_) => "unverified".to_string(),
    };
    println!(
        "{{\"path\":{:?},\"format_version\":{},\"class\":{:?},\"key_id\":{},\"artifact_version\":{},\"parent_commitment\":{:?},\"payload_len\":{},\"file_len\":{},\"commitment\":{:?},\"signature\":{:?}}}",
        path,
        header.format_version,
        header.class.as_str(),
        header.key_id,
        header.artifact_version,
        hex32(&header.parent_commitment),
        header.payload_len,
        buf.len(),
        commitment,
        sig_verdict
    );
}

fn hex32(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in b {
        use std::fmt::Write as _;
        let _ = write!(s, "{byte:02x}");
    }
    s
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    for (i, a) in args.iter().enumerate() {
        if a == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(v) = a.strip_prefix(&prefix) {
            return Some(v.to_string());
        }
    }
    None
}

fn write_line(out: &mut impl Write, json: &str) {
    let _ = writeln!(out, "{json}");
    let _ = out.flush();
}

/// `reflexer sign` — mint one PUBLIC-RELEASE vessel (instinct Proposal
/// 001 T4). Fail-closed on every axis: no key → exit 2 naming all three
/// key sources; an over-cap payload → exit 2 with the bound; a
/// re-verification failure of the freshly minted bytes → exit 1 (never
/// write an output this crate cannot open).
fn cmd_sign(args: &[String]) {
    let Some(input) = flag_value(args, "--in") else {
        eprintln!("reflexer sign: --in <payload-file> is required");
        std::process::exit(2);
    };
    let Some(out_path) = flag_value(args, "--out") else {
        eprintln!("reflexer sign: --out <vessel-path> is required");
        std::process::exit(2);
    };
    let key_id: u32 = match flag_value(args, "--key-id") {
        Some(v) => v.parse().unwrap_or_else(|_| {
            eprintln!("reflexer sign: --key-id must be a u32, got {v:?}");
            std::process::exit(2);
        }),
        None => {
            eprintln!(
                "reflexer sign: --key-id <u32> is required (the PinTable identity of the minting key)"
            );
            std::process::exit(2);
        }
    };
    let artifact_version: u64 = match flag_value(args, "--artifact-version") {
        Some(v) => v.parse().unwrap_or_else(|_| {
            eprintln!("reflexer sign: --artifact-version must be a u64, got {v:?}");
            std::process::exit(2);
        }),
        None => 1,
    };
    let mut parent = [0u8; 32];
    if let Some(hex) = flag_value(args, "--parent") {
        parent = hex_decode32(hex.trim()).unwrap_or_else(|e| {
            eprintln!("reflexer sign: --parent: {e}");
            std::process::exit(2);
        });
    }

    // The key: flag → key-file → env. Fail-closed without one.
    let key = if let Some(hex) = flag_value(args, "--key") {
        vessel::writer::signing_key_from_seed_hex(hex.trim())
    } else if let Some(path) = flag_value(args, "--key-file") {
        vessel::writer::signing_key_from_file(std::path::Path::new(&path))
    } else if let Ok(hex) = std::env::var("REFLEXER_SIGN_KEY") {
        vessel::writer::signing_key_from_seed_hex(hex.trim())
    } else {
        Err(
            "no signing key: pass --key <64-hex-seed>, --key-file <path> \
            (64-hex text or 32 raw bytes), or set REFLEXER_SIGN_KEY"
                .to_string(),
        )
    }
    .unwrap_or_else(|e| {
        eprintln!("reflexer sign: {e}");
        std::process::exit(2);
    });

    // Bound BEFORE the read: the public cap is a write-time law; a
    // hostile-sized --in is refused from metadata, never loaded.
    let input_path = std::path::Path::new(&input);
    let meta = std::fs::metadata(input_path).unwrap_or_else(|e| {
        eprintln!("reflexer sign: --in {}: {e}", input);
        std::process::exit(2);
    });
    if !meta.is_file() || meta.len() > vessel::MAX_PAYLOAD as u64 {
        eprintln!(
            "reflexer sign: payload {} B exceeds the public cap {} B (--in must be a regular file)",
            meta.len(),
            vessel::MAX_PAYLOAD
        );
        std::process::exit(2);
    }
    let payload = std::fs::read(input_path).unwrap_or_else(|e| {
        eprintln!("reflexer sign: read {}: {e}", input);
        std::process::exit(2);
    });

    let minted = vessel::writer::sign_public(&key, key_id, artifact_version, parent, &payload)
        .unwrap_or_else(|e| {
            eprintln!("reflexer sign: mint refused: {e}");
            std::process::exit(2);
        });
    // The mint-side round-trip: verify the freshly minted bytes against
    // a wildcard pin of their own key BEFORE writing anything.
    let pins = vessel::PinTable::empty().with_wildcard(key.verifying_key());
    let verified = vessel::writer::verify_roundtrip(&minted.bytes, &pins).unwrap_or_else(|e| {
        eprintln!("reflexer sign: minted vessel failed re-verification ({e}) — nothing written");
        std::process::exit(1);
    });
    assert_eq!(
        verified.commitment(),
        minted.commitment,
        "re-verify commitment drift"
    );
    if let Some(parent_dir) = std::path::Path::new(&out_path).parent()
        && !parent_dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent_dir).unwrap_or_else(|e| {
            eprintln!("reflexer sign: create {}: {e}", parent_dir.display());
            std::process::exit(2);
        });
    }
    std::fs::write(&out_path, &minted.bytes).unwrap_or_else(|e| {
        eprintln!("reflexer sign: write {out_path}: {e}");
        std::process::exit(1);
    });
    println!(
        "{{\"out\":{:?},\"key_id\":{},\"artifact_version\":{},\"commitment\":{:?},\"verifying_key\":{:?},\"payload_len\":{},\"vessel_len\":{}}}",
        out_path,
        key_id,
        artifact_version,
        hex32(&minted.commitment),
        vessel::writer::verifying_key_hex(&key),
        payload.len(),
        minted.bytes.len(),
    );
    eprintln!(
        "reflexer sign: wrote {} ({} B payload → {} B vessel); pin the commitment beside the artifact, \
         and the verifying key as the consumer-side trust anchor",
        out_path,
        payload.len(),
        minted.bytes.len()
    );
}
