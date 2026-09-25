# Plan 002 — the vessel format crate (P3)

**Status:** OPEN — filed 2026-09-25 at the P2 close (landing `11314ec`,
dual-arch proven). Scope source: `.proposals/001` §"What ships here" (the
vessel format crate) + riir-train Research 457 §Sequencing P3 (the private
design record). Minting tooling stays in riir-train — this repo ships the
READ/VERIFY/APPLY half plus the public format.

Branch: `develop`. Session markers on every commit.

## Premise (measured, standing)

- The engine bin (P2) serves the frozen champion `68cae9d382014662` from
  the compiled substrate. A vessel is how a DIFFERENT genome reaches the
  bin without a rebuild — the artifact class boundary is the product.
- P2's lane already proves the fingerprint discipline the apply gate
  needs: wire ≡ oracle bit-identity + dual-arch execution + cross-arch
  decision identity measured (`.benchmarks/001`).
- katgpt-rs freeze/thaw is the lineage precedent (atomic versioned swap,
  BLAKE3-checked); the vessel format is its public decision-artifact
  shape, not a copy of it.

## Tasks

- [ ] T1 Crate skeleton: workspace member `crates/reflexer-vessel`
      (public, MIT; zero deps beyond blake3/serde/ed25519-dalek already in
      tree — no new dep classes). BOUNDARY `Owns` already claims it.
- [ ] T2 Artifact format: whole-snapshot genome container + lineage
      header (parent commitment chain; the `Genome::to_line`/`from_line`
      wire is the payload — never a weighted blend).
- [ ] T3 Two-class header: PUBLIC-RELEASE vs HOSTED-ONLY (class declared
      in the header; HOSTED-ONLY never written by any path in THIS repo —
      the class exists so the READ side can refuse it on uncontrolled
      hardware, fail-closed).
- [ ] T4 Key-id rotation (read side): multi-key pin table, key-id in the
      header, revocation path — a signature under a revoked or unknown
      key-id FAILS CLOSED (the vessel never opens).
- [ ] T5 Apply semantics: ATOMIC VERSIONED SWAP at the engine boundary —
      the bin takes a vessel path, verifies, swaps the genome whole;
      fingerprints proven by execution on aarch64 AND x86_64 before any
      artifact crosses machines (consume P2's lane for the proof).
- [ ] T6 G1 gate: vessel-apply replay — champion-from-vessel decisions
      byte-identical to champion-from-substrate at every P2 geometry
      (reuse `tests/g1_champion_replay.rs`'s battery shape); G3: substrate
      untouched.
- [ ] T7 Docs: README §vessel (format, classes, rotation, apply law),
      AGENTS Current-state, HISTORY entry, BOUNDARY true-up, `.benchmarks/`
      gate record.

## Non-goals (private homes, forever)

- Minting/improvement tooling (riir-train), hosted routes + settlement
  (riir-dapps), any economics.
