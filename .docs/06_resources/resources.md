# Reflexer — the rulebook engine, and its relation to Reflex

Reflexer is the family's rulebook engine — it speaks Reflex's question
types and answer envelope over a different state, the game board: one
frozen-genome search per turn answers where to place, how the position
stands, and whether it survives. [Rulebook: a fixed list of rules that
scores every board a move can lead to. Frozen genome: shipped as constant
bytes — never retrained, never blended.]

Shown on the education page at
[reflex.gist.rs/resources/#reflexer](https://reflex.gist.rs/resources/#reflexer);
this doc is the source of truth, the site carries a byte-identical mirror.

## What Reflexer is

A public, open-source (MIT) engine — the sibling of Reflex, built on the
same open substrate underneath both ([KatGPT-RS](https://github.com/katopz/katgpt-rs)).
Where Reflex answers questions about text you wrote, Reflexer runs a game:
its state is a board, and one lookahead search per request scores every
board each legal move can lead to. The rulebook — the genome — is fixed
bytes inside the binary; nothing is trained at runtime, and the same
request always produces the same answer, on every host.

## One vocabulary, two states

Both engines speak the same question vocabulary — **choice** (pick one
option from a list), **score** (rate on an ordered scale), **yes-no** —
and both answer in the same envelope: one typed answer per question with
probabilities, and an abstain as a first-class answer, never an exception
to catch. What differs is the state:

- **Reflex's state is text you wrote** — a sentence, a paragraph, a JSON
  object. Answers come from a corpus you author; off-corpus, it abstains.
- **Reflexer's state is the game facts** — the board, the falling piece,
  the preview, what is left in the bag. Its three questions are fixed:
  **place** (choice — where to put the falling piece), **state** (score —
  how the position stands, on a fixed rubric from critical to excellent),
  **survive** (yes-no — does a legal move exist).

The relation in one line: Reflex is the question engine; Reflexer is the
same wire played as a game.

## Where you have already seen it

The arena's "Reflex · rulebook" boards ARE this engine — the lane is
family-branded, the engine behind it is Reflexer, and the Reflex question
engine does not need it. It runs in three hosts from one set of bytes:
the local binary (one JSON line in, one envelope line out — malformed
input answers a typed error and the pipe stays alive), the browser tab
(WebAssembly, inside the arena page), and a stateless Cloudflare Worker.
The three answer identically and differ only by the network hop.

## The vessel format — one signed file, applied whole

Reflexer also gave the family its artifact format: the **vessel** — a
BLAKE3-locked file (hashed and signed, so any change is detectable) that
swaps the engine's genome whole at boot. It is applied monotonically, so
a downgrade or a lineage fork refuses; and it is never a weighted blend,
because blends do not preserve move rankings. The format is a public
crate anyone can mint their own artifacts with; the family's artifacts —
public and hosted alike — ship in it. [Vessel: one signed file carrying
an artifact; the class bit inside the signed header says where it may
run.]

## Measure it

Every performance claim on this page is a class, not a figure — decisions
carry their own in-engine timing in the envelope, and the engine never
makes a network call in a measurement path. The measured side lives on
the site's bench lanes and the arena's recorded boards.

- GitHub: [gist-rs/riir-reflexer](https://github.com/gist-rs/riir-reflexer)
- The arena's rulebook boards:
  [reflex.gist.rs/arena](https://reflex.gist.rs/arena/)
- The engine's own end-to-end flow:
  [decision_flow.md](https://github.com/gist-rs/riir-reflexer/blob/develop/.docs/03_decision_flow/decision_flow.md)

## The relation figure

The figure below is rendered from the ` ```gfflow ` block in this doc by
the fleet renderer — the desktop swimlane and the narrow-screen card list
(`reflexer_relation_flow.svg` + `_m`) are committed beside this doc and
mirrored into the site's assets. Never hand-edit the SVGs; edit the block
and re-render.

```gfflow
file  = "reflexer_relation_flow.svg"
title = "Reflex and Reflexer: one question vocabulary, two engines"
accent = "reflex"

[[lane]]
id = "wire"; label = "The shared wire"; note = "one vocabulary, one envelope"; color = "reflex"
[[lane]]
id = "rx"; label = "Reflex"; note = "the question engine — free, on your machine"; color = "reflex"
[[lane]]
id = "rf"; label = "Reflexer"; note = "the rulebook engine — the arena's rulebook boards"; color = "reflex"

[[step]]
id = "req"; n = "1"; lane = "wire"; col = 0
title = "The request"; body = "a state plus typed questions — choice, score, yes-no"
status = "live"
[[step]]
id = "rxt"; n = "2a"; lane = "rx"; col = 1
title = "State is text"; body = "the question engine reads it"
status = "live"
[[step]]
id = "rft"; n = "2b"; lane = "rf"; col = 1
title = "State is a board"; body = "the game turn: board, piece, bag"
status = "live"
[[step]]
id = "rxa"; n = "3a"; lane = "rx"; col = 2
title = "Your corpus"; body = "answers from documents you author, or abstains"
status = "live"
[[step]]
id = "rfa"; n = "3b"; lane = "rf"; col = 2
title = "One frozen search"; body = "the genome scores every board a move can lead to"
status = "live"
[[step]]
id = "env"; n = "4"; lane = "wire"; col = 3
title = "The envelope"; body = "one typed answer per question — or an abstain"
status = "live"
[[step]]
id = "vsl"; n = "5"; lane = "rf"; col = 4
title = "The vessel"; body = "one signed file swaps the genome whole"
status = "live"; note = "public format"

[[edge]]
from = "req"; to = "rxt"; label = "a text question"
[[edge]]
from = "req"; to = "rft"; label = "a game turn"
[[edge]]
from = "rxt"; to = "rxa"
[[edge]]
from = "rft"; to = "rfa"
[[edge]]
from = "rxa"; to = "env"; label = "answer or abstain"
[[edge]]
from = "rfa"; to = "env"; label = "place · state · survive"
[[edge]]
from = "vsl"; to = "rfa"; back = true; label = "a different genome, whole"
```
