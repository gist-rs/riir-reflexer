# BOUNDARY.md — riir-reflexer (public)

Visibility: public

The authoritative per-repo contract. On any conflict with AGENTS.md or
proposals, this file wins.

## Domain test

Is this the **public decision ENGINE** — running genomes over sims, serving
typed decisions over `decision_wire`, the vessel READ format, the trajectory
submission client, and public game-domain sims? YES → here. NO → another
repo; file there.

## Owns

- The engine bin (`reflexer`): load a genome, run it over a sim, answer
  `decision_wire` requests, with in-engine decision time in the response
  envelope.
- The vessel FORMAT crate (public): serialize/commit/verify for decision
  artifacts (whole snapshots + lineage header), the two-class header
  (PUBLIC-RELEASE / HOSTED-ONLY), key-id rotation on the read side.
- The trajectory submission client (free, opt-in; signs against the public
  wire contract; zero billing code).
- Public game-domain sims (public-origin list below).
- The engine's wasm build (`crates/reflexer-wasm`, `wasm32-wasip1`) and its
  free, stateless demo host (`cloudflare/reflexer-worker`: the line protocol
  over `POST /v1/decide` — no keys, no storage, no metering, no settlement;
  the stdio bin over HTTP). The same bytes run in-tab on the arena site.

## Does not own

| Concern | Correct home |
|---|---|
| The improvement loop and artifact minting | riir-train (private) |
| Hosted serving (metered/keyed/settled), settlement, contribution economics | riir-dapps (private) — the free stateless demo Worker above is not this |
| Deploy orchestration | riir-deployer (private) |
| The arena site / serving product | riir-reflex |
| Public substrate (board sim, lookahead, rulebook evaluator, reference genome) | the katgpt-rs Issue-893 module — consumed, never duplicated |
| VSL1/ARTB asset vessels | riir-neuron-db — a deliberate artifact-class split, recorded both sides; this repo's vessel format is its own public crate |

## May depend on

| dep | status |
|---|---|
| `katgpt-core` | non-optional (`decision_wire` enabled — the wire; `hint_regret` lands WITH its consumer, not yet wired; both opt-in features on katgpt-core's side) |
| `katgpt-tetris` (the katgpt-rs Issue-893 substrate crate) | landed 2026-09-25, katgpt-rs `243c38a1b` — the board sim + lookahead + rulebook evaluator + reference genome; wired into this repo's engine bin at P2 |
| `reflexer-vessel` (in-workspace member, P3) | the format crate — crates.io deps only (blake3, ed25519-dalek; both already root deps — no new dep classes), zero substrate coupling: payload-agnostic by design |

**Zero riir-\* dependencies — the leaf law.** `Visibility: public` above is
machine-read by the workspace boundary guard: any path dep on a workspace
sibling other than `katgpt-rs` fails the contract check mechanically — the
table above cannot be widened into compliance by an edit. No tokenomics code,
no signing keys, no improvement loop — ever, in this repo.

## Public-origin sims

- tetris (via the katgpt-rs Issue-893 module). Nothing from riir-games*,
  mmorpg, or seal domains may enter.

## Drift ledger

**None** at birth.
