# HISTORY.md — riir-reflexer (public)

## 2026-10-04 — `.docs/06_resources`: the relation section + the first mirrored figure (`ba943d8`)

Plan 004 (verdict AGREE round 2): the Reflex ↔ Reflexer relation gets the
`#reflex` treatment on reflex.gist.rs `/resources/#reflexer` — placed AFTER
`#rethink` (the ladder stays contiguous). This repo became the owning doc:
`.docs/06_resources/resources.md` (public-copy law compliant — its mirror is
served) + the `reflexer_relation_flow` gfflow block (branch 2a text / 2b game
turn, merge at the envelope, the vessel back edge), rendered by the fleet
renderer to desktop + 390px `_m`. The mirror is the decision
`03_decision_flow/decision_flow.md` anticipated — this repo joined the
renderer's SOURCES and sync_mirror's roots (3 pairs). Landed with the Plan 620
P1 takeover (reflex-site `fe69bd2` + `12538b9`, riir-rethink `7b86e8b`);
section + mirrors live 2026-10-04.

## 2026-09-30 — sigmoid delegated to the substrate, adjudicated pick-level (`fb6ac8f`; issue 002 closed)

The substrate-first wave audit (2026-09-30 window) flagged `engine.rs`'s
module-local one-branch sigmoid (`1/(1+exp(-x))`) as an unexplained body
beside `katgpt_core::exact_sigmoid_f64` — the substrate built for exactly
this caller class. Adjudication-gated because the two forms are NOT
bit-identical on the reachable domain: the margin path's args
(`(v − v_max)/scale`) are ≤ 0 always, precisely the branch where the
stable two-branch form (`exp(x)/(1+exp(x))` for x < 0) rounds differently.

**The verdict turned on one question: is the G1 pin pick-level or
prob-level?** Read of every gate: pick-level. The champion-replay battery
asserts decision sequences (`(use_hold, index)` pairs), stats, and genome
id — the wire path reads only `Outcome::Choice { index }`; vessel gates
assert the same triple; `proto_gates` holds probabilities to a 1e-3
sum tolerance, not bits; the only score-path pin is the level
calibration (`floor(q·5) ≥ 1` on an empty board) with real margin. So the
ulp-level prob change the delegation makes is admissible → resolution (a),
delegate:

- The local body is DELETED; `sigmoid` is now
  `use katgpt_core::exact_sigmoid_f64 as sigmoid` (x ≥ 0 shares the op
  sequence with the frozen body — bit-identical; x < 0 ulp-bounded).
- The permanent arm is `sigmoid_delegation_matches_frozen_legacy_body`
  (engine.rs test module): the frozen legacy body kept verbatim as the
  reference — bit-identity on x ≥ 0 (incl. `36.0`, the value that
  discriminates a ±40 saturation early-exit), an 8-ulp bound on the
  reachable negative domain, the death tail (`DEATH_VALUE/V_REF = -5e9`
  → exactly `0.0` in both forms) and `sigmoid(0) = 0.5`. If the substrate
  kernel ever drifts (saturation, narrowed intermediate, polynomial
  swap), the pin reds and forces re-adjudication rather than silently
  absorbing the change.
- One honest far-tail note recorded in the pin's doc: on
  `(-745.1, -709.8)` the legacy body saturated to `0.0` while the stable
  form keeps the true subnormal — an accuracy IMPROVEMENT, and
  unreachable after the f32 narrowing every call site performs.

Gates at the landing: full `cargo test` exit 0 (lib 10/10 incl. the new
pin and the score-level calibration; g1 5/5; vessel 8/8; proto 9/9;
serve-parity 1/1); `cargo clippy --all-targets -- -D warnings` clean;
`cargo check -p reflexer-wasm --target wasm32-wasip1` clean;
`cargo fmt --check` clean.

Issue file `.issues/002_local_f64_sigmoid_beside_exact_sigmoid_f64.md`
removed at closure (the numbering stays monotonic — 002 is consumed).
Precedent lineage: riir-reflex Issue 014 (delegated, bit-identical
domain), riir-chain Issue 156 (recorded-refusal arm — the menu this
adjudication came from), ndb Issue 611 (delegated).

## 2026-09-27 — the class-aware payload cap (`049a583`): HOSTED-ONLY may carry 16 MiB; the public reader's 1 MiB untrusted bound is UNCHANGED

The specialist vessels (riir-train minter → riir-instinct hosted lane) carry i8
weights far larger than the genome-era ceiling — banking77 is ~9.6 MiB, five of
the six winners exceed 1 MiB — and the cap exists to bound hostile inputs, not
to size artifact classes. The two classes read on two different trust surfaces,
so the bound is per-class:

- **`peek()`** reads the class from the header it just parsed (the class bit is
  IN the signed header, before any payload work), so a HOSTED-ONLY vessel may
  carry up to `MAX_HOSTED_PAYLOAD` (16 MiB, ~1.6× headroom over banking77)
  while PUBLIC-RELEASE stays at `MAX_PAYLOAD` (1 MiB). A size legal for hosted
  is still refused the moment the header says public.
- **`open()`** reads the 132-byte prefix FIRST and refuses a file DECLARING the
  hosted class before any payload byte (`HostedOnlyPath` — a structural fact
  about the bytes, pre-auth by construction; the authenticated `HostedOnly`
  refusal and its only-said-about-authentic-vessels law are untouched). The
  untrusted read path's allocation bound therefore stays 1 MiB even though
  hosted vessels are far larger; reflexer-wasm never loosens.
