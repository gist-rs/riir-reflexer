//! The manifest-schema v0 fixtures validate against the SPEC's
//! vocabulary (`.docs/04_vessel_format/artifact_manifest_spec.md`).
//!
//! Until `artifact-sync lint` (riir-deployer Issue 012 / Plan 623 T7)
//! exists, THIS is the schema's mechanical pin: every fixture row parses,
//! every enum value is in vocabulary, and the field laws hold (dek_scope
//! required on protected non-keys rows; required ABSENT on public rows
//! and on `keys` rows; public rows carry no `provenance`). The linter, when
//! it lands, must agree with these checks or the disagreement is a finding.

use serde::Deserialize;

const PUBLIC_ROWS: &str = include_str!("fixtures/manifest_v0/public_rows.toml");
const PROTECTED_SHAPE: &str = include_str!("fixtures/manifest_v0/protected_shape.toml");
const SOURCE_PINS: &str = include_str!("fixtures/manifest_v0/source_pins.toml");

const KINDS: [&str; 5] = ["heads", "weights", "vessels", "corpora", "keys"];
const CLASSES: [&str; 2] = ["public", "protected"];
const ENVS: [&str; 5] = ["none", "localnet", "devnet", "testnet", "mainnet"];
/// `serve-<env>` prefixes over the env vocabulary (minus `none`).
const SERVE_SCOPES: [&str; 4] = [
    "serve-localnet",
    "serve-devnet",
    "serve-testnet",
    "serve-mainnet",
];

#[derive(Deserialize, Clone)]
struct ArtifactRow {
    name: String,
    kind: String,
    class: String,
    #[serde(default)]
    dek_scope: Option<String>,
    env: String,
    blake3_plain: String,
    blake3_cipher: String,
    plain_bytes: u64,
    cipher_bytes: u64,
    #[serde(default)]
    provenance: Option<String>,
    #[serde(default)]
    remote: Vec<String>,
}

/// A `[[source_pin]]` row — the not-an-artifact table (base models,
/// in-house blobs; nothing placed, nothing encrypted). The field set is
/// EXACTLY this struct: an unknown field refuses, same as artifact rows.
#[derive(Deserialize, Clone)]
struct SourcePin {
    name: String,
    sha256: String,
    blake3: String,
    bytes: u64,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    provenance: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "artifact", default)]
    artifacts: Vec<ArtifactRow>,
    #[serde(rename = "source_pin", default)]
    source_pins: Vec<SourcePin>,
}

fn in_vocab(v: &str, vocab: &[&str]) -> bool {
    vocab.contains(&v)
}

/// The shared row-level legality walk — the schema test's core. Returns a
/// plain error string so BOTH the fixture test and the negative arm
/// (unknown enum value → refusal) assert through the SAME predicate: the
/// fixture test needs every row to pass it, the negative arm needs a
/// planted violation to FAIL it. One implementation, two directions.
fn validate_row(row: &ArtifactRow) -> Result<(), String> {
    if !in_vocab(&row.kind, &KINDS) {
        return Err(format!("row {}: kind {:?} off-vocabulary", row.name, row.kind));
    }
    if !in_vocab(&row.class, &CLASSES) {
        return Err(format!("row {}: class {:?} off-vocabulary", row.name, row.class));
    }
    if !in_vocab(&row.env, &ENVS) {
        return Err(format!("row {}: env {:?} off-vocabulary", row.name, row.env));
    }
    for h in [&row.blake3_plain, &row.blake3_cipher] {
        if h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("row {}: hash {:?} is not 64-hex", row.name, h));
        }
    }
    // Size law: sizes are exact, and a protected ciphertext carries the
    // age envelope — it can never be smaller than the plaintext.
    if row.plain_bytes == 0 || row.cipher_bytes == 0 {
        return Err(format!("row {}: sizes must be exact (non-zero)", row.name));
    }
    if row.class == "protected" && row.cipher_bytes < row.plain_bytes {
        return Err(format!(
            "row {}: protected cipher_bytes {} < plain_bytes {} (the age envelope adds bytes)",
            row.name, row.cipher_bytes, row.plain_bytes
        ));
    }
    match (row.class.as_str(), row.kind.as_str()) {
        ("protected", "keys") => {
            // Wrapped-DEK rows are exempt from dek_scope — REQUIRED ABSENT.
            if row.dek_scope.is_some() {
                return Err(format!("row {}: keys rows must NOT carry dek_scope", row.name));
            }
        }
        ("protected", _) => {
            let scope = row.dek_scope.as_deref().ok_or_else(|| {
                format!("row {}: protected rows REQUIRE dek_scope", row.name)
            })?;
            let legal = scope == "train"
                || scope == "serve-shared"
                || in_vocab(scope, &SERVE_SCOPES);
            if !legal {
                return Err(format!(
                    "row {}: dek_scope {:?} off-vocabulary (train|serve-<env>|serve-shared)",
                    row.name, scope
                ));
            }
        }
        ("public", _) => {
            if row.dek_scope.is_some() {
                return Err(format!("row {}: public rows must NOT carry dek_scope", row.name));
            }
        }
        _ => unreachable!("class checked in vocabulary above"),
    }
    if row.remote.is_empty() {
        return Err(format!("row {}: remote is required (content-addressed)", row.name));
    }
    Ok(())
}

