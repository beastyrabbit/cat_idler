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

Research is one prerequisite graph, not one panel or lane per content category. Its presentation
layout is derived from the catalog DAG: `research_hut` is the only root, every dependency moves to
a later left-to-right layer, and studies in the same layer are packed without overlap around their
parents. Building, recipe/resource, and upgrade colors are accents only; never use them to split
the graph into separate coordinate ranges or full-height regions.

Adding a study requires catalog data only. The client recomputes the layer count, widest layer,
canvas bounds, centered root, cards, and connectors from that data. Do not add a screen-specific
coordinate or resize the canvas by hand. At rest, show only the root's first branches; selecting a
study reveals its complete prerequisite path back to the root. This keeps the 487-node overview
legible while preserving every dependency in the model and search. Tests must retain the
single-root, dependency-order, non-overlap, fixed-entity-count, and selected-ancestry invariants.

## Verification

Focused checks live in `ui_shell.rs` and relevant feature modules. The release-quality visual gate
is the generalized native/WASM framebuffer process in [`FIX_LOG.md`](FIX_LOG.md), not a single
happy-path screenshot. A surface is not complete if content exists but cannot be reached by mouse,
keyboard, or touchpad. Native matrix runs may set `CAT_UI_SCALE=1`, `1.15`, or `1.3` to override
the persisted player preference without modifying the signed session file.
