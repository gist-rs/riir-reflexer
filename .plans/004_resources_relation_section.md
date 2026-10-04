# Plan 004 — the Reflex ↔ Reflexer relation section on reflex.gist.rs/resources

**Status:** PLANNED — verdict AGREE (round 2, 2026-10-04; two execution notes folded in below)

## Goal

Repeat the treatment of the `#reflex` section (reflex.gist.rs/resources/) for a
new `#reflexer` section that explains the **relation between Reflex and
Reflexer**. The source of truth for the new content lives in THIS repo
(`.docs/`); the section renders on reflex-site; every asset rides the
two-mirror law (doc dir ↔ site `assets/`+`docs/`). This is the exact decision
`.docs/03_decision_flow/decision_flow.md` anticipated: *"Adding this doc to
the renderer's SOURCES — and thereby to the site's drift `--check` — is a
reflex-site decision"* — this plan is that decision.

## The relation (what the section says — sourced from this repo)

- **Reflex answers questions; Reflexer plays a game.** Both are public, MIT,
  built on KatGPT-RS, and both speak a typed decision wire with abstention as
  a first-class answer. Reflex's state is text you wrote and its answers come
  from a corpus you author. Reflexer's state is a game board (Tetris-class)
  and ONE frozen-genome search per request answers `place` / `state` /
  `survive` — bit-identical on every host (bin, in-tab wasm, the Worker).
- **You have already seen it — link, don't redraw.** The arena's
  "Reflex · rulebook" boards (in-tab wasm + Cloudflare) ARE the reflexer
  engine, and the arena already carries the engine's own figures
  (`/assets/tetris_flow_rulebook.svg`, `tetris_flow_modes.svg`, with their
  recorded step-through walk). The section LINKS the arena's rulebook figure
  and boards; it does not draw a second picture of the same search
  (the `#reflex` "one asset, two readers" law cuts the other way here: the
  asset already has its reader).
- **Reflexer gave the family the vessel format.** One signed file, verified
  and applied WHOLE at boot — never a weighted blend; the public crate
  `reflexer-vessel` is the format the family's artifacts — public and hosted
  alike — ship in (`04_vessel_format/vessel_format.md`; a class fact on the
  page, no lane or head named).
- **Both measure honestly.** The envelope carries in-engine decision time
  (serde + stdio excluded); the measurement lane refuses an invalid box state
  (`05_measurement_lane/`). On the page this stays a CLASS, never a figure
  (numbers law — link `/bench/`, no typed numbers).

## Section design (mirror the `#reflex` pattern)

- **Placement:** AFTER `#rethink` — the page reads as an upgrade ladder
  (Reflex → Instinct inside `#reflex` → Rethink "one rung deeper"), and a
  lateral sibling engine must not sit inside it. Section order becomes
  learn → overview → reflex → rethink → **reflexer** → development; the hero
  CTA order matches (free floor → encoder tier → rulebook engine → build
  your own).
- **Shape (the `#reflex` treatment):** `h2 > a.hlink` self-linked title ·
  `p.sub` with bracketed jargon glosses · ONE figure · `.teaser` two-cell
  contrast · short prose · `.faqs` (details) incl. the deep write-up links.
- **Figure:** a NEW gfflow TOML block authored inside the new
  `.docs/06_resources/resources.md` (the dev_flow.md precedent — one doc per
  lane carries its block), rendered OFFLINE by
  `reflex-site/scripts/render_flows.py` to `relation_flow.svg` +
  `relation_flow_m.svg` (390 px card list), rename-mapped site-side to
  `reflexer_relation_flow*.svg`. Content: TWO states in — "a text question"
  (Reflex: corpus → answer or abstain) and "a game turn" (Reflexer: one
  frozen-genome search → place/state/survive) — branching 2a/2b and merging
  into the SAME answer-envelope shape. The shared thing is the QUESTION TYPES
  and the ENVELOPE, not one request: the entry boxes say "same question types,
  same envelope shape, different state" (per `02_wire_protocol/
  measurement_contract.md` — Reflexer's state is a strict board shape, never
  free text). The branch shape IS the relation. All lanes wear the
  Reflex accent (`family:#ff8a3d`); no status chips needed, no numbers.
  Embedded per §8 with `<picture>` + `<source media="(max-width: 720px)">`.
