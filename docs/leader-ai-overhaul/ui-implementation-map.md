# LAI.28-31 UI implementation architecture map

> Historical first-cutover readiness evidence only. The current client is the five-screen/six-tab
> Plan 2 shell in [`integrated-implementation-map.md`](integrated-implementation-map.md). The deleted
> `research_ui.rs`, Shrine/Favor progression, and earlier navigation below are not current targets.

This map is additive implementation guidance for the LAI.28-31 Bevy client
surfaces. It does not implement client code, does not edit protocol, server,
persistence, or `world_tick`, and does not update board status.

## Sources read

- Current Bevy client root:
  [`../../crates/cat-client/src/lib.rs`](../../crates/cat-client/src/lib.rs)
- Research UI leaf:
  [`../../crates/cat-client/src/research_ui.rs`](../../crates/cat-client/src/research_ui.rs)
- Station layout leaf:
  [`../../crates/cat-client/src/station_layout.rs`](../../crates/cat-client/src/station_layout.rs)
- LAI.28 red contract:
  [`../../crates/cat-client/tests/lai28_plans_ui_contract.rs`](../../crates/cat-client/tests/lai28_plans_ui_contract.rs)
- LAI.29 red contract:
  [`../../crates/cat-client/tests/lai29_world_task_footprint_contract.rs`](../../crates/cat-client/tests/lai29_world_task_footprint_contract.rs)
- LAI.30 red contract:
  [`../../crates/cat-client/tests/lai30_cat_care_ui_contract.rs`](../../crates/cat-client/tests/lai30_cat_care_ui_contract.rs)
- LAI.31 red contract:
  [`../../crates/cat-client/tests/lai31_progression_ui_contract.rs`](../../crates/cat-client/tests/lai31_progression_ui_contract.rs)
- Wire and snapshot contracts:
  [`wire-persistence-ui.md`](wire-persistence-ui.md),
  [`snapshot-implementation-map.md`](snapshot-implementation-map.md), and
  [`action-implementation-map.md`](action-implementation-map.md)
- Server routing/redaction readiness:
  [`server-implementation-map.md`](server-implementation-map.md)
- Feature docs:
  [`planner-and-beliefs.md`](planner-and-beliefs.md),
  [`spatial-task-contract.md`](spatial-task-contract.md),
  [`cats-and-care.md`](cats-and-care.md),
  [`hole-research-progression.md`](hole-research-progression.md), and
  [`diplomacy-trade.md`](diplomacy-trade.md)
- Browser evidence:
  [`testing-cutover.md`](testing-cutover.md),
  [`browser-playtests/playwright-scenario-manifest.md`](browser-playtests/playwright-scenario-manifest.md),
  and [`extending-the-system.md`](extending-the-system.md)

## Visual direction

Choose a product-normal, world-first direction: the settlement map remains the
first visual object, and LAI panels behave like practical field ledgers placed
around it. The UI should feel like paper, wood, stone, olive fabric, and rusted
iron in a forest settlement, not a futuristic dashboard.

Design rationale:

- The player is supervising a living settlement, so the world canvas must stay
  readable behind every planning/care/progression surface.
- The new LAI surfaces are operational tools, not marketing pages. They need
  predictable navigation, dense but scan-friendly rows, and clear action states.
- The existing client already uses parchment/ink colors and board-game glyphs;
  LAI.28-31 should refine that language instead of replacing it.

Mandatory style rules:

- No hero blocks, KPI-card grids, fake charts, decorative progress widgets, glass
  panels, glow, blur, blue-purple gradients, pill spam, or theatrical dashboard
  language.
- Panels use restrained 8-12px radius. The existing `UI_RADIUS` is 6px; the
  production owner can either keep legacy panels unchanged and use an 8px LAI
  panel token, or move the shared token deliberately in one client-root visual
  pass.
- Surfaces use solid paper/wood fills, 1-2px borders, limited shadow/depth, and
  clear section headers.
- Typography hierarchy: 16px panel titles, 13-14px section labels, 12-13px body,
  11-12px metadata. Avoid negative letter spacing and avoid uppercase eyebrow
  labels except for existing compact shortcut text.
- Use one accent family for primary actions: rust/oxide. Use olive for
  selected/positive, stone for neutral, and muted red for destructive/blocking.
