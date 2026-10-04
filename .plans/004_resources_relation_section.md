# Plan 004 — the Reflex ↔ Reflexer relation section on reflex.gist.rs/resources

**Status:** COMPLETE — 2026-10-04. T0 resolved by TAKING OVER Plan 620's idle
WIP (P1 landed: reflex-site `fe69bd2`, riir-rethink `7b86e8b`, riir-ai `0d5f852a1`);
F16 landed BEFORE Group R starts (the amended collision order, recorded in 620).
Section live at reflex.gist.rs/resources/#reflexer (reflex-site `12538b9`, deployed
2026-10-04; riir-reflexer docs `ba943d8`).

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

- [x] T0 — precondition, RESOLVED BY TAKEOVER: the sibling session's Plan 620
  WIP was idle and uncommitted; this session took it over, completed it
  (payload regen + re-render + gates + the P1.5 halves: pre-deploy prose +
  web_family_gate S4 with the shrink-only pin, canary-verified) and LANDED
  P1 across three repos (reflex-site `fe69bd2`, riir-rethink `7b86e8b`,
  riir-ai `0d5f852a1`). F16 is registered in 620's inventory (row F16) with
  the amended collision order: it lands BEFORE Group R; Group R then
  migrates F2–F8 around it. The smoke counts were re-derived from the
  actual (pre-Group-R) page: 9 → 10 imgs.
- [x] T1 — reflexer docs: `.docs/06_resources/` (`README.md` + `resources.md`
  with the gfflow block) created; `.docs/README.md` (Folders + Figures) and
  `AGENTS.md` updated. Commit `ba943d8` (HISTORY line rides the close-out).
- [x] T2 — reflex-site plumbing: `reflexer_root` + the Source row landed; the
  riir-reflexer root + 3 pairs in sync_mirror (16/16 in sync). The renderer
  validated the block (three refuses en route — card title width, a 3-line
  body, a blocked one-bend route — fixed by shortening and moving the vessel
  card to col 4; the tool working as designed).
- [x] T3 — the `#reflexer` section landed after `#rethink` (framing sub with
  bracketed glosses, <picture> figure + write-up links, teaser cells, FAQs
  incl. the arena-lane naming answer, deep write-up mirror + GitHub); hero
  CTA + title/meta + overview name the fourth name; two glossary rows added
  (rulebook, frozen genome).
- [x] T4 — smoke extended: six sections, the 4th framing pin, `wanted` + img
  count re-derived (10), stale header comments fixed ("four sections"/
  "three framing sentences" → six/four).
- [x] T5 — gates all green: resources smoke PASS (six sections, four framings,
  10 images, 32 glossary terms, numbers + moat + seal laws clean) ·
  public_copy_gate PASS (7 pages × 2 sizes, 22 served md/svg) · render_flows
  --check all-in-sync · sync_mirror --check 16/16 · web_family_gate static
  PASS (S4: 19 flow SVGs, 4 migrated, 15 pinned) and --live PASS (4/4 fronts,
  one type signature, geometry aligned; --canary exits 1) · home + bench
  smokes PASS.
- [x] T6 — landed + deployed: riir-reflexer `ba943d8` (docs + SVGs) ·
  reflex-site `12538b9` (section + smoke + plumbing + mirrors), both pushed.
  Deployed BOTH fronts (reflex-site via `npx wrangler deploy`; the rethink
  site via `site/scripts/deploy.sh` — the taken-over trust flow rode along).
  Live-verified: `/resources/#reflexer` serves, both figure variants + the
  mirrored doc 200, rethink's gfflow trust flow + walk + walker 200.
- [x] T7 — close-out: this status update + the HISTORY entries + the 620 F16
  row flip (landed).
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