- **Teaser cells:** "REFLEX · QUESTIONS ABOUT ANYTHING" (state is text you
  write; answers from your corpus; abstains off-corpus) vs "REFLEXER · THE
  WIRE AS A GAME" (state is a board; one frozen-genome search answers
  place/state/survive; same answer on every host).
- **Framing sentence (verbatim smoke pin, draft — final wording at T3):**
  "Reflexer is the family's rulebook engine — it speaks Reflex's question
  types and answer envelope over a different state, the game board: one
  frozen-genome search per turn answers where to place, how the position
  stands, and whether it survives." Glosses in brackets:
  [Rulebook: a fixed list of rules that scores every board a move can lead
  to. Frozen genome: shipped as constant bytes — never retrained, never
  blended.] (verdict round-2 wording note: the search scores RESULTING
  BOARDS, not moves — "scores every board a move can lead to", never
  "scores every possible move".)
- **FAQs (draft):** Why do the names sound alike? — must answer the arena
  naming head-on: the boards are labelled "Reflex · rulebook" because the
  lane is family-branded; the ENGINE behind them is Reflexer, and the Reflex
  question engine does not need it. · Do I need Reflexer to run Reflex? (no —
  separate engines) · What did Reflexer give the whole family? (the vessel
  format — stated as a class fact: the family's artifacts, public and hosted,
  ship in one signed format; name no hosted lanes or heads, per the moat law)
  · Deep write-up (mirror + GitHub links).
- **Glossary:** add `rulebook` and `frozen genome` rows to the `#learn`
  "More words" `dl.gf-gloss` box (every term needs a real definition — the
  smoke counts dd vs dt).
- **Page copy touch-ups (honesty, minimal):** hero CTA gains "The rulebook
  engine" (`#reflexer`, ordered with the sections); `<title>`/meta description
  mention Reflexer; `#overview` sub-line and one sentence name the second
  engine linking `#reflexer`. The three existing verbatim framing pins stay
  byte-untouched — only appended to.

## Tasks

- [ ] T0 — precondition: riir-ai **Plan 620** (`.plans/620_gf_flow_figures.md`,
  IN EXECUTION) owns this page — **Group R** re-embeds every figure on
  `resources/index.html` (F2–F8). Wait for 620 P1 + P2 Group R to LAND (not
  just the `render_flows.py`/`sync_mirror.py` diffs currently uncommitted in
  reflex-site), then rebase onto Group R's version of the page and its smoke.
  After the rebase, register this plan as the first gfflow source added after
  620 — one row appended to Plan 620's figure table (or a note under its
  shared-file collisions section) so 620 stays the single list of figures;
  coordinate with the owning session if 620 is still in flight.
- [ ] T1 — reflexer docs: create `.docs/06_resources/` (`README.md` index +
  `resources.md`: the relation write-up in the voice of
  `../riir-reflex/.docs/05_resources/resources.md`, carrying the gfflow
  block). Update `.docs/README.md` (Folders table + the Figures note, which
  currently says "no site mirror" — now it has one), `AGENTS.md`
  (Documentation folder table), `HISTORY.md` (one line). The write-up obeys
  the public-copy laws (below) because its mirror is SERVED.
- [ ] T2 — reflex-site plumbing: add `reflexer_root` + the
  `Source(reflexer_root, ".docs/06_resources/resources.md", {rename map},
  "family:#ff8a3d")` row to `render_flows.py` SOURCES; add the riir-reflexer
  root + 3 pairs to `sync_mirror.py` (`resources.md` → `docs/reflexer/
  resources.md`; the two SVGs → `assets/reflexer_relation_flow*.svg`). Run
  the renderer (writes doc-side SVGs + site mirrors) and `sync_mirror.py`
  (writes the manifest rows). Commit doc + SVG together in reflexer.