/// The `[[source_pin]]` legality walk — the same shape as `validate_row`:
/// one implementation, positive fixtures and planted negative arms both
/// go through it.
fn validate_source_pin(pin: &SourcePin) -> Result<(), String> {
    for (label, h) in [("sha256", &pin.sha256), ("blake3", &pin.blake3)] {
        if h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("pin {}: {label} {:?} is not 64-hex", pin.name, h));
        }
    }
    if pin.bytes == 0 {
        return Err(format!("pin {}: bytes must be exact (non-zero)", pin.name));
    }
    if pin.source_url.is_none() && pin.provenance.is_none() {
        return Err(format!(
            "pin {}: a pin names its origin (source_url) or its maker (provenance) — neither present",
            pin.name
        ));
    }
    Ok(())
}

fn validate_manifest(toml_text: &str, label: &str) -> Manifest {
    let m: Manifest =
        toml::from_str(toml_text).unwrap_or_else(|e| panic!("{label}: fixture must parse: {e}"));
    for row in &m.artifacts {
        if let Err(why) = validate_row(row) {
            panic!("{label}: {why}");
        }
    }
    for pin in &m.source_pins {
        if let Err(why) = validate_source_pin(pin) {
            panic!("{label}: {why}");
        }
    }
    m
}

#[test]
fn public_rows_fixture_is_schema_legal() {
    let m = validate_manifest(PUBLIC_ROWS, "public_rows.toml");
    // The public-repo law, on the fixture that models it: zero protected
    // rows, and no provenance (inventory references stay private).
    assert!(
        m.artifacts.iter().all(|r| r.class == "public"),
        "public fixture must model the public-only manifest"
    );
    assert!(
        m.artifacts.iter().all(|r| r.provenance.is_none()),
        "public rows carry no provenance"
    );
}

#[test]
fn protected_shape_fixture_is_schema_legal() {
    let m = validate_manifest(PROTECTED_SHAPE, "protected_shape.toml");
    // The shape fixture must actually EXERCISE the protected vocabulary:
    // at least one true protected row and one keys row, so the dek_scope
    // laws below are pinned by presence, not vacuously.
    assert!(
        m.artifacts.iter().any(|r| r.class == "protected" && r.kind != "keys"),
        "fixture must carry a protected non-keys row"
    );
    assert!(
        m.artifacts.iter().any(|r| r.kind == "keys"),
        "fixture must carry a keys row"
    );
    assert!(
        m.artifacts
            .iter()
            .any(|r| r.dek_scope.as_deref() == Some("serve-shared")),
        "fixture must carry the serve-shared relaxation"
    );
}

