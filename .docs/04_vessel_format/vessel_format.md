# Vessel format — one signed file, applied whole at boot

A vessel is how a DIFFERENT genome reaches the bin without a rebuild: one
signed file (`crates/reflexer-vessel`, public, MIT; deps blake3 +
ed25519-dalek only). Boot order is fixed — verify → refuse hosted-only →
monotonic gate → construct the engine WHOLE → serve — and every vessel
failure is a LOUD boot failure (exit 1); there is no silent fallback to
the compiled champion (`src/bin/reflexer.rs::load_vessel_engine`). The
crate is payload-agnostic: bytes in, verified bytes out — the bytes→genome
binding lives in the engine, `Engine::from_vessel_payload`
(`Genome::from_line`) being the only seam (`src/engine.rs`).

## Format v1 anatomy

Manual LE parse, zero serde (`crates/reflexer-vessel/src/lib.rs`):

| off | size | field | notes |
|---|---|---|---|
| 0 | 8 | magic | `b"RFLEXVSL"` |
| 8 | 4 | format_version | u32 LE (this crate: 1) |
| 12 | 4 | flags | u32 LE — bit0 class (0 PUBLIC, 1 HOSTED-ONLY); all other bits reserved-0, nonzero fails closed |
| 16 | 4 | key_id | u32 LE — pin-table identity of the minting key |
| 20 | 8 | artifact_version | u64 LE — monotonic per lineage |
| 28 | 32 | parent_commitment | blake3 of the parent's signed region (zeros = genesis) |
| 60 | 8 | payload_len | u64 LE |
| 68 | 64 | signature | ed25519-STRICT over `bytes[0..68] ‖ payload` |
| 132 | n | payload | `payload_len` bytes, capped (below) |

`commitment = blake3(header ‖ payload)` — the lineage chain's link and the
signature cover the SAME bytes (`commitment_of`).

**The payload law:** the payload is `Genome::to_line` — the substrate's own
whole-snapshot genome line. Applying a vessel swaps the genome WHOLE;
weighted blends do not preserve move rankings, so they are not a legal
payload shape. Lineage discipline (strictly-increasing `artifact_version`)
is the minter's job, enforced at the reader by the monotonic gate.

## The two-class law

- **PUBLIC-RELEASE** — runs anywhere; extractable, accepted (release lag is
  the moat, not obfuscation).
- **HOSTED-ONLY** — the class bit exists so the reader refuses it on
  uncontrolled hardware. It is an accident guard + audit signal, NOT
  encryption: no ciphertext and no decryption exist at this layer; the real
  wall is distribution plus the private lane's encryption-at-rest.

The class bit lives INSIDE the signed header — it cannot be flipped without
breaking the signature. This repo has no writer path for class 1
(`encode_raw` is `pub(crate)`, reachable only by the crate's own
hostile-forgery tests) and no decryption. Refusal order is load-bearing:
`decode` refuses hosted-only only AFTER authenticity (structure/caps → pin
resolution → strict signature → class refusal), so a forged file cannot
spoof the refusal message; and `open` reads the 132-byte prefix FIRST,
refusing a file DECLARING the hosted class before any payload byte
(`VesselError::HostedOnlyPath` — a structural fact, pre-auth by
construction).

## Class-aware caps (2026-09-27, `049a583`)

The cap bounds hostile inputs, not artifact sizing — and the two classes
read on two different trust surfaces, so the bound is per-class:

| const | value | applies to |
|---|---|---|
| `MAX_PAYLOAD` | 1 MiB (`1 << 20`) | PUBLIC-RELEASE, on every untrusted read path (reflexer-wasm never loosens) |
| `MAX_HOSTED_PAYLOAD` | 16 MiB (`1 << 24`) | HOSTED-ONLY, at `peek()` only |

`peek()` reads the class from the header it just parsed (before any payload
work), so a HOSTED-ONLY vessel may carry up to 16 MiB — specialist vessels
carry i8 weights (banking77 is ~9.6 MiB; 16 MiB leaves ~1.6× headroom) —
while a size legal for hosted is still refused the moment the header says
public. Because `open()` refuses declared-hosted at the prefix, its
allocation bound stays the 1 MiB public cap.

## Keys rotate; unknown fails closed

- `key_id` in the header; `PinTable` holds pinned verifying keys, a revoked
  set (revocation is forever — fail closed), and an optional wildcard.
- The compiled-in pin table `DEFAULT_PIN_KEYS` is EMPTY until the first
  public artifact ships — every unverified-key vessel fails `UnknownKey`,
  the correct posture. Pin the first minting key in the same change that
  ships the first artifact. `MIN_ARTIFACT_VERSION` (0 today) is the
  compiled release floor, set alongside the first pin: a validly-signed
  vessel OLDER than the floor refuses to boot.
- `--vessel-pubkey` is the operator wildcard pin (the SEAL
  `SEAL_VESSEL_PUBKEY` precedent). Compiled pins resolve FIRST and the
  wildcard only ADDS trust — a pinned key verifies with no flag (the
  no-flag path must work the day the first artifact ships).

## Apply law — monotonic, both gates

