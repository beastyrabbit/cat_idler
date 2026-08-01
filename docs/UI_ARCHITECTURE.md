# Bevy UI architecture

This is the maintained contract for `cat-client` layout. The visual language is defined by
[`migration/specs/p18-visual-polish.md`](migration/specs/p18-visual-polish.md); this document defines
how screens fit, scroll, scale, and own input.

## Supported layout matrix

Every change must remain usable at these native and WASM viewport sizes:

| Viewport | Interface scale |
| --- | --- |
| 1024×768 | 100%, 115%, 130% |
| 1280×800 | 100%, 115%, 130% |
| 1920×1080 | 100%, 115%, 130% |
| 2560×1440 and 3840×2160 | automatic density scale × 100%, 115%, 130% |

Phone layouts are not supported yet. Do not make desktop behavior depend on a phone breakpoint.
`UiScale` scales the whole interface—text, controls, spacing, and hit targets together. Layout
decisions use effective dimensions (`viewport / UiScale`), not raw window pixels.

## Surface model

Every top-level UI entity has `UiSurfaceRoot(UiSurfaceKind::…)` and belongs to one category:

- `Hud`: compact world information and navigation. It never owns long dynamic lists.
- `ContextPanel`: inspectors and short-lived work surfaces. It may appear over the world or a
  primary screen, but is bounded between the top navigation and bottom dock.
- `PrimaryScreen`: a major destination. Exactly one may be visible. `UiRouter` is its only source
  of truth; feature resources may mirror router state for rendering but must not arbitrate it.
- `Modal`: a blocking layer with explicit Escape behavior.
- `Tooltip`: non-persistent feedback that never contains required actions.

The maintained primary screens are Log, Stores, Village, and Research. Their buttons remain in
the top bar; activating another tab replaces the current screen, and Escape returns to the world.
Add a `PrimaryScreen` variant and route through `UiRouter` when introducing another destination.
Never create an independently toggled full-window Boolean.

Layer order is centralized in `ui_shell.rs`: HUD < primary screen < context panel < feedback <
modal/start. Avoid unexplained numeric `GlobalZIndex` values for new surfaces.

### Start-screen contract

The entry form lives in `cat-client/src/start_screen.rs`; the staged background lives separately in
`cat-client/src/landing_showcase.rs`. The surface is one blocking `Modal` charter over a
save-independent, deliberately aspirational mature village. The showcase must reuse the tracked
game art and established road, cutaway-station, prop, cat-atlas, animation, and depth grammar; it
must not introduce a parallel visual language.

The showcase occupies a dedicated off-map coordinate and a resolution-independent 72-tile camera
overview. Wide layouts place the charter beside it; compact layouts centre the charter. The scene
may be deterministic and authored, but it must remain strictly presentational: no
`WorldSnapshot`/`ColonySnapshot`, server action, simulation tick, persistence write, player
selection, or save-state mutation. Entering the game must restore the authoritative village camera
and stop showcase motion immediately.

The scene should communicate a settlement that has succeeded for roughly two in-game years:
multiple districts, mature production chains, civic buildings, storage yards, agriculture,
defences, connected but imperfect streets, and enough independently moving cats to feel inhabited.
Tests enforce minimum scene dimensions, building/type diversity, road density, population, and
route containment. Gameplay HUD, feedback, order markers, and other actionable annotations must
not leak into the showcase.

The charter owns all required information. Its banner contains only the game title; status or
tagline copy belongs in the body. Its primary action remains outside visual competition with the
showcase, and its body uses `spawn_vertical_scroll_area()`. Destination cards
show the authoritative village name and population when known, keep a persistent `AUSGEWÄHLT`
state plus the explicit `Ausgewählt: …` summary, and never auto-enter. A persisted village may be
preselected only after the snapshot proves that destination still exists. The connection state,
disabled primary action, and nearby helper copy must explain loading, missing name, missing
destination, missing settlement name, and pending foundation without relying on a toast.

Text inputs require a four-part focus treatment: accent border, lighter field fill, accent label,
and left focus rail. Responsive decisions use effective dimensions. Destination cards stack below
860 effective pixels wide or 640 high; the charter stays within 92% of the effective viewport and
the scroll body keeps every action reachable. When adding another entry destination or field,
extend the typed start state, readiness tests, selected-summary copy, focus order, and compact
layout test together.

## Scrolling and sizing

Use `primary_screen_node()` for a primary-screen root. Keep the title/navigation outside the
scroll viewport and put the dynamic body in `spawn_vertical_scroll_area()`. The helper provides:

- a viewport constrained by its parent;
- mouse-wheel/trackpad scrolling through Bevy's `ScrollArea`;
- a draggable scrollbar that hides when content fits;
- Page Up, Page Down, Home, and End while hovered;
- scroll reset when a hidden surface is reopened.

