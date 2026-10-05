//! The protected-class round-trip laws (Plan 623 T2 acceptance).
//!
//! 1. **Reader transparency**: the vessel layer verifies the DECRYPTED
//!    bytes exactly as raw bytes — decryption is the storage layer's job
//!    (age, `artifact-sync`), and this reader cannot tell and must not
//!    care. Proven with a REVERSIBLE TRANSFORM standing in for age (the
//!    stand-in is declared as such; the real-tool arm below runs when
//!    `age` is installed).
//! 2. **Ciphertext grants no read access**: a file that DECLARS the
//!    hosted-only class refuses here even when it arrives
//!    post-decryption — the class bit is in the SIGNED header, so no
//!    storage-layer hop can launder a hosted artifact onto this lane.
//! 3. **Real age round-trip (skip-loud)**: with the `age` binary on
//!    PATH, a signed vessel is age-encrypted to an EPHEMERAL in-test
//!    identity (created under the pid-scoped temp dir, deleted in the
//!    same test — test plumbing, NOT the key ritual) and the decrypted
//!    bytes re-verify byte-identically through the stock pins.

use ed25519_dalek::SigningKey;
use reflexer_vessel as vessel;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn signed_public(k: &SigningKey, key_id: u32, payload: &[u8]) -> Vec<u8> {
    vessel::writer::sign_public(k, key_id, 1, [0u8; 32], payload)
        .expect("payload within cap")
        .bytes
}

/// The reversible stand-in for the age layer: XOR against a fixed pad.
/// NOT cryptography — it exists to prove the reader treats post-decryption
/// bytes identically to raw bytes. The real confidentiality law is age's,
/// exercised by the skip-loud arm below.
fn fake_wrap(bytes: &[u8], pad: &[u8; 32]) -> Vec<u8> {
    bytes.iter().enumerate().map(|(i, b)| b ^ pad[i % pad.len()]).collect()
}

#[test]
fn decrypted_bytes_verify_exactly_as_raw_bytes() {
    let k = key(61);
    let payload = b"the genome line the vessel carries";
    let raw = signed_public(&k, 1, payload);

    let pad = [0x5Au8; 32];
    let ciphertext = fake_wrap(&raw, &pad);
    assert_ne!(ciphertext, raw, "the stand-in wrap must transform the bytes");
    let decrypted = fake_wrap(&ciphertext, &pad);
    assert_eq!(decrypted, raw, "the stand-in round-trip is lossless");

    // Verify BOTH byte sets through the SAME pins: identical verdict,
    // identical commitment — the vessel layer is transparent to whatever
    // happened to the bytes before `decode` saw them.
    let pins = vessel::PinTable::with_key(1, k.verifying_key());
    let from_raw = vessel::decode(&raw, &pins).expect("raw verifies");
    let from_decrypted = vessel::decode(&decrypted, &pins).expect("decrypted verifies");
    assert_eq!(from_raw, from_decrypted);
    assert_eq!(from_raw.commitment(), from_decrypted.commitment());
    assert_eq!(from_raw.payload(), payload);
}

