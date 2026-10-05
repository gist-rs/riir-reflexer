# The artifact-class vocabulary + manifest schema (v0 draft)

**Status: draft — pending A10 ratification (riir-ai Plan 623 T0).** The
schema version is `v0`: an owner amendment to the at-rest law bumps the
version, it never rewrites a published contract in place. Content here is
public-safe BY CONSTRUCTION — class names, enum values, the field table,
the naming law; no artifacts, no inventory, no key material, no bucket
names. (riir-ai Proposal 054 owns the at-rest LAW and the private
inventory; riir-deployer Issue 012 owns the `artifact-sync` tool that
lints against this schema; riir-ai Issue 1033 owns the enforcement gate
that shells out to it — one implementation each.)

## Why this repo owns the vocabulary

The vessel format already defines the two artifact CLASSES
(`vessel_format.md` §"The two-class law"): PUBLIC-RELEASE runs anywhere;
HOSTED-ONLY is refused on uncontrolled hardware. The workspace at-rest
taxonomy adopts that vocabulary so ONE home defines it (this repo,
public) and every other repo references it — never a second dialect.

## The crosswalk (one dialect, not two)

| Vessel class (this crate's wire bit) | At-rest class (manifest) | Rider |
|---|---|---|
| PUBLIC-RELEASE | `public` | — |
| HOSTED-ONLY | `protected` | **+ the serve rider**: hosted-only PLAINTEXT never exists off controlled hardware; its CIPHERTEXT may live anywhere. The reader refuses the hosted class structurally either way — encryption is the storage layer's job, not the vessel's. |

`hosted-only` is not a third at-rest class: it is `protected` PLUS the
serve law above.

## Class definitions

| Class | At rest | Remote lane class | Readable by |
|---|---|---|---|
| `public` | plain bytes + BLAKE3 sidecar | public | everyone — extractable, accepted |
| `protected` | age-encrypted (storage-confidentiality layer; see below) | encrypted object store | key holders only |

## Manifest schema v0 — `artifacts/manifest.toml`

One manifest per artifact-owning repo; the manifest is the repo's ONLY
tracked file under `artifacts/` (the payloads are gitignored — the
directory law: `artifacts/**` ignored, `!artifacts/manifest.toml`
excepted). Field table:

| Field | Type | On which rows | Meaning |
|---|---|---|---|
| `name` | string | all | the artifact's name (inventory lives in the manifest) |
| `kind` | enum: `heads` \| `weights` \| `vessels` \| `corpora` \| `keys` | all | the kind axis; `keys` = wrapped-DEK rows (PRIVATE manifests only) |
| `class` | enum: `public` \| `protected` | all | the class axis (crosswalk above) |
| `dek_scope` | enum: `train` \| `serve-<env>` \| `serve-shared` | protected rows ONLY — REQUIRED there; REQUIRED ABSENT on `kind = "keys"` and on public rows | which DEK set decrypts the row (`serve-<env>` names an env from the vocabulary below; `serve-shared` is the DECLARED multi-env relaxation) |
| `env` | enum: `none` \| `localnet` \| `devnet` \| `testnet` \| `mainnet` | all | the serving env the row is bound to (`none` = env-independent) |
| `blake3_plain` | 64-hex | all | BLAKE3 of the PLAINTEXT — the identity + the authenticity root (anyone holding a recipient key can encrypt a forgery; the manifest's plain hashes are the trust root) |
| `blake3_cipher` | 64-hex | all | BLAKE3 of the ciphertext file — the remote-integrity pin |
| `plain_bytes` | integer | all | exact plaintext size — the history-scan prefilter |
| `cipher_bytes` | integer | all | exact ciphertext size — the prefilter for committed ciphertext |
| `provenance` | string | private manifests | the train-side reference (never in a public manifest) |
| `remote` | array of URIs | all | content-addressed remote keys (`<blake3_cipher>`, optionally `<kind>/<blake3_cipher>`) — filenames stay out of remote keys |

Lint law: rows may be ZERO (a repo with no artifacts is legal); the
SCHEMA may not drift (an unknown field or an off-vocabulary enum value
refuses). A PUBLIC repo's manifest carries ZERO protected rows — a
protected artifact a public binary consumes is owned by a PRIVATE
manifest and pulled into the public repo's cache at serve time.

### Source pins — the `[[source_pin]]` table

Base/foundation models and other blobs that are re-downloadable or
produced in-house are NOT artifacts: nothing is placed, nothing is
encrypted, no remote-lane row exists for them. The manifest records
them beside the artifact table as **source pins**:

| Field | Type | On which rows | Meaning |
|---|---|---|---|
| `name` | string | all | the pinned blob's name |
| `sha256` | 64-hex | all | SHA-256 of the bytes — the upstream-verifiable pin (publishers and download tooling speak sha256) |
| `blake3` | 64-hex | all | BLAKE3 of the bytes — the house hash; joins the enforcement scan's hash set (a committed blob matching a source pin is a weights-never-commit violation, name-independent) |
| `bytes` | integer | all | exact size |
| `source_url` | string | optional | where the bytes come from (absent for in-house-produced blobs — `provenance` names the producing run instead) |
| `provenance` | string | private manifests | the producing run's reference (in-house blobs) |
| `note` | string | optional | re-derivation notes, risks |

Laws: at least one of `source_url` / `provenance` must be present (a
pin that names neither its origin nor its maker is not a pin);
`name` is unique within the table; source pins carry NO artifact-lane
fields (no `kind`/`class`/`env`/`dek_scope`, no hashes-but-different
names, no `remote`) and NEVER satisfy a placement — an `[[artifact]]`
row is the only thing a file under `artifacts/<kind>/<class>/` can
answer to.

### Fixture fixtures

Machine-validated fixtures live beside the format crate
(`crates/reflexer-vessel/tests/fixtures/manifest_v0/`): every row there
is parsed and checked against the enum vocabulary by
`tests/manifest_schema_fixtures.rs`. Until `artifact-sync lint` (T7)
exists, the enum/field legality is what the fixtures pin; full schema
linting lands with T7 and must agree with this table.

## The env vocabulary + the reader refusal

`env` uses the same five words the READER pins
(`reflexer_vessel::env::Env`): `none | localnet | devnet | testnet |
mainnet`. The reader-side enforcement point is the per-env pin table
(`src/env.rs`): a vessel is verified against ONLY the pin set of the env
it is served in, so a devnet-signed vessel can never satisfy a mainnet
pin even with every script bypassed — cross-env trust is structural, not
policy. v1 vessels carry no env field; the binding is the pin set (a
signed `env` header field is the v2-design alternative, deliberately not
taken in this draft). An env with no pins refuses loud
(`VesselError::EnvUnpinned`, naming the env) — a config gap is never a
silent fallback to the env-independent root.

## At-rest naming + the age-wrap layer

- Filename law: `<name>.<class>.<ext>` — the class infix is the portable
  marker; a file copied out of its directory still declares itself.
- The age layer is UNIFORM storage confidentiality: `protected` bytes are
  age ciphertext (sniffable header `age-encryption.org/v1`); vessels keep
  their signed format INSIDE it. Two layers, two jobs — the vessel
  envelope is integrity + class; age is storage confidentiality.
- Round-trip law: decrypt → verify the signature + class caps on the
  decrypted bytes EXACTLY as on raw bytes. The vessel layer is
  transparent to decryption — the reader cannot tell, and must not care.
- Key scheme (scoped DEKs, never a copied login key; the full procedure
  is the private key-ritual page, not this public spec): TWO data keys by
  scope — `dek-train` and `dek-serve-<env>` — plus a cold recovery key
  that is a recipient on every protected file AND every wrapped DEK.
  Rotation protects future artifacts only; published ciphertext stays
  decryptable by old keys forever.

## `DEFAULT_PINS` and the first public artifact

The compiled pin table carries the authority root (key-id 1) today. The
standing law for the first PUBLIC-RELEASE artifact publish: its pin is
added to `DEFAULT_PIN_KEYS` in the SAME change that ships the artifact —
it happens then, not later. Until then the pin table stays at the root.

## The directory convention (per artifact-owning repo)

```
artifacts/
  manifest.toml            # the ONE index — the only tracked file
  heads/{public,protected}/
  weights/{public,protected}/
  vessels/{public,protected}/
  corpora/{public,protected}/
  cache/                   # decrypted pulls land here — the worst possible leak, gated hardest
```

Gitignore by DIRECTORY, not extension: `artifacts/**` +
`!artifacts/manifest.toml`. Base/foundation models are NOT archived —
the manifest pins their source URL + sha256 instead.
