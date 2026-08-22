# Improvement roadmap

Standing backlog for post-migration gameplay and playability work, assembled 2026-08-21 from a
full-project audit (docs/board scan, Forgejo issues/PRs, code-gap scan, playability review).
Ranking is by player impact; sizes are S (<1 day), M (~1–3 days), L (>3 days).

## Done in this pass

- **Zone removal UI** — painted avoid/gather zones can now be clicked in Inspect mode and removed
  via the existing "Remove designation" panel (`ClientAction::RemoveZone`), instead of waiting out
  the 30-minute TTL.
- **Proactive alerts** — new `push_snapshot_alerts` system toasts new raid/crisis/election/death
  events and starving/dehydrating cats (threshold 15, recovery latch at 30), so urgent state no
  longer hides in the passive event log. Reconnects do not replay old history.

## Playability backlog (next up)

1. **Raid-in-progress world feedback** [S] — red HUD banner + minimap blips while
   `colony.raiders` is non-empty; "Defend raid" currently sits buried in Orders.
2. **Election visibility** [S] — badge/toast when `colony.election` is open (vote UI exists but is
   never announced beyond one log line).
3. **Event log capacity** [S] — raise the 4-line cap, separate the alert lane from event history,
   click a line to zoom to its subject.
4. **Production/consumption rates** [M] — diff consecutive snapshots to show "+2.1 food/h ·
   empty in 3h" in Stores; prerequisite for smart low-stock warnings.
5. **Staged onboarding checklist** [M] — contextual goals ("designate a gather zone ✓ … send a
   scout") driven by colony state, replacing the single static help card.
6. **Game-speed controls** [L] — per-colony server-side tick scaling + client pause/fast-forward.
7. **Transport layer UI** [L] — rail/dock/vehicle/route tools (`DesignateRail`, `BuildDock`,
   `BuildTransportVehicle`, `CreateTransportRoute`, `CancelTransportRoute` are sim-complete with
   zero client surface; purchasable rail/shipping research nodes are currently inert).
8. **Legacy idle layer decision** [M] — `ClientAction::Boost` / `PurchaseUpgrade` / six upgrade
   tracks are live on the wire but unreachable from the UI: either surface them or retire them.
9. **Audio feedback** [S] — raid horns, election chime, ambient loop; zero audio exists today.
10. **Distinct enemy kinds** [S/M] — fox/badger/bear sprites are already tracked
    (`docs/assets/SELECTION.md`); add an enemy-kind field to the protocol and render them so all
    raids stop using the single generic raider sheet.
11. **Remaining crafted-goods glyphs** [S] — replace the last generic `goods` fallbacks
    (`docs/assets/items_ui.md`).

## Infrastructure / project health

- **Forgejo CI is broken systemically** — "Build test archive once" fails ~26 min even on
  `main`; fix the runner cap/timeout before merging PRs #19–#25 (real bugfixes, oldest first),
  then close stale issue #3 (bridges shipped on `main` in `55cd0ba`) and delete the nine dead
  TS-era `codex/issue-*` branches.
- **WASM bundle** — 9.1 MB gzip / ~40 PNG fetches; atlasing + asset manifest per
  `docs/migration/WASM.md` is the biggest web-playability lever. Large-colony wasm perf unmeasured.
- **Snapshot/persistence scaling (profiling-gated)** — dirty-tracked SQLite saves and delta WS
  snapshots are pre-scoped in `docs/reviews/RESOLUTION.md`; drive with profiling data first.
- **Deeper inter-village encounters** — visits/joint projects beyond summary contact + caravans
  (`docs/IMPLEMENTATION_AUDIT.md`). Optional breadth.
- **Hold-to-confirm for large offerings** — flagged in `docs/reviews/UI_UX_REVIEW.md`, never built.
- **Control rebind** — promised in the P15 board rollup, unverified in evidence rows: confirm or finish.

## Known pre-existing failures (prior WIP, not regressions from this file's work)

Three `cat-client` tests fail against the uncommitted building-picking/renderer rework already in
the tree: `full_footprint_right_click_hits_every_building_tile`,
`moving_cat_entities_do_not_share_the_same_actual_animation_frame`,
`den_renderer_has_a_roof_but_no_wooden_floor_tiles`. None exist at HEAD; they belong to that
in-flight slice and should be fixed as part of it.
