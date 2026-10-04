# vessel-mint genesis record

The two PUBLIC verifying keys of the autopilot's two-tier key scheme
(agreed design: offline owner ROOT + online public-class-only MINT key).
Verifying keys are public by construction — the SEEDS exist only as
`~/cold/vessel-root.key` (offline, owner custody) and the
`VESSEL_MINT_SEED` Worker secret (write-only, CF). Never commit seeds.

| tier | key_id | verifying key (hex) | created | custody |
|---|---|---|---|---|
| ROOT (offline) | — | `54ca330be69da55420d19885cc5ef5b1bc589645f77efdd5d91d554b661739fa` | 2026-10-04 | `~/cold/vessel-root.key` → move to cold storage; signs delegations/revocations/floor-override only |
| MINT (online, public-class-only) | 1 | `bc94fae4b1052a3a65cd698f697bb6f1d7db073dde2f3a456a9d633e884b9155` | 2026-10-04 | CF Worker secret `VESSEL_MINT_SEED` on `vessel-mint`; rotation = new key + key_id + root-signed revocation |

Worker: `vessel-mint` @ https://vessel-mint.foxfox.workers.dev —
placeholder deployment (all routes 503, no mint path) holding the genesis
secret until the autopilot lands at the same name.

Loss/recovery: CF secrets are write-only — a lost mint seed is recovered
by ROOT re-delegation (mint a new key, revoke key_id 1 via the signed
feed), never by readback. A lost ROOT seed means the two-tier scheme
degrades to single-tier until a new root is pinned by release; guard the
cold copy accordingly.
