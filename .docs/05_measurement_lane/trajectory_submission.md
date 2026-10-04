# Trajectory submission — free, opt-in, OFF by default

`--record <path.jsonl>` (`src/record.rs`): every ANSWERED request appends one
replayable row, and at clean EOF the bin writes a signed manifest beside the
rows. The default posture records nothing; error envelopes are never rows.

## Rows

One JSON line per answered request — `seq`, the exact request (the parsed
`DecisionRequest`), and the engine's outcomes (`question_id` + `Outcome` per
answer). Replay-verification is a pure function of the row: hand the request
to the same engine and the outcomes must reproduce.

The hash covers the ROW bytes (not the raw stdin lines), accumulated as the
rows are written — the manifest's `rows_blake3` is a commitment to the exact
file contents, so any later edit to the rows file breaks verification.

## The manifest (`<path>.manifest.json`)

| field | what |
|---|---|
| `proto` | the wire protocol version |
| `rows` | row count |
| `rows_blake3` | BLAKE3 of the exact rows-file bytes |
| `genome` | the serving genome digest |
| `verifying_key` | the Ed25519 verifying key, hex — the signer's identity (the public half) |
| `signature` | Ed25519 over the domain-separated message below, hex |

The signed message is exactly
`reflexer-trajectory-v1\n{rows}\n{rows_blake3}\n{genome}\n` —
version-pinned domain tag, no ambient bytes. For a `--record rows.jsonl`
run the manifest lands at `rows.manifest.json` and is announced on stderr
at EOF; a manifest write failure exits 1 (never a silent missing manifest).

## The per-machine key

The 32-byte seed lives at `$REFLEXER_SUBMISSION_KEY` or
`~/.config/reflexer/submission.ed25519` — created 0600 on first use
(OsRng). The public half rides the manifest so anyone can verify; the
private half never leaves the machine.

`record::verify_manifest(manifest, rows_bytes)` is the public verification
half anyone can run — BLAKE3 the rows, resolve the key, check the
signature. The round-trip and the one-flipped-byte tamper arm are unit-
pinned (`record.rs` tests).

## No billing code, no tokenomics — ever

This client signs rows; it prices nothing. The private improvement loop
replay-verifies rows against its own engine before crediting anything — the
anti-poisoning gate lives there, not here.
