//! The DEV-posture vessel gates (`dev_pins` builds only — Issue 003 T1
//! split the trust postures): every bin-level scenario that needs the
//! operator wildcard `--vessel-pubkey`.
//!
//! A stock build REFUSES the flag outright (see `vessel_apply_gates.rs`
//! for that gate) — these scenarios are the dev trust posture by design.
//! The fixtures mint under key-id 7: the wildcard only ADDS trust, it
//! never overrides a COMPILED key-id, and key-id 1 is the authority root
//! now (`DEFAULT_PIN_KEYS`) — a wildcard fixture claiming id 1 would
//! verify against the authority pin and fail for the wrong reason.

use ed25519_dalek::SigningKey;
use katgpt_tetris::rulebook::Genome;
use katgpt_tetris::sim::Board;
use reflexer::engine::Engine;
use reflexer::lane::{BinLane, local_play_with_decisions, stats_eq, wire_play_game};
use reflexer_vessel::{self as vessel, HEADER_LEN, SIG_LEN};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflexer")
}

fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

/// The DEV fixture key-id — outside the compiled table (see the module
/// doc: the wildcard cannot override key-id 1).
const DEV_ID: u32 = 7;

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
        "reflexer-devpin-{}-{name}.vessel",
        std::process::id()
    ));
    std::fs::write(&p, bytes).expect("write temp vessel");
    p
}

fn champion_payload() -> Vec<u8> {
    Genome::champion_hybrid().to_line().into_bytes()
}

fn champion_vessel(version: u64) -> Vec<u8> {
    vessel::encode_public(&test_key(), DEV_ID, version, [0u8; 32], &champion_payload())
}

fn pubkey_hex_flag() -> String {
    format!("--vessel-pubkey={}", pubkey_hex())
}

fn expect_boot_failure(name: &str, bytes: &[u8], extra: &[&str]) {
    let p = tmp_vessel(name, bytes);
    let mut cmd = std::process::Command::new(bin());
    cmd.arg("--vessel").arg(&p).arg("--vessel-state").arg(tmp_state(name));
    for e in extra {
        cmd.arg(e);
    }
    let out = cmd.stdin(std::process::Stdio::null()).output().expect("run bin");
    assert!(
        !out.status.success(),
        "{name} vessel booted (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&p);
}

fn tmp_state(name: &str) -> std::path::PathBuf {
    // Fresh state per scenario — the persisted floor is not under test
    // here (that is vessel_apply_gates).
    std::env::temp_dir().join(format!("reflexer-devpin-{}-{name}.state", std::process::id()))
}

/// Forge a HOSTED-ONLY vessel by header bit-flip (class gate at the bin).
fn forge_hosted_only() -> Vec<u8> {
    let mut b = champion_vessel(1);
    let flags = u32::from_le_bytes(b[12..16].try_into().expect("len")) | 1;
    b[12..16].copy_from_slice(&flags.to_le_bytes());
    // The signature now covers WRONG bytes — but `open` verifies the
    // signature before the class gate, so re-sign the forged header to
    // reach the class refusal for the right reason.
    resign(&mut b);
    b
}

fn resign(v: &mut [u8]) {
    // header ‖ sig ‖ payload — strip the old sig, re-sign, reassemble.
    let payload_at = HEADER_LEN + SIG_LEN;
    let (header, rest) = v.split_at(HEADER_LEN);
    let (_, payload) = rest.split_at(SIG_LEN);
    let mut msg = Vec::with_capacity(header.len() + payload.len());
    msg.extend_from_slice(header);
    msg.extend_from_slice(payload);
    use ed25519_dalek::Signer as _;
    let sig = test_key().sign(&msg);
    let mut out = Vec::with_capacity(v.len());
    out.extend_from_slice(header);
    out.extend_from_slice(&sig.to_bytes());
    out.extend_from_slice(payload);
    v.copy_from_slice(&out);
    let _ = payload_at;
}

// ── G1 through the wire (dev posture boot) ──────────────────────────────

#[test]
fn g1_bin_from_vessel_matches_oracle_hold_and_no_hold() {
    let vessel_path = tmp_vessel("champion", &champion_vessel(1));
    let vs = vessel_path.to_str().unwrap();
    let engine = Engine::champion();
    let mut lane = BinLane::spawn(bin(), &["--vessel", vs, "--vessel-pubkey", &pubkey_hex()])
        .expect("spawn vessel bin");
    for seed in 1..=4u64 {
        let (wire_stats, wire_decisions) =
            wire_play_game(&mut lane, &engine, seed, 240, Board::empty(), true).expect("wire game");
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
    let mut t = champion_vessel(1);
    let last = t.len() - 1;
    t[last] ^= 1;
    expect_boot_failure("tampered", &t, &[&pubkey_hex_flag()]);
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
            "--vessel",
            vs,
            "--vessel-pubkey",
            &pubkey_hex(),
            "--vessel-force-downgrade",
        ],
    )
    .expect("forced boot");
    let engine = Engine::champion();
    let (wire_stats, _) = wire_play_game(&mut lane, &engine, 1, 200, Board::empty(), true)
        .expect("serves after force");
    let (stats, _) = local_play_with_decisions(&engine, 1, 200, Board::empty(), true);
    assert!(stats_eq(&wire_stats, &stats));
    lane.finish().expect("clean exit");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn wrong_key_pinned_is_a_bad_signature() {
    // The operator pins a key that is not the minting key → BadSignature.
    let wrong = SigningKey::from_bytes(&[9u8; 32]);
    expect_boot_failure(
        "wrongpin",
        &champion_vessel(1),
        &[&format!("--vessel-pubkey={}", hex(&wrong.verifying_key().to_bytes()))],
    );
}

#[test]
fn vessel_print_verifies_under_the_wildcard() {
    let p = tmp_vessel("print", &champion_vessel(3));
    let out = std::process::Command::new(bin())
        .arg("--vessel-print")
        .arg(&p)
        .arg("--vessel-pubkey")
        .arg(pubkey_hex())
        .output()
        .expect("run print");
    assert!(
        out.status.success(),
        "print failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("print emits one JSON line");
    assert_eq!(json["class"], "public-release");
    assert_eq!(json["artifact_version"], 3);
    assert_eq!(json["key_id"], DEV_ID);
    assert_eq!(json["signature"], "verified");
    assert!(json["commitment"].as_str().is_some_and(|c| c.len() == 64));
    let _ = std::fs::remove_file(&p);
}
