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

**P2 COMPLETE (engine bin + measurement lane + submission client).** The
cargo package `reflexer` (lib + bin) serves `decision_wire` over stdio
line-JSON: `place` (choice over the canonical `decide_scored` enumeration),
`state` (5-level rubric), `survive` (noul); unknown questions abstain;
typed error envelopes never kill the pipe; EOF exits 0. G1 bit-identity
through the wire is gate-tested (`tests/g1_champion_replay.rs` — hold,
no-hold, garbage-board geometries, all byte-identical to `play_game`). The
measurement lane (`examples/measure.rs`, run `--release`) enforces the G2
budget with box-state preflight (refuses on battery / MAX_LOAD, default 6).
The trajectory submission client signs rows with the per-machine Ed25519
key (opt-in `--record`). P0/P1 history: HISTORY.md. Next: P3 (the vessel
format crate).

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
