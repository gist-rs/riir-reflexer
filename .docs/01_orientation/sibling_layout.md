# Sibling layout — the dependency graph and where everything else lives

Path deps assume this on-disk layout:

```
/git/riir-reflexer    ← this repo (public, MIT)
/git/katgpt-rs        ← katgpt-core (decision_wire) + katgpt-tetris —
                        the substrate, non-optional path deps
```

`../katgpt-rs` must sit beside this checkout for **any** cargo command, not
just builds: cargo reads every path dep's manifest while loading the
workspace graph, so `cargo check`, `cargo test` and even `cargo metadata`
fail without it. There is no `optional` escape, and none is wanted — the
substrate *is* the engine (from `../katgpt-rs`, per the root `Cargo.toml`):

- `katgpt-core` (`decision_wire` feature) — the typed decision wire.
- `katgpt-tetris` (the katgpt-rs Issue-893 substrate crate) — board sim +
  lookahead + rulebook evaluator + the frozen Bench-892 reference genome
  `68cae9d382014662`.

## The leaf law

**Zero `riir-*` dependencies.** `BOUNDARY.md` carries
`Visibility: public`, and that line is machine-read by the workspace
boundary guard: check **C3b** (the public-leaf law, landed at this repo's
P1 birth) makes a public repo path-depping any sibling other than
`katgpt-rs` a mechanical violation — the `May depend on` table cannot be
widened into compliance by an edit. No tokenomics code, no signing keys, no
improvement loop — ever, in this repo.

## Workspace members

| member | role |
|---|---|
| root `reflexer` (lib + bin) | the engine: reference genome + question mapping (`src/engine.rs`), the stdio line protocol (`src/proto.rs`, `src/serve.rs`), the wire state schema (`src/state_codec.rs`), the confidence readout (`src/readout.rs`), the bin-only measurement lane (`src/lane.rs`), the trajectory submission client (`src/record.rs`) |
| `crates/reflexer-vessel` | the vessel format crate (P3): format v1 read/verify, two-class header (PUBLIC-RELEASE / HOSTED-ONLY), key-id rotation, monotonic apply — payload-agnostic; blake3 + ed25519-dalek only |
| `crates/reflexer-wasm` | the engine as ONE `wasm32-wasip1` module speaking the same line protocol (`serve::envelope_line`) — the bytes both wasm hosts load |

## Where everything else lives (BOUNDARY.md §Does not own)

| concern | correct home |
|---|---|
| The improvement loop and artifact minting | riir-train (private) |
| Hosted serving (metered/keyed/settled), settlement, contribution economics | riir-dapps (private) — this repo's free stateless demo Worker is not that |
| Deploy orchestration | riir-deployer (private) |
| The arena site / serving product | riir-reflex (site repo `gist-rs/reflex-site`) — this engine's wasm boards run there |
| Public substrate (board sim, lookahead, rulebook evaluator, reference genome) | the katgpt-rs Issue-893 module (`katgpt-tetris`) — consumed, never duplicated |
| VSL1/ARTB asset vessels | riir-neuron-db — a deliberate artifact-class split, recorded both sides; this repo's vessel format is its own public crate |

## The two wasm hosts — same bytes, one network hop apart

The engine builds as ONE wasm module (`crates/reflexer-wasm`, target
`wasm32-wasip1`) that runs in two hosts:

- **Browser, in-tab** — the reflex-site arena's "Reflexer · wasm local"
  board.
- **Cloudflare Worker** — `https://reflexer.foxfox.workers.dev`, the arena's
  "Reflexer · Cloudflare" board + the playground (`POST /v1/decide`).

Both load the same wasm bytes through
`cloudflare/reflexer-worker/reflexer_host.mjs` — the zero-dependency WASI
shim + host class — so the two boards answer identically and differ only by
the network hop. The site mirrors that host file as
`assets/reflexer_host.js`. A trap poisons a wasm instance; both hosts
re-instantiate rather than serve from a poisoned one.

## Verify

```sh
cargo check                                           # workspace resolves (needs ../katgpt-rs)
cargo test                                            # protocol gates + G1 bit-identity
cargo check -p reflexer-wasm --target wasm32-wasip1   # the wasm lane compiles
```
