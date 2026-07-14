# P15 — Playtest feedback backlog (user, 2026-07-10)

> **Living feedback spec.** Movement smoothing, the booster, infinite-map streaming, and
> shrine-return scouting, visible authored/traffic roads, exact footprints/depth, and secure
> global/personal village foundations are verified. Coordinate placement, selectable/removable
> designations, election controls, and the first physical Sawmill inspector/queue are verified;
> the baseline Leader's browser/native shrine-return close-out is also verified. Broader physical
> stations and shared-world depth remain open in
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
- **Enough useful work.** The current founding is 15 cats, and strict vacant offices deliberately
  wait for player orders rather than silently filling every paw. Guided and staged-officer
  campaigns prove the ownership model; future playtests should judge whether each stage still
  offers more useful work than labor without treating intentional manual vacancies as an AI bug.
- **Food storage & shrine overlap — resolved.** Exact multi-tile footprints, reservations, and
  client footprint rendering prevent the old point-sprite overlap.
- **"2.5D but not really" — resolved.** Base-y depth sorting and exact footprints make cats pass
  behind buildings and trees while preserving the flat top-down view.

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
- **Roads visible — resolved.** Authored stone and traffic-formed dirt are disjoint snapshot
  surfaces with distinct rendering and 175%/105% movement effects; forbidden terrain cannot form
  dirt paths.
- **Workshops + production chains + routes — partial.** Mill/Sawmill, grain→flour→food,
  logs→lumber, fibre/hide→cloth/leather, ore→metal, and useful tools are live. The Sawmill now
  has a fully physical finite-store→station→store route, local ledgers, conserved cargo, and an
  editable queue; extend that contract to the remaining stations.
- **Fog of war + scout-driven discovery (detailed 2026-07-10).** The keystone exploration loop:
  - **World starts tiny** — only ~2 tiles outside the village are revealed at founding; everything
    beyond is fog.
  - **Deficit-driven scouting** — the leader notices a resource gap ("missing ~5 wood spots") and
    dispatches a **scout** to find it (extends the existing `scout` leader goal + explore job).
  - **Baseline Leader safety floor — close-out.** A true fresh one-second-cadence campaign exposed eight
    unattended-collapse resets in 48 hours after strict officer gating removed primitive food work.
    Before specialist offices exist, the founding Leader keeps deficit-scaled hunting, emergency
    water, and this scouting loop, capped at six/two/one in flight at 15 cats and scaled
    proportionally thereafter; it does not automate farms,
    production, research, rituals, or defense.
    The allocation/vacancy-cleanup fix passes three personal seeds and the 30-cat communal village
    through exact 48-hour one-second campaigns with fog growth and no specialist leakage; complete
    the fresh native founding confirmation; the optimized browser path is verified.
  - **Random-walk search — verified.** A scout follows deterministic knowledge-blind wander legs,
    changes direction with bounded route retries, recognizes a resource only after physical
    observation, and returns after success, survey/deadline exhaustion, or giving up. No hidden
    target exists before a genuine hit.
  - **Provisional vs committed reveal** — while the scout is out, fog lifts only *partially/dimly*
    around it; the discovery is **fully committed to the map only when the scout returns to the
    shrine** (knowledge delivered on arrival). So the shrine is the map's "known world" ledger.
  - Reuses: `reveal_and_wear_walked_tiles`, explorer reveal radius, the leader director scout goal,
    and the shrine-arrival pattern. Tiny initial reveal and two-tier provisional/committed knowledge
    are verified. The founding Leader now keeps autonomous Scout labor while Loremaster is vacant,
    without enabling research/ritual work; only the native/browser visual close-out remains.
- **Cat booster.** Look at a cat → give it a boost that makes it more likely to be picked for
  jobs/roles. New per-cat "priority/boost" (sim) + inspector button (client). Pairs with the
  matchCatsToSlots fit scoring.
- **Elections auto-run.** Term elections exist (`elections.rs`); verify `runElectionLifecycle`
  fires each tick and resolves terms without player action; surface the schedule in the HUD.
- **Unlimited map / multiple settlements.** Terrain is infinite and authoritative multi-colony
  play is live. Each stable signed identity can found one deterministic distant personal village,
  retain ownership and selection across restart, discover another village only through explicit
  returned-scout delivery provenance, and configure barter through signed capped atomic
  propose/accept actions while foreign state stays summary-only.

## Assets
- **Better wall asset** — the palisade could read better; source a nicer fence/wall (Roguelike/
  Tiny Town).
- **Cleaner top-down 2D cat?** Current `cat-sheet` is near-top-down P&W. A future art-polish
  pass may evaluate alternatives, but P&W remains the selected runtime sheet.

## Existing foundations (verify before extending)
- Cat movement, chunked-infinite terrain, authoritative multi-colony state, and term elections
  exist in the sim. Shrine-return fog/scouting plus usable global/personal founding, ownership,
  discovery, barter, visible roads, election controls, exact designation tools, and the physical
  Sawmill queue/inspector are verified. Broader station-local physical routes and inspection remain
  product work; authoritative data structures alone are not completion.
