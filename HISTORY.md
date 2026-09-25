# HISTORY.md — riir-reflexer (public)

## 2026-09-25 — pre-birth

- Design closed (reviewed; three negotiation rounds + filing checks). Owner
  visibility decision: **public engine** — the riir-shader posture. The full
  design record lives privately in riir-train.
- Docs landed md-only (no `.git` yet, by design): this file, AGENTS.md,
  BOUNDARY.md, `.proposals/001_reflexer_engine.md`.
- P0 filed and committed in katgpt-rs: Issue 893 — promote the tetris
  substrate (sim + lookahead + rulebook) to a public feature-gated module,
  bit-identity GOAT, one copy, forward freeze at the Bench-892 champion.
- Next: P1 public birth (licence, registration with the visibility
  attribute) → P2 engine bin + measurement lane + submission client →
  P3 vessel format crate. Improvement/hosted/deploy concerns live in
  private sibling repos.

## 2026-09-25 — PUBLIC BIRTH (P1)

- Licence: MIT (matches the katgpt-rs public form; the declared posture of
  the design's riir-shader reference).
- BOUNDARY.md gained the `Visibility: public` attribute + the `## Owns`
  section; the workspace boundary guard grew check **C3b** (public-leaf
  law): a public repo path-depping any sibling other than katgpt-rs is a
  mechanical violation, not satisfiable by widening the allowlist table.
- Registered: workspace repo_set + the riir-ai CANONICAL matrix row
  (`riir-reflexer` → none INTO riir-ai; katgpt-rs path deps only).
- Remote: gist-rs/riir-reflexer (public); develop is the working branch.
- Next: P2 (engine bin + bin-only measurement lane + submission client),
  gated on katgpt-rs Issue 893 (P0) landing.

## 2026-09-25 — P0 LANDED (the substrate crate exists)

- katgpt-rs Issue 893 RESOLVED (`243c38a1b`, header touch-up `4db7824de`):
  `katgpt-tetris` leaf crate — sim byte-identical, lookahead/rulebook with
  4 module-path rewrites total, full Bench-891/892 bit-identity battery
  PASS, G2 no-regression (0.32-0.33 ms/decision), champion genome
  `68cae9d382014662` pinned by crate test as the frozen REFERENCE.
  Record: katgpt-rs HISTORY.md §Issue 893.
- BOUNDARY `May depend on` row updated: katgpt-tetris landed, P2 wires it.
- Next: P2 — the engine bin + the bin-only measurement lane + the
  trajectory submission client (plan: `.plans/001`).

## 2026-09-25 — P2 LANDED (engine bin + measurement lane + submission client)

- The cargo package is born: lib + bin `reflexer` (edition 2024, MIT,
  `publish = false`). Deps: katgpt-core (`decision_wire`) + katgpt-tetris
  (both path deps into `../katgpt-rs` — the C3b leaf law) + serde/serde_json
  + blake3 + ed25519-dalek/rand_core (the submission key). Cargo.lock
  committed (a bin whose artifacts pin binary BLAKE3 wants reproducible
  builds).
- The engine: ONE genome serves the bin — the frozen Bench-892 champion
  (`68cae9d382014662`, asserted at load). One `decide_scored` search answers
  a whole request: `place` (choice, first-strict-argmax — `decide`'s exact
  fold), `state` (5-level rubric via σ(v/200)), `survive` (noul).
  Unknown questions ABSTAIN (confidence 0). Abstention is structural
  (top-out), never a threshold inside game replay — bit-identity demands it.
- Probabilities: sigmoid-margin weights at the population-std scale — the
  tetris_09 site-walk convention, inherited (sigmoid, never softmax).
  Confidence: the Bench-817 dispatch, inherited (`src/readout.rs`).
- The line protocol: response envelopes carry `in_engine_decision_ns`
  (serde + stdio excluded); typed error codes (`bad_json`, `bad_request`,
  `bad_state`, `question_kind`, `options_count`, `internal`); the pipe
  survives every malformed line; EOF exits 0. `options_count` is
  fail-closed bit-identity defense: a client whose enumeration diverges
  from `decide_scored`'s candidate order gets a typed refusal, not a
  silently-misaligned index.
- G1 gate (`tests/g1_champion_replay.rs`): wire-driven games are
  byte-identical to the in-process oracle at three geometries (hold
  `play_game` posture seeds 1..=6 cap 240; no-hold h2h posture seeds 1..=4
  cap 200; garbage-board starts seeds 11..=13) — decision sequences AND
  game stats. The oracle itself is sanity-asserted against
  `katgpt_tetris::rulebook::play_game`.
- The measurement lane (`examples/measure.rs`, `src/lane.rs` — one driver
  shared with the G1 test): subprocess the built bin; binary BLAKE3 +
  genome digest pinned per artifact; G1 proven inline on every measured
  game; in-engine vs round-trip rendered per posture; REFUSES on battery
  and over MAX_LOAD (default 6, the reflex bench-preflight law); G2 budget
  enforced in-run (in-engine p50 ≤ 500 µs, p99 ≤ 1500 µs — policy vs the
  substrate's own 0.32–0.33 ms/decision record).
- Trajectory submission (opt-in `--record`): replayable rows + a signed
  manifest (Ed25519 over `reflexer-trajectory-v1\n{rows}\n{blake3}\n{genome}\n`),
  per-machine key at `$REFLEXER_SUBMISSION_KEY` or
  `~/.config/reflexer/submission.ed25519` (0600). No billing code, ever.
- BOUNDARY `May depend on` trued up: katgpt-core's `hint_regret` feature
  lands WITH its consumer (not yet wired); only `decision_wire` is enabled.
- Next: P3 — the vessel format crate (read/verify, class header, rotation).
