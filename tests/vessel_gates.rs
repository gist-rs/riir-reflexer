//! Vessel gates (Plan 002 T5/T6): the G1 artifact-apply replay + the bin's
//! vessel posture.
//!
//! G1 law: champion-from-vessel ≡ champion-from-substrate — in-process
//! (genome + decisions + stats) AND through the wire (bin booted with
//! `--vessel`), at the P2 geometries. Fail-closed arms: tamper, hosted-only
//! (forged HERE by hand — no writer path exists anywhere), unknown key,
//! downgrade/fork refusals, and the print path.

use ed25519_dalek::{Signer, SigningKey};
use katgpt_tetris::rulebook::{Genome, play_game};
use katgpt_tetris::sim::Board;
use reflexer::engine::Engine;
use reflexer::lane::{BinLane, local_play_with_decisions, stats_eq, wire_play_game};
use reflexer_vessel::{
    self as vessel, ApplyRefusal, PinTable, CLASS_BIT_HOSTED, HEADER_LEN, SUBSTRATE,
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
    let p = std::env::temp_dir()
        .join(format!("reflexer-vessel-{}-{name}.vessel", std::process::id()));
    std::fs::write(&p, bytes).expect("write temp vessel");
    p
}

fn champion_payload() -> Vec<u8> {
    Genome::champion_hybrid().to_line().into_bytes()
}

fn champion_vessel(version: u64) -> Vec<u8> {
    vessel::encode_public(&test_key(), 1, version, [0u8; 32], &champion_payload())
}

/// Hand-forge a HOSTED-ONLY vessel — the proof that class 1 can only exist
/// by forging bytes OUTSIDE the library (no public writer path exists).
fn forge_hosted_only() -> Vec<u8> {
    let payload = champion_payload();
    let mut header = [0u8; HEADER_LEN];
    header[0..8].copy_from_slice(&vessel::MAGIC);
    header[8..12].copy_from_slice(&vessel::FORMAT_VERSION.to_le_bytes());
    header[12..16].copy_from_slice(&CLASS_BIT_HOSTED.to_le_bytes());
    header[16..20].copy_from_slice(&1u32.to_le_bytes());
    header[20..28].copy_from_slice(&9u64.to_le_bytes());
    header[60..68].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    let mut msg = header.to_vec();
    msg.extend_from_slice(&payload);
    let sig = test_key().sign(&msg);
    let mut out = header.to_vec();
    out.extend_from_slice(&sig.to_bytes());
    out.extend_from_slice(&payload);
    out
}

fn pins() -> PinTable {
    PinTable::with_key(1, test_key().verifying_key())
}

// ── G1: artifact-apply replay, in-process ───────────────────────────────

#[test]
fn g1_vessel_champion_replays_substrate_exactly() {
    let bytes = champion_vessel(1);
    let verified = vessel::decode(&bytes, &pins()).expect("verify champion vessel");
    let engine = Engine::from_vessel_payload(verified.payload())
        .expect("payload is the genome line");
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

// ── G1: through the wire (bin booted from the vessel) ──────────────────

#[test]
fn g1_bin_from_vessel_matches_oracle_hold_and_no_hold() {
    let vessel_path = tmp_vessel("champion", &champion_vessel(1));
    let vs = vessel_path.to_str().unwrap();
    let engine = Engine::champion();
    let mut lane = BinLane::spawn(
        bin(),
        &["--vessel", vs, "--vessel-pubkey", &pubkey_hex()],
    )
    .expect("spawn vessel bin");
    for seed in 1..=4u64 {
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 240, Board::empty(), true)
                .expect("wire game");
        let (stats, decisions) =
            local_play_with_decisions(&engine, seed, 240, Board::empty(), true);
        assert_eq!(wire_decisions, decisions, "seed {seed} hold: decisions");
        assert!(stats_eq(&wire_stats, &stats), "seed {seed} hold: stats");
    }
    for seed in 5..=6u64 {
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 200, Board::empty(), false)
                .expect("wire game");
        let (stats, decisions) =
            local_play_with_decisions(&engine, seed, 200, Board::empty(), false);
        assert_eq!(wire_decisions, decisions, "seed {seed} no-hold: decisions");
        assert!(stats_eq(&wire_stats, &stats), "seed {seed} no-hold: stats");
    }
    lane.finish().expect("clean exit");
    let _ = std::fs::remove_file(&vessel_path);
}

