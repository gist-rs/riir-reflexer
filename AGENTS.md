# AGENTS.md — riir-reflexer (public)

The global `~/.agents/` rules apply where compatible with a PUBLIC repo;
this file documents repo-local context.

## Role

The public decision ENGINE. Design of record: this repo's
`.proposals/001_reflexer_engine.md` (public-facing); the full private
design record lives in riir-train. One line: public substrate plays public
genomes; this repo is the engine, the format, and the submission client —
nothing else.

## The laws

1. **Modelless-first mandate, verbatim.** No training, no backprop, no
   gradient descent. The only weight mutations allowed are artifact
   hot-swap (atomic versioned swap — NEVER a weighted blend; blends do not
   preserve move rankings) and latent-space updates. Improvement machinery
   does not grow here.
2. **The leaf law.** Zero riir-* dependencies (katgpt-core + the katgpt-rs
   substrate module only). No tokenomics code, no signing keys.
3. **The forward freeze.** The katgpt-rs example/module surface stops at
   the Bench-892 champion `68cae9d382014662` as reference; evolved VALUES
   land in the private improvement home — never accreted into public
   examples.
4. **Artifact classes.** PUBLIC-RELEASE runs anywhere (extractable,
   accepted). HOSTED-ONLY never reaches uncontrolled hardware; encrypted
   at rest on an encrypted-storage lane; served as decisions only.

## Measurement laws

Zero network calls in measurement paths. Response envelopes carry
in-engine decision time; harnesses render it beside the round-trip. Every
comparison row pins binary BLAKE3 + artifact digest. Artifact fingerprints
are proven on aarch64 AND x86_64 by execution before crossing machines.

## Current state

**BORN PUBLIC (P1 complete).** MIT licence (the katgpt-rs LICENSE form);
`Visibility: public` in BOUNDARY.md is machine-read by the workspace
boundary guard — a path dep on any sibling other than katgpt-rs fails the
contract check mechanically (check C3b). Registered in the workspace
contract set (repo_set + the riir-ai canonical matrix). No Cargo.toml yet —
code starts at P2. katgpt-rs Issue 893 (P0, the substrate module) is filed;
its landing gates P2's engine bin.

## Numbering Discipline

Monotonic, never reused: read the target dir's `.highwater`, use value+1,
write back. `.proposals/.highwater` = 001.

## Branch

`develop` at birth, per the global rule.