#[test]
fn planted_off_vocabulary_values_are_refused() {
    // The negative arms: the validator must CATCH each planted violation
    // class, not just happen to pass well-formed fixtures. Each case
    // takes a valid row and breaks ONE field.
    let base = ArtifactRow {
        name: "planted".into(),
        kind: "heads".into(),
        class: "protected".into(),
        dek_scope: Some("train".into()),
        env: "none".into(),
        blake3_plain: "0".repeat(64),
        blake3_cipher: "0".repeat(64),
        plain_bytes: 1,
        cipher_bytes: 2,
        provenance: None,
        remote: vec!["r2://bucket/deadbeef".into()],
    };

    let mut bad_kind = base.clone();
    bad_kind.kind = "models".into(); // not in KINDS
    assert!(validate_row(&bad_kind).is_err(), "off-vocabulary kind must refuse");

    let mut bad_class = base.clone();
    bad_class.class = "confidential".into();
    assert!(validate_row(&bad_class).is_err(), "off-vocabulary class must refuse");

    let mut bad_env = base.clone();
    bad_env.env = "staging".into();
    assert!(validate_row(&bad_env).is_err(), "off-vocabulary env must refuse");

    let mut bad_scope = base.clone();
    bad_scope.dek_scope = Some("serve-staging".into());
    assert!(validate_row(&bad_scope).is_err(), "off-vocabulary dek_scope must refuse");

    let mut missing_scope = base.clone();
    missing_scope.dek_scope = None;
    assert!(validate_row(&missing_scope).is_err(), "protected without dek_scope must refuse");

    let mut keys_with_scope = base.clone();
    keys_with_scope.kind = "keys".into();
    assert!(validate_row(&keys_with_scope).is_err(), "keys row WITH dek_scope must refuse");

    let mut public_with_scope = base.clone();
    public_with_scope.class = "public".into();
    assert!(validate_row(&public_with_scope).is_err(), "public row with dek_scope must refuse");

    let mut short_hash = base.clone();
    short_hash.blake3_plain = "abcd".into();
    assert!(validate_row(&short_hash).is_err(), "non-64-hex hash must refuse");

    let mut no_remote = base.clone();
    no_remote.remote = vec![];
    assert!(validate_row(&no_remote).is_err(), "empty remote must refuse");

    let mut cipher_below_plain = base.clone();
    cipher_below_plain.plain_bytes = 2;
    cipher_below_plain.cipher_bytes = 1;
    assert!(
        validate_row(&cipher_below_plain).is_err(),
        "protected cipher smaller than plain must refuse"
    );
    let mut zero_size = base.clone();
    zero_size.plain_bytes = 0;
    assert!(validate_row(&zero_size).is_err(), "zero size must refuse");
}

#[test]
fn source_pins_fixture_is_schema_legal() {
    let m = validate_manifest(SOURCE_PINS, "source_pins.toml");
    // The fixture must EXERCISE both origin laws: one row with source_url,
    // one in-house row with provenance (never both absent).
    assert!(
        m.source_pins.iter().any(|p| p.source_url.is_some()),
        "fixture must carry a source_url pin"
    );
    assert!(
        m.source_pins.iter().any(|p| p.provenance.is_some()),
        "fixture must carry an in-house (provenance) pin"
    );
    // `note` is optional re-derivation context, but the fixture exercises it.
    assert!(
        m.source_pins.iter().any(|p| p.note.is_some()),
        "fixture exercises the optional note field"
    );
}

#[test]
fn planted_source_pin_violations_are_refused() {
    let base = SourcePin {
        name: "planted".into(),
        sha256: "0".repeat(64),
        blake3: "1".repeat(64),
        bytes: 1,
        source_url: Some("https://example.invalid/x".into()),
        provenance: None,
        note: None,
    };

    let mut short_sha = base.clone();
    short_sha.sha256 = "abcd".into();
    assert!(validate_source_pin(&short_sha).is_err(), "non-64-hex sha256 must refuse");

    let mut short_blake3 = base.clone();
    short_blake3.blake3 = "abcd".into();
    assert!(validate_source_pin(&short_blake3).is_err(), "non-64-hex blake3 must refuse");

    let mut zero_bytes = base.clone();
    zero_bytes.bytes = 0;
    assert!(validate_source_pin(&zero_bytes).is_err(), "zero size must refuse");

    let mut originless = base.clone();
    originless.source_url = None;
    assert!(
        validate_source_pin(&originless).is_err(),
        "a pin with neither source_url nor provenance must refuse"
    );
}

