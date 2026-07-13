> **Historical / superseded (P11 cutover, 2026-07-11).** This records the engine-selection
> spike that chose Bevy. It is not a current implementation plan. The shipped architecture is
> documented in `docs/ARCHITECTURE.md`; live gaps are tracked in
> `docs/IMPLEMENTATION_AUDIT.md`.

# Engine + MCP evaluation (2026-07-09)

Comparing the three engine spikes (`engine/bevy-port`, `engine/godot-port`,
`engine/love2d-port`) for a **standalone local build + server**, an optional
**more-performant browser build**, and **AI-agent-driven development**.

Every MCP server below was actually installed and exercised against a real
engine on this machine — not asserted from docs. Config lives in the repo-root
`.mcp.json`; servers are vendored under `mcp-servers/`.

## Render parity (consistent starting point)

All three render the same tracked isometric art at comparable fidelity once the
asset root is correct.

- **Bevy** initially rendered an empty map — the raw binary resolves
  `AssetPlugin.file_path = "."` against the exe dir (`target/debug/`), not the
  repo root. Fix: run with `BEVY_ASSET_ROOT=<worktree>` (or `cargo run`). With
  that, full terrain + village + 96 cats render (`artifacts/bevy-live-fixed.png`,
  `visible tiles 4378`).
- **Godot** rendered fine but self-reported **12 FPS @ 96 cats / 1598 tiles** in
  its own HUD — naive Node2D-per-cat, fixable with MultiMesh/servers, but the
  default idiom stalls at the game's baseline scale.
- **Love2D** textured map + 80+ cats (not re-captured this pass).

## MCP: tested, not assumed

| Engine | Server | Tools | What was verified live |
|---|---|---|---|
| Bevy | `bevy_brp_mcp` (Rust, `cargo install`) | **45** | The spike was **upgraded 0.14 → 0.19** and wired with `RemotePlugin`+`RemoteHttpPlugin`. Driving the actual `bevy_brp_mcp` server (v0.20.1), `world_query` returned **"Found 96 entities"** with live `energy`/`hunger`/`job` (ticking between reads = live world). Server also exposes `world_spawn_entity`, `world_mutate_components`, live `*_watch`, `brp_extras_*`. |
| Godot | `Coding-Solo/godot-mcp` (Node) | 14 | `get_godot_version` → `4.7.stable`; `get_project_info` introspected the real godot worktree (1 scene, 1 script, 153 assets). Also `launch_editor`, `run_project`, `get_debug_output`, `create_scene`, `add_node`, `save_scene`. |
| Love2D | `shayarnett/love2d-mcp` (Node) | 3 | Ran the example `love game/`; through the MCP server `list_objects` returned the 5 live balls with live x/y and `run_lua` executed arbitrary Lua in the running game (returned `5`). Bridge = TCP `:12345` via `mcp_bridge.lua`. |

### MCP verdict

Earlier claim that "Godot/Love2D have no real MCP story" was **wrong** — all
three have working servers. But they differ in kind:

- **Bevy BRP is the deepest live-world integration.** Because the ECS world is
  reflection-serializable, the agent gets *typed, structural* access — query by
  component, mutate components/resources, spawn/despawn, subscribe to change
  watches, screenshot, inject input. This is genuine closed-loop verification of
  a running sim. **Now wired: the spike is on Bevy 0.19 and `bevy_brp_mcp`'s
  `world_query` returns the live cats through the MCP layer.**
  - **BRP method rename gotcha (found by testing, not docs):** Bevy renamed BRP
    methods `bevy/*` → `world.*`. `bevy_brp_mcp 0.20.1` speaks `world.*`, so it
    only matches Bevy ~0.19; against the 0.16 build it returned
    `-32601 method not found`. Keep the app's Bevy version aligned with the MCP.
  - **Remaining:** `brp_status` reports "not responding" and the `brp_extras_*`
    input/screenshot tools need the app to also add `bevy_brp_extras`'s
    `BrpExtrasPlugin` (one dep + one plugin line). Core query/mutate works
    without it.
- **Godot MCP is editor/process automation**, not deep runtime state — drive the
  editor, run the project, scrape debug output, script scene edits. Useful, but
  the agent reads the game mostly through printed logs.
- **Love2D MCP is a thin, powerful escape hatch**: 3 tools, but `run_lua` means
  arbitrary live introspection/mutation. No types, no structure — you build any
  richer view yourself in Lua.

## Recommendation: Bevy

Unchanged after actually testing MCP — and now better supported. Bevy wins the
two axes that motivated the migration (ECS runtime performance; best WASM path
for a faster browser build) and, with BRP, gives the richest agent-driven
runtime introspection of the three. The Rust compiler is also a correctness gate
for the autonomous agent loops this project already runs.

### Status / next steps
1. ✅ **DONE** — Bevy spike upgraded **0.14 → 0.19** (clean build, rendering
   intact, 0 asset errors) with `RemotePlugin`+`RemoteHttpPlugin`; `Cat`/`Role`/
   `Job`/`CatCoat` are `Reflect`-registered and queryable. `bevy_brp_mcp`
   `world_query` verified returning 96 live cats. Diff: `Cargo.toml`,
   `Cargo.lock`, `src/main.rs` on `engine/bevy-port` (uncommitted).
2. Add `bevy_brp_extras` `BrpExtrasPlugin` to green `brp_status` and unlock the
   `brp_extras_*` screenshot/input tools (agent can then screenshot + drive the
   game via MCP).
3. Fix the asset-root default for the packaged standalone binary (don't depend
   on cwd / `BEVY_ASSET_ROOT`).
4. Decide **where the authoritative sim lives** — one shared Rust crate across
   native app / headless server / WASM, vs. keeping the TS worker as the server
   and making Bevy a thin render+input client. This outranks the engine choice.

## Reproduce
- Bevy BRP proof: `scratchpad/brp-test/` — `cargo run`, then
  `curl -X POST :15702 -d '{"jsonrpc":"2.0","id":1,"method":"bevy/query",...}'`.
- MCP servers: `mcp-servers/` (+ `mcp-servers/README.md`). Rebuild Node servers
  with `npm install && npm run build`.
