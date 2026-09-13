# GUI redesign — mockups

**Start with [`GUIDE.html`](GUIDE.html)**: the implementation handoff for the chosen Glass Clearing
direction (tokens, layout, every component with live examples and states, behaviour, copy rules,
the data each component needs, engine notes for Unity UI Toolkit and Bevy, accessibility, and the
migration checklist). Everything below is the exploration that led to it.

Published copy (self-contained, screenshots hosted on Schaffa): https://schaffa.dev/p/yeoyjeqykxc440xk
Rebuild it with `python3 docs/ui-redesign/build_schaffa_page.py` (reads `schaffa-uploads.tsv` and
`schaffa-assets.json`, writes `schaffa-guide.html`); re-publish with `PUT /api/pages/yeoyjeqykxc440xk`
so the URL stays stable.

Static HTML mockups and PNG renders for a redesign of the Idle Cat Forest management HUD.
They answer the brief in the "current UI handoff" page (Unity build, `codex/unity-migration`):
make causal state legible (what a cat does, where it goes, what it carries, why a job is
blocked, which report is stale, who owns automation, how far an expedition is, what to do next)
without hiding the world.

Nothing here is wired into the game. Every screen draws the same colony moment on the same
world scene (`mockups/world.js`, built from the tracked sprites under `public/images/`), so
only the chrome differs between directions.

## Regenerate

```bash
docs/ui-redesign/render.sh docs/ui-redesign/mockups/d3-atlas.html docs/ui-redesign/renders/d3-atlas.png
# strips: append 1920,440 · narrow window: 1280,720
```

Needs Google Chrome (headless) and network access for Google Fonts.

## Decision (2026-09-13)

Direction 4, **Glass Clearing**, was chosen. Field Atlas was judged too busy. Attention uses one
sealed counter (option 5 of the attention strip) that opens a popup. The developed Glass screens
are the `renders/g*.png` set; the `p*` strips cover the three choices still open inside Glass.

## Glass Clearing, developed (`renders/g*.png`)

| File | Screen | What it demonstrates |
| --- | --- | --- |
| g0-overview | Overview, seal closed | Village chip + attention seal, resource pills, grouped icon dock, tethered inspector card, need rings only below 30 % |
| g1-overview-popup | Overview, popup open | Anchored popover under the seal: four items, cause line, primary action, "Show" pans the map |
| g2-cats | Cats drawer | Left glass drawer with the attention-sorted census; map and card stay visible |
| g3-work | Work drawer | Officer ownership pills, job state chips, "why blocked" box, Sawmill card tethered on the map |
| g4-build | Build mode | Catalog tray above the dock, ghost footprint, checklist card tethered to the ghost |
| g5-stores | Stores drawer | Per-pile counts with COUNTED / EST. chips, Accountant's round, Stone pile card |
| g6-world | World drawer | Site table, readiness table, expedition card tethered to the dungeon entrance |
| g7-research | Research, wide drawer | Families × stages grid with the selected study in a side column; map dimmed |
| g8-narrow | 1280 × 720 | Same chrome scaled: smaller pills, dock, card; minimap hidden |
| g9-control | Direct control (Tab) | Dock and resources retreat; reins bar, nearby interactables, E prompt |

## Open choices inside Glass (`renders/p*.png`), five options each

| File | Decision | Recommended |
| --- | --- | --- |
| p1-alert-popup | What the attention seal opens | Anchored popover under the seal |
| p2-sheets | How Cats, Work, Stores, Research open | Left glass drawer, wide variant for Research |
| p3-dock | Labelling fourteen dock icons | Icons only, grouped by dividers, label on the active one |

Shared kit: `mockups/glass.css`, `mockups/glass.js` (chrome helper), `mockups/gopts.css` (strips).

## Five directions (`renders/d*.png`, comparison in `renders/00-compare.png`)

| # | Name | Idea | Type | Palette |
| --- | --- | --- | --- | --- |
| 1 | Hearthwood | Wood rail + parchment panels from the tracked Adventure UI pack | Pixelify Sans, Nunito | bark, plank, parchment, moss, ember |
| 2 | Overseer's Ledger | Dense, keyboard-first, DOING/FROM/TO/BECAUSE/THEN block | IBM Plex Mono, Plex Sans Condensed | slate, amber |
| 3 | Field Atlas | Vellum margins, framed map, thumb index, one-sentence "ledger line", wax-sealed notes | Alegreya, Alegreya Sans, Courier Prime | vellum, ink, madder, moss, brass |
| 4 | **Glass Clearing** (chosen) | Minimal glass chrome, tethered inspector card, need rings on cats | Manrope, Instrument Serif | frosted green glass, sun |
| 5 | Command Deck | RTS console: minimap, subject cells, 4×3 hotkey command grid | Barlow Condensed, Barlow | olive steel, brass |

## Field Atlas, developed (`renders/a*.png`, superseded by the Glass set)

| File | Screen | What it demonstrates |
| --- | --- | --- |
| a1-cats | Cats sheet | Census sorted by attention; one causal line per cat; needs as four sparks; bed column |
| a2-work | Work sheet | Officer ownership strip; job state chips (automated / manual / blocked / waiting); "why blocked" with fixes |
| a3-build | Build mode | Catalog strip with gates and costs; footprint legality per tile; entrance check; materials with freshness |
| a4-stores | Stores sheet | Per-pile physical counts with COUNTED / EST. stamps and age; the Accountant's round |
| a5-world | World and expedition | Site list, readiness table, floor progress, cargo slots, route-home bar, recall |
| a6-research | Research ledger | Families × stages grid (487 studies), affordable-now filter, study card that names the alert it would fix |
| a7-narrow | 1280 × 720 | Tally collapses to icons, index becomes a strip, ledger becomes a bottom sheet, notes fold into one seal |
| a8-control | Direct control (Tab) | Chrome retreats; reins bar with keys; nearby interactables with E prompts; automation order paused |

## Option strips (`renders/o*.png`), five options each

| File | Decision | Recommended |
| --- | --- | --- |
| o1-navigation | Where the 14 sections live | Thumb index on the right |
| o2-freshness | Marking counted vs. estimated reports | Italic caption with age; ≈ for estimates |
| o3-attention | Presenting "needs attention" | Pinned notes along the bottom, each with its action |
| o4-needs | Showing a cat's need on the map | Red tick only when a need drops below 30 % |
| o5-ledger | How the ledger describes a subject | One sentence, then a table of exact figures |

## Files

- `mockups/world.js` — shared canvas scene (terrain, roads, buildings, piles, farms, fog, cats, cargo, route)
- `mockups/icons.js` — 24 px line icons and the section list
- `mockups/atlas.css`, `mockups/atlas.js` — Field Atlas chrome shared by the `a*` screens
- `mockups/options.css` — option-strip layout
- `render.sh` — headless Chrome screenshot helper
