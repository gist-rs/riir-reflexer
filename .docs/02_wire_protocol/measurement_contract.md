# The measurement contract — the line protocol, the state schema, the envelope

One JSON request per stdin line in; one JSON envelope per stdout line out.
Malformed input answers a typed error envelope and the pipe stays alive; EOF
exits 0. Authority chain: `src/state_codec.rs` (state schema),
`src/proto.rs` (envelopes), the `src/engine.rs` module doc (question
mapping), `src/serve.rs` (the transport-free core) — gated by
`tests/g1_champion_replay.rs`, `tests/proto_gates.rs` and
`tests/serve_parity.rs`. The README stays the build surface; this is the
book copy.

## The state schema

`DecisionRequest.state` carries the game facts as one JSON object — the
wire's string field holds the serialized `GameState` (`src/state_codec.rs`):

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

| field | contract |
|---|---|
| `board` | exactly 20 rows × 10 chars of `#`/`.` — exactly `Board::to_strings` output, row 0 = top; `Board::from_strings` round-trips the cell grid bit-for-bit |
| `cur` | the piece to place, one-letter id (`I O T S Z J L`) |
| `next` | the preview piece id |
| `held` | the held piece id, or `null` |
| `hold_ready` | hold not yet used this drop (the guideline rule); defaults `true` — the `play_game` posture |
| `bag` | the 7-bag remainder AFTER `next` was drawn, in draw order; empty = a fresh full bag is next |

Decode is fail-closed on shape: wrong row count, row width, row chars or an
unknown piece id each answer a typed `bad_state` naming the offender
(`StateError`). The bag is not advisory — depth-3 search support reads it,
so an inexact transport breaks decision bit-identity.

## The questions

| id | kind | contract |
|---|---|---|
| `place` | `choice` | Options MUST be the engine's canonical enumeration: `decide_scored` candidate order (no-hold landing options first, then the hold swap), labeled `h{0\|1}i{index}`. Outcome = the first STRICT argmax — the same fold `decide` is, so wire games are bit-identical to `play_game`. A diverging option COUNT is a typed `options_count` error (fail-closed). No legal placement → ABSTAIN. |
| `state` | `score` | Rubric MUST be the canonical 5 levels `critical, poor, fair, good, excellent` (lowest first). Level from σ(v/200) of the position's value-to-go; probabilities are piecewise-linear between level centers. |
| `survive` | `noul` | yes = a legal placement exists (the candidate set is non-empty); `p_yes` = σ(v/200). |

All questions in one request share ONE search — the value-to-go is computed
once (`src/engine.rs`). σ is `katgpt_core::exact_sigmoid_f64` over
`V_REF = 200.0`; probabilities are sigmoid-margin weights (sigmoid, never
softmax — the tetris_09 site-walk convention) and live only in the readout,
never in the argmax. On top-out (no candidates anywhere): `place` ABSTAINS,
`state` scores `critical`, `survive` answers no. Answers validate against
`DecisionResponse::validate_against`.

**Unknown question id → ABSTAIN, confidence 0.** Abstention is first-class
on this wire, and the policy is STRUCTURAL — the gate is "the engine has no
legal answer" (top-out), never a confidence threshold. A threshold would sit
inside game replay and break bit-identity with the substrate battery.

## The envelope

```json
{"proto":1,"genome":"68cae9d382014662","response":{...},"in_engine_decision_ns":312000}
```

| field | meaning |
|---|---|
| `proto` | line-protocol version (currently 1; bump on any breaking envelope change) |
| `genome` | the genome digest that served (`Genome::id()`, blake3-16) — the compiled champion `68cae9d382014662` unless a vessel replaced it at boot |
| `response` | the `DecisionResponse` |
| `in_engine_decision_ns` | the engine's in-process decision time for THIS request — serde + stdio excluded |

`in_engine_decision_ns` is the zero-network measurement law: render it beside
the round-trip, never display a sub-ms decision through a same-size
subprocess overhead (the pipe alone adds ~150 µs p50 over the engine's own
time — [Bench 001](../.benchmarks/001_p2_engine_bin_gates.md)).

The error envelope is `{proto, error:{code, message, request_index}}`:
`message` is display text, never parsed; `request_index` is the 1-based
stdin line the request arrived on. The pipe stays alive after every error
envelope.

## Error codes

Stable vocabulary, one per failure class — machine-branchable.

| code | when |
|---|---|
| `bad_json` | the line did not parse as a `DecisionRequest` |
| `bad_request` | wire-structural validation failed (`request.validate()`) |
| `bad_state` | the `state` field did not decode (shape or piece id) |
| `question_kind` | a known question id arrived with the wrong kind (`place` must be `choice`, `state` a `score`, `survive` a `noul`) |
| `options_count` | a `place` option count or `score` rubric size diverged from the canonical enumeration — fail-closed: bit-identity demands one enumeration |
| `internal` | the engine panicked. Where unwinding exists, `catch_unwind` answers the envelope; under `panic=abort` (every wasm build) the trap surfaces to the host, which re-creates the instance and answers `internal` itself |

## The HTTP twin

`POST /v1/decide` on the Cloudflare Worker
(`https://reflexer.foxfox.workers.dev`) answers the bin's exact stdout
envelope for the same request (`serve::envelope_line`; gated line-for-line
against the bin by `tests/serve_parity.rs`):

- **200** success — the success-envelope bytes.
- **422** typed error — the error-envelope bytes.
- **500** `internal` — a wasm trap; the instance is poisoned and re-created
  for the next request.

`GET /` answers `{name, version, proto, genome, endpoints}`; bodies over
64 KiB are refused (413). The Worker is stateless and free — no secrets, no
bindings, no storage, no request-body logging; CORS open.

**`X-Reflexer-Clock: frozen-during-compute`** — Workers freeze both clocks
during pure compute (they advance on I/O only), so `in_engine_decision_ns`
reads ~0 on the Worker; the caller's round trip is the honest latency there.

## Where the contract is proven

- `tests/g1_champion_replay.rs` — G1: wire-driven games byte-identical to
  the in-process oracle (decision sequences AND game stats) at three
  geometries — hold `play_game` posture (seeds 1..=6, cap 240), no-hold h2h
  (seeds 1..=4, cap 200), garbage-board starts (seeds 11..=13, 8 rows @
  60%). Every measured game in the latency lane re-proves G1 inline.
- `tests/proto_gates.rs` — the envelope and error surface.
- `tests/serve_parity.rs` — Worker ≡ bin, line for line.
- [Bench 001](../.benchmarks/001_p2_engine_bin_gates.md) — the G2 budget
  record (in-engine p50 353 µs no-hold / 368 µs hold, against the 500 /
  1000 µs budgets) and the box-state variance disclosure.
