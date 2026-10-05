# Issue 004 — Artifact class vocabulary + manifest schema spec + runtime env refusal

**Status:** LANDED 2026-10-05 (Plan 623 T2, delegated-owner verdict round 1: fixture/mock only,
spec ships as **v0 draft** pending the T0 A10 ratification; `core.hooksPath` enrollment stays T8)
— executed in riir-reflexer:

- **Spec + crosswalk + schema v0** — `.docs/04_vessel_format/artifact_manifest_spec.md` (draft
  header, field table incl. `dek_scope` + the `keys` kind, PUBLIC-RELEASE↔public /
  HOSTED-ONLY↔protected+serve-rider crosswalk, naming law, age two-layers-two-jobs, directory
  law); README index row added.
- **Schema fixtures + mechanical pin** — `crates/reflexer-vessel/tests/fixtures/manifest_v0/{public_rows,protected_shape}.toml`
  + `tests/manifest_schema_fixtures.rs` (4 tests: enum/field legality both shapes, planted
  violations refused per class, unknown-field detection — the same table walk T7's linter must
  agree with). Dev-deps `toml` + `serde` (test-only; shipped tree stays blake3 + ed25519-dalek).
- **Runtime per-env pin refusal** — `crates/reflexer-vessel/src/env.rs`: `Env` vocabulary
  (repr(u8) discriminants pinned, `FromStr` refuses unknown), `EnvPinTable` (Default = compiled
  root under `Env::None`, named envs unset; `empty()` = all-unset), `decode_for_env`/`open_for_env`
  (the pin set IS the env binding — cross-env trust is structural; `VesselError::EnvUnpinned`
  names the env, never a silent root fallback); `PinTable::is_empty()` additive. **Additive-only:
  no `decode`/`open` call site changed; unarmed byte-identity pinned by test**
  (`unarmed_none_posture_is_byte_identical_to_the_stock_path`). 9 env tests.
- **Round-trip laws** — `tests/artifact_protected_roundtrip.rs`: reader transparency (decrypted
  bytes verify exactly as raw, stand-in transform), post-decryption hosted class still refuses
  (`HostedOnlyPath` at open / the bit-flip is `BadSignature` at decode — the class bit lives
  inside the signed region), real `age` round-trip skip-loud when `age`+`age-keygen` are on PATH
  (ephemeral in-test identity, pid-scoped temp dir — test plumbing, NOT the key ritual).
- **DEFAULT_PINS law recorded** — the spec's "first public artifact" section: the first
  PUBLIC-RELEASE artifact's pin joins `DEFAULT_PIN_KEYS` in the SAME change that ships it. The
  compiled table carries the authority root (key-id 1) today.

Still open here: nothing — the remaining T2 acceptance ("docs gate green; env cross-refusal
proven at the reader; no behavior change pinned") is met by the items above; the v0→v1 bump is
the A10-ratification follow-up (owner), and T7's `artifact-sync lint` must agree with the
fixture validator (recorded in the spec).

## Why

The workspace consolidation needs ONE vocabulary home that public repos can depend on. This repo
is it: the vessel format already defines PUBLIC-RELEASE / HOSTED-ONLY with signing + BLAKE3
commitments. The workspace adopts this vocabulary for at-rest artifacts with a crosswalk, a
versioned manifest schema spec, and — critically — the RUNTIME env-key refusal.

## Scope

- [ ] **Versioned class vocabulary + manifest schema SPEC** (public doc + fixtures): class
      (`public|protected`), kind (`heads|weights|vessels|corpora|keys` — `keys` = wrapped-DEK
      rows, private manifests only), env (`none|localnet|devnet|testnet|mainnet`),
      `blake3_plain` / `blake3_cipher` / `plain_bytes` / `cipher_bytes` / `dek_scope`
      (`train|serve-<env>|serve-shared` — protected rows only; `serve-shared` is the DECLARED
      dual-use/multi-env relaxation) / `provenance` / `remote` fields — the schema
      `artifact-sync lint` validates against
- [ ] **Crosswalk** (one dialect, not two): `PUBLIC-RELEASE` ↔ `public`; `HOSTED-ONLY` ↔
      `protected` + the serve rider (hosted-only plaintext never exists off controlled hardware;
      ciphertext may live anywhere)
- [ ] **Runtime per-env pin refusal in the reader** — the fail-closed enforcement point:
      per-env signing pin tables (or a signed `env` header field) so a devnet-signed vessel can
      never satisfy a mainnet pin even with every script bypassed; manifest `env` columns are the
      redundant cross-check the workspace gate audits
- [ ] **At-rest naming + age-wrap layer documented** (`<name>.<class>.<ext>`; age binary with
      sniffable header as the uniform storage-confidentiality layer around the signed vessel —
      two layers, two jobs); `.protected.` fixture round-trip: decrypt → signature + class caps
      verified exactly as raw bytes
- [ ] `DEFAULT_PINS` population hook tracked for the FIRST public release artifact publish (the
      standing law — it happens then, not later)

## Acceptance

- Docs gate green; schema fixtures validate via the spec
- Fixture round-trip green both postures; env cross-refusal proven at the reader (devnet
  vessel ⊬ mainnet pin, and vice versa)
- No behavior change for existing `.vessel` reads (byte-identical unarmed posture pinned)

## Notes

Public repo: vocabulary, schema, crosswalk, naming, runtime refusal. No specific artifacts,
paths, or private-class inventories are named here.
