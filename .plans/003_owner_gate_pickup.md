# Owner-gate pickup plan — riir-reflexer

**Status:** CLOSED 2026-10-05 — the owner-gate backlog is dispositioned per the delegated verdict (session f9134c0d); the sibling pickup issue is removed (number stays spent); final dispositions live in this repo's HISTORY.md §2026-10-05 and the workspace record in riir-ai HISTORY.md §2026-10-05.

Master: `../riir-ai/.issues/1016_workspace_owner_gated_decisions_summary.md` (riir-ai commit e8cbd91ff; tracker removed 2026-10-05 — surviving record: riir-ai HISTORY.md §2026-10-05). Sibling issue: ~~`.issues/001_owner_gate_pickup.md`~~ (removed 2026-10-05 per the noise-reduction rule; records: HISTORY.md §2026-10-05 + git history; E18).

## Agent-pickupable tasks

- [x] E18 prep: sanity-check the issue's 3-line first-mint runbook against `reflexer sign --help` / `reflexer --help` (flag names drift; do NOT run a mint). **DONE — the VERIFIED runbook (incl. the two 2026-10-01 corrections: payload is `--in` not positional; `--vessel-print` early-returns) lives at `.plans/005_default_pins_first_mint.md` (HISTORY.md §2026-10-05).**
- [ ] E18: at the first public vessel mint, record the verifying-key hex + commitment in the issue and file the owner call for the `DEFAULT_PINS` row. **Superseded by `.plans/005_default_pins_first_mint.md`'s single live `- [ ]` row (2026-10-05) — that plan owns the trigger now.**

## Deferred (gated)

- [-] E18 execution: land the `DEFAULT_PINS` row + the consumers' operator pin — owner-gated on the first public vessel mint existing (minting is deterministic; re-mint moves no pin). **Trigger lives in `.plans/005_default_pins_first_mint.md` (2026-10-05).**
