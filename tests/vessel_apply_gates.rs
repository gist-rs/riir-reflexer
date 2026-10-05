//! reflexer Issue 003 gates (Plan 009 P1): the stock build's trust
//! posture. A fork mints its own key, passes it via `--vessel-pubkey`,
//! and expects the official bin to load its vessels — these gates pin
//! every leg of that refusal, plus the persisted apply floor.

use ed25519_dalek::SigningKey;
use katgpt_tetris::rulebook::Genome;
use reflexer_vessel::{self as vessel};

/// A fork's key: deterministic, NOT the authority root (whose private
/// half is not in any test — the compiled pin table holds its public
/// half only).
fn fork_key() -> SigningKey {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&(0xC0FFEE_u64).to_be_bytes());
    SigningKey::from_bytes(&seed)
}

/// A REAL genome payload (the engine must construct for the process
/// gates — a refused/failed boot must be the FLOOR's refusal, never a
/// payload decode error).
fn champion_payload() -> Vec<u8> {
    Genome::champion_hybrid().to_line().into_bytes()
}

fn mint_fork(key: &SigningKey, key_id: u32, version: u64) -> Vec<u8> {
    vessel::encode_public(key, key_id, version, [0u8; 32], &champion_payload())
}

#[test]
fn the_pin_table_carries_the_authority_root() {
    // The Issue 003 T1 posture: non-empty, key-id 1, and the bytes PARSE
    // — `pins_from_bytes` silently SKIPS bad key bytes, so a transcription
    // typo in the table would surface as `UnknownKey` here, not as a
    // crash. Resolution (without any verification claim) must succeed.
    assert_eq!(vessel::DEFAULT_PIN_KEYS.len(), 1);
    assert_eq!(vessel::DEFAULT_PIN_KEYS[0].0, 1);
    vessel::default_pins()
        .resolve_key(1)
        .expect("the authority root key-id 1 must resolve (bytes parsed)");
}