- Map overlays are minimal semantic shapes and labels; they must not obscure
  cats, selected building interiors, or route endpoints.

Suggested palette tokens:

| Token | Role | Color direction |
| --- | --- | --- |
| `LAI_PAPER` | panel body | warm paper, close to existing `UI_BG` |
| `LAI_PAPER_DARK` | title bands and active tabs | dark wood/bark |
| `LAI_INK` | primary text | dark brown/ink |
| `LAI_MUTED` | metadata | muted taupe |
| `LAI_BORDER` | panel separators | translucent wood/stone |
| `LAI_RUST` | primary action/accent | oxide/rust |
| `LAI_OLIVE` | selected/healthy/available | muted olive |
| `LAI_STONE` | inactive/disabled | warm grey stone |
| `LAI_DANGER` | destructive/error | low-saturation red |
| `LAI_WATER` | water task marker only | existing water blue, toned down |

## Current client shape

`crates/cat-client/src/lib.rs` is currently a large Bevy client root. It owns
WebSocket polling, `WorldSnapshot` storage, the top-down world renderer, command
dock, HUD, inspectors, officers/orders panels, goods/trader panels, tooltip
logic, village selection, and action sending. `research_ui.rs` is an existing
large full-page research ledger, and `station_layout.rs` owns building visuals.

The LAI.28-31 production owner should not keep adding all behavior directly to
`lib.rs`. Use focused modules and only register resources/systems/plugins from
`lib.rs`. Because the current red tests source-scan `lib.rs`, actual plugin
registration names, module names, and exported constants must appear there as
real code, not fake marker comments.

## Information architecture

Use a stable four-region structure:

1. World canvas: always primary. It owns terrain, cats, buildings, routes, and
   LAI.29 task markers.
2. Left status column: compact colony status, event log, selected village, and
   visible global feedback.
3. Right inspector lane: selected cat/building/care/task details. Cat Care
   replaces the legacy cat inspector when LAI.30 snapshot fields exist.
4. Bottom command dock: stable categories and primary action entry points.

Add one new top-level "Council" workspace opened from the bottom dock or `P`.
It has normal tabs, not pill chips:

- `Plans`: LAI.28 top-eight plans, standing orders, officer reports.
- `Tasks`: a compact list of visible task rows tied to LAI.29 map markers.
- `Care`: LAI.30 selected cat care panel and eligible controls.
- `Progress`: LAI.31 Shrine, Favor, research, boosts, diplomacy, and trade.

The Council should be a single wide ledger panel in the clear center lane,
around 720-860px desktop width, with tabs as a simple top border/underline.
Avoid stacking separate floating cards for each feature. Reuse the right
inspector lane for selected row details rather than opening nested cards.

## Component and state map

| Slice | Components/resources | Snapshot input | Action output | State and refresh behavior |
| --- | --- | --- | --- | --- |
| Shared shell | `LeaderAiUiPlugin`, `CouncilPanelRoot`, `CouncilTab`, `CouncilSelection`, `LeaderAiSnapshotView`, `LeaderAiActionFeedback` | `LeaderAiSnapshotEnvelope` after LAI.24 | Routes to slice builders below | Stores selected tab, selected row IDs, scroll/focus anchors, pending action IDs, and update-required/loading/error states. Disposable; server snapshot is authority. |
| LAI.28 Plans | `PlansPanelPlugin`, `PlansPanelRoot`, `PlanRowStableId`, `StandingOrdersPanel`, `OfficerReportPanel` | plans, officer requests, report estimates, regen report gate, expected versions | `LeaderAiPlanNudgeAction`, `LeaderAiStandingOrderAction`, officer controls | Top eight rows preserve server order. Removed rows despawn controls. Stale refresh preserves selected row/draft only if row still exists. |
| LAI.29 Markers | `VisibleTaskMarkerPlugin`, `TaskMarkerEntity`, `TaskSnapshotIdMarkerKey`, `StrictSiteRefMarkerResolver` | `VisibleTaskSnapshot` and strict `SiteRefSnapshot` variants | Marker focus only; actions belong to owning panels | Dedupe/update/despawn by task ID, marker kind, semantic site/stage, and cell index. No local marker from cat destination. |
| LAI.30 Cat Care | `CatCarePanelPlugin`, `CatCarePanelRoot`, `CatCareStableCatId`, `CatAnatomyPanel`, `CatProstheticPanel` | cat care snapshot, anatomy, stress, injury, treatment, prosthetic, active task refs | `build_cat_care_action_envelope` | Selected cat persists across refresh when visible; removed/dead cat clears safely. Drafts survive stale refresh only when version hint still applies. |
| LAI.31 Progress | `ProgressionPanelPlugin`, `ShrineOfferingPanel`, `FavorLedgerSummaryPanel`, `ResearchFrontierPanel`, `DivineBoostPanel`, `DiplomacyPanel`, `TradeContractsPanel` | Shrine/Favor/research/boost/diplomacy/trade snapshots | `build_progression_action_envelope` | Selected progression tab/row persists on refresh. Exact Favor displays from ledger only; optimistic local balance is forbidden. |
| Shared actions | `LeaderAiActionEnvelopeBuilder`, `StableIdempotencyId`, `ExpectedVersionBundle`, `ActionConflictRefreshHandler` | selected snapshot versions and authenticated player state | LAI.25 envelopes | Button press creates one pending action. Accepted, duplicate, stale, unauthorized, update-required, malformed, insufficient-Favor, and route-blocked results render bounded feedback. |

