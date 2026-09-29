# Issue 002 — engine-local one-branch f64 sigmoid beside `katgpt_core::exact_sigmoid_f64` (adjudication-gated)

**Status:** OPEN — detection-only, filed by the substrate-first wave audit
(2026-09-30 window, katgpt-rs skill run log). NOT fixed in the same commit as
detection, per the skill's law.

## Finding

`src/engine.rs:311` defines a module-local sigmoid:

```rust
#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
```

- **No rationale comment, no pin.** The doc comment nearby documents the
  `sigmoid_margin_probs` convention ("the tetris_09 site-walk convention,
  inherited (sigmoid, never softmax)") — but nothing speaks for the local
  sigmoid BODY.
- **Substrate exists and was built for exactly this caller class**:
  `katgpt_core::exact_sigmoid_f64` (katgpt-core `src/lib.rs:63`), whose doc
  reads "the form callers compute in when they narrow to f32 only at the end
  (riir-chain's congestion/forensic paths; Issue 156). Narrowing first and
  using the f32 variant would be a numerics change, not a delegation."
  This crate deps katgpt-core (`Cargo.toml:13`) and consumes it elsewhere.
- **Vocabulary translation (why a name grep misses it):** searched
  `fn sigmoid|fn fast_sigmoid|fn logistic`; the substrate ships as
  `exact_sigmoid_f64` — the "exact"/"_f64" tokens are the mismatch.

## Why this is adjudication-gated, not a blind fix

1. **The forms differ on the reachable domain — every call.** The substrate
   is the TWO-BRANCH stable form (`exp(x)/(1+exp(x))` for x < 0);
   the local body is the ONE-BRANCH `1/(1+exp(-x))`. `sigmoid_margin_probs`
   arguments are `(v − v_max)/scale` — **≤ 0 always** — precisely the branch
   where the two forms differ by rounding (different op sequences), not just
   in the overflow tail. Delegation is a numerics change on every served
   `choice` answer, not a bit-identical swap. (Contrast riir-reflex Issue 014,
   where the reachable domain was x ≥ 0 and delegation WAS bit-identical.)
2. **The output is the pinned decision surface.** `sigmoid_margin_probs`
   feeds `Answer::choice(id, best_at, probs, confidence)` (`engine.rs:240`)
   — the served envelope. The crate's G1 gate is bit-identity through the
   wire (`tests/g1_champion_replay.rs`, the Bench-892 champion `68cae9d3…`);
   changing probs changes the envelope the pin asserts.

## Admissible resolutions (the chain-Issue-156 menu)

- **(a) Delegate + permanent divergence-pin against the frozen legacy body**
  (`exact_sigmoid_f64` + a `sigmoid_delegation_matches_frozen_legacy_body`
  test) — ONLY IF the champion-replay pin is pick-level and survives the
  prob-level change; if probs are envelope-pinned, (a) is refuted by G1.
- **(b) Keep the local form + recorded-refusal marker + to_bits pin** (the
  `congestion::inclusion_probability` pattern chain landed for the same
  reason): in-source rationale naming the one-branch body as load-bearing
  for the pinned envelope, plus a bit-frozen test so substrate drift is
  detected rather than silently absorbed.

Either way the audit's actual complaint — an unexplained local body beside
available substrate — is discharged.

## Classification

DRY violation, **adjudication-gated** on the numerics caveat (the
Cephes-vs-libm standing lesson; reflex-014 / ndb-611 / chain-156 lineage).

## References

- Substrate: `katgpt_core::exact_sigmoid_f64` (Issue 156's f64 arm)
- Precedents: riir-reflex Issue 014 (delegated — bit-identical domain);
  riir-chain Issue 156 (recorded-refusal arm); ndb Issue 611 (delegated)
- Consumer surface: `src/engine.rs:240` (`Answer::choice`), `engine.rs:318`
  (`sigmoid_margin_probs`)
