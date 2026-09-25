# Plan 002 — the vessel format crate (P3)

**Status:** DONE — 2026-09-25, landed `587ed9d` (code, 47/47 gates green
on aarch64 + x86_64) + the T7 docs close. Scope source: `.proposals/001`
§"What ships here" (the vessel format crate) + riir-train Research 457
§Sequencing P3 (the private design record). Minting tooling stays in
riir-train — this repo ships the READ/VERIFY/APPLY half plus the public
format.

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

- [x] T1 Crate skeleton: workspace member `crates/reflexer-vessel`
      (public, MIT; zero deps beyond blake3/serde/ed25519-dalek already in
      tree — no new dep classes). BOUNDARY `Owns` already claims it.
      (Landed with blake3 + ed25519-dalek ONLY — the header is manual LE
      bytes, serde turned out unnecessary: a smaller parse surface than
      planned.)
- [x] T2 Artifact format: whole-snapshot genome container + lineage
      header (parent commitment chain; the `Genome::to_line`/`from_line`
      wire is the payload — never a weighted blend). (Payload-agnostic
      crate: bytes in, verified bytes out; the from_line binding lives at
      `Engine::from_vessel_payload` — the only bytes→genome seam.)
- [x] T3 Two-class header: PUBLIC-RELEASE vs HOSTED-ONLY (class declared
      in the header; HOSTED-ONLY never written by any path in THIS repo —
      the class exists so the READ side can refuse it on uncontrolled
      hardware, fail-closed). (`encode_public` is the only public writer;
      the raw class-1 encoder is `pub(crate)`, reachable only by the
      crate's own hostile-forgery tests; the reader refuses hosted-only
      only AFTER authenticity so the message cannot be spoofed.)
- [x] T4 Key-id rotation (read side): multi-key pin table, key-id in the
      header, revocation path — a signature under a revoked or unknown
      key-id FAILS CLOSED (the vessel never opens). (Plus the
      `--vessel-pubkey` operator wildcard pin — the SEAL
      `SEAL_VESSEL_PUBKEY` precedent — because the compiled-in
      `DEFAULT_PINS` is deliberately EMPTY until the first artifact
      ships.)
- [x] T5 Apply semantics: ATOMIC VERSIONED SWAP at the engine boundary —
      the bin takes a vessel path, verifies, swaps the genome whole;
      fingerprints proven by execution on aarch64 AND x86_64 before any
      artifact crosses machines (consume P2's lane for the proof).
      (Constructive apply: the engine is built WHOLE from the verified
      payload before anything serves — no partially-swapped state exists
      to observe; 47/47 green on m3 + the 4090, same eight result lines.)
- [x] T6 G1 gate: vessel-apply replay — champion-from-vessel decisions
      byte-identical to champion-from-substrate at every P2 geometry
      (reuse `tests/g1_champion_replay.rs`'s battery shape); G3: substrate
      untouched. (`tests/vessel_gates.rs` — in-process + through the wire,
      hold + no-hold; G3: katgpt-rs consumed, never edited.)
- [x] T7 Docs: README §vessel (format, classes, rotation, apply law),
      AGENTS Current-state, HISTORY entry, BOUNDARY true-up, `.benchmarks/`
      gate record. (Bench 002; BOUNDARY gained the reflexer-vessel dep
      row.)

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

- [x] Single-read discipline: read the file ONCE into memory; hash / verify /
      apply the SAME buffer — no verify-then-re-read TOCTOU window.
      (`open()` = `File::open` + `take(cap+1)` + one `read_to_end` — no
      stat-then-read window at all; the cap bounds the allocation even if
      the file grows mid-flight.)
- [x] Verify-before-parse: the signature over header+lengths is checked
      BEFORE deep payload parsing; every length capped (no unbounded
      allocation on unverified input). (Structure + payload_len==file
      before crypto; MAX_PAYLOAD 1 MiB.)
- [x] ed25519-dalek STRICT verification (canonical; reject malleable
      signatures). (`verify_strict` at every verification site, bin
      included.)
- [x] Monotonic apply (the T5 addition this review forced): refuse an
      artifact that is a lineage ANCESTOR of / older-version than the
      current genome unless explicitly forced; the force path logs.
      (`check_monotonic`: `OlderThanCurrent` + `VersionFork`; the bin's
      `--vessel-force-downgrade` logs and serves — both arms tested.)
- [x] Atomic swap via write-temp + rename (crash mid-apply leaves the old
      genome serving; no partial states). (Satisfied by construction in
      v1: apply is boot-time CONSTRUCTIVE — the engine is built whole
      before serving, so no partial state exists to observe. The
      write-temp+rename form binds any future on-disk apply cache, which
      v1 deliberately does not have.)
- [-] cargo-fuzz corpus over the container parser before any public
      artifact ships. (DEFERRED — the trigger is the first public
      artifact ship, which has not happened; the always-on deterministic
      2000-mutation sweep + every-truncation arm gate the parser
      meanwhile.)
- [x] Unknown ANYTHING fails closed: class, key-id, version, lineage
      break. (Magic, format version, flag bits, key-id, class — every
      unknown byte refuses, tested per class.)

## Non-goals (private homes, forever)

- Minting/improvement tooling (riir-train), hosted routes + settlement
(riir-dapps), any economics.