## LAI.28 Plans and officer reports

Layout:

- Main panel: two-column ledger. Left column is top-eight plan rows; right column
  shows selected plan details, officer report details, and standing order editor.
- Plan rows show lifecycle, responsible actor, domain, visible dependency count,
  bounded rationale, confidence/range/age, and provenance count.
- Use quiet row separators and one selected-row olive border. Avoid badges for
  every field; use compact labels and aligned values.

Controls:

- Move up/down use small arrow buttons with accessible labels, not text-only
  pills. The action delta is exactly +1500/-1500 basis points.
- Dismiss is a muted destructive button with a disabled reason when emergency
  or current epoch rules block it.
- Standing orders use a normal form: domain picker, priority/bias controls,
  expiry, and create/update/remove buttons. Administration slot use is a compact
  text meter, not a decorative progress chart.

Privacy:

- Regeneration below effective report level 4 renders
  `RegenerationUnavailableBelowReportLevel4`.
- Level 4+ renders report-derived ranges and provenance only.
- The UI must not infer hidden stock, hidden regeneration, omitted planner
  candidates, private officer notes, or another colony's plans.

Accessibility/locators:

- `ACCESSIBLE_PLANS_PANEL_LABEL`
- `ACCESSIBLE_STANDING_ORDERS_PANEL_LABEL`
- `PLAN_ROW_TEST_ID_PREFIX`
- `STANDING_ORDER_ROW_TEST_ID_PREFIX`
- `PLAN_CONTROL_TEST_ID_PREFIX`
- `OFFICER_REPORT_TEST_ID_PREFIX`
- `VISIBLE_BROWSER_CHECKPOINT_PLANS_TOP_EIGHT`

## LAI.29 world task footprints

Renderer rule: markers come only from `VisibleTaskSnapshot`. Cat destinations,
job names, current animation targets, route guesses, radial fallback points, and
screen-space positions cannot create or keep a marker.

Marker visual language:

- Objective footprint: low-opacity olive/stone fill with a single 1px outline.
- Work slot: small square/chevron marker in rust.
- Endpoint: stable stone/rust pin marker distinct from work slot.
- Route/contact: thin dotted line or small stepping marks, never the same symbol
  as endpoint.
- Blocked/redacted/missing site: no world entity. Show the bounded reason only
  in Plans/Tasks rows.

Hard mappings:

- Hunt renders `HuntObjectiveCaveOrSourceMarker` for the actual revealed
  reachable cave/source identity from the snapshot.
- Fetch Water renders `FetchWaterSourceMarker`,
  `FetchWaterDryBankWorkMarker`, and
  `FetchWaterPinnedDeliveryEndpointMarker` as three separate facts.
- Workshop renders `WorkshopObjectiveNineRowMajorCells`: exactly nine 3x3
  row-major cells from the snapshot, plus distinct work-slot and delivery
  endpoint markers.
- Logging/tree tasks render all six canonical cells when present.

Dedupe/despawn:

