# Owner-gate pickup — riir-reflexer

**Status:** OPEN — pickup tasks from the workspace owner-gated summary (riir-ai 1016); execution gates per item; prep/recording tasks are agent-pickupable.

Master: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai commit e8cbd91ff). Owner direction 2026-09-28: repos riir-mmorpg-examples / seal-online-remaster / seal-game-editor / sealm-toolkit are DEFERRED; ALL mainnet actions are ON HOLD. Non-deferred repos carry pickup issues + plans; this is this repo's.

## E18 — DEFAULT_PINS empty until the first public vessel mint

Sources: `.benchmarks/002_vessel_format_gates.md:110` (Honest caveat 1: `DEFAULT_PINS` is EMPTY — with no public artifact minted yet, every real-world vessel fails `UnknownKey` until the owner pins the first minting key; deliberate; the `--vessel-pubkey` operator pin is the interim path, the SEAL precedent) + AGENTS.md P3.1 (`writer::sign_public` re-verifies its own output before writing and prints the commitment + the VERIFYING key hex; minting is deterministic — same inputs → byte-identical vessels; re-minting moves no pin).

OWNER-GATED trigger: pin the verifying key at the FIRST public mint (the key exists at mint time; the pin is a compile-time `DEFAULT_PINS` row + the consumers' operator pin).

- [-] Land the `DEFAULT_PINS` row + consumer operator pin — owner-gated on the first public vessel mint existing.
- [ ] Agent prep: first-mint runbook (drafted below; verify flag names against `--help` at pickup — do NOT run a mint):
  1. Mint: `reflexer sign --key-file <64-hex-seed-file> <payload>` → prints the BLAKE3 commitment + the VERIFYING key hex (deterministic; re-minting moves no pin).
  2. Verify: `reflexer --vessel <out.vessel> --vessel-pubkey <verifying-key-hex> --vessel-print` → strict ed25519 + BLAKE3 verify + print (the interim operator-pin path).
  3. Pin: file the owner call with the verifying-key hex → land the `DEFAULT_PINS` row (compile-time pin table) + set the consumers' operator pin (e.g. `RIIR_REFLEX_HEADS_PUBKEY` on the reflex side); re-run the vessel gates.
