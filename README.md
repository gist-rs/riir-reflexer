# reflexer

The public decision ENGINE: runs genomes over game sims and answers typed
decisions over [`decision_wire`] — `choice` / `score` / `noul`, with
abstention as a first-class answer. Public substrate, public genomes;
improvement machinery lives elsewhere (see `BOUNDARY.md`).

- Engine substrate: [`katgpt-tetris`] (board sim + lookahead + rulebook
  evaluator + the frozen Bench-892 reference genome `68cae9d382014662`).
- Wire: `katgpt_core::decision_wire` (opt-in feature of [`katgpt-core`]).

Zero network calls, in the bin or any measurement path. Zero `riir-*`
dependencies. MIT.

## Build

```sh
cargo build --release --bin reflexer
cargo test                      # lib + protocol gates + G1 bit-identity
cargo clippy --all-targets -- -D warnings
```

The path deps expect `../katgpt-rs` beside this repo.

## Run

```sh
./target/release/reflexer [--record <path.jsonl>] [--version]
```

One JSON request per stdin line → one JSON envelope per stdout line.
Malformed input answers a typed error envelope and the pipe stays alive;
EOF exits 0. `--record` appends replayable trajectory rows and, at EOF,
writes a signed `<path>.manifest.json` (see §Trajectory submission).

## The measurement contract (wire schema)

`DecisionRequest.state` carries a JSON object with the game facts:

```json
{
  "board":   ["..........", "... 20 rows × 10 chars of '#'/'.' ..."],
  "cur":     "I",
  "next":    "T",
  "held":    null,
  "hold_ready": true,
  "bag":     ["T", "L"]
}
```

- `board` is exactly `Board::to_strings` output (row 0 = top); the decode
  is fail-closed on shape.
- `bag` is the 7-bag remainder AFTER `next` was drawn (depth-3 search
  support reads it — an inexact transport breaks bit-identity).

Questions the engine knows (anything else ABSTAINS, confidence 0):

| id | kind | contract |
|---|---|---|
| `place` | `choice` | Options MUST be the engine's canonical enumeration: `decide_scored` candidate order (no-hold landing options first, then the hold swap), labeled `h{0\|1}i{index}`. Outcome = the first strict argmax — the same fold `decide` is, so wire games are bit-identical to `play_game`. A diverging option COUNT is a typed `options_count` error (fail-closed). No legal placement → ABSTAIN. |
| `state` | `score` | Rubric MUST be the canonical 5 levels `critical, poor, fair, good, excellent`. Level from σ(v/200) of the position's value-to-go; probabilities are piecewise-linear between level centers. |
| `survive` | `noul` | yes = a legal placement exists; `p_yes` = σ(v/200). |

All questions in one request share ONE search (the value-to-go is
computed once). Answers validate against
`DecisionResponse::validate_against`.

### Envelope

```json
{"proto":1,"genome":"68cae9d382014662","response":{...},"in_engine_decision_ns":312000}
```

`in_engine_decision_ns` is the engine's own in-process decision time
(serde + stdio excluded) — render it beside the round-trip; never display
a sub-ms decision through a same-size subprocess overhead.

### Error codes

`bad_json` · `bad_request` (wire-structural) · `bad_state` ·
`question_kind` · `options_count` · `internal`. The pipe stays alive after
any error envelope.

## The measurement lane

```sh
cargo build --release --bin reflexer --example measure
cargo run --release --example measure            # + MAX_LOAD / --seeds / --cap / --out
```

Subprocess the built bin; pinned seeds; per-row artifact pins (binary
BLAKE3 + genome digest); G1 bit-identity proven inline against the
in-process oracle on every measured game; in-engine vs round-trip side by
side; REFUSES on battery or over the load ceiling (`MAX_LOAD`, default 6)
— a number from an invalid box state measures the scheduler, not the
engine. Artifact JSON lands in `.benchmarks/data/001/artifact.json`.

## Trajectory submission (free, opt-in, OFF by default)

`--record rows.jsonl` writes one replayable row per answered request and a
signed manifest at EOF:

- rows = the exact request JSON + the engine's outcomes;
- manifest = row count + BLAKE3 of the rows file + genome digest + the
  Ed25519 verifying key + a signature over
  `reflexer-trajectory-v1\n{rows}\n{rows_blake3}\n{genome}\n`.

The signing key is the per-machine key at `$REFLEXER_SUBMISSION_KEY` or
`~/.config/reflexer/submission.ed25519` (created 0600 on first use). No
billing code, no tokenomics — the private improvement loop
replay-verifies rows against its own engine before crediting anything.

## Wasm + the Cloudflare Worker (`crates/reflexer-wasm`, `cloudflare/reflexer-worker`)

The engine also builds as ONE `wasm32-wasip1` module that runs in two
hosts — the browser (the reflex-site arena's "Reflexer · wasm local" board)
and a Cloudflare Worker (`https://reflexer.foxfox.workers.dev`, the arena's
"Reflexer · Cloudflare" board + the playground). Same bytes, so the two
answer identically and differ only by the network hop.

```sh
cloudflare/reflexer-worker/build.sh [--site ../reflex-site]   # wasm (+ the site mirror)
curl -s https://reflexer.foxfox.workers.dev/                  # {name, version, proto, genome}
curl -s -X POST https://reflexer.foxfox.workers.dev/v1/decide -d @request.json
```

- `POST /v1/decide` takes one `DecisionRequest` and answers the bin's exact
  stdout envelope (`serve::envelope_line`, gated line-for-line against the
  bin by `tests/serve_parity.rs`): 200 success · 422 typed error · 500
  `internal` (a wasm trap; the instance is re-created).
- Stateless and free: no secrets, bindings, storage, or request logging;
  CORS open.
- `in_engine_decision_ns` reads ~0 on Workers (their clock is frozen during
  compute — `X-Reflexer-Clock: frozen-during-compute`); the caller's round
  trip is the honest latency.
- `reflexer_host.mjs` is the zero-dependency WASI shim + host class both
  hosts load (the site mirrors it as `assets/reflexer_host.js`).
- Deploy: manual from the M3; `.github/workflows/reflexer_worker.yml` is the
  main-only CI mirror (needs the `CLOUDFLARE_API_TOKEN` secret).

## Repo map

- `src/engine.rs` — the engine (reference genome + question mapping)
- `src/lane.rs` — the bin-only measurement lane driver (one copy shared
  by the G1 test and the `measure` example)
- `src/state_codec.rs` — the wire state schema
- `src/proto.rs` — the line envelopes
- `src/readout.rs` — the Bench-817 confidence dispatch (inherited)
- `src/serve.rs` — one request line → one envelope line, transport-free
- `crates/reflexer-wasm` · `cloudflare/reflexer-worker` — the wasm build + its Worker
- `src/record.rs` — the trajectory submission client
- `examples/measure.rs` — the measurement lane + G2 budget gate
- `.proposals/001` · `.plans/001` — design + execution record

[`decision_wire`]: ../katgpt-rs/crates/katgpt-core/src/decision_wire.rs
[`katgpt-tetris`]: ../katgpt-rs/crates/katgpt-tetris
[`katgpt-core`]: ../katgpt-rs/crates/katgpt-core
