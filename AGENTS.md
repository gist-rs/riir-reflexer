# AGENTS.md — riir-reflexer (public)

The global `~/.agents/` rules apply where compatible with a PUBLIC repo;
this file documents repo-local context.

## Role

The public decision ENGINE. Design of record: this repo's
`.proposals/001_reflexer_engine.md` (public-facing); the full private
design record lives in riir-train. One line: public substrate plays public
genomes; this repo is the engine, the format, and the submission client —
nothing else.

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

**P3 COMPLETE (the vessel format crate) — P2 stands.** The cargo
workspace is `reflexer` (lib + bin) + `crates/reflexer-vessel` (public,
MIT; blake3 + ed25519-dalek only). The engine serves `decision_wire` over
stdio line-JSON (`place`/`state`/`survive`, abstention first-class, typed
error envelopes, EOF exits 0); G1 bit-identity through the wire is
gate-tested (`tests/g1_champion_replay.rs` + `tests/vessel_gates.rs` —
the P3 battery adds champion-from-vessel ≡ champion-from-substrate, both
geometries, proven by execution on aarch64 AND x86_64, Bench 002).
Vessels: format v1 (68B header + strict ed25519 over `header ‖ payload` +
payload), two-class header (no HOSTED-ONLY writer path anywhere in this
repo; readers refuse fail-closed after authenticity), key-id rotation
(`DEFAULT_PINS` EMPTY until the first artifact ships; `--vessel-pubkey`
is the operator pin), monotonic apply (downgrade + fork refused; force
logs), single-read bounded open, 1 MiB caps — the security posture lives
in `.plans/002` and the gates in `.benchmarks/002`. Bin flags:
`--vessel`, `--vessel-pubkey[=hex]`, `--vessel-force-downgrade`,
`--vessel-print`. The measurement lane (`examples/measure.rs`) enforces
the G2 budget with box-state preflight; the trajectory client signs rows
with the per-machine Ed25519 key (opt-in `--record`). P0–P2 history:
HISTORY.md. Next: P4/P5 — hosted serving + deployment, private homes
(riir-dapps / riir-deployer); this repo's part ends at the wire and the
format.

## Build Commands

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test                                   # incl. G1 (wire vs in-process oracle)
cargo build --release --bin reflexer --example measure
cargo run --release --example measure        # the G2 budget gate (box-state preflight)
```

Path deps expect `../katgpt-rs` beside this repo. Use an isolated
`CARGO_TARGET_DIR` when a sibling build holds the lock. The measurement
contract (wire state schema, question ids, envelopes, error codes) is
README.md §"The measurement contract" — the one home; do not restate it
here.

## Numbering Discipline

Monotonic, never reused: read the target dir's `.highwater`, use value+1,
write back. `.proposals/.highwater` = 001.

## Branch

`develop` at birth, per the global rule.