`check_monotonic(current: &ApplyState)` refuses `OlderThanCurrent` (the
replay/downgrade arm; exact re-apply is idempotent) and `VersionFork` (same
version, different commitment — a lineage fork, not a successor).
`check_floor(MIN_ARTIFACT_VERSION)` is the release-time anti-downgrade
baseline (no prior state needed); the bin runs BOTH gates and names WHICH
one fired. `SUBSTRATE` (version 0, zero commitment) is the compiled-in
genesis baseline. `--vessel-force-downgrade` is the operator override: it
LOGS the bypass and serves — tested both arms.

## Hardening (the security posture, `.plans/002` §Security posture)

- **Single-read discipline** — `open()` pre-checks regular-file (a FIFO or
  device on the vessel path refuses, never hangs boot), then reads ONCE via
  `take(cap+1)`: bounded allocation even if the file grows mid-flight; no
  stat-then-read window; hash/verify/apply see the same buffer.
- **Verify-before-parse, strict signatures** — structure and
  `payload_len == file` before any signature work, the signature before the
  payload is handed out; `verify_strict` everywhere (malleable /
  small-order keys rejected).
- **Constructive apply; unknown anything fails closed** — the engine is
  built whole before anything serves (no partially-swapped state to
  observe); magic, format version, flag bits, key-id, class, lineage —
  every unknown byte refuses.
- **Always-on fuzz-lite** — a deterministic 2000-mutation seeded sweep
  (zero panics, zero opens) + the every-truncation-length arm (0..=131)
  gate the parser; a cargo-fuzz corpus rides before the first public
  artifact ships.

## The public writer (P3.1, instinct Proposal 001 T4)

`writer::sign_public` is the FIRST writer in this repo and the only public
one — the public format repo owns the public-class writer so anyone can
mint vessels for THEIR OWN genomes on the same carrier. Over
`encode_public` it adds a Result-based mint (the 1 MiB cap enforced AT
WRITE TIME as `PayloadTooLarge`), the commitment out (`MintedVessel {
bytes, commitment }`), and fail-closed key handling. Minting is
deterministic (same inputs → byte-identical vessels; re-minting moves no
pin).

```sh
reflexer sign --in <payload> --out <vessel> --key-id <u32> \
  [--artifact-version <u64>=1] [--parent <blake3-hex>] \
  [--key <64-hex-seed> | --key-file <path>]   # env REFLEXER_SIGN_KEY
```

The key resolves from `--key` (64-hex seed), then `--key-file` (64-hex
text or 32 raw bytes), then `REFLEXER_SIGN_KEY` — fail-closed (exit 2)
without one, naming all three sources. An over-cap `--in` is refused from
file metadata, never loaded. The mint is re-verified against a wildcard
pin of its own key BEFORE anything is written, and prints the commitment
(the pin consumers record) AND the verifying key hex (the consumer-side
trust anchor, e.g. `RIIR_REFLEX_HEADS_PUBKEY`).

## Reader capability features (A1: bytes are runtime; capability is compile-time)

`vessel_public_read` / `vessel_hosted_read` — both DEFAULT-ON
(`crates/reflexer-vessel/Cargo.toml`); the default build is
behavior-identical for every existing consumer. A class whose reader is
not compiled refuses `VesselError::ClassNotReadable` — at `peek`
(structural) and post-signature at `decode`. The class REFUSALS are not
capabilities: `open`'s `HostedOnlyPath` prefix check and `decode`'s
authenticated `HostedOnly` refusal stay UNCONDITIONAL. reflex (public)
selects `vessel_public_read` only — the hosted reader never compiles into
the public consumer.

## Bin flags

| flag | effect |
|---|---|
| `--vessel <path>` | boot the engine from the signed artifact instead of the compiled substrate |
| `--vessel-pubkey[=<hex>]` | operator wildcard pin; requires `--vessel` (exit 2 alone) |
| `--vessel-force-downgrade` | operator override of the downgrade gates — LOGS the bypass, serves |
| `--vessel-print <path>` | inspect without applying: header/lineage/commitment always (via `peek`), signature verdict when a pin resolves (`verified` / `BAD` / `unverified: no pin for key-id N` / `REVOKED`) |

On a successful apply the bin logs one line: commitment digest, artifact
version, key-id, class, and the derived genome id.

## G1 and the gate record

G1 law: champion-from-vessel ≡ champion-from-substrate — decisions and
stats identical in-process AND through the wire (bin booted with
`--vessel`), at both P2 geometries. Proven by execution on aarch64 AND
x86_64 — [Bench 002](../../.benchmarks/002_vessel_format_gates.md): 47/47
at the landing `587ed9d`, 52/52 after the verdict-round fixes, same result
lines on both arches. The fail-closed battery (tamper by region, every
truncation, unknown/revoked keys, hand-forged hosted-only, fork/downgrade
boot refusals) lives in `tests/vessel_gates.rs` and the crate's unit
tests. Minting tooling for real artifacts lives in the private improvement
home (riir-train); `encode_public` / `writer::sign_public` exist so the
public can mint vessels for their own genomes on the same carrier.

## Honest caveats

1. `DEFAULT_PIN_KEYS` is EMPTY — every real-world vessel fails `UnknownKey`
   until the first minting key is pinned; `--vessel-pubkey` is the interim
   operator path.
2. The lineage chain is one parent deep per artifact (ancestor traversal is
   a reader-side walk over files, not shipped state).
3. `payload_len` is u64 on the wire but capped by the reader; writers assert
   the same cap — a larger field is a refusal, never an allocation.
4. Freeze-attack / no-freshness is accepted out of scope under the
   release-lag model (Bench 002 addendum).
