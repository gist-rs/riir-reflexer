# Plan 001 — the engine bin + the bin-only measurement lane + the submission client (P2)

**Status:** COMPLETE — landed 2026-09-25 (all T-boxes green; G2 numbers in .benchmarks/001). Filed after P0 (katgpt-tetris `243c38a1b`) and
P1 (public birth) landed. Scope source: `.proposals/001` §Sequencing P2 +
riir-train Research 457 §Sequencing P2 (the private design record).

Branch: `develop`. Session markers on every commit.

## Premise (measured, not assumed)

- The substrate EXISTS: `katgpt-tetris` (katgpt-rs `243c38a1b`) — sim
  byte-identical to the Bench-892-verified originals; champion reference
  genome `68cae9d382014662` pinned by crate test; ~0.32-0.33 ms/decision at
  the h2h geometry (G2 record, AC, load 4.33).
- The wire EXISTS: `katgpt_core::decision_wire` (opt-in feature) —
  `choice`/`score`/`noul`, abstention first-class. Consumer #1:
  riir-reflex. This repo's bin is consumer #2.
- Zero-network measurement law: no network calls in any measurement path;
  the bin speaks line-JSON over stdio; harnesses render in-engine decision
  time beside round-trip.

## Tasks

- [x] T1 Workspace + skeleton: root package `reflexer` (bin `reflexer`),
      `license = "MIT"`, edition 2024, `publish = false`; deps
      `katgpt-core` (path `../katgpt-rs/crates/katgpt-core`, feature
      `decision_wire` — verify the exact feature name against
      katgpt-core's manifest before typing it) + `katgpt-tetris` (path
      `../katgpt-rs/crates/katgpt-tetris`). `cargo check` green at
      default features. C3b note: BOTH deps resolve into the `katgpt-rs`
      repo — the only sibling a public repo may touch.
- [x] T2 Engine core: load the REFERENCE genome
      (`Genome::champion_hybrid()`), run it over the tetris sim; map game
      decisions onto decision_wire requests (choice over enumerated
      landing options; score for state evaluation; noul for the
      no-options question class; abstention when the gate says so —
      inherit katgpt-rs Bench 817's confidence readout dispatch, never
      re-derive).
- [x] T3 Serving edge: line-JSON over stdin/stdout; one request → one
      response envelope; malformed input → typed error envelope, never a
      panic; EOF exits 0.
- [x] T4 The envelope carries `in_engine_decision_ns` (the engine's own
      in-process decision time) so harnesses render it beside the
      round-trip — never display a sub-ms decision through a same-size
      subprocess overhead.
- [x] T5 Measurement lane (bin-only, the arena posture): subprocess the
      built bin; seeds pinned; per-row artifact pins (binary BLAKE3 +
      genome digest); render in-engine vs round-trip side by side;
      zero network. This is the lane riir-reflex's arena measures this
      engine through — same treatment as every external engine.
- [x] T6 Trajectory submission client (free, opt-in, OFF by default):
      records decision trajectories + signs them against the public wire
      contract; emits a submission artifact the private loop
      replay-verifies. NO billing code, NO tokenomics, NO keys beyond the
      per-machine submission key. (Anti-poisoning gate lives
      riir-train-side — not here.)
- [x] T7 G1 gate: champion replay bit-identity — the bin's per-seed
      decision fingerprints match the katgpt-tetris example battery on
      THIS arch, and the artifact fingerprint is proven by execution on
      aarch64 AND x86_64 before any artifact crosses machines.
- [x] T8 G2 gate: matched-time latency budget under `--release` with box
      state recorded (AC/battery, load, concurrent jobs — the
      bench-preflight law); publish the numbers in this repo's
      `.benchmarks/` with the provenance line.
- [x] T9 Docs: README (build + run + the measurement contract), AGENTS
      Current-state update, HISTORY entry. BOUNDARY unchanged (the dep
      rows already declare both katgpt paths).

## Non-goals (P3+ / private homes)

- The vessel format crate (P3 — read/verify + class header + rotation).
- Any improvement machinery, minting, hosted routes, deployment — private
  sibling repos, forever (BOUNDARY §Does not own).