#[test]
fn fork_signed_vessels_are_refused_by_the_stock_pins() {
    let fork = mint_fork(&fork_key(), 1, 1);
    let tmp = tmp_file("fork_vessel");
    std::fs::write(&tmp, &fork).expect("write fork vessel");
    let err = vessel::open(&tmp, &vessel::default_pins()).expect_err("fork must be refused");
    drop(err);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn unknown_key_ids_fail_closed() {
    let fork = mint_fork(&fork_key(), 999, 1);
    let tmp = tmp_file("unknown_key_vessel");
    std::fs::write(&tmp, &fork).expect("write unknown-key vessel");
    let err = vessel::open(&tmp, &vessel::default_pins()).expect_err("unknown key must refuse");
    let _ = err;
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn revoked_key_ids_fail_closed_before_signature_work_matters() {
    let fork = mint_fork(&fork_key(), 1, 1);
    let tmp = tmp_file("revoked_vessel");
    std::fs::write(&tmp, &fork).expect("write revoked vessel");
    let pins = vessel::default_pins().revoke(1);
    let err = vessel::open(&tmp, &pins).expect_err("revoked key must refuse");
    // The revocation must be the REASON (resolve order), not a signature
    // failure that happens to coincide.
    assert!(matches!(err, vessel::VesselError::RevokedKey(1)), "got: {err:?}");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn the_stock_bin_refuses_the_operator_wildcard_flag() {
    // The T1 posture at the PROCESS boundary: a stock build (no dev_pins)
    // exits 2 on `--vessel-pubkey` BEFORE reading any file. The fork's
    // whole bootstrap is the flag; the flag is the refusal.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_reflexer"))
        .arg("--vessel")
        .arg("/nonexistent/vessel")
        .arg("--vessel-pubkey")
        .arg("00".repeat(32))
        .output()
        .expect("spawn the stock bin");
    if cfg!(feature = "dev_pins") {
        // This test binary was built WITH dev_pins — the posture under
        // test is the default one; the dev bin ACCEPTS the flag and then
        // fails loudly on the missing file (fail-closed, exit 1).
        assert_eq!(out.status.code(), Some(1), "missing vessel must fail loudly");
    } else {
        assert_eq!(
            out.status.code(),
            Some(2),
            "the wildcard flag must be a usage refusal (exit 2), stdout={}, stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stderr).contains("Issue 003"));
    }
}

// ── the persisted apply floor (unit gates live in src/vessel_apply.rs;
// here: the process-level shape the operator sees) ─────────────────────

#[test]
fn the_apply_floor_blocks_rollback_across_restarts() {
    use reflexer::vessel_apply::ApplyFloor;

    let dir = tmp_dir("floor_process");
    let key = fork_key();
    let v1 = tmp_in(&dir, "v1.vessel");
    let v2 = tmp_in(&dir, "v2.vessel");
    // Boot v2 with the dev-wildcard posture (floor mechanics, not the
    // trust posture, are under test here): applies, writes the floor.
    // Key-id 7 is deliberately OUTSIDE the compiled table — the operator
    // wildcard only ADDS trust, it never overrides a compiled key-id, so
    // a dev fixture must claim an unpinned id for the wildcard to resolve.
    const DEV_ID: u32 = 7;
    std::fs::write(&v1, mint_fork(&key, DEV_ID, 1)).unwrap();
    std::fs::write(&v2, mint_fork(&key, DEV_ID, 2)).unwrap();
    let state = dir.join("state.json");

    // Boot v2 with the dev-wildcard posture (floor mechanics, not the
    // trust posture, are under test here): applies, writes the floor.
    let ok = std::process::Command::new(env!("CARGO_BIN_EXE_reflexer"))
        .arg("--vessel")
        .arg(&v2)
        .arg("--vessel-state")
        .arg(&state)
        .arg("--vessel-pubkey")
        .arg(hex(&key.verifying_key()))
        .output()
        .expect("spawn bin");
    if !cfg!(feature = "dev_pins") {
        // Stock build refuses the wildcard — the floor mechanics are
        // unit-gated instead; assert the refusal so this test can never
        // silently pass vacuously under either feature posture.
        assert_eq!(out_code(&ok), Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    } else {
        assert_eq!(
            out_code(&ok),
            Some(0),
            "v2 must apply — stdout={}, stderr={}",
            String::from_utf8_lossy(&ok.stdout),
            String::from_utf8_lossy(&ok.stderr)
        );
        assert!(state.exists(), "boot must persist the floor");
        // Rollback to v1: refused (exit 1), and the floor is UNCHANGED.
        let back = std::process::Command::new(env!("CARGO_BIN_EXE_reflexer"))
            .arg("--vessel")
            .arg(&v1)
            .arg("--vessel-state")
            .arg(&state)
            .arg("--vessel-pubkey")
            .arg(hex(&key.verifying_key()))
            .output()
            .expect("spawn bin for rollback");
        assert_eq!(out_code(&back), Some(1), "rollback must refuse");
        assert!(String::from_utf8_lossy(&back.stderr).contains("rollback"));
        let floor = ApplyFloor::read(&state).unwrap().expect("floor intact");
        assert_eq!(floor.artifact_version, 2);
        assert_eq!(floor.key_id, DEV_ID);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ── helpers ──────────────────────────────────────────────────────────────

fn out_code(out: &std::process::Output) -> Option<i32> {
    out.status.code()
}

fn hex(b: &ed25519_dalek::VerifyingKey) -> String {
    b.as_bytes().iter().map(|x| format!("{x:02x}")).collect()
}

/// The shared-temp-path law: fixed /tmp names race concurrent processes —
/// every fixture carries the pid (katgpt-rs Issue 832's class).
fn tmp_file(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("reflexer_003_{name}_{}", std::process::id()))
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("reflexer_003_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp dir");
    d
}

fn tmp_in(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    dir.join(name)
}
