# Bench 002 — P3 vessel format gates (Plan 002 T1–T7)

**Status:** DONE — 2026-09-25, reflexer `587ed9d`.
**Scope:** the vessel format crate + bin apply: G1 artifact-apply replay
(champion-from-vessel ≡ champion-from-substrate, in-process AND through
the wire), the fail-closed refusal battery, the monotonic gate, and the
security-posture hardening checklist (Plan 002 §Security posture).

## Environments

| run | host | arch | result |
|---|---|---|---|
| `cargo test --workspace` | m3-max-metal (AC) | aarch64-apple-darwin | **47/47 green** · clippy `--workspace --all-targets` 0 findings |
| `cargo test --workspace` | 4090-win (CARGO_TARGET_DIR isolated, cleaned after) | x86_64-pc-windows-msvc | **47/47 green** — same 8 target result lines (9/0/5/9/8/16/0/0) |

T5's dual-arch law: fingerprints proven BY EXECUTION on both arches — the
G1 battery replays the champion from a signed vessel and through the
bin-booted-from-vessel wire at both P2 geometries on both hosts.

## G1 — artifact-apply replay (the bit-identity claim)

- `g1_vessel_champion_replays_substrate_exactly` — in-process: vessel
  genome id == compiled champion id; seeds 1..=4 (240 pieces, hold):
  decisions + stats identical to `Engine::champion()` AND to
  `play_game(champion_genome, …)`.
- `g1_bin_from_vessel_matches_oracle_hold_and_no_hold` — the bin booted
  with `--vessel <file> --vessel-pubkey <hex>`: seeds 1..=4 hold + 5..=6
  no-hold, wire decisions/stats ≡ oracle. The vessel path adds ZERO
  semantic drift through: file → header parse → strict ed25519 → payload
  → `Genome::from_line` → engine → wire.
- Vessels minted in-test with the deterministic key `[7u8;32]` (key-id 1,
  artifact v1) over `Genome::champion_hybrid().to_line()` — the payload
  wire IS the substrate's own genome line, never a re-encoding.

## Fail-closed battery (16 crate unit + bin boot arms)

| arm | refusal |
|---|---|
| bad magic / unknown format version / unknown flag bits | structural, before any crypto |
| every truncation length 0..=131 | `Truncated` |
| payload_len mismatch (grow / shrink / field tamper) | `PayloadLenMismatch` |
| payload > 1 MiB cap | `PayloadTooLarge` — before allocation-heavy work |
| tamper: magic region → BadMagic; sig + payload regions → BadSignature; header fields → structural-or-signature | region-classed, all closed |
| 2000-mutation seeded sweep (fuzz-lite) | zero panics, zero opens |
| unknown key-id / revoked key-id (empty DEFAULT_PINS posture included) | `UnknownKey` / `RevokedKey` |
| HOSTED-ONLY (forged via `pub(crate)` arm in-crate; hand-forged bytes in the integration test — no public writer exists) | `HostedOnly`, refused only AFTER authenticity so the message cannot be spoofed by a forged file |
| bin boots: tampered / no pin / wrong pin / hosted-only / lineage fork (v0 ≠ substrate commitment) | exit != 0, loud stderr naming the refusal |
| forced downgrade | boots, the force is LOGGED, serves correctly |

## Security-posture checklist (Plan 002, binding items)

- [x] Single-read discipline — `open()` is `File::open` + `take(cap+1)` +
      one `read_to_end`; hash/verify/apply see the same buffer; no
      stat-then-read window at all.
- [x] Verify-before-parse + size caps — structure and `payload_len == file`
      checked before crypto; `MAX_PAYLOAD` (1 MiB) bounds every
      allocation on unverified input.
- [x] ed25519 STRICT — `VerifyingKey::verify_strict` everywhere
      (canonical sigs, small-order keys rejected).
- [x] Monotonic apply — `check_monotonic` refuses `OlderThanCurrent` and
      `VersionFork`; the bin's `--vessel-force-downgrade` is the operator
      path and logs (tested).
- [x] Atomic swap — constructive apply: `Engine::from_vessel_payload`
      builds the WHOLE engine before anything serves; no partially-swapped
      state exists to observe (write-temp+rename binds a future on-disk
      apply cache, which v1 does not have).
- [-] cargo-fuzz corpus — DEFERRED to the first public artifact ship (the
      plan's own trigger); the always-on deterministic 2000-mutation
      sweep + every-truncation arm gate the parser meanwhile.
- [x] Unknown-anything fails closed — magic, version, flags, key-id,
      class, lineage: every unknown byte refuses.

## Honest caveats

1. `DEFAULT_PINS` is EMPTY — with no public artifact minted yet, every
   real-world vessel fails `UnknownKey` until the owner pins the first
   minting key (deliberate; the `--vessel-pubkey` operator pin is the
   interim path, the SEAL precedent).
2. The lineage chain is one parent deep per artifact (the header carries
   the parent commitment; ancestor-history traversal is a reader-side
   walk over files, not shipped state).
3. `payload_len` is u64 on the wire but capped at 2^20 by the reader;
   writers assert the same cap — a larger field is a refusal, never an
   allocation.