#[test]
fn post_decryption_hosted_class_still_refuses_here() {
    // The protected shape: a HOSTED-ONLY vessel's bytes (structurally
    // forged here the same way the format crate's own hostile-forgery
    // tests do it is NOT possible — class-1 minting has no writer — so
    // the class bit is flipped on a SIGNED header, which breaks the
    // signature; the reader refuses on the STRUCTURE either way).
    let k = key(62);
    let raw = signed_public(&k, 1, b"payload");
    let mut hosted_bytes = raw.clone();
    hosted_bytes[12..16].copy_from_slice(&vessel::CLASS_BIT_HOSTED.to_le_bytes());

    let pad = [0xC3u8; 32];
    let ciphertext = fake_wrap(&hosted_bytes, &pad);
    let decrypted = fake_wrap(&ciphertext, &pad);

    // The DECLARING bytes refuse at open() before any payload read —
    // raw or post-decryption, identical refusal.
    let tmp = std::env::temp_dir().join(format!(
        "reflexer_protected_roundtrip_{}.rflexvsl",
        std::process::id()
    ));
    std::fs::write(&tmp, &decrypted).expect("write fixture");
    let err = vessel::open(&tmp, &vessel::default_pins()).unwrap_err();
    assert!(
        matches!(err, vessel::VesselError::HostedOnlyPath),
        "post-decryption hosted bytes refuse structurally, got: {err}"
    );
    let _ = std::fs::remove_file(&tmp);

    // And at decode() the flip breaks the strict signature (the class
    // bit is INSIDE the signed region) — either way, no path from
    // ciphertext to a read. The exact class is pinned: BadSignature.
    let err = vessel::decode(&decrypted, &vessel::default_pins()).unwrap_err();
    assert!(
        matches!(err, vessel::VesselError::BadSignature),
        "the flipped class bit breaks the signature, got: {err}"
    );
}

#[test]
fn age_roundtrip_when_tool_present() {
    let Ok(age) = which("age") else {
        eprintln!(
            "SKIP loud: age binary not on PATH — the storage-confidentiality layer's real round-trip runs where it is installed"
        );
        return;
    };
    let Ok(age_keygen) = which("age-keygen") else {
        eprintln!("SKIP loud: age present but age-keygen absent — identity generation is the keygen binary");
        return;
    };

    let run = |bin: &std::path::Path, args: &[&str], stdin: Option<&[u8]>| -> Vec<u8> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut cmd = Command::new(bin)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("tool spawns");
        if let Some(input) = stdin {
            cmd.stdin.as_mut().expect("stdin").write_all(input).expect("write stdin");
        }
        let out = cmd.wait_with_output().expect("tool waits");
        assert!(
            out.status.success(),
            "{} {:?} failed: {}",
            bin.display(),
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    };

    // Ephemeral in-test identity under the pid-scoped temp dir (the
    // shared-temp rule): created here, deleted here, never committed —
    // this is test plumbing, not the key ritual.
    let dir = std::env::temp_dir().join(format!("reflexer_age_rt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let identity = dir.join("identity.txt");
    let id_str = identity.to_str().expect("utf8").to_owned();
    let _ = run(&age_keygen, &["-o", &id_str], None);

    // The recipient is the PUBLIC key in the identity file's comment line
    // (`# public key: age1...`) — age-keygen's own documented layout.
    let identity_text = std::fs::read_to_string(&identity).expect("read identity");
    let pub_line = identity_text
        .lines()
        .find_map(|l| l.split("# public key: ").nth(1))
        .expect("age1 recipient line")
        .to_owned();
    assert!(pub_line.starts_with("age1"), "recipient shape: {pub_line}");

    let k = key(63);
    let raw = signed_public(&k, 1, b"age round trip payload");
    let pins = vessel::PinTable::with_key(1, k.verifying_key());

    // encrypt (armored so the bytes are a well-defined byte stream either
    // way): age -r <recipient> -a
    let cipher = run(&age, &["-r", &pub_line, "-a"], Some(&raw));
    assert_ne!(cipher, raw, "age must transform the bytes");
    // decrypt: age -d -i <identity>
    let plain = run(&age, &["-d", "-i", &id_str], Some(&cipher));
    assert_eq!(plain, raw, "age round-trip is lossless");

    let from_age = vessel::decode(&plain, &pins).expect("decrypted bytes verify");
    let from_raw = vessel::decode(&raw, &pins).expect("raw bytes verify");
    assert_eq!(from_age, from_raw);

    let _ = std::fs::remove_dir_all(&dir);
}

/// PATH lookup without a new dependency (the `which` crate would be one).
fn which(bin: &str) -> Result<std::path::PathBuf, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(())
}