- Key by `(task_id, marker_kind, semantic_site_id, stage, cell_index)`.
- Same key updates in place; absent key despawns.
- Coincident markers dedupe only when semantic site and stage match.
- Selected colony changes clear private/stale markers before new markers spawn.

Accessibility/locators:

- `TASK_MARKER_OBJECTIVE_TEST_ID`
- `TASK_MARKER_WORK_SLOT_TEST_ID`
- `TASK_MARKER_ENDPOINT_TEST_ID`
- `TASK_MARKER_CELL_TEST_ID`
- `ACCESSIBLE_TASK_OBJECTIVE_LABEL`
- `ACCESSIBLE_TASK_WORK_SLOT_LABEL`
- `ACCESSIBLE_TASK_ENDPOINT_LABEL`
- `PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST`
- `VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT`
- `VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER`
- `VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE`
- `VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION`

## LAI.30 cat care

Cat Care replaces the current compact cat inspector for authorized LAI.30 data,
but it should reuse the right inspector lane and selection behavior. It is a
work surface, not a character sheet popover.

Sections:

- Identity and capability: stable cat ID, name, migrated innate attributes,
  learned skills, office experience, personality axes, acquired traits.
- Readiness: stress/recovery, refusal state, willingness reasons, bounded
  eligibility/block reasons.
- Anatomy: four paws, two eyes, tail in fixed order; each body part has compact
  state text and color, not oversized icon tiles.
- Injury/treatment: injury state, treatment status, hours remaining, active task
  refs, site refs, cargo refs, patient/medic/fitter refs.
- Prosthetics: fitted item ID, side/type, restoration percent, durability hours,
  wear, adaptation, cap, repair status.

Controls:

- Treatment, consent/refusal, prosthetic fit/remove/repair controls sit at the
  bottom of the inspector and use authenticated expected-version/idempotent
  actions.
- Disabled controls remain visible with `CatCareControlDisabledReason`.
- Typed feedback appears in the shared feedback strip, not as modal spam.

Privacy and conservation:

- Display only authorized snapshot projection. No client recomputation from
  hidden needs, hidden treatment truth, hidden regeneration, or private plans.
- Prosthetic and cargo IDs are rendered exactly when report-safe and never
  synthesized in client state.
- Multi-colony selection filters cat care rows before selection/controls render.

Accessibility/locators:

- `ACCESSIBLE_CAT_CARE_PANEL_LABEL`
- `CAT_CARE_PANEL_TEST_ID_PREFIX`
- `CAT_CARE_BODY_PART_TEST_ID_PREFIX`
- `CAT_CARE_CONTROL_TEST_ID_PREFIX`
- `CAT_CARE_TASK_REF_TEST_ID_PREFIX`
- `PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST`
- `VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL`
- `VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC`
- `VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY`

## LAI.31 progression

Progression is one tab with sub-sections in a ledger stack. Do not make a grid
of KPI cards. Use row groups with clear labels, exact IDs where required, and
small action controls.

Shrine and Favor:

- `ShrineOfferingPanel` shows endless offering status, package, belief rationale,
  provenance, source/haul/ritual stage, cargo disposition, pinned Shrine endpoint,
  omission/block reason.
- `FavorLedgerSummaryPanel` shows exact micro-Favor balance, version, and event
  rows from `FavorLedgerSnapshot` only. No mirrored currency, no inventory cargo
  alias, no optimistic local balance.

Research and boosts:

- `ResearchFrontierPanel` shows 531-study manifest coverage, ready frontier,
  prerequisites, automatic seven-day quota used/limit/window, Insight,
  preparation, scholar reassignment, committed prices, and the player 25%
  preparation discount.
- `DivineBoostPanel` shows four controls only: Bountiful Labor, Fleet Paws,
  Inspired Work, Restorative Grace. Each row shows cost, duration picker, effect
  stage, start/expiry, and same-type disabled reason.
- Leaders/officers do not get boost activation controls.

Diplomacy and trade:

- `DiplomacyPanel` shows relationship/consent state, alliance approval, immediate
  block, expected diplomacy version, and bounded conflict feedback.
- `TradeContractsPanel` shows proposal valuation report refs, confidence, escrow
  summary, route endpoint, cargo stage, recovery state, consent-required accept/
  reject, and route-block feedback.
- Two-session flows must keep each colony's private plans, beliefs, stock, auth
  material, and private route danger absent.

