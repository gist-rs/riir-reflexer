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