- Format stays v1, layout unchanged; no encoder for class 1 exists (unchanged
  by design). Tests (+5, forged payload_len — no big fixtures): hosted ~10 MB
  admits at peek while decode still refuses; hosted 16 MiB+1 refused; public
  1 MiB+1 refused even though hosted-legal + exactly-1-MiB public admitted;
  open refuses declared-hosted at the prefix with a truncated file. Workspace
  58 passed / 0 failed; clippy clean.
- Verdict-reviewed (2 rounds): the single-cap raise was REVISEd to this
  class-aware shape — the public reader is where untrusted input actually
  arrives; raising its bound to serve the private lane gets the risk backwards.
  Consumer side: riir-instinct serves banking77 as one 10 MB vessel
  (`source: Vessel`); the cap-pin gate is instinct `f2fd12a`.

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
  enforced in-run (in-engine no-hold p50 ≤ 500 µs, hold p50 ≤ 1 000 µs,
  p99 ≤ 3 000 µs both postures — the `BUDGET_*` constants in
  `examples/measure.rs`, policy vs the substrate's own 0.32–0.33
  ms/decision record; corrected 2026-10-04 — this line originally read
  "p50 ≤ 500 µs, p99 ≤ 1500 µs", which matched neither the landed source
  nor Bench 001's per-posture budgets).
- Trajectory submission (opt-in `--record`): replayable rows + a signed
  manifest (Ed25519 over `reflexer-trajectory-v1\n{rows}\n{blake3}\n{genome}\n`),
  per-machine key at `$REFLEXER_SUBMISSION_KEY` or
  `~/.config/reflexer/submission.ed25519` (0600). No billing code, ever.
- BOUNDARY `May depend on` trued up: katgpt-core's `hint_regret` feature
  lands WITH its consumer (not yet wired); only `decision_wire` is enabled.
- Next: P3 — the vessel format crate (read/verify, class header, rotation).

## 2026-09-25 — P3 LANDED (the vessel format crate)

- **`crates/reflexer-vessel`** (public, MIT; deps blake3 + ed25519-dalek
  only — no new dep classes): format v1 = fixed 68-byte header (magic ·
  format version · flags · key-id · artifact version · parent commitment ·
  payload len) + 64-byte ed25519-STRICT signature over `header ‖ payload`
  + payload. Manual LE parse, zero serde. `commitment = blake3(header ‖
  payload)` — the lineage chain and the signature cover the same bytes.
  The crate is PAYLOAD-AGNOSTIC by design: bytes in, verified bytes out;
  `Genome::from_line` binding lives in the engine (`from_vessel_payload`)
  — the only bytes→genome seam.
- **Two-class law**: `encode_public` is the ONLY public writer; HOSTED-ONLY
  has no writer path outside `pub(crate)` hostile-forgery unit tests; the
  reader refuses it fail-closed AND only after authenticity (a forged file
  cannot spoof the hosted-only refusal message).
- **Rotation**: key-id in the header, compiled-in pin table (`DEFAULT_PINS`
  EMPTY until the first artifact ships — with nothing minted, every
  unverified-key vessel fails closed, the correct posture), revocation,
  and the `--vessel-pubkey` operator wildcard pin (the SEAL
  `SEAL_VESSEL_PUBKEY` precedent).
- **Monotonic apply**: `check_monotonic` refuses `OlderThanCurrent` and
  `VersionFork`; the bin's `--vessel-force-downgrade` is the operator
  path and LOGS (tested both arms).
- **Hardening** (the security-posture section, added `50409ec` after the
  pre-T1 review): single-read bounded `open` (`take(cap+1)` — no
  stat-then-read window), verify-before-parse with the 1 MiB payload cap,
  `verify_strict` everywhere, constructive whole-engine apply (no
  partially-swapped state observable), unknown-anything fails closed.
  cargo-fuzz DEFERRED to the first public-artifact ship (the plan's own
  trigger); the always-on deterministic 2000-mutation sweep +
  every-truncation arm gate the parser meanwhile.
- **Bin**: `--vessel` / `--vessel-pubkey[=hex]` / `--vessel-force-downgrade`
  / `--vessel-print` (inspection without applying — structure always, sig
  verdict when a pin resolves). Every vessel failure is a LOUD boot
  failure (exit 1); there is no silent fallback to the compiled champion.
- **Gates** ([Bench 002](.benchmarks/002_vessel_format_gates.md)): 47/47
  green on m3 (aarch64) AND the 4090 (x86_64) — same eight result lines;
  G1 vessel-apply replay in-process AND through the wire at hold/no-hold;
  the full fail-closed battery (tamper classes by region, truncations,
  unknown/revoked keys, hosted-only, fork/downgrade boot refusals);
  clippy `--workspace --all-targets` 0 findings.
- The replay/downgrade gap the pre-T1 review found (an old VALIDLY-signed
  vessel was openable by the filed plan) is CLOSED by the monotonic gate
  and pinned by tests on both arches.
- Next: P4/P5 — hosted serving + deployment, both in private homes
  (riir-dapps / riir-deployer); this repo's part ends at the wire and the
  format.
