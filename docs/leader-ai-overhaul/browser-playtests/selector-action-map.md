# Browser Selector and Action Map

This map is the executable selector contract for `tests/browser-playtests/leader-ai-overhaul.spec.cjs`.
Selectors name only stable, report-safe UI identifiers. Entity IDs are discovered from the
authoritative snapshot rendered by the client; no test hard-codes a task, cat, site, cargo, or
colony ID.

## Run

Build the real web client and run the server against an existing deterministic SQLite fixture:

```sh
GAME_DB_PATH=/absolute/path/to/existing-fixture.db \
LAI_PLAYTEST_FIXTURE=/absolute/path/to/manifest.json \
scripts/leader-ai-browser-fixture.sh --run
```

Those variables are optional for the committed authoritative pair at
`fixtures/lai33a/manifest.json` and `fixtures/lai33a/authoritative.sqlite3`; `--check` validates the
manifest path and SHA-256 before launch. `--run` copies that database into a temporary runtime
directory so the committed source fixture is never mutated. In another shell, capture the named
route and run discovery or the acceptance suite:

```sh
export LAI_PLAYTEST_BROWSER_URL="https://leader-ai-browser.localhost:1355"
NODE_PATH=/usr/lib/node_modules playwright test --config=playwright.config.cjs --list
NODE_PATH=/usr/lib/node_modules playwright test --config=playwright.config.cjs
```

The launcher does not create, edit, or seed a database. It fails before launch if the fixture or
SQLite path is absent. `LAI_PLAYTEST_FIXTURE` must describe a fixture produced by the authoritative
campaign/persistence tooling; a hand-written snapshot is invalid.

## Selector rules

| Surface | Stable selector contract | Real action requirement |
|---|---|---|
| Startup/selection | `lai-colony:selected`, `lai-connection:status` | Selected colony comes from the authenticated snapshot and survives reload/reconnect. |
| Plans | `lai-ui:plans:panel`, `lai-ui:plans:control:move-up:<id>`, `:move-down:<id>`, `:dismiss:<id>` | Controls send signed LAI.25 envelopes with snapshot expected versions and deterministic idempotency IDs. |
| Standing orders | `lai-ui:standing-orders:control:create:new`, `:edit:<id>`, `:delete:<id>` | Inputs are bounded and authenticated; disabled authority states remain visible and typed. |
| Progression | `lai-ui:shrine:panel`, `lai-ui:progression:control:purchase:<id>`, `:activate:<kind>` | Favor debit, study, preparation, and boost actions use report-safe prices/stages and expected versions. |
| Cat care | `lai-ui:care:panel`, `lai-ui:cats:control:treat:<id>`, `:fit:<id>`, `:repair:<id>` | Treatment/prosthetic actions preserve item/cargo identity and never expose hidden regeneration. |
| Diplomacy/trade | `lai-ui:diplomacy:panel`, `lai-ui:diplomacy:control:propose:<pair>`, `lai-ui:trade:control:accept\|reject:<id>` | Two authenticated sessions prove consent and colony isolation; no private inventory leaks. |
| Spatial tasks | `lai-ui:tasks:task:<task>:site:<site>:objective`, `:work-slot`, `:endpoint`, and `:cell-<0..8>` plus report-safe role labels | Markers are derived only from `VisibleTaskSnapshot/SiteRef`; labels distinguish water source/bank and Workshop roles, which share the canonical marker kinds. |
| Feedback/recovery | `lai-feedback:action:<result>`, `lai-feedback:update-required`, `lai-ui:connection:control:reload` | Accepted/rejected/duplicate/stale and UPDATE_REQUIRED results are typed and bounded; reload uses the shipped visible control. |

The production entities carry AccessKit `Pane`, `Button`, `ListItem`, `Status`, and `Alert` nodes,
stable descriptions, names, disabled state, live-region state, and focus/click actions. Native winit
adapters expose that tree. `accesskit_winit 0.32.2` selects its null adapter for `wasm32`, so current
Chromium cannot expose those canvas nodes as DOM elements; the suite therefore falls back to real
pointer interaction at the fixed 1280x720 production layout. This is a documented production
fallback, not a hidden test surface. It captures each checkpoint and never reads or writes DOM
state. Browser screen-reader exposure beyond this fallback depends specifically on a web adapter in
the Bevy/AccessKit winit stack.

| Fallback checkpoint | Viewport point |
|---|---:|
| Selected colony | `(32,32)` |
| Plans panel / nudge / dismiss | `(168,48)` / `(64,132)` / `(64,202)` |
| Standing orders create / edit / delete | `(80,285)` / `(64,356)` / `(64,390)` |
| Research / boost | `(1024,324)` / `(1024,487)` |
| Cat Care treatment / prosthetic | `(710,442)` / `(710,486)` |
| Diplomacy / trade | `(80,604)` / `(1024,604)` |
| Hunt/Water task selection | `(888,340)` |
| Workshop footprint | `(888,360)` |
| Reconnect/update feedback | `(32,688)` |

These points are a production contract, not test-only controls; they must remain aligned with the
fixed world-first layout. If the actual rendered surface moves, update the production layout and
this table together, then rerun visible-browser screenshots. The suite must never downgrade to
generic coordinates, hidden DOM state, or fabricated snapshots.

## Action and evidence contract

Each click must be preceded by a visible, enabled control check and followed by typed feedback. The
executable harness records the pre-click snapshot count, observes the intended LAI.25 action,
matches its idempotency ID to an accepted/rejected/duplicate response, and waits for the following
LAI.24 snapshot before another mutation.
The suite may read visible text, accessible roles/labels, screenshots, console events, failed
requests, and WebSocket reconnect metadata. It must not call private endpoints, inspect DOM state
injection hooks, read auth/session material, mutate `window`, manufacture inventory/Favor, or skip
time outside a shipped control. Pair every Playwright checkpoint with the visible-browser
accessibility/screenshot/DevTools checkpoint from the scenario manifest.

## Adding a new surface

1. Add the selector to the owning LAI UI contract and Bevy accessibility bridge.
2. Add it here with the expected signed action, version, authority, and report-safe fields.
3. Add a manifest checkpoint with seed/preconditions, IDs/ticks, screenshot, restart linkage, and
   cleanup.
4. Add a browser test that discovers IDs from the authoritative UI and asserts the real control.
5. Run `playwright test --list` before services; run the suite only through named Portless routes.
