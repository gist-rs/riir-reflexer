//! Vessel gates (Plan 002 T5/T6): the G1 artifact-apply replay + the bin's
//! STOCK vessel posture.
//!
//! G1 law: champion-from-vessel ≡ champion-from-substrate — in-process
//! (genome + decisions + stats) at the P2 geometries. Fail-closed arms:
//! unknown key, a fork claiming the compiled AUTHORITY key-id, and the
//! Issue 003 T1 posture itself (the operator wildcard flag refused by a
//! stock build).
//!
//! The wildcard-dependent bin scenarios (dev trust posture) live in
//! `tests/vessel_dev_gates.rs` — `required-features = ["dev_pins"]` —
//! since Issue 003 T1 a stock build refuses `--vessel-pubkey` outright.

use ed25519_dalek::SigningKey;
use katgpt_tetris::rulebook::{Genome, play_game};
use katgpt_tetris::sim::Board;
use reflexer::engine::Engine;
use reflexer::lane::{local_play_with_decisions, stats_eq};
use reflexer_vessel::{
    self as vessel, ApplyRefusal, PinTable, SUBSTRATE,
};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflexer")
}

fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn pubkey_hex() -> String {
    hex(&test_key().verifying_key().to_bytes())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn tmp_vessel(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    // pid-scoped temp path (the shared-temp law — concurrent test processes
    // never collide)
    let p = std::env::temp_dir().join(format!(
        "reflexer-vessel-{}-{name}.vessel",
        std::process::id()
    ));
    std::fs::write(&p, bytes).expect("write temp vessel");
    p
}

fn champion_payload() -> Vec<u8> {
    Genome::champion_hybrid().to_line().into_bytes()
}

fn champion_vessel(version: u64) -> Vec<u8> {
    champion_vessel_at(1, version)
}

/// The fixture minter with an explicit key-id (the stock-posture arms
/// need fixtures at BOTH the compiled authority id and an unpinned one).
fn champion_vessel_at(key_id: u32, version: u64) -> Vec<u8> {
    vessel::encode_public(&test_key(), key_id, version, [0u8; 32], &champion_payload())
}

fn pins() -> PinTable {
    PinTable::with_key(1, test_key().verifying_key())
}

// ── G1: artifact-apply replay, in-process ───────────────────────────────

#[test]
fn g1_vessel_champion_replays_substrate_exactly() {
    let bytes = champion_vessel(1);
    let verified = vessel::decode(&bytes, &pins()).expect("verify champion vessel");
    let engine =
        Engine::from_vessel_payload(verified.payload()).expect("payload is the genome line");
    let champion = Engine::champion();
    assert_eq!(
        engine.genome_id(),
        champion.genome_id(),
        "vessel genome must BE the compiled champion genome"
    );
    for seed in 1..=4u64 {
        let (v_stats, v_decisions) =
            local_play_with_decisions(&engine, seed, 240, Board::empty(), true);
        let (c_stats, c_decisions) =
            local_play_with_decisions(&champion, seed, 240, Board::empty(), true);
        assert_eq!(v_decisions, c_decisions, "seed {seed}: decisions diverged");
        assert!(stats_eq(&v_stats, &c_stats), "seed {seed}: stats diverged");
        // and the substrate oracle itself
        let reference = play_game(engine.genome(), seed, 240, Board::empty());
        assert!(stats_eq(&v_stats, &reference), "seed {seed}: vs play_game");
    }
}

// ── fail-closed arms through the BIN (stock posture) ───────────────────

#[test]
fn bin_refuses_unknown_key_and_missing_pubkey() {
    // no --vessel-pubkey, key-id 999: nothing pins it → UnknownKey.
    let p = tmp_vessel("nopin", &champion_vessel_at(999, 1));
    let out = std::process::Command::new(bin())
        .arg("--vessel")
        .arg(&p)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run bin");
    assert!(!out.status.success(), "booted without any pin");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not pinned"),
        "stderr should name the unknown key: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&p);
    // no flag, key-id 1 (the compiled AUTHORITY pin): a test-key vessel
    // claiming the authority id fails the SIGNATURE gate — the fork
    // posture Issue 003 exists to close.
    let p = tmp_vessel("fork-id1", &champion_vessel_at(1, 1));
    let out = std::process::Command::new(bin())
        .arg("--vessel")
        .arg(&p)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run bin");
    assert!(!out.status.success(), "a fork claiming key-id 1 booted");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("signature"),
        "stderr should name the signature failure: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&p);
    // the flag itself is the stock refusal (exit 2, naming Issue 003)
    let out = std::process::Command::new(bin())
        .arg("--vessel")
        .arg("/nonexistent")
        .arg("--vessel-pubkey")
        .arg(pubkey_hex())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run bin");
    #[cfg(not(feature = "dev_pins"))]
    {
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("Issue 003"));
    }
    #[cfg(feature = "dev_pins")]
    assert_eq!(out.status.code(), Some(1), "missing file must fail loudly");
}

// ── the monotonic gate at the library seam ──────────────────────────────

#[test]
fn monotonic_gate_refusals_are_typed() {
    let v1 = vessel::decode(&champion_vessel(1), &pins()).unwrap();
    assert!(v1.check_monotonic(&SUBSTRATE).is_ok());
    assert_eq!(
        v1.check_monotonic(&vessel::ApplyState {
            artifact_version: 2,
            commitment: [0; 32]
        }),
        Err(ApplyRefusal::OlderThanCurrent {
            current: 2,
            offered: 1
        })
    );
}

// ── --vessel-print: inspection without applying (stock posture) ─────────

#[test]
fn vessel_print_reports_header_and_verdict() {
    // Unpinned key-id: the verdict names the gap, structure still reads.
    let p = tmp_vessel("print", &champion_vessel_at(999, 3));
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg(&p)
        .output()
        .expect("run print");
    assert!(out.status.success(), "print failed");
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("print emits one JSON line");
    assert_eq!(json["class"], "public-release");
    assert_eq!(json["artifact_version"], 3);
    assert_eq!(json["key_id"], 999);
    assert!(
        json["signature"]
            .as_str()
            .is_some_and(|s| s.starts_with("unverified: no pin")),
        "verdict was: {}",
        json["signature"]
    );
    let _ = std::fs::remove_file(&p);
    // A test-key vessel claiming the AUTHORITY id: resolved against the
    // real pin, verdict BAD (the fork visibility arm).
    let p = tmp_vessel("print-bad", &champion_vessel_at(1, 3));
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg(&p)
        .output()
        .expect("run print");
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["signature"], "BAD");
    let _ = std::fs::remove_file(&p);
    // The wildcard flag on print is the same stock refusal (exit 2).
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg("/nonexistent")
        .arg("--vessel-pubkey")
        .arg(pubkey_hex())
        .output()
        .expect("run print");
    #[cfg(not(feature = "dev_pins"))]
    assert_eq!(out.status.code(), Some(2));
    #[cfg(feature = "dev_pins")]
    assert_eq!(out.status.code(), Some(1), "missing file must fail loudly");
}
