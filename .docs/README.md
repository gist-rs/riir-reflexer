# reflexer — Documentation

The public decision-engine repo's `.docs/` book, organized the fleet way:
**numbered folders** for sort order, **bare slugs** for files, a `README.md`
index in every folder, and this file as the top-level index.

## Convention

- **Folders are numbered** (`01_orientation/`, `02_wire_protocol/`, …) for
  sort order. This matches `katgpt-rs/.docs/` and the rest of the fleet.
- **Files inside have NO number prefix** — add a new doc by dropping
  `slug.md` in the right folder and adding one line to that folder's
  `README.md` index table.
- `README.md` (repo root) is the build surface; this book is the narrative
  behind it. Where the two disagree with source, the source module docs win.

## Folders

| Folder | What it covers |
|---|---|
| [`01_orientation/`](01_orientation/) | Repo orientation: the sibling dependency layout, the leaf law, the workspace members, where everything else lives |
| [`02_wire_protocol/`](02_wire_protocol/) | The measurement contract: the line protocol, the state schema, the three questions, the envelope, the error codes, the HTTP twin |
| [`03_decision_flow/`](03_decision_flow/) | The end-to-end engine flow narrative + its SVG diagram (request line in → one search → question mapping → envelope out) |
| [`04_vessel_format/`](04_vessel_format/) | The vessel format crate: format v1 anatomy, the two-class law, key rotation, monotonic apply, hardening, the public writer |
| [`05_measurement_lane/`](05_measurement_lane/) | The measurement lane + trajectory submission: artifact pins, the box-state law, the G1/G2 gates, the signed manifest |

## Figures

`03_decision_flow/decision_flow.svg` is rendered from the `%% file:`-headed
mermaid block in `03_decision_flow/decision_flow.md` with the fleet renderer
(`../reflex-site/scripts/render_flows.py` — the gist.rs web-family palette,
the Reflex-family accent: this engine is the arena's *Reflex · rulebook*
lane). Unlike reflex's hero figure it carries **no site mirror** — this repo
is not in the renderer's `SOURCES`; re-render locally (the doc's
"Re-rendering" section has the exact procedure) and always commit the doc
and its SVG together.