- [ ] T3 — reflex-site page: add the `#reflexer` section to
  `resources/index.html` per the design above; hero CTA, title/meta, overview
  touch-up. Every `<img>` carries a real alt (the block's aria sentence).
- [ ] T4 — extend `scripts/resources_page_smoke.cjs`: sections array 5 → 6
  (`reflexer` after `rethink`); append the 4th framing sentence; append
  `/assets/reflexer_relation_flow.svg` to `wanted` and re-derive the expected
  `<img>` count from the POST-Group-R page (do not type "landed count + 1"
  from today's tree — Group R changes the set; the `_m` variant is a
  `<source>`, not an `<img>`). Fix the stale header comments while in there
  (they say "four sections" / "three framing sentences" against the code's
  five). Leave the existing framing pins byte-identical.
- [ ] T5 — gates, all green before commit: `resources_page_smoke.cjs` ·
  `public_copy_gate.cjs` · `render_flows.py --check` · `sync_mirror.py
  --check` · `web_family_gate.mjs` static + `--live` (+ `--canary` exits 1)
  from the riir-ai root. Mobile invariant: no sideways page scroll at 390
  (the `_m` card list is what makes the figure fit).
- [ ] T6 — land: commit BOTH repos in one family (reflexer `docs:` — docs +
  rendered SVGs + highwater; reflex-site `feat:` — page + smoke + SOURCES +
  mirror manifest + assets). Named-path staging ONLY (never `git add -A` —
  reflex-site carries sibling WIP in adjacent files). Push both. Deploy
  (`npx wrangler deploy`) and verify live: `/resources/#reflexer` anchor,
  figure + `_m` load, mirrors byte-check, footer/bar chrome intact.
- [ ] T7 — close-out: mark tasks done, status → COMPLETE, note the landing
  commits in both HISTORY files.
- [-] T8 — deferred: (a) converting `.docs/03_decision_flow/decision_flow.md`'s
  mermaid-era hero block to gfflow (its re-render snippet dies with the
  mermaid path 620 removes; the committed SVG keeps rendering — not required
  for this section); (b) a step-through WALK for the relation figure — the
  instrument covers ONE branch (the arena's recorded rulebook game:
  `rulebook_walk.mjs` + `demo_oracle.json` + 620's walk panel drives the 2b
  game-turn branch; the 2a text-question branch needs Reflex's own captured
  wire from the release binary's capture lane — reflex-site Issue 006, the
  lane Plan 620 already uses; plan for BOTH sources); never typed IN/OUT
  payloads; (c) a 4th `#overview` page-card.

## Copy-law checklist (web-family §5, enforced by the gates)

- No typed measured numbers anywhere in the section or the mirrored doc —
  classes only, link `/bench/` (numbers law; FAQ copy stays digit-free).
- No internal record ids (`Plan/Proposal/Issue/Bench N`), no "seal" (say
  lock/locked — the README's `SEAL_VESSEL_PUBKEY` precedent phrasing must
  NOT cross into served copy), no training recipes (moat law — nothing here
  teaches how hosted heads are built; PUBLIC-RELEASE vs HOSTED-ONLY is
  explained as a class fact only).
- Jargon glossed at first sight (rulebook, frozen genome, vessel, wire);
  sigmoid-never-softmax stays engine-internal, not page copy.
- Wire-claim wording law: "same question types and answer envelope, different
  state" — never "the same request" (Reflexer's state is a strict board
  shape; Reflex's is text/JSON).
- Losses visible: the section claims no performance wins — only structure.

## Risks

- **Sibling WIP in reflex-site** (T0): Plan 620's uncommitted groups touch the
  two plumbing files AND the page itself; landing order is a hard
  precondition, the smoke counts must be re-derived after the rebase, and
  staging must be surgical.
- **Hero/overview copy surgery** could destabilize pinned sentences —
  mitigated: existing pins stay byte-identical, additions only.
- **gfflow block authoring**: numbering/branch rules are renderer-VALIDATED
  (2a/2b branch labels, merge numbering) — expect one or two refused renders
  before it validates; that is the tool working.

## Gates summary

resources_page_smoke · public_copy_gate · render_flows --check ·
sync_mirror --check · web_family_gate static+live+canary · live deploy verify.
