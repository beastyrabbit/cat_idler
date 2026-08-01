# Browser play-test contracts

This directory defines the replayable browser evidence contract and shipped browser acceptance
harness for LAI.33A. The harness starts only through the root launcher, runs against a temporary copy
of the authoritative SQLite fixture, and drives the real signed server/client controls.

## Files

- [playwright-scenario-manifest.md](playwright-scenario-manifest.md) defines the ordered Playwright
  journey and checkpoint contract.
- [evidence-schema.md](evidence-schema.md) defines the immutable evidence directory and manifest
  schema used by the later execution owner.
- [selector-action-map.md](selector-action-map.md) maps the in-repo Playwright selectors to real
  signed actions, launch inputs, AccessKit semantics, and the documented wasm canvas fallback.

These contracts and their executable suite extend, but do not replace, the release gate in
[testing-cutover.md](../testing-cutover.md#real-browser-acceptance-lai33a). The independently
operated visible-browser layer remains mandatory for every Playwright checkpoint.

## Extension Rules

When adding a new Workshop-like building, visible task kind, care/progression panel, action control,
or server-visible UI state:

1. Add or update stable accessible roles, labels, and test IDs in the owning LAI.28-LAI.31 UI
   contract before writing browser automation.
2. Add the new checkpoint to [playwright-scenario-manifest.md](playwright-scenario-manifest.md) with
   preconditions, seed, locator contract, permitted user action, expected authoritative IDs/ticks/
   state, report-safe assertions, forbidden hidden values, screenshot names, console/network rules,
   restart linkage, cleanup, and the paired visible-browser checkpoint.
3. Update [evidence-schema.md](evidence-schema.md) only when the evidence artifact shape changes.
   Additive fields must be bounded, deterministic, and replayable from a commit, seed, fixture, and
   named Portless URL.
4. Do not rely on DOM/state injection, private endpoints, auth bypass, manufactured inventory or
   Favor, hidden test hooks, or undocumented simulation time skips. If a scenario cannot be reached
   through shipped player controls and documented fixtures, block the card instead of weakening the
   evidence.
5. Preserve the one-to-one mapping between each Playwright checkpoint and a visible-browser
   accessibility tree, screenshot, and DevTools Console capture.

## In-repo harness

`playwright.config.cjs` and `tests/browser-playtests/leader-ai-overhaul.spec.cjs` are the package-free
Playwright harness. `scripts/leader-ai-browser-fixture.sh --check` validates the committed
authoritative manifest/SQLite pair (or an explicit pair) by path and SHA-256, while `--run` starts
the real `cat-server` and Trunk client through named Portless routes against a temporary copy.
Run `NODE_PATH=/usr/lib/node_modules playwright test --config=playwright.config.cjs --list` before
services. Full execution is required for acceptance; it prefers production AccessKit selectors and
uses the fixed-viewport canvas fallback documented in `selector-action-map.md` when Chromium cannot
observe the canvas tree. Missing authoritative fixture fields remain named blockers, not skips or
acceptance claims; fallback clicks never bypass the shipped action queue.

The executable order is P00 through P07:

- startup/authentication, current protocol, console/network, and regeneration secrecy;
- plan nudge/dismiss;
- standing-order create/edit/delete;
- Shrine/Favor/research and player-only boost;
- Cat Care treatment and prosthetic fitting;
- two authenticated production sessions for diplomacy and physical trade;
- authoritative cave Hunt, water source/bank/endpoint, and exact nine-cell Workshop semantics; and
- shipped reload/reconnect with preserved selected colony and current protocol.

Each mutating click waits for its LAI.25 frame, the matching typed action response, and a subsequent
authoritative LAI.24 snapshot. The suite has `workers: 1`, `fullyParallel: false`, no retry, no
service worker, and no direct API or state injection. P00 establishes one signed primary browser
session and P01–P07 reuse that same browser context, matching a real ordered player journey and
staying inside the production new-session abuse limit. P05 alone boots and closes a second signed
context. P05 has a 90-second bound because it boots that second WASM client; P07 has a 45-second
bound for the production reload.

The remaining browser-screen-reader dependency is precise: Bevy 0.19 currently reaches
`accesskit_winit 0.32.2`, whose `wasm32` target selects the null platform adapter. Native windows
receive the production AccessKit tree and browser users retain visible pointer/keyboard fallback,
but exposing the same semantic tree to Chromium requires upstream web-adapter support rather than
test-only DOM controls.
