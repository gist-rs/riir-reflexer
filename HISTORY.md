# HISTORY.md — riir-reflexer (public)

## 2026-10-05 — issue 001 closed: the E18 first-mint runbook moves to Plan 005 (delegated verdict)

E18 (`DEFAULT_PINS` empty until the first public vessel mint) closes TRIGGER-ONLY: the
VERIFIED runbook (mint → verify → pin; payload is `--in` not positional; `--vessel-print`
early-returns and does not combine with `--vessel`) lives at `.plans/005_default_pins_first_mint.md`
with the one `- [ ]` row — land the `DEFAULT_PINS` row + the consumers' operator pin at the
first public vessel mint. Deterministic minting — re-minting moves no pin. Issue file removed
(full text in git history); `.plans/003`'s E18 rows superseded by Plan 005.

## 2026-10-04 — vessel-mint genesis: the keygen tool, the stub worker, and the public key record (`e81d66d` + `d29de0b` + `4b40003`)

- **`vmint-keygen`** — the mint-key generation tool (Ed25519 seed → CF secret); openssl
  ladder repair `e81d66d`; `--root` `mkdir -p`s the parent dir 0700 under umask 077
  (`d29de0b`).
- **`cloudflare/vessel-mint`** — placeholder deployment (all routes 503, no mint path, never
  reads the seed binding) so the genesis secret has a home BEFORE the real worker lands
  (the autopilot replaces the code at the same name; secrets persist across deploys);
  `wrangler` 4.147's `secret put` takes `--name`, not `--worker`.
- **`cloudflare/vessel-mint/GENESIS.md`** — the two PUBLIC verifying keys (ROOT offline,
  signs delegations/revocations only + MINT online, public-class-only, key_id 1) with
  custody notes; a lost mint seed is recovered by ROOT re-delegation (CF secrets are
  write-only, never readback). Seeds exist only in `~/cold` and the CF secret; never
  committed (the `.wrangler/` account-id cache joined `.gitignore` at amend time, before push).
- Same day, later: the relation-flow WALK joined `.docs/06_resources` (`2433a80`, riir-ai Plan 620
  Group R — `reflexer_relation_flow.walk.json`, derived by reflex-site
  `scripts/build_flow_walks.py`, never typed).

## 2026-10-04 — `.docs/06_resources`: the relation section + the first mirrored figure (`ba943d8`)

Plan 004 (verdict AGREE round 2): the Reflex ↔ Reflexer relation gets the `#reflex` treatment
on reflex.gist.rs `/resources/#reflexer` — placed AFTER `#rethink` (the ladder stays
contiguous). This repo became the owning doc: `.docs/06_resources/resources.md` (public-copy
law compliant — its mirror is served) + the `reflexer_relation_flow` gfflow block (branch 2a
text / 2b game turn, merge at the envelope, the vessel back edge), rendered by the fleet
renderer to desktop + 390px `_m`. This repo joined the renderer's SOURCES and sync_mirror's
roots (3 pairs). Landed with the riir-ai Plan 620 P1 takeover (reflex-site `fe69bd2` + `12538b9`,
riir-rethink `7b86e8b`); section + mirrors live 2026-10-04.

## 2026-09-30 — sigmoid delegated to the substrate, adjudicated pick-level (`fb6ac8f`; issue 002 closed)

