# cat-client reference — salvaged Bevy spike

`spike-bevy-0.19.rs` + `spike-Cargo.toml` are the **verified Bevy 0.19** render
prototype (from the retired `engine/bevy-port` branch / worktree). They are the
**starting point for Phase 9** (client render + UI), not compiled into the crate.

What the spike already provides to lift into `cat-client`:
- Isometric projection + depth sort + fog dimming + path/road autotiling.
- Camera controls (pan / wheel-zoom / reset), tool modes, tile/cat picking.
- Cat sprite-atlas animation (8 dirs × 4 frames + work-spin) and role rings.
- **BRP wired**: `RemotePlugin` + `RemoteHttpPlugin` on `:15702`; `Cat`/`Role`/
  `Job`/`CatCoat` are `Reflect` components. Verified: `bevy_brp_mcp` `world_query`
  returns the live cats through the MCP.

What it is NOT: its colony sim is a throwaway toy. Real behaviour comes from
`cat-sim` over the network via `cat-protocol`. In P9 we also add
`bevy_brp_extras` (`BrpExtrasPlugin`) so `brp_status` + screenshot/input MCP tools
go green.

> Note: the old `README.engine.md` on `engine/bevy-port` is stale (it still says
> "Bevy 0.14 / MCP skipped"). This file supersedes it.
