# Issue 005 — Manifest.proto: format field written from a const, read by nothing

**Status:** OPEN — filed 2026-10-17 (riir-refine idle loop, orphan-report sweep intake; adjudicated per the tool's downstream-grep law)
**Claimed:** zed-m3-idle-1017c, 2026-10-17 12:05+07, expires 2026-10-19T12:00+07 — owner-gated adjudication (keep-vs-remove on Manifest.proto) via the Claude verdict lane; fix + close-out inside the claim window

`Manifest.proto` (`src/record.rs:46`, populated at `:140` from `crate::proto::PROTO`) is written into every manifest and read by **zero** call sites across the five repos that consume this crate (`riir-reflex`, `riir-instinct`, `riir-rethink`, `riir-refine`, this repo — grep `\.proto\b` over all five `src/` + `tests/`, 2026-10-17: no hits outside the decl/constructor).

Found by `cargo refine --orphan-report` (stillborn row, 0 read-shaped occurrences at HEAD and every history probe).

## The question (owner call, not a drive-by edit)

This is the vessel FORMAT repo — the manifest schema is the A1/A5 wire surface, and removing a field is a format change, not a cleanup. Two honest resolutions:

1. **If `proto` is forward-compat surface** (a reader is supposed to branch on it someday, or hosted readers outside this workspace check it): keep it, and add the deliberate-write-only doc marker so the next orphan sweep renders it `documented-write-only` instead of re-filing it.
2. **If it is vestigial**: remove the field (writers stop emitting it; serde default-tolerant readers keep parsing old manifests). Any byte-identity golden on the manifest bytes needs a pin bump in the same commit.

Do NOT delete the field from a reader side while writers still emit it, and do not widen the manifest schema in the same change.

## Evidence

- decl: `src/record.rs:46` | constructor: `src/record.rs:140` (`proto: crate::proto::PROTO`)
- consumer greps 2026-10-17: reflex/instinct/rethink/refine/self — zero reads
- sweep: `cargo refine --orphan-report ../riir-reflexer` -> 1 stillborn row (this field)
