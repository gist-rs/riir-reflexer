# Owner-gate pickup — riir-reflexer

**Status:** OPEN — pickup tasks from the workspace owner-gated summary (riir-ai 1016); execution gates per item; prep/recording tasks are agent-pickupable.

Master: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai commit e8cbd91ff). Owner direction 2026-09-28: repos riir-mmorpg-examples / seal-online-remaster / seal-game-editor / sealm-toolkit are DEFERRED; ALL mainnet actions are ON HOLD. Non-deferred repos carry pickup issues + plans; this is this repo's.

## E18 — DEFAULT_PINS empty until the first public vessel mint

Sources: `.benchmarks/002_vessel_format_gates.md:110` (Honest caveat 1: `DEFAULT_PINS` is EMPTY — with no public artifact minted yet, every real-world vessel fails `UnknownKey` until the owner pins the first minting key; deliberate; the `--vessel-pubkey` operator pin is the interim path, the SEAL precedent) + AGENTS.md P3.1 (`writer::sign_public` re-verifies its own output before writing and prints the commitment + the VERIFYING key hex; minting is deterministic — same inputs → byte-identical vessels; re-minting moves no pin).

OWNER-GATED trigger: pin the verifying key at the FIRST public mint (the key exists at mint time; the pin is a compile-time `DEFAULT_PINS` row + the consumers' operator pin).

- [-] Land the `DEFAULT_PINS` row + consumer operator pin — owner-gated on the first public vessel mint existing.
- [x] Agent prep: first-mint runbook — **verified against `src/bin/reflexer.rs` at `60408d2` (2026-10-01); the draft below was corrected in two places** (the drafted mint line would have exit-2'd — payload is `--in`, not positional, and `--out`/`--key-id` are required; the drafted verify line never booted — `--vessel-print` takes the path itself and early-returns, so it does not combine with `--vessel`):
  1. **Mint**: `reflexer sign --in <payload-file> --out <vessel-path> --key-id <u32> [--artifact-version <u64>=1] [--parent <blake3-hex>] (--key <64-hex-seed> | --key-file <path> | env REFLEXER_SIGN_KEY)` — `--key-file` accepts 64-hex text or 32 raw bytes. Fail-closed exit 2 on a missing key/`--in`/`--out`/`--key-id` or an over-cap payload (1 MiB public cap, enforced from metadata before load); exit 1 if the mint-side re-verification of the fresh bytes fails. Success prints one JSON line: `commitment` (the blake3 pin consumers record), `verifying_key` (the trust anchor to pin), `key_id`, `artifact_version`, `payload_len`, `vessel_len`. Deterministic — same inputs → byte-identical vessel; re-minting moves no pin.
  2. **Verify** (inspect without applying): `reflexer --vessel-print <vessel-path> --vessel-pubkey <verifying-key-hex>` — peeks the header, resolves the pin (compiled `default_pins()` first, the flag's wildcard only ADDS), strict-ed25519-verifies, prints JSON incl. `"signature": "verified" | "BAD" | "unverified: no pin for key-id N"` + the commitment. (Full boot-verify instead: `reflexer --vessel <path> --vessel-pubkey <hex>` — verifies, applies the monotonic gate, serves stdio.)
  3. **Pin**: file the owner call with the `verifying_key` hex from step 1 → land the `DEFAULT_PINS` row (compile-time pin table, keyed by the `key_id` used at mint) + set the consumers' operator pin (e.g. `RIIR_REFLEX_HEADS_PUBKEY` on the reflex side); re-run the vessel gates (`cargo test` in `crates/reflexer-vessel`).
