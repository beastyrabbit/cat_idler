# LAI.33A evidence schema

The LAI.33A execution owner stores immutable browser evidence under:

`docs/leader-ai-overhaul/evidence/lai33a/<commit>-seed-<seed>/`

The directory is append-only for a given run. If any required artifact is missing or a command must
be rerun from a different commit, seed, SQLite fixture, named URL, browser, or action sequence, start
a new evidence directory.

## Required Files

| Path | Contents |
|---|---|
| `manifest.json` | Machine-readable run manifest using the schema below. |
| `README.md` | Human summary with PASS/FAIL, warnings, failures, and reproduction commands. |
| `playwright/actions.jsonl` | Ordered Playwright actions, locators, assertions, URL, tick, and IDs. |
| `playwright/console.jsonl` | Console errors/warnings with checkpoint ID and disposition. |
| `playwright/network.jsonl` | Failed requests and WebSocket reconnect events. |
| `playwright/screenshots/*.png` | Before/after screenshots named by checkpoint. |
| `visible-browser/accessibility/*.json` | `orca-ide computer` accessibility trees for paired checkpoints. |
| `visible-browser/screenshots/*.png` | Visible-browser screenshots for paired checkpoints. |
| `visible-browser/devtools/*.json` | DevTools Console accessibility trees and screenshots with warning dispositions. |
| `server/*.log` | Redacted server/client startup, reconnect, and restart logs. |
| `sqlite/checksum.txt` | Fixture path and checksum before startup and after restart. |

## Manifest JSON

`manifest.json` is a single object:

```json
{
  "schema": "lai33a-browser-evidence-v1",
  "result": "PASS",
  "commit": "full git SHA",
  "dirtyDiffHash": "sha256 or null",
  "seed": 123456789,
  "fixture": {
    "sqlitePath": "relative/path/to.sqlite3",
    "sqliteSha256Before": "hex",
    "sqliteSha256After": "hex"
  },
  "versions": {
    "protocol": "reported protocol version",
    "persistence": "reported persistence version",
    "server": "cat-server package/version/commit",
    "client": "cat-client package/version/commit"
  },
  "portless": {
    "apiUrl": "https://leader-ai-api.localhost:1355",
    "webSocketUrl": "wss://leader-ai-api.localhost:1355/ws",
    "browserUrl": "https://leader-ai-browser.localhost:1355"
  },
  "browser": {
    "name": "Chromium",
    "version": "exact browser version",
    "os": "OS and version",
    "viewport": { "width": 1280, "height": 800 },
    "deviceScaleFactor": 1
  },
  "commands": {
    "server": "exact portless cat-server command",
    "client": "exact portless trunk command",
    "playwright": "exact Playwright command"
  },
  "runTicks": {
    "start": 0,
    "end": 0,
    "restartBefore": 0,
    "restartAfter": 0
  },
  "checkpoints": [
    {
      "id": "P00-startup-console-network",
      "journey": "startup",
      "result": "PASS",
      "tick": 0,
      "url": "https://leader-ai-browser.localhost:1355",
      "selectedColonyId": "colony-1",
      "authoritativeIds": ["world-id", "colony-1"],
      "playwrightTrace": "playwright/actions.jsonl#P00-startup-console-network",
      "screenshots": ["playwright/screenshots/P00-startup-console-network-before.png"],
      "visibleBrowser": {
        "accessibility": "visible-browser/accessibility/P00-startup-console-network.json",
        "screenshot": "visible-browser/screenshots/P00-startup-console-network.png",
        "devtools": "visible-browser/devtools/P00-startup-console-network.json"
      },
      "console": "PASS",
      "network": "PASS",
      "forbiddenValuesScan": "PASS",
      "cleanup": "none"
    }
  ],
  "warnings": [
    {
      "checkpoint": "P00-startup-console-network",
      "message": "warning text",
      "disposition": "classified benign or FAIL"
    }
  ],
  "failures": []
}
```

`result` is `PASS` only when every checkpoint passed in both Playwright and visible-browser layers,
all warnings have dispositions, no failed request is unclassified, replay artifacts are present, and
the forbidden-value scan is clean.

## Validation Rules

- `commit`, `seed`, SQLite checksums, Portless URLs, protocol/persistence versions, browser/viewport,
  action sequence, and checkpoint IDs must match the run described in
  [playwright-scenario-manifest.md](playwright-scenario-manifest.md).
- Every Playwright checkpoint must have exactly one paired visible-browser accessibility tree,
  screenshot, and DevTools Console capture.
- Evidence may read DOM/accessibility state, screenshots, console messages, network events, and
  downloaded artifacts. It must not mutate through JavaScript evaluation, direct private endpoints,
  auth bypass, manufactured inventory/Favor, or undocumented simulation time skips.
- Hidden stock, exact regeneration below effective report level 4, private beliefs/plans, auth
  material, another colony's private state, and unbounded error strings are forbidden in visible
  text, labels, tooltips, screenshots, accessibility trees, logs, console output, and conflict
  feedback.
