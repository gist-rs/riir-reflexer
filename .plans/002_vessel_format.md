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

## Security posture (threat model — on record, reviewed before T1)

**Verdict: tamper-evident + fail-closed, by construction. Not unhackable
— the format authenticates the ARTIFACT channel; endpoint compromise and
public-artifact extraction are out of its scope by design (proposal
§Honest caveats).**

| attack | stopped by | residual |
|---|---|---|
| tampered bytes | BLAKE3 commit + Ed25519 over header+payload; any byte change breaks the chain; the engine never opens it | tamper is DETECTED, never accepted — an attacker may hand you a broken file, not a modified one that opens |
| forged vessel (own key) | key-id pin table, fail-closed on unknown/revoked key-id (T4) | a rebuilt/forked bin with an attacker pin table — the BINARY channel's scope (SHA256SUMS / brew hash), not the format's |
| replay of an old valid vessel | version + lineage commitments + MONOTONIC APPLY (binding on T5 below) | forced-override is an operator action, logged |
| HOSTED-ONLY on uncontrolled hardware | class bit lives INSIDE the signed header (cannot be flipped); this repo contains no decryption — only refusal | the real wall is the private lane's encryption-at-rest; single leak ⇒ treated-as-public (accepted, on record) |
| PUBLIC-RELEASE extraction | nothing — accepted by design | release lag is the moat, not obfuscation |
| malicious-but-signed genome | genome is DATA interpreted by the engine — no code-execution path in apply | authenticity ≠ competence; the G1 replay gate + eval pins own competence at the measurement layer |
| malformed-file parser attacks | hardening checklist below | fuzz corpus at T6 |

Binding implementation hardening (T2/T4/T5 carry these):

- [ ] Single-read discipline: read the file ONCE into memory; hash / verify /
      apply the SAME buffer — no verify-then-re-read TOCTOU window.
- [ ] Verify-before-parse: the signature over header+lengths is checked
      BEFORE deep payload parsing; every length capped (no unbounded
      allocation on unverified input).
- [ ] ed25519-dalek STRICT verification (canonical; reject malleable
      signatures).
- [ ] Monotonic apply (the T5 addition this review forced): refuse an
      artifact that is a lineage ANCESTOR of / older-version than the
      current genome unless explicitly forced; the force path logs.
- [ ] Atomic swap via write-temp + rename (crash mid-apply leaves the old
      genome serving; no partial states).
- [ ] cargo-fuzz corpus over the container parser before any public
      artifact ships.
- [ ] Unknown ANYTHING fails closed: class, key-id, version, lineage
      break.

## Non-goals (private homes, forever)

- Minting/improvement tooling (riir-train), hosted routes + settlement
  (riir-dapps), any economics.
