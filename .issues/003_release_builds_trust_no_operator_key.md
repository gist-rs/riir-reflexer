# Issue 003 — release builds trust whatever key the operator passes: no compiled-in authority pin, wildcard pin, no persisted floor

**Status:** OPEN — filed 2026-10-04 from riir-rethink Issue 022 / Plan 009 P1.

- `crates/reflexer-vessel/src/lib.rs:374` `DEFAULT_PIN_KEYS = []`, `:368` `MIN_ARTIFACT_VERSION = 0`.
- `src/bin/reflexer.rs:133-139` `--vessel-pubkey` installs a WILDCARD pin (any key-id).
- `reflexer.rs:144-153` monotonic check compares against the v0 baseline only — no apply-state across runs.

Effect: a fork mints its own key, passes it, and a stock release build loads its vessels as authentic.

- [ ] T1 Compiled-in authority pins (per key-id) for release builds; `--vessel-pubkey` wildcard only under a `dev_pins` feature (default off).
- [ ] T2 Persist the applied `(key_id, artifact_version, commitment)` and refuse rollback across restarts.
- [ ] T3 Gate: fork-signed vessel refused by a release build; unknown key-id refused; revoked key refused.
- Note: this repo is PUBLIC; the pins are PUBLIC keys (safe to commit). Never commit a seed.
