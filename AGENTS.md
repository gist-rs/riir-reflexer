# AGENTS.md — riir-reflexer (public)

The global `~/.agents/` rules apply where compatible with a PUBLIC repo.
**Role:** the public decision ENGINE — design of record
`.proposals/001_reflexer_engine.md` (public-facing), the full private design
record lives in riir-train. One line: public substrate plays public genomes;
this repo is the engine, the format, and the submission client — nothing else.

## The laws

1. **Modelless-first mandate, verbatim.** No training, no backprop, no
   gradient descent. The only weight mutations allowed are artifact
   hot-swap (atomic versioned swap — NEVER a weighted blend; blends do not
   preserve move rankings) and latent-space updates. Improvement machinery
   does not grow here.
2. **The leaf law.** Zero riir-* dependencies (katgpt-core + the katgpt-rs
   substrate module only). No tokenomics code, no signing keys.
3. **The forward freeze.** The katgpt-rs example/module surface stops at
   the Bench-892 champion `68cae9d382014662` as reference; evolved VALUES
   land in the private improvement home — never accreted into public
   examples.
4. **Artifact classes.** PUBLIC-RELEASE runs anywhere (extractable,
   accepted). HOSTED-ONLY never reaches uncontrolled hardware; encrypted
   at rest on an encrypted-storage lane; served as decisions only.

## Measurement laws

Zero network calls in measurement paths. Response envelopes carry
in-engine decision time; harnesses render it beside the round-trip. Every
comparison row pins binary BLAKE3 + artifact digest. Artifact fingerprints
are proven on aarch64 AND x86_64 by execution before crossing machines.

## Current state

**P3 COMPLETE (the vessel format crate) — P2 stands.** Workspace: `reflexer` (lib + bin) +
`crates/reflexer-vessel` (public, MIT; blake3 + ed25519-dalek only). Engine serves
`decision_wire` over stdio line-JSON (`place`/`state`/`survive`, abstention first-class, typed
error envelopes, EOF exits 0); G1 bit-identity gate-tested (`tests/g1_champion_replay.rs` +
`tests/vessel_gates.rs` — champion-from-vessel ≡ champion-from-substrate, both geometries,
proven by execution on aarch64 AND x86_64, Bench 002). Vessels: format v1 (68B header + strict
ed25519 over `header ‖ payload` + payload), two-class header (no HOSTED-ONLY writer path in
this repo; readers refuse fail-closed after authenticity), key-id rotation (`DEFAULT_PINS`
EMPTY until the first artifact ships; `--vessel-pubkey` operator pin), monotonic apply
(downgrade + fork refused; force logs), single-read bounded open, class-aware payload caps
(`049a583`: PUBLIC-RELEASE 1 MiB — `MAX_PAYLOAD`; HOSTED-ONLY up to `MAX_HOSTED_PAYLOAD`
16 MiB at `peek()`, class read from the SIGNED header before any payload work; `open()`
refuses a file DECLARING the hosted class at the 132-byte prefix — `HostedOnlyPath`, pre-auth
by construction; format stays v1, no encoder for class 1). Security posture: `.plans/002`;
gates: `.benchmarks/002`. Bin flags: `--vessel`, `--vessel-pubkey[=hex]`,
`--vessel-force-downgrade`, `--vessel-print`. Measurement lane (`examples/measure.rs`)
enforces the G2 budget with box-state preflight; the trajectory client signs rows with the
per-machine Ed25519 key (opt-in `--record`). Next: P4/P5 — hosted serving + deployment,
private homes (riir-dapps / riir-deployer); this repo's part ends at the wire and the format.

**P3.1 — the PUBLIC-RELEASE writer + the reader-capability features
(instinct Proposal 001 T3+T4, 2026-09-27):**

- **`writer` module + `reflexer sign`** — the public FORMAT repo owns the public-class
  writer (riir-train `vessel_mint` stays HOSTED-class only; no writer for class 1 exists
  here). `writer::sign_public` is the Result-based mint API (the 1 MiB cap enforced AT
  WRITE TIME as `PayloadTooLarge`) returning vessel bytes + blake3 commitment; fail-closed
  key handling (`--key <64-hex-seed>` / `--key-file` / env `REFLEXER_SIGN_KEY`). Re-verifies
  its own output (a wildcard pin of its own key) and prints the commitment + VERIFYING key
  hex — the consumer-side trust anchor (e.g. reflex's `RIIR_REFLEX_HEADS_PUBKEY`).
  Deterministic minting: same inputs → byte-identical vessels — re-minting moves no pin.
- **`vessel_public_read` / `vessel_hosted_read`** — the reader capability axis (A1: bytes
  are runtime; capability is compile-time). Both DEFAULT-ON: the default build is
  behavior-identical for existing consumers. A class whose reader is not compiled refuses
  `ClassNotReadable` (at `peek` structurally; `decode` post-signature). The class REFUSALS
  are not capabilities: `open`'s `HostedOnlyPath` prefix check and `decode`'s authenticated
  `HostedOnly` refusal stay UNCONDITIONAL. reflex (public) selects `vessel_public_read`
  only — the hosted reader never compiles into the public consumer.

## Build Commands

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test                                   # incl. G1 (wire vs in-process oracle)
cargo build --release --bin reflexer --example measure
cargo run --release --example measure        # the G2 budget gate (box-state preflight)
```

Path deps expect `../katgpt-rs` beside this repo; use an isolated `CARGO_TARGET_DIR` when a
sibling build holds the lock. The measurement contract lives in README.md §"The measurement
contract" beside its book copy `.docs/02_wire_protocol/measurement_contract.md` — edit the
two together or not at all; do not restate either here.

## Documentation

`.docs/` is the repo's book, the fleet way: numbered folders, bare slugs, a `README.md` index
per folder, `.docs/README.md` top index. Folders: `01_orientation` · `02_wire_protocol` ·
`03_decision_flow` (engine flow + `decision_flow.svg`; re-render procedure in
`.docs/03_decision_flow/decision_flow.md`) · `04_vessel_format` · `05_measurement_lane` ·
`06_resources` (education write-up + the mirrored gfflow relation figure). README is the
build surface; where prose disagrees with source, the source module docs win.

## Numbering Discipline

Monotonic, never reused: read the target dir's `.highwater`, use value+1,
write back. `.proposals/.highwater` = 001.

## Branch

`develop` at birth, per the global rule.
