# Integrated visual specification

This package is the source-owned visual companion to the stored Plan 1 and Plan 2 integration. It is documentation only: diagrams describe intended authoritative state, report-safe projection, interaction, geometry, and renderer responsibility; they are not implementation evidence or generated artwork.

## Visual language and accessibility

The atlas uses solid parchment (`#efe2bd`), wood (`#6d482b`), dark forest (`#173c2e`), ink (`#271b16`), moss (`#4f7040`), and restrained rust (`#a4492d`). It intentionally avoids gradients, glass, glow, and pill-dashboard treatment. Each SVG has a native `viewBox`, an accessible `<title>` and `<desc>`, high-contrast labels, a legend, and a prose fallback in the SVG itself. Every source has a checked-in 1600×1000 PNG rendering, and the complete set is available as a [contact sheet](rendered/contact-sheet.png). The reproducible validation record is in [QA.md](QA.md).

## Contents

| Source | Render | Covers |
|---|---|---|
| [authority-visibility-stale.svg](authority-visibility-stale.svg) | [PNG](rendered/authority-visibility-stale.png) | Report-safe authority loop, five-level visibility ladder, bounded stale-action refresh/retry path. |
| [progression-hole-states.svg](progression-hole-states.svg) | [PNG](rendered/progression-hole-states.png) | Notes/Void split, both research lanes, Hole feed and axis-upgrade state machines. |
| [task-footprints.svg](task-footprints.svg) | [PNG](rendered/task-footprints.png) | Fixed Hole 5×5/central 3×3/ring, full Workshop/Cookhouse, Fishing Hut/dock/water geometry. |
| [hunting-food-items.svg](hunting-food-items.svg) | [PNG](rendered/hunting-food-items.png) | Lair band/selection/party/loot/respawn, food-quality-cooking flow, exact item-detail layers. |
| [family-governance.svg](family-governance.svg) | [PNG](rendered/family-governance.png) | Family, enterprise, housing, mentoring, election, officer succession and expulsion cleanup. |
| [construction-storage.svg](construction-storage.svg) | [PNG](rendered/construction-storage.png) | 20/60/20 construction, physical cargo, storage zones, containers, fullness, linked workshop store. |
| [diplomacy-barter.svg](diplomacy-barter.svg) | [PNG](rendered/diplomacy-barter.png) | Alliance/Neutral/Enemy stance, escrow, caravan, route failure and recovery. |
| [shell-responsive.svg](shell-responsive.svg) | [PNG](rendered/shell-responsive.png) | Five-screen shell, six Council tabs, viewport/scale behavior for native and WASM. |
| [implementation-dag.svg](implementation-dag.svg) | [PNG](rendered/implementation-dag.png) | LAI.35–LAI.70 dependency and single-hot-root integration order. |
| [asset-state-matrix.svg](asset-state-matrix.svg) | [PNG](rendered/asset-state-matrix.png) | Renderer asset/state matrix and authoritative triggers. |

## Reading rules

- Diagram labels describe report-safe player-facing values unless marked **server truth**.
- A task footprint is complete and row-major; no center-point or client-derived fallback is implied.
- Solid arrows are authoritative data/physical flow. Dashed arrows are projection, report, or recovery-only flow.
- Concrete item IDs, art keys, snapshots, and protocol bounds remain governed by the two plans and the integrated implementation map, not by this package.

Primary authorities: `final-hole-hunting-content-plan.md`, `final-integrated-overhaul-plan.md`, and `integrated-implementation-map.md`. Requirement traceability and acceptance ownership remain in the two boards.