Accessibility/locators:

- `ACCESSIBLE_SHRINE_OFFERING_PANEL_LABEL`
- `ACCESSIBLE_FAVOR_LEDGER_PANEL_LABEL`
- `ACCESSIBLE_RESEARCH_FRONTIER_PANEL_LABEL`
- `ACCESSIBLE_DIVINE_BOOST_PANEL_LABEL`
- `ACCESSIBLE_DIPLOMACY_PANEL_LABEL`
- `ACCESSIBLE_TRADE_CONTRACTS_PANEL_LABEL`
- `PROGRESSION_ROW_TEST_ID_PREFIX`
- `VISIBLE_BROWSER_CHECKPOINT_LAI31_OFFERING_RESTART`
- `VISIBLE_BROWSER_CHECKPOINT_LAI31_RESEARCH_BOOST`
- `VISIBLE_BROWSER_CHECKPOINT_LAI31_DIPLOMACY_TRADE`
- `PLAYWRIGHT_PROGRESSIONS_NO_DOM_STATE_INJECTION`

## Input, focus, and accessibility

Keyboard:

- Preserve existing camera controls, `O` Officers, `P` Orders/Council, `M` Map,
  `U` Research, `Esc` close, and `R` camera reset.
- Add tab traversal inside Council panels. Arrow keys move within visible row
  lists when focus is in a list. `Enter` activates the focused primary action.
  `Esc` closes transient dropdowns before closing the panel.
- World controls are disabled only while focus is in text input, a modal, or a
  visible `WorldInputBlocker`.

Pointer:

- Left-click selects world objects/markers in Inspect mode.
- Right-click keeps building inspection behavior.
- Marker hover tooltips are report-safe and never include hidden stock,
  regeneration below L4, private beliefs, private plans, or auth material.

Accessibility:

- Add Bevy `Name`/accessibility-compatible labels where available for panel
  roots, rows, controls, and marker entities.
- Every test ID must be stable and constructed from authoritative snapshot IDs,
  not display text, row index alone, or screen position.
- Disabled controls expose a reason label next to the control, not only color.
- Focus restoration uses stable IDs after refresh; if the entity disappeared,
  focus moves to the nearest valid parent panel and stale controls despawn.

## Native and WASM responsive behavior

Current native startup is 1024x768, and WASM uses `fit_canvas_to_parent`. LAI UI
must be verified at native 1024x768, 1280x800, 1440x900, and browser/mobile-ish
canvas widths.

Rules:

- The world remains visible at every supported size. Panels occupy reserved
  lanes or a single center sheet; they do not cover the entire map except the
  existing full research tree view.
- At widths above `NARROW_LAYOUT_MAX_WIDTH`, use left status, center Council,
  right inspector, bottom dock.
- At narrow widths, collapse Council to a bottom sheet above the command dock and
  keep the right inspector as a single stacked sheet. Hide nonessential columns
  inside rows, not whole action controls.
- Text must fit in buttons and rows. Long IDs use middle truncation in visual
  text but keep the full ID in the accessible/test ID.
- Map markers keep stable world positions and supported zoom behavior; viewport
  culling cannot re-key entities by screen position.
- WASM loading, reconnect, update-required, and incompatible protocol states
  must be visible without requiring DevTools.

## Loading, stale, update-required, empty, and error states

Use a shared feedback model:

- Loading: "Waiting for snapshot" or "Reconnecting" in the connection/status
  strip; skeleton rows are not needed.
- Empty: concise row text such as no visible plans, no active care work, no
  trade contracts, no visible task markers. Empty states do not explain product
  features.
- Pending: disable the exact pressed control, show action ID/status inline, and
  leave unrelated controls usable.
- Accepted: update from the next authoritative snapshot; do not fabricate
  optimistic local state.
- Duplicate replay: show the original bounded result.
- Stale/version mismatch: preserve selected tab/row/draft when still valid,
  refresh from snapshot, and disable removed controls.
- Update required: block mutating controls, show a persistent plain banner, and
  keep report-safe read-only snapshot if compatible.
- Unauthorized/malformed/foreign: opaque bounded feedback only.
- Network failure: preserve last report-safe snapshot visually but mark controls
  disabled until reconnect.

## Playwright and visible-browser checkpoints

