//! The reflexer bin — line-JSON over stdio.
//!
//! One request line in → one envelope line out. Malformed input answers a
//! typed error envelope and the pipe stays alive; EOF exits 0 (writing the
//! signed trajectory manifest first when `--record` is active).
//!
//! Usage:
//! ```text
//! reflexer [--record <path.jsonl>] [--vessel <path> [--vessel-pubkey <hex64>]
//!          [--vessel-force-downgrade]] [--vessel-print <path>] [--version]
//! ```
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
use reflexer::proto::{ErrorEnvelope, GENOME_ID, PROTO, ProtoError, ResponseEnvelope, codes};
use reflexer_vessel::{self as vessel, PinTable, VesselError};
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
    let record_path = flag_value(&args, "--record");
    let vessel_path = flag_value(&args, "--vessel");
    let pubkey_hex = flag_value(&args, "--vessel-pubkey");
    let force_downgrade = args.iter().any(|a| a == "--vessel-force-downgrade");

    if let Some(path) = flag_value(&args, "--vessel-print") {
        print_vessel(&path, pubkey_hex.as_deref());
        return;
    }
    if pubkey_hex.is_some() && vessel_path.is_none() {
        eprintln!("reflexer: --vessel-pubkey needs --vessel (it pins the key a vessel is verified against)");
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
                let envelope = ResponseEnvelope {
                    proto: PROTO,
                    genome: engine.genome_id(),
                    response,
                    in_engine_decision_ns: ns,
                };
                write_line(
                    &mut out,
                    &serde_json::to_string(&envelope).expect("serialize envelope"),
                );
            }
            Err(err) => {
                let envelope = ErrorEnvelope {
                    proto: PROTO,
                    error: ProtoError {
                        code: err.code.to_string(),
                        message: err.message,
                        request_index,
                    },
                };
                write_line(
                    &mut out,
                    &serde_json::to_string(&envelope).expect("serialize error"),
                );
            }
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
    let mut pins = PinTable::empty();
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
    if let Err(refusal) = verified.check_monotonic(&vessel::SUBSTRATE) {
        if !force {
            eprintln!("reflexer: vessel refused: {refusal}");
            std::process::exit(1);
        }
        eprintln!("reflexer: monotonic gate FORCED by operator flag — downgrade applied (logged)");
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
    let mut pins = PinTable::empty();
    if let Some(hex) = pubkey_hex {
        pins = pins.with_wildcard(parse_pubkey(hex).unwrap_or_else(|e| {
            eprintln!("reflexer: --vessel-pubkey: {e}");
            std::process::exit(2);
        }));
    }
    let sig_verdict: String = match pins.resolve_key(header.key_id) {
        Ok(key) => {
            let sig = ed25519_dalek::Signature::from_bytes(&sig);
            let mut msg = Vec::with_capacity(
                vessel::HEADER_LEN + (buf.len() - vessel::PREFIX_LEN),
            );
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

/// A typed line failure (code + message) — never a panic.
struct LineError {
    code: &'static str,
    message: String,
}

fn handle_line(
    engine: &Engine,
    line: &str,
) -> Result<
    (
        katgpt_core::decision_wire::DecisionRequest,
        katgpt_core::decision_wire::DecisionResponse,
        u64,
    ),
    LineError,
> {
    let request: katgpt_core::decision_wire::DecisionRequest =
        serde_json::from_str(line).map_err(|e| LineError {
            code: codes::BAD_JSON,
            message: e.to_string(),
        })?;
    request.validate().map_err(|e| LineError {
        code: codes::BAD_REQUEST,
        message: format!("{e:?}"),
    })?;
    // Defense in depth: the engine is deterministic and should never
    // panic, but a panic must answer an envelope, not kill the pipe.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.answer(&request)));
    match result {
        Ok(Ok((response, ns))) => Ok((request, response, ns)),
        Ok(Err(e)) => Err(LineError {
            code: e.code(),
            message: e.message(),
        }),
        Err(p) => Err(LineError {
            code: codes::INTERNAL,
            message: format!("engine panicked: {p:?}"),
        }),
    }
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
