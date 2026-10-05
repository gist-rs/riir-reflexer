# Issue 004 — Artifact class vocabulary + manifest schema spec + runtime env refusal

**Status:** OPEN — Plan 623 T2 (workspace Proposal 054; verdict round 1 moved the vocabulary home
HERE — this repo already publicly owns the vessel class vocabulary)

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