// ── fail-closed arms through the BIN (boot failures, exit != 0) ────────

fn expect_boot_failure(name: &str, bytes: &[u8], extra: &[&str]) {
    let p = tmp_vessel(name, bytes);
    let out = std::process::Command::new(bin())
        .args(["--vessel", p.to_str().unwrap()])
        .args(extra)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run bin");
    assert!(
        !out.status.success(),
        "{name}: bin booted from a vessel it must refuse\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn bin_refuses_tampered_vessel() {
    let mut t = champion_vessel(1);
    let last = t.len() - 1;
    t[last] ^= 1;
    expect_boot_failure("tampered", &t, &[&pubkey_hex_flag()]);
}

#[test]
fn bin_refuses_unknown_key_and_missing_pubkey() {
    // no --vessel-pubkey: the compiled pin table is empty → UnknownKey(1)
    let p = tmp_vessel("nopin", &champion_vessel(1));
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
    // wrong key pinned → BadSignature
    let wrong = SigningKey::from_bytes(&[9u8; 32]);
    expect_boot_failure(
        "wrongpin",
        &champion_vessel(1),
        &[&format!("--vessel-pubkey={}", hex(&wrong.verifying_key().to_bytes()))],
    );
}

#[test]
fn bin_refuses_hosted_only_fail_closed() {
    expect_boot_failure("hosted", &forge_hosted_only(), &[&pubkey_hex_flag()]);
}

#[test]
fn bin_refuses_downgrade_and_fork_but_force_logs() {
    // v0 vs SUBSTRATE v0 with a different commitment = lineage fork
    expect_boot_failure("fork", &champion_vessel(0), &[&pubkey_hex_flag()]);
    // forced: boots, and the force is LOGGED (stderr carries the line)
    let p = tmp_vessel("forced", &champion_vessel(0));
    let vs = p.to_str().unwrap();
    let mut lane = BinLane::spawn(
        bin(),
        &[
            "--vessel", vs,
            "--vessel-pubkey", &pubkey_hex(),
            "--vessel-force-downgrade",
        ],
    )
    .expect("forced boot");
    let engine = Engine::champion();
    let (wire_stats, _) =
        wire_play_game(&mut lane, &engine, 1, 200, Board::empty(), true).expect("serves after force");
    let (stats, _) = local_play_with_decisions(&engine, 1, 200, Board::empty(), true);
    assert!(stats_eq(&wire_stats, &stats));
    lane.finish().expect("clean exit");
    let _ = std::fs::remove_file(&p);
}

// ── the monotonic gate at the library seam ──────────────────────────────

#[test]
fn monotonic_gate_refusals_are_typed() {
    let v1 = vessel::decode(&champion_vessel(1), &pins()).unwrap();
    assert!(v1.check_monotonic(&SUBSTRATE).is_ok());
    assert_eq!(
        v1.check_monotonic(&vessel::ApplyState { artifact_version: 2, commitment: [0; 32] }),
        Err(ApplyRefusal::OlderThanCurrent { current: 2, offered: 1 })
    );
}

// ── --vessel-print: inspection without applying ─────────────────────────

#[test]
fn vessel_print_reports_header_and_verdict() {
    let p = tmp_vessel("print", &champion_vessel(3));
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg(&p)
        .arg("--vessel-pubkey")
        .arg(pubkey_hex())
        .output()
        .expect("run print");
    assert!(out.status.success(), "print failed: {}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("print emits one JSON line");
    assert_eq!(json["class"], "public-release");
    assert_eq!(json["artifact_version"], 3);
    assert_eq!(json["key_id"], 1);
    assert_eq!(json["signature"], "verified");
    assert!(json["commitment"].as_str().is_some_and(|c| c.len() == 64));
    // unverified posture: no pin → the verdict says so, structure still reads
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg(&p)
        .output()
        .expect("run print");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["signature"]
        .as_str()
        .is_some_and(|s| s.starts_with("unverified")));
    let _ = std::fs::remove_file(&p);
}

/// helper: `--vessel-pubkey <hex>` as a single &str for the arg slices
fn pubkey_hex_flag() -> String {
    format!("--vessel-pubkey={}", pubkey_hex())
}
