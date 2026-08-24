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
- **Raid banner** — an advancing warband keeps a persistent alarm banner on screen pointing at
  "Defend raid" [P]; minimap blips already existed.
- **Election visibility** — each newly opened election toasts once toward the Governance vote UI.
- **Event log capacity** — raised from four to six lanes.
- **Client WIP completion** — fixed the three red tests left by the building-picking/renderer
  slice: footprint-aware right-click hit-testing, per-cat walk-animation phase (no lockstep),
  and roofed residences no longer render a wooden station floor.
- **CI unblocked** — the serialized quality gates (quick / capped full suite on `cat-idler-heavy`
  / WASM transfer budget) plus nightly playtests and weekly coverage landed; the systemic
  "Build test archive once" failure is retired with the old workflow. Forgejo backlog cleared:
  PRs #19–#25 merged (rebase), stale issue #3 closed as implemented (`55cd0ba`), docs PR #24
  closed as landed, nine dead TS-era `codex/issue-*` branches deleted.

## Playability backlog (next up)

1. **Raid-in-progress world feedback** [S] — minimap blips + banner done; remaining: red screen
   vignette while raiders are adjacent to the gate.
2. **Production/consumption rates** [M] — diff consecutive snapshots to show "+2.1 food/h ·
   empty in 3h" in Stores; prerequisite for smart low-stock warnings.
3. **Staged onboarding checklist** [M] — contextual goals ("designate a gather zone ✓ … send a
   scout") driven by colony state, replacing the single static help card.
4. **Game-speed controls** [L] — per-colony server-side tick scaling + client pause/fast-forward.
5. **Transport layer UI** [L] — rail/dock/vehicle/route tools (`DesignateRail`, `BuildDock`,
   `BuildTransportVehicle`, `CreateTransportRoute`, `CancelTransportRoute` are sim-complete with
   zero client surface; purchasable rail/shipping research nodes are currently inert).
6. **Legacy idle layer decision** [M] — `ClientAction::Boost` / `PurchaseUpgrade` / six upgrade
   tracks are live on the wire but unreachable from the UI: either surface them or retire them.
7. **Audio feedback** [S] — raid horns, election chime, ambient loop; zero audio exists today.
8. **Distinct enemy kinds** [S/M] — fox/badger/bear sprites are already tracked
   (`docs/assets/SELECTION.md`); add an enemy-kind field to the protocol and render them so all
   raids stop using the single generic raider sheet.
9. **Remaining crafted-goods glyphs** [S] — replace the last generic `goods` fallbacks
   (`docs/assets/items_ui.md`).

## Known red: playtest scenario journeys (full suite only)

The `cat-server` real-socket scenario harness (nightly / full-suite tier) has deterministic
reds on `main`. The merge-gating quick profile is green; these are exhaustive-inventory
failures, several seeded only under `CAT_PLAYTEST_SEED_TIER=high-risk`. Fixture bugs found and
fixed in this pass: the field parcel now clears generated terrain, and the Captain fixture owns
the Barracks prerequisite plus an exclusive exact warrior. The shared haul bug is fixed (see
Done); what remains open is per-journey tails:

- **FIXED: station-output haul starvation** — HaulGatherSpot jobs carried a queue-time
  `ends_at` stamp and were force-completed by the generic due-jobs phase while the mover was
  still walking, so any handoff farther than one job-duration never delivered. Crops rotted in
  farm handoffs ("awaiting physical haulage" forever). Now excluded from deadline completion
  like CarryOffering; the crop-yield journey is green again.
- **Captain weapon last mile — root-caused, needs a design decision.** The physical journey
  is proven correct end-to-end (instrumented runs: ore→metal→weapon crafted, hauled, credited,
  and `Equipped { cat-28 }` within ~40 min sim time, stable for hours). What fails is
  *observation*: the socket privacy projection (`redact_exact_functional_equipment`) strips
  tool/weapon/armor identities unless the Accountant's ledger is exactly current at the instant
  a snapshot is built, and the leader director's constant hunting mutates food faster than any
  observer can catch an accurate window. Resolution options: (a) let the fixture world go quiet
  after the credit (suppress hunts), (b) expose exact equipment to the colony's controlling
  player whenever their own session caused the change, or (c) observe through a server-side
  attestation instead of the wire. Pick one, then the scenario goes green without weakening it.
  Fixture prerequisites landed meanwhile: Barracks (Captain office survival) and a staffed
  Accounting Tent (required for any exposure at all).
- **ReplantTree stump reachability — partially fixed.** The static A* reachability pre-filter
  on replant sites flapped while walls staged and the claimed ring grew, rejecting stumps the
  colony itself had just felled; it is removed in favour of logging's contract (movers route,
  validation doesn't path-probe). Residual: with the director's near-total employment fill,
  eager `RequestJob` worker assignment races single-tick idle windows ("No available worker"
  despite a just-idle cat). Options: queue player jobs unassigned until a worker frees up, or
  widen `select_best_cat` to traveling-but-taskless cats.
- **System-journey tails** (each its own investigation): poor-guidance runs lack bounded
  visible consequences; prosperous migration produces no physical arrival; an appointed
  Loremaster shows no owned automation effect; trader visit 2 restocks identically to visit 1;
  peer-shrine contact flag never sets after a completed scout exchange.

## Infrastructure / project health

- **First green CI run** — the new serialized gates are live; confirm the first full
  `cat-idler-heavy` suite passes on `main` and keep the nightly playtest schedule honest.
- **WASM bundle** — 9.1 MB gzip / ~40 PNG fetches; atlasing + asset manifest per
  `docs/migration/WASM.md` is the biggest web-playability lever. Large-colony wasm perf unmeasured.
- **Snapshot/persistence scaling (profiling-gated)** — dirty-tracked SQLite saves and delta WS
  snapshots are pre-scoped in `docs/reviews/RESOLUTION.md`; drive with profiling data first.
- **Deeper inter-village encounters** — visits/joint projects beyond summary contact + caravans
  (`docs/IMPLEMENTATION_AUDIT.md`). Optional breadth.
- **Hold-to-confirm for large offerings** — flagged in `docs/reviews/UI_UX_REVIEW.md`, never built.
