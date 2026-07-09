# Engine MCP servers

Model Context Protocol servers for the three engine spikes, so an AI agent can
inspect/drive a **running** game (not just edit source). Registered in the repo
root `.mcp.json` — Claude Code picks them up automatically for this project.

All three were installed and tested end-to-end against the real engines on
2026-07-09. See `docs/ENGINE_MCP_EVALUATION.md` for the results.

## bevy (`bevy_brp_mcp`, 45 tools)

- Rust binary: `cargo install bevy_brp_mcp` → `~/.cargo/bin/bevy_brp_mcp`
- Talks to a Bevy app over the **Bevy Remote Protocol** (BRP, JSON-RPC on
  `http://127.0.0.1:15702`). Requires Bevy **0.15+** with `RemotePlugin` +
  `RemoteHttpPlugin` (the `bevy_remote` feature). For input/screenshot tools,
  the app also needs the `bevy_brp_extras` plugin.
- The 0.14 spike has **no** BRP wiring yet — this is the concrete reason to do
  the Bevy version bump. A minimal 0.16 proof lives in
  `scratchpad/brp-test/` (headless app + curl queries).
- Tools: `world_query`, `world_spawn_entity`, `world_mutate_components`,
  `world_get/insert/remove_components`, `world_*_resources`, live `*_watch`
  subscriptions, `brp_extras_screenshot`, `brp_extras_send_keys`, mouse/gesture
  input, log capture, `brp_launch`/`brp_shutdown`.

## godot (`Coding-Solo/godot-mcp`, 14 tools)

- Node/TypeScript. Built to `godot-mcp/build/index.js`. Needs `GODOT_PATH`
  (here `/usr/bin/godot`, Godot 4.7).
- Editor/CLI oriented: `launch_editor`, `run_project`, `get_debug_output`,
  `stop_project`, `create_scene`, `add_node`, `load_sprite`, `save_scene`,
  `get_project_info`, `get_godot_version`, UID tools.
- Rebuild after pulling: `cd godot-mcp && npm install && npm run build`.

## love2d (`shayarnett/love2d-mcp`, 3 tools)

- Node/TypeScript. Built to `love2d-mcp/build/index.js`.
- Talks to a running LÖVE game over **TCP `localhost:12345`** via the
  `game/mcp_bridge.lua` module the game must embed (see that repo's README).
- Tools: `list_objects`, `get_object`, `run_lua` (arbitrary Lua in the live
  game context).
- Rebuild after pulling: `cd love2d-mcp && npm install && npm run build`.

## Notes

- `node_modules/` and `build/` are gitignored; run `npm install && npm run
  build` in each Node server after a fresh checkout.
- These are vendored clones (upstream repos), kept here so the project is
  self-contained. Update by `git pull` inside each dir + rebuild.