The same body helper is required for context panels, onboarding, and modal content whenever
the content is dynamic or can grow through localization, settings, or future features. A fixed
height plus `Overflow::clip*` is never an acceptable substitute. Clipping belongs only on small
intentional canvases such as progress fills, sprites, and the research canvas viewport.

Prefer `min_height` over `height` for buttons and fields so content can enlarge the control.
Potentially long labels or server-provided values use `ui_text_wrapped`. Flex/grid children that
may shrink set `min_width: 0` or `min_height: 0`. Responsive navigation removes decorative content
before hiding or shrinking an action.

## Input ownership

- Top-bar buttons select a primary screen through `UiRouter`; letter keys do not open surfaces.
- Escape closes the highest applicable surface and returns primary screens to the world before it
  opens the pause menu.
- Scroll wheel events over a `ScrollArea` are consumed by that area; the world camera must not also
  zoom.
- Interactive surface roots use `WorldInputBlocker` so clicks never fall through to world tools.
- Research text entry keeps ownership of its keys until the search field releases focus.
- Pointer panning uses logical `CursorMoved` deltas so native and WASM track the same movement.
  World panning is middle-drag or Space-left-drag; canvas-style primary screens may use direct
  left-drag inside their computed viewport. Keep the shared gain zoom-independent in screen space
  and cover it with a pure delta test.

## Adding a panel or full-screen destination

Before merging a new surface:

1. Classify it with `UiSurfaceKind`; if it is a major destination, add it to `PrimaryScreen` and
   `UiRouter` rather than adding a visibility Boolean.
2. Use the shared bounded root and scroll body. Keep required actions reachable at the bottom of an
   overfilled test body.
   If the body is an intentional canvas, give it direct pointer panning, visible category/state
   hierarchy, Home/zoom controls, and a contained inspector instead of pretending it is a list.
3. Define Escape, button, world-input, and layer behavior. Do not add a keyboard shortcut that
   opens the surface.
4. Check long translated text, empty data, normal data, and deliberately excessive data.
5. Run the full supported layout matrix at settled frames, including 1024×768 at 130%.
6. Add or update unit tests for routing, responsive thresholds, and the layout node contract.

### Research-tree contract

Research is one vertical prerequisite graph shown through normalized technology tracks. The 495
raw catalog nodes remain the stable save/effect ledger. The left catalog has 88 meaningful
technologies. Milestones, buildings, and production families each keep one recognizable graph
node; global modifiers expand into explicit levels 1–10 and a separate infinite terminal. This
avoids ten visually duplicated copies of every ordinary technology while keeping the finite-to-
infinite transition explicit. Grouping and finite-level projection live in
`cat-sim::research_tracks`; the client must not maintain a second family list.

The screen has three durable regions: a scrollable catalog and active queue on the left, the
fixed-scale dependency canvas in the center, and the selected technology inspector on the right.
With no selection the complete graph is visible. Selecting a catalog row or graph node isolates
the selected node, every transitive prerequisite, and every transitive downstream unlock. Every
graph node uses `research_icon_path()` and contains only its icon and name; type, description,
requirements, effects, state, and cost belong in the inspector. Focus mode recentres and compacts
each visible depth layer instead of preserving the wide overview coordinates.

Layout is derived from prerequisites. Every dependency moves to a later top-to-bottom tier, cards
in a tier cannot overlap, and the larger vertical gap must leave connector routes readable.
Categories are accents, never separate trees. Scale is fixed and deliberately legible. Root
centering and panning use the effective scaled viewport after both side columns—not raw window
pixels.

Prerequisites use AND semantics and must be visibly projected. Curated junction studies merge
independent disciplines, and later family-stage gates are included on their collapsed card so the
screen does not hide a real requirement. The client regression floor requires at least 24
player-facing cards with two or more incoming paths. Raw and projected graphs must both remain
acyclic.

Wheel ownership follows pointer location. Wheel/trackpad input over the left scroll area scrolls
only the catalog; over the canvas it pans only the tree; over the inspector it scrolls only the
details. Canvas drag remains available for two-dimensional travel. Do not restore global wheel
handling or Ctrl-wheel zoom.

The inspector queues a full missing prerequisite path. It does not purchase immediately and does
not require the current point balance. Queue order, funding, partial time, and repeatable level are
authoritative snapshot state; the UI only dispatches signed queue actions. Queue controls remain
buttons, never letter shortcuts.

See [`RESEARCH_ARCHITECTURE.md`](RESEARCH_ARCHITECTURE.md) for progression, persistence, formulas,
and the checklist for adding technologies or upgradeable buildings.

## Verification

Focused checks live in `ui_shell.rs` and relevant feature modules. The release-quality visual gate
is the generalized native/WASM framebuffer process in [`FIX_LOG.md`](FIX_LOG.md), not a single
happy-path screenshot. A surface is not complete if content exists but cannot be reached by mouse,
keyboard, or touchpad. Native matrix runs may set `CAT_UI_SCALE=1`, `1.15`, or `1.3` to override
the persisted player preference without modifying the signed session file.