The substrate-first wave audit (2026-09-30 window) flagged `engine.rs`'s module-local
one-branch sigmoid beside `katgpt_core::exact_sigmoid_f64` — not bit-identical on the
negative domain (the margin path's args are ≤ 0 always). Verdict: the G1 pin is PICK-level
(decision sequences + stats + genome id; proto_gates holds probabilities to a 1e-3 sum
tolerance, not bits) → delegated: the local body DELETED, `sigmoid` is now
`use katgpt_core::exact_sigmoid_f64 as sigmoid` (bit-identical x ≥ 0 incl. `36.0`; 8-ulp
bound on the reachable negative domain). Permanent arm
`sigmoid_delegation_matches_frozen_legacy_body` keeps the frozen legacy body verbatim as the
reference — if the substrate kernel ever drifts (saturation, narrowed intermediate,
polynomial swap), the pin reds and forces re-adjudication rather than silently absorbing it.
Honest far-tail note: on `(-745.1, -709.8)` the legacy body saturated to `0.0` while the
stable form keeps the true subnormal — an accuracy IMPROVEMENT, unreachable after the f32
narrowing. Gates: full `cargo test` exit 0 (lib 10/10; g1 5/5; vessel 8/8; proto 9/9;
serve-parity 1/1); clippy `--all-targets -- -D warnings` clean; wasm32-wasip1 clean; fmt
clean. `.issues/002_local_f64_sigmoid_beside_exact_sigmoid_f64.md` removed at closure (002
consumed). Precedent lineage: riir-reflex Issue 014 (delegated, bit-identical domain),
riir-chain Issue 156 (recorded-refusal arm — the menu this adjudication came from), riir-neuron-db
Issue 611 (delegated).

## 2026-09-27 — the class-aware payload cap (`049a583`): HOSTED-ONLY may carry 16 MiB; the public reader's 1 MiB untrusted bound is UNCHANGED

The cap exists to bound hostile inputs, not to size artifact classes (banking77 is ~9.6 MiB;
five of the six winners exceed 1 MiB); the classes read on two different trust surfaces, so
the bound is per-class: `peek()` reads the class from the SIGNED header before any payload
work (HOSTED-ONLY may carry up to `MAX_HOSTED_PAYLOAD` 16 MiB, ~1.6× headroom over
banking77, while PUBLIC-RELEASE stays at `MAX_PAYLOAD` 1 MiB); `open()` reads the 132-byte
prefix FIRST and refuses a file DECLARING the hosted class before any payload byte
(`HostedOnlyPath` — a structural fact about the bytes, pre-auth by construction);
reflexer-wasm never loosens. Format stays v1, layout unchanged; no encoder for class 1
exists (unchanged by design). Verdict-reviewed (2 rounds): the single-cap raise was REVISEd
to this class-aware shape — the public reader is where untrusted input actually arrives.
Consumer side: riir-instinct serves banking77 as one 10 MB vessel (`source: Vessel`); the
cap-pin gate is instinct `f2fd12a`.

## 2026-09-25 — pre-birth

Design closed (reviewed; three negotiation rounds + filing checks); owner visibility:
**public engine** (the riir-shader posture); the full design record lives privately in
riir-train. Docs landed md-only (no `.git` yet, by design): this file, AGENTS.md,
BOUNDARY.md, `.proposals/001_reflexer_engine.md`. P0 filed and committed in katgpt-rs:
Issue 893 — promote the tetris substrate (sim + lookahead + rulebook) to a public
feature-gated module, bit-identity GOAT, one copy, forward freeze at the Bench-892 champion.
Improvement/hosted/deploy concerns live in private sibling repos.

## 2026-09-25 — PUBLIC BIRTH (P1)

Licence MIT; BOUNDARY.md gained the `Visibility: public` attribute + the `## Owns` section;
the workspace boundary guard grew check **C3b** (public-leaf law): a public repo path-depping
any sibling other than katgpt-rs is a mechanical violation. Registered: workspace repo_set +
the riir-ai CANONICAL matrix row (`riir-reflexer` → none INTO riir-ai; katgpt-rs path deps
only). Remote: gist-rs/riir-reflexer (public); develop is the working branch. Next: P2, gated
on katgpt-rs Issue 893 (P0) landing.

## 2026-09-25 — P0 LANDED (the substrate crate exists)

katgpt-rs Issue 893 RESOLVED (`243c38a1b`, header touch-up `4db7824de`): `katgpt-tetris`
leaf crate — sim byte-identical, lookahead/rulebook with 4 module-path rewrites total, full
Bench-891/892 bit-identity battery PASS, G2 no-regression (0.32-0.33 ms/decision), champion
genome `68cae9d382014662` pinned by crate test as the frozen REFERENCE. Record: katgpt-rs
HISTORY.md §Issue 893. BOUNDARY `May depend on` row updated (P2 wires it). Next: P2 (plan:
`.plans/001`).

## 2026-09-25 — P2 LANDED (engine bin + measurement lane + submission client)

- Package born: lib + bin `reflexer` (edition 2024, MIT, `publish = false`); deps
  katgpt-core (`decision_wire`) + katgpt-tetris (both path deps into `../katgpt-rs` — the
  C3b leaf law) + serde/serde_json + blake3 + ed25519-dalek/rand_core; Cargo.lock committed.
- The engine: ONE genome serves the bin — the frozen Bench-892 champion
  (`68cae9d382014662`, asserted at load). One `decide_scored` search answers a whole
  request: `place` (first-strict-argmax), `state` (5-level rubric via σ(v/200)), `survive`
  (noul). Unknown questions ABSTAIN (confidence 0) — abstention is structural (top-out),
  never a threshold inside game replay; bit-identity demands it. Sigmoid-margin weights at
  the population-std scale (never softmax); the Bench-817 confidence dispatch
  (`src/readout.rs`).
- Wire: response envelopes carry `in_engine_decision_ns`; typed error codes (`bad_json`,
  `bad_request`, `bad_state`, `question_kind`, `options_count`, `internal`); the pipe
  survives every malformed line; EOF exits 0; `options_count` is the fail-closed
  bit-identity defense (a diverging client enumeration gets a typed refusal, not a
  silently-misaligned index).
- G1 gate (`tests/g1_champion_replay.rs`): wire-driven games byte-identical to the
  in-process oracle at three geometries — decision sequences AND game stats.
- Measurement lane (`examples/measure.rs`, `src/lane.rs`): subprocess the built bin; binary
  BLAKE3 + genome digest pinned per artifact; REFUSES on battery and over MAX_LOAD (default
  6); G2 budget enforced in-run (`BUDGET_*`: in-engine no-hold p50 ≤ 500 µs, hold p50 ≤
  1 000 µs, p99 ≤ 3 000 µs both postures — corrected 2026-10-04; this line originally read
  "p50 ≤ 500 µs, p99 ≤ 1500 µs", which matched neither the landed source nor Bench 001's
  per-posture budgets).
- Trajectory submission (opt-in `--record`): replayable rows + a signed manifest (Ed25519
  over `reflexer-trajectory-v1\n{rows}\n{blake3}\n{genome}\n`), per-machine key at
  `$REFLEXER_SUBMISSION_KEY` or `~/.config/reflexer/submission.ed25519` (0600). No billing
  code, ever.
- BOUNDARY trued up: katgpt-core's `hint_regret` feature lands WITH its consumer (not yet
  wired); only `decision_wire` is enabled. Next: P3.