The implementation must satisfy both Playwright and independent visible-browser
evidence. Do not use DOM/state injection, private endpoints, auth bypass,
manufactured inventory/Favor, synthetic snapshots, or undocumented time skips.

Checkpoint coverage:

- Startup/console/network: all panel labels and locator manifests resolve.
- Workshop: nine row-major objective cells plus work slot and endpoint.
- Hunt/Water: actual cave/source and water source/bank/endpoint.
- Plans/officers/regeneration secrecy: top eight, officer reports, no exact
  below-L4 regeneration anywhere.
- Shrine/Favor/research/boost: exact Favor ledger, offering stages, research
  frontier/quota, boost cost/duration/expiry.
- Cat Care: selected cat, anatomy, injury/treatment, prosthetic, stale privacy.
- Diplomacy/trade two sessions: relationship consent, contract, escrow, cargo,
  route, recovery, multi-colony privacy.
- Save/restart: stable row/marker/control IDs and selected context.
- Stale-action/reload: duplicate replay, stale conflict, refresh, selector
  preservation.

Every checkpoint records accessible labels/IDs, screenshot, console/network
state, selected colony, authoritative row/task/site/cargo/action IDs, expected
versions, tick, and restart linkage when relevant.

## Smallest production ownership slices

1. Shared client shell:
   add `crates/cat-client/src/leader_ai_ui.rs` with common Council resources,
   tabs, feedback, action-envelope helper boundaries, accessibility/test-ID
   helpers, and visual tokens. `lib.rs` only registers `LeaderAiUiPlugin` and
   real marker constants/names needed by current red source scans.
2. LAI.28:
   add `crates/cat-client/src/plans_ui.rs`; own plans, standing orders, officer
   reports, regen gate display, nudge/dismiss/order actions, stale refresh.
3. LAI.29:
   add `crates/cat-client/src/task_markers.rs`; own strict SiteRef resolver,
   world marker entities, dedupe/despawn, tooltips, zoom/viewport, marker
   accessibility. Touch `lib.rs` only for plugin registration and system order.
4. LAI.30:
   add `crates/cat-client/src/cat_care_ui.rs`; own care panel, anatomy,
   stress/refusal, prosthetics, care actions, selected-cat preservation.
5. LAI.31:
   add `crates/cat-client/src/progression_ui.rs`; own Shrine/Favor/research/
   boost/diplomacy/trade surfaces and actions. Reuse/refactor `research_ui.rs`
   only deliberately; do not duplicate the 531-study rendering model.
6. Protocol/action adapter:
   add a minimal client-side action builder module only after LAI.25 DTOs exist.
   It must build typed envelopes from stable IDs and expected versions, never
   from display labels or inferred coordinates.

## Future workshop/resource/action extension steps

For every new Workshop, resource, task, or player action:

1. Add protocol snapshot/action fields and expected versions first.
2. Add server authorization/redaction/idempotency tests before client controls.
3. Add one focused client module or extend the owning one; do not branch direct
   legacy `ClientAction` paths for LAI actions.
4. Define stable component names, accessible labels, and test IDs using
   authoritative IDs.
5. Render full objective/work/endpoint/route data from snapshots; no fallback
   marker from cat movement or screen position.
6. Add disabled, pending, accepted, duplicate, stale, update-required,
   unauthorized, malformed, and empty states.
7. Add Playwright and visible-browser checkpoint coverage with screenshots,
   accessibility trees, console/network evidence, and privacy token scans.
8. Re-run focused UI tests, marker cardinality tests, native framebuffer checks,
   WASM smoke where affected, rustfmt, Clippy, and whitespace checks.

## Anti-pattern checklist for production review

- First viewport is the world, not a dashboard hero.
- No glassmorphism, glow, blur haze, glossy cards, fake charts, or KPI grid.
- No pill-tab/button/badge system; tabs are plain, buttons are rectangular, and
  badges are sparse and functional.
- No in-app text that describes how the UI works unless it is an actual status,
  label, or bounded reason.
- No local recomputation of hidden truth, hidden regeneration, or private colony
  state.
- No stale controls that can mutate a removed row.
- No marker if the snapshot does not authorize the exact site.
- No client-side optimistic Favor, research, cargo, prosthetic, diplomacy, or
  trade mutation.
