# Proposal 001 — riir-reflexer: the public decision ENGINE

**Status:** DESIGN AGREED (reviewed 2026-09-25). Owner visibility decision:
**public engine** (the riir-shader posture). The full design record lives
privately in riir-train; this document is the public-facing engine proposal
and deliberately carries no strategy or economics detail.

Branch: `develop` (at birth)
Owner: katopz
Related: katgpt-rs Issue 893 (the substrate promotion — the board sim,
lookahead, rulebook evaluator, and reference genome move to a public
feature-gated katgpt-rs module), riir-reflex (the serving repo — measures
this engine through a bin-only lane), `katgpt_core::decision_wire` (the wire)

## TL;DR

A PUBLIC engine repo for self-contained decision engines: it runs genomes
over game sims and serves typed decisions (`choice`/`score`/`noul`,
abstention first-class) over `decision_wire`. Zero riir-* dependencies —
`katgpt-core` plus the katgpt-rs substrate module only (the riir-shader
leaf law). Devs can build their own engines, author genomes, add game
domains, and integrate the bin into their own stacks. Improvement
machinery, hosted serving, and deployment live in private sibling repos
and are out of scope here by design.

## What ships here

- **The engine bin** (`reflexer`): loads a genome, runs it over a sim,
  answers decision_wire requests. Response envelopes carry the engine's
  own in-process decision time so harnesses can render it beside the
  round-trip (never display a sub-ms decision through a same-size
  subprocess overhead).
- **The vessel format crate** — ONE public format: serialize/commit/verify
  for decision artifacts (whole snapshots + lineage header), with a
  two-class header:
  - PUBLIC-RELEASE: lagged champions; run anywhere; content is
    extractable and that is accepted.
  - HOSTED-ONLY: served as decisions only; never shipped to uncontrolled
    hardware; encrypted at rest on an encrypted-storage lane.
  - Key rotation: key-id in the header, multi-key pin, revocation path;
    the pin fails closed.
  - Apply semantics: ATOMIC VERSIONED SWAP — never a weighted blend
    (blends do not preserve move rankings). Fingerprints proven on both
    aarch64 and x86_64 by execution before any artifact crosses machines.
- **The trajectory submission client** (free, opt-in): signs decision
  trajectories against the public wire contract. No billing code in this
  repo, ever.
- **Game-domain sims** as public source (public-origin list maintained in
  BOUNDARY.md; the tetris substrate is consumed from the katgpt-rs module —
  one copy, never duplicated here). Nothing from private game domains may
  enter.

## Measurement laws

The arena lane this repo's bin appears in measures it the same way it
measures every external engine: subprocess, seeds pinned, per-row artifact
pins (binary BLAKE3 + genome/vessel digest). Zero network calls in any
measurement path.

## GOAT gates

- **G1 (bit-identity):** promotions and artifact applies reproduce the
  Bench-892 fingerprint discipline (per-seed h2h digits, byte-identical
  fixture replay, `eval ≡ eval_with`, dual-arch).
- **G2 (perf):** matched-time comparisons only; references re-pinned under
  `--release` with box state recorded.
- **G3 (no regression):** katgpt-rs semantics untouched by consumption.
- **Report-the-Floor:** hosted lanes must beat the public champion
  (`68cae9d382014662`) at matched time before charging anything.

## Honest caveats

1. PUBLIC-RELEASE artifacts are extractable once shipped — accepted by
   design; the answer to that is release lag, not obfuscation.
2. HOSTED-ONLY artifacts must never reach uncontrolled hardware; a single
   leak moves that artifact to treated-as-public.
3. This repo holds no improvement machinery: engines here get better
   elsewhere and land here as artifacts. Do not grow a search loop here.
4. Licence is a deliberate birth-time choice (matching riir-shader);
   attribution headers on ported substrate follow the katgpt-rs convention.

## Sequencing

- P0 — katgpt-rs Issue 893 (substrate promotion; filed).
- P1 — PUBLIC birth: named-file staging only; licence; registration with a
  public/visibility attribute so any future riir-* dependency fails the
  boundary check mechanically.
- P2 — engine bin + the bin-only measurement lane + submission client.
- P3 — vessel format crate (read/verify, class header, rotation).
- P4/P5 — hosted-serving and deployment concerns live in private sibling
  repos; this repo's part ends at the wire and the format.
