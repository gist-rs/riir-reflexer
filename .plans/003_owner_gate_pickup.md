# Owner-gate pickup plan — riir-reflexer

**Status:** OPEN — prep/recording tasks for the owner-gated pickup (issue 001); execution is owner-gated.

Master: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai commit e8cbd91ff). Sibling issue: `.issues/001_owner_gate_pickup.md` (E18).

## Agent-pickupable tasks

- [ ] E18 prep: sanity-check the issue's 3-line first-mint runbook against `reflexer sign --help` / `reflexer --help` (flag names drift; do NOT run a mint).
- [ ] E18: at the first public vessel mint, record the verifying-key hex + commitment in the issue and file the owner call for the `DEFAULT_PINS` row.

## Deferred (gated)

- [-] E18 execution: land the `DEFAULT_PINS` row + the consumers' operator pin — owner-gated on the first public vessel mint existing (minting is deterministic; re-mint moves no pin).