## 2026-09-25 — P3 LANDED (the vessel format crate)

- **`crates/reflexer-vessel`** (public, MIT; deps blake3 + ed25519-dalek only): format v1 =
  fixed 68-byte header (magic · format version · flags · key-id · artifact version · parent
  commitment · payload len) + 64-byte ed25519-STRICT signature over `header ‖ payload` +
  payload; manual LE parse, zero serde; `commitment = blake3(header ‖ payload)` — the
  lineage chain and the signature cover the same bytes; PAYLOAD-AGNOSTIC (the only
  bytes→genome seam is the engine's `from_vessel_payload`).
- **Two-class law**: `encode_public` is the ONLY public writer; the reader refuses
  HOSTED-ONLY fail-closed AND only after authenticity (a forged file cannot spoof the
  hosted-only refusal message).
- **Rotation**: key-id in the header, compiled-in pin table (`DEFAULT_PINS` EMPTY until the
  first artifact ships), revocation, the `--vessel-pubkey` operator wildcard pin (the SEAL
  `SEAL_VESSEL_PUBKEY` precedent).
- **Monotonic apply**: `check_monotonic` refuses `OlderThanCurrent` and `VersionFork`; the
  bin's `--vessel-force-downgrade` is the operator path and LOGS (tested both arms).
- **Hardening** (`50409ec`): single-read bounded `open` (`take(cap+1)` — no stat-then-read
  window), verify-before-parse with the 1 MiB payload cap, `verify_strict` everywhere,
  constructive whole-engine apply, unknown-anything fails closed; cargo-fuzz DEFERRED to the
  first public-artifact ship (the deterministic 2000-mutation sweep + every-truncation arm
  gate the parser meanwhile).
- **Bin**: `--vessel` / `--vessel-pubkey[=hex]` / `--vessel-force-downgrade` /
  `--vessel-print`; every vessel failure is a LOUD boot failure (exit 1) — no silent
  fallback to the compiled champion.
- **Gates** ([Bench 002](.benchmarks/002_vessel_format_gates.md)): 47/47 green on m3
  (aarch64) AND the 4090 (x86_64) — same eight result lines. The pre-T1 replay/downgrade gap
  (an old VALIDLY-signed vessel was openable by the filed plan) is CLOSED by the monotonic
  gate, pinned by tests on both arches. Next: P4/P5 — hosted serving + deployment, private
  homes (riir-dapps / riir-deployer); this repo's part ends at the wire and the format.

## 2026-10-06 — Issue 004 CLOSED: artifact class vocabulary + manifest schema v0 + runtime per-env pin refusal (riir-ai Plan 623 T2)

ONE vocabulary home for at-rest artifacts that public repos can depend on — this repo
carries the class crosswalk, the versioned manifest schema, and the runtime env-key refusal.
Landed `3757649` → `e251274` → `76853af` → `593a30f` (spec + fixtures + schema v0 draft +
env pin refusal + the `[[source_pin]]` table); issue file removed per the noise-reduction
rule. Verdict recorded there: **fixture/mock only, spec ships as v0 DRAFT pending the T0 A10
ratification; `core.hooksPath` enrollment stays T8.** The v0→v1 bump is the
A10-ratification follow-up (owner); T7's `artifact-sync lint` must agree with the fixture
validator. Nothing open in this repo.

## 2026-10-05 — Issue 003 CLOSED: compiled-in authority pins, dev_pins-gated wildcard, persisted apply floor (riir-rethink Plan 009 P1)

Trust defect (filed 2026-10-04 from riir-rethink Issue 022 / Plan 009 P1): a stock release
build trusted whatever key the operator passed — `DEFAULT_PIN_KEYS` was empty,
`--vessel-pubkey` installed a wildcard for ANY key-id, and nothing persisted across runs
("a fork's next artifact would need our signature" was NOT true). Closed at `0c7d7fb` (all
three gates green in both feature postures + wasm32-wasip1):

- **T1 — compiled-in authority pins.** `DEFAULT_PIN_KEYS` carries the AUTHORITY ROOT key-id
  1 (the `gist-rs vessel-sign v1` derivation, public key `d390c0ae…3c99`; the seed never
  left the M3, never in any repo); a const assert makes an emptied table a COMPILE error;
  `--vessel-pubkey` is refused at parse time (exit 2, naming Issue 003) unless the bin was
  built with the `dev_pins` feature (default off).
- **T2 — the persisted apply floor** (`src/vessel_apply.rs`): `{key_id, artifact_version,
  commitment}` written atomically (tmp + rename, 0600) after every fully successful apply;
  refusals for rollback / same-version fork / non-advancing key rotation across restarts;
  corrupt state is LOUD (never genesis — that would re-open the rollback hole); force-able
  via the existing `--vessel-force-downgrade` audit trail; `--vessel-state` overrides the
  default `<vessel>.state.json`.
- **T3 — the gates.** `tests/vessel_apply_gates.rs` (fork-signed refused by the stock pins;
  unknown key-id fail-closed; revocation is the REASON; the flag refusal at the process
  boundary; the apply floor's process-level rollback e2e; the transcription gate —
  `pins_from_bytes` silently skips bad bytes). The wildcard-dependent scenarios moved
  verbatim to `tests/vessel_dev_gates.rs` (`required-features = ["dev_pins"]`, fixtures at
  key-id 7 — the wildcard cannot override the compiled authority id 1).
- E2e proof (release build): a fork key minted at key-id 1 and booted with NO flag →
  `signature failed strict verification`, exit 1; with its own pubkey passed → the Issue 003
  refusal, exit 2. Live-verified on aarch64 (M3).
- Key-separation context: the derivation substrate + the separation gates landed the same
  day in riir-rethink (`52dfc54`, Plan 009 P0.1+P0.3); the mint-side key split
  (`--sign-key` / `--payload-key`) is riir-train Issue 612 — the remaining half of Issue 022
  T1.
