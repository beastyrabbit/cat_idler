# P15 — Playtest feedback backlog (user, 2026-07-10)

> **Living feedback spec.** Movement smoothing, the booster, infinite-map streaming, and
> shrine-return scouting are verified. Exact player controls, global/personal villages, richer
> inspectors, and complete visible roads remain open in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

Captured from live `cargo dev` playtesting. Triaged; "already there" notes from a code survey.

## Bugs / feel (do first)
- **Cats appear static / "moving in place".** Sim DOES move cats (`world_tick` phase_33/34 set
  `cat.position` via `walk_path`), but snapshot positions are per-tile integers updated ~1×/s.
  Client fix: **constant walk-SPEED chase** — advance the rendered position toward the latest
  snapshot tile at a fixed pace (~2–4 tiles/s) every frame; never snap/teleport, so cats visibly
  walk tile-to-tile even when the sim advances them several tiles between snapshots (falling a
  little behind the sim is fine). Walk-anim only while moving; idle-frame when arrived. User wants
  it "realistic": cats always walk one tile to the next, never jump. (Sim already walks every tile
  internally; this is purely the client render. If a fast cat lags badly, cap the max lag.)
- **Too many idle cats.** User: nearly every cat should normally have a job; idle should be rare.
  Sim: raise job saturation (leader director / job generation keeps cats busy — "more jobs than
  cats"). Today ~16 jobs / 20 cats leaves several idle.
- **Food storage & shrine overlap.** Sim placement is now non-overlapping (P14.1 footprints), but
  the client still draws buildings as point sprites — fix with the footprint render (P14.5).
- **"2.5D but not really."** Add proper **y-sort depth** (sprites layer by base-y so cats pass
  behind buildings/trees) + footprint render → a convincing layered top-down. (P14.5 + depth.)

## Controls (client)
- **Middle-drag = pan map** (keep as-is). **Right-click = select building.**
  **Left-click = select cat** (already). (Revised 2026-07-10: middle stays pan, building→right.)
- **Click on ALL items** — buildings + stockpiles + cats all inspectable. Done (b52ea92/9fac089):
  cats (left-click) + building panel (right-click) + stockpile-remove.

## Inspection UX — hover (small) vs open (big), + cycle (client + sim data)
Two-tier inspector, driven by the cursor:
- **Hover** (no click) → a **small tooltip** of whatever is under the mouse (any entity: cat /
  building / stockpile / tile). Name + key LIVE status. e.g. a **workshop**: its name, what it's
  currently producing, and which cat is using it.
- **Right-click** → the **big detailed menu** for the thing under the cursor. e.g. a **workshop**:
  production **queue length**, what's **inbound** (being hauled to it), what's in its **storage**,
  staffing, etc.
- **Shift + right-click** → **cycle** through overlapping entities under the cursor (a cat on a
  workshop tile, stacked piles, …) to target the one you want.
- **Requires sim data**: the big menu needs the snapshot to expose richer per-building state —
  current production item + progress, queue, inbound hauls, on-site storage, assigned cats. Small
  sim/protocol addition alongside the workshop/production work (P12.4b).

## Features (sequence)
- **Roads visible.** Sim paves wear-trails (`roads.rs`) but roads aren't in the snapshot or
  rendered. Expose paved tiles + render (part of P14.4 road/accessibility).
- **Workshops + production chains + routes.** Discussed but not built — P12.4b (new-resource
  chains: mill/clothier/sawmill + grain/fibre/cloth/lumber) + the visible haul routes between
  them. Bigger card.
- **Fog of war + scout-driven discovery (detailed 2026-07-10).** The keystone exploration loop:
  - **World starts tiny** — only ~2 tiles outside the village are revealed at founding; everything
    beyond is fog.
  - **Deficit-driven scouting** — the leader notices a resource gap ("missing ~5 wood spots") and
    dispatches a **scout** to find it (extends the existing `scout` leader goal + explore job).
  - **Random-walk search** — the scout wanders a random direction for a while, changes direction,
    repeats, until it finds the needed resource tiles (or gives up), then heads back.
  - **Provisional vs committed reveal** — while the scout is out, fog lifts only *partially/dimly*
    around it; the discovery is **fully committed to the map only when the scout returns to the
    shrine** (knowledge delivered on arrival). So the shrine is the map's "known world" ledger.
  - Reuses: `reveal_and_wear_walked_tiles`, explorer reveal radius, the leader director scout goal,
    and the shrine-arrival pattern. Adds: tiny initial reveal, deficit→scout trigger, random-walk
    scout behavior, and the two-tier (provisional-while-out / committed-on-return) reveal state.
- **Cat booster.** Look at a cat → give it a boost that makes it more likely to be picked for
  jobs/roles. New per-cat "priority/boost" (sim) + inspector button (client). Pairs with the
  matchCatsToSlots fit scoring.
- **Elections auto-run.** Term elections exist (`elections.rs`); verify `runElectionLifecycle`
  fires each tick and resolves terms without player action; surface the schedule in the HUD.
- **Unlimited map / multiple settlements.** Terrain already infinite (per-chunk) + multi-colony
  supported. Verify the client can roam/render arbitrary chunks and found new villages anywhere.

## Assets
- **Better wall asset** — the palisade could read better; source a nicer fence/wall (Roguelike/
  Tiny Town).
- **Cleaner top-down 2D cat?** Current `cat-sheet` is near-top-down P&W. A future art-polish
  pass may evaluate alternatives, but P&W remains the selected runtime sheet.

## Existing foundations (verify before extending)
- Cat movement, chunked-infinite terrain, authoritative multi-colony state, and term elections
  exist in the sim. Shrine-return fog/scouting is now verified. Usable global/personal founding
  and ownership, election controls, and several richer inspection/road paths remain product work;
  authoritative data structures alone are not completion.