#[test]
fn unknown_fields_are_refused_by_the_parser() {
    // Schema drift dies at the field-set check: the LAW is that the row
    // field set is EXACTLY the schema's (unknown field = refuse). The
    // linter (T7) implements the refusal; this test pins the detection on
    // a planted drift field via the same table walk it will use.
    let drifted = r#"
        [[artifact]]
        name = "drift"
        kind = "heads"
        class = "public"
        env = "none"
        blake3_plain = "0000000000000000000000000000000000000000000000000000000000000001"
        blake3_cipher = "0000000000000000000000000000000000000000000000000000000000000002"
        plain_bytes = 1
        cipher_bytes = 1
        remote = ["hf://example/x"]
        not_in_the_schema = true
    "#;
    let schema_fields = [
        "name",
        "kind",
        "class",
        "dek_scope",
        "env",
        "blake3_plain",
        "blake3_cipher",
        "plain_bytes",
        "cipher_bytes",
        "provenance",
        "remote",
    ];
    const SOURCE_PIN_FIELDS: [&str; 7] = [
        "name", "sha256", "blake3", "bytes", "source_url", "provenance", "note",
    ];
    #[derive(Deserialize)]
    struct LooseManifest {
        #[serde(rename = "artifact", default)]
        artifacts: Vec<toml::Table>,
    }
    let m: LooseManifest = toml::from_str(drifted).expect("parses as loose tables");
    let unknown: Vec<&String> = m.artifacts[0]
        .keys()
        .filter(|k| !schema_fields.contains(&k.as_str()))
        .collect();
    assert_eq!(
        unknown,
        vec!["not_in_the_schema"],
        "the planted drift field is detected"
    );
    // And the LEGAL fixtures carry no unknown fields under the same walk.
    for (label, text) in [("public", PUBLIC_ROWS), ("protected", PROTECTED_SHAPE)] {
        let m: LooseManifest = toml::from_str(text).expect("legal fixtures parse");
        for row in &m.artifacts {
            let unknown: Vec<&String> = row
                .keys()
                .filter(|k| !schema_fields.contains(&k.as_str()))
                .collect();
            assert!(unknown.is_empty(), "{label}: unknown fields {unknown:?}");
        }
    }

    // Source pins get the same walk: the fixture is clean, and a planted
    // drift field on a pin is detected.
    #[derive(Deserialize)]
    struct LoosePins {
        #[serde(rename = "source_pin", default)]
        pins: Vec<toml::Table>,
    }
    for row in toml::from_str::<LoosePins>(SOURCE_PINS)
        .expect("source fixture parses")
        .pins
    {
        let unknown: Vec<&String> = row
            .keys()
            .filter(|k| !SOURCE_PIN_FIELDS.contains(&k.as_str()))
            .collect();
        assert!(unknown.is_empty(), "source_pins.toml: unknown fields {unknown:?}");
    }
    let drifted_pin = r#"
        [[source_pin]]
        name = "drift_pin"
        sha256 = "0000000000000000000000000000000000000000000000000000000000000003"
        blake3 = "0000000000000000000000000000000000000000000000000000000000000004"
        bytes = 1
        source_url = "https://example.invalid/x"
        not_in_the_schema = true
    "#;
    let m: LoosePins = toml::from_str(drifted_pin).expect("parses as loose tables");
    let unknown: Vec<&String> = m.pins[0]
        .keys()
        .filter(|k| !SOURCE_PIN_FIELDS.contains(&k.as_str()))
        .collect();
    assert_eq!(
        unknown,
        vec!["not_in_the_schema"],
        "the planted pin drift field is detected"
    );
}
