# The measurement lane — the arena posture, one shared driver

`examples/measure.rs` over `src/lane.rs` — ONE lane driver shared by the
`measure` example and the G1 gate test. The posture is the arena's: this
repo's engine is measured exactly the way every external engine is —
subprocess the built bin, drive whole games through the wire, pinned seeds,
zero network. Bit-identity construction: the harness mirrors `play_game`'s
loop exactly, replacing its `decide` call with a wire round trip; the option
list is the engine's OWN canonical enumeration in both processes, so the
outcome index and the enumeration cannot drift.

```sh
cargo build --release --bin reflexer --example measure
cargo run --release --example measure            # refuses on battery / MAX_LOAD
```

## What every run pins

- **Binary BLAKE3** of the bin bytes + the **genome digest** — printed at
  the head of the run and carried in the artifact JSON, so a latency row is
  always attributable to exact bytes.
- **A PROVENANCE line** — arch, power source, loadavg, core count. A
  latency number without its box state is not a measurement.

## The box-state law (refuses, never warns)

- **Battery → REFUSED** (exit 1). A box with no battery (desktop) proceeds
  with a loud "no battery" note.
- **loadavg > `MAX_LOAD` (default 6, env override) → REFUSED** — a sibling
  session on the box means the number would measure the scheduler, not the
  engine (the reflex bench-preflight law).

Measured why (Bench 001): shared-box contention inflates WHOLE runs, not
samples — two single runs at the same 1-min load measured no-hold p50
**330 µs vs 631 µs**. Hence the read discipline: the battery runs
`--runs` times (default 3, the Issue-723 best-of discipline) and the budget
reads the BEST per-posture p50/p99 across runs — a real regression raises
the floor across runs, contention does not.

## G2 budgets (enforced in-run; exit 1 on breach)

| posture | p50 budget | p99 budget |
|---|---|---|
| no-hold (h2h geometry) | ≤ 500 µs | ≤ 3000 µs |
| hold (`play_game` geometry) | ≤ 1000 µs | ≤ 3000 µs |

Policy, with provenance: the substrate's published record is 0.32–0.33
ms/decision at the h2h (no-hold) geometry — the no-hold budget is that
record with headroom. The hold posture structurally searches ~2× the root
set (the hold candidate adds a second piece's landing tree), so its budget
carries the same headroom over ~2× the record. Re-pin only with provenance.

Accepted record ([Bench 001](../../.benchmarks/001_p2_engine_bin_gates.md),
M3 aarch64, AC, best-of-3): no-hold p50 **353 µs**, hold p50 **368 µs**;
pooled round-trip p50 505 µs.

## In-engine vs round-trip, side by side

Every envelope carries `in_engine_decision_ns` — the engine's own in-process
decision time (serde + stdio excluded). `BinLane` records (round-trip ns,
in-engine ns) per request and both are rendered. Never display a sub-ms
decision through a same-size subprocess overhead: the pipe adds ~150 µs p50
(subprocess + two serde hops) over the engine's own time. On the Cloudflare
Worker the clock is frozen during compute, so the caller's round trip is the
honest latency there.

## CLI knobs

| knob | default | effect |
|---|---|---|
| `--hold-seeds` | 6 | hold-posture seeds (seed 1 onward) |
| `--seeds` | 10 | no-hold seeds (seed 1 onward) |
| `--cap` | 500 | pieces per game; the hold posture runs `min(240, cap)` |
| `--runs` | 3 | battery repetitions; budget reads best-of |
| `--out` | `.benchmarks/data/001` | artifact dir |
| `--bin` | auto | explicit bin path (else `CARGO_BIN_EXE_reflexer` / `target/{release,debug}`) |
| `MAX_LOAD` (env) | 6.0 | the loadavg refusal ceiling |

## The artifact

JSON at `<out>/artifact.json` (schema `reflexer-measure-v1`): `bin_blake3`,
`genome`, `proto`, `provenance` (arch/power/loadavg), the budget block with
pass verdict, per-run stats, best-of per-posture stats, pooled round-trip
stats, and one row per measured game (`posture`, `seed`, `cap`, `pieces`,
`lines`, `g1_pass`). The run exits 1 if G1 or G2 fails — the lane is a gate,
not a reporter.

## G1, proven inline on every measured game

Each measured game's wire decision sequence AND game stats are compared
against the in-process oracle (`local_play_with_decisions` — `play_game`'s
exact loop with the decisions recorded) before it counts. The oracle itself
is sanity-asserted against `katgpt_tetris::rulebook::play_game`
(`tests/g1_champion_replay.rs::oracle_matches_play_game_exactly`), so the
reference cannot drift.

The G1 gate battery (`tests/g1_champion_replay.rs`, debug + release, every
run) holds three geometries:

| posture | seeds | cap | start |
|---|---|---|---|
| hold (`play_game`) | 1..=6 | 240 | empty board |
| no-hold (h2h) | 1..=4 | 200 | empty board |
| hold, garbage boards | 11..=13 | 200 | 8 rows @ 60% (`garbage_board`) |

Dual-arch law: fingerprints proven by execution on aarch64 AND x86_64 —
cross-arch decision identity is measured (all 16 measure-lane games produced
byte-identical stats on both arches; the decision path is f64 arithmetic +
comparisons, and the sigmoid lives only in the probability readout, never in
the argmax). Artifacts may cross machines with that proof in hand.
