# LAI.47 Static Protocol-Cutover Inventory

Recorded: 2026-07-25

This inventory is a read-only checkpoint taken while LAI.46 owns the simulation hot roots. It
does not claim that LAI.47 is complete and it does not replace either locked implementation plan
or the append-only board. No Cargo, test, build, formatter, Clippy, server, client, browser, or
image-generation command was run for this checkpoint.

## Canonical wire foundation already present

- `cat_protocol::PROTOCOL_VERSION` is `3`.
- `cat_protocol::lai64::CANONICAL_SNAPSHOT_SCHEMA_VERSION` is `2`.
- `cat_protocol::lai64::CANONICAL_ACTION_SCHEMA_VERSION` is `2`.
- `CanonicalSnapshotEnvelope` and `CanonicalActionEnvelope` perform header-first protocol/schema
  rejection before accepting the nested payload.
- The canonical snapshot already has bounded, ordered types for plans, officer requests, standing
  orders, physical tasks, cats, jobs, homes, governance, Notes/Void research, construction,
  storage, the Hole, divine state, diplomacy, quality lots, exact items, typed food, Hunting,
  rare materials, augmentations, fixtures, Cookhouse batches, Fishing Huts, visual state, events,
  and diagnostics.
- Canonical physical tasks already carry a typed site, full footprint, work sites, delivery site,
  route, cargo, reservations, workers, refusals, anatomy requirements, and bounded blockers.
- Canonical validation already checks the Hole's complete 5×5 footprint and central 3×3 work
  footprint, task/site combinations, ordered identities, bounded routes and cargo, report-safe
  regeneration, selected-colony partitioning, and exact action version lanes.
- The God action union is the newer restricted authority surface rather than the legacy
  placement/production action family.

## Live dual-path evidence that still prevents acceptance

The canonical DTO family exists, but it is not yet the only production protocol.

1. `crates/cat-protocol/src/lib.rs` still compiles and publicly re-exports
   `lai24_snapshot::*` and `lai25_action::*` beside `lai64::*`.
2. The large legacy `WorldSnapshot`/`ClientAction` family remains in the protocol root.
3. `lai24_snapshot.rs` still names and serializes Shrine, Favor, automatic-research quota, and
   Insight DTOs.
4. `lai25_action.rs` still names Favor-funded research/boost actions and broad legacy mutation
   authority.
5. `cat-server/src/main.rs` imports both the canonical boundary and
   `LeaderAiActionEnvelope`; multiple live handlers still accept or construct the latter.
6. `cat-server/src/leader_ai_action_routing.rs` remains a complete legacy action/snapshot route.
7. `cat-client/src/leader_ai_canonical_live.rs` is canonical, while
   `cat-client/src/leader_ai_live.rs` still decodes and queues the legacy envelopes.
8. The client root and the older Plans, Cat Care, Progression, interaction, and live-render paths
   still construct or consume `LeaderAiActionEnvelope`.
9. Historical protocol/server/client tests still compile against the legacy public exports. They
   must be dispositioned with their production owners; deleting only the tests would not prove a
   single live protocol.

Therefore a green canonical DTO test alone cannot prove LAI.47, LAI.52, LAI.64, LAI.65, or LAI.70
complete.

## Ordered ownership for the remaining cutover

### LAI.46

- Finish authoritative simulation task geometry, receipts, and report-safe physical state first.
- Supply stable, bounded simulation values that protocol projection can serialize without
  reconstructing hidden state.

### LAI.47 / LAI.64 protocol root

- Reconcile every final LAI.46 field into `lai64` with one stable type and one ordering rule.
- Preserve protocol v3 and snapshot/action schema v2 header-first rejection.
- Keep God actions limited to the explicit plan-authorized mutations.
- Reject unknown, malformed, future, duplicate, unordered, over-bound, and wrong-partition input
  before mutation.
- Do not add compatibility aliases for Shrine, Favor, Blessings, Insight, generic food, legacy
  research counts, or generic/fallback task markers.
- Remove the legacy public exports only in the coordinated cutover where every production
  dependent has moved; an intermediate private compatibility adapter is not acceptance.

### LAI.48 / LAI.65 server and persistence boundary

- Project canonical snapshots directly from authoritative simulation aggregates.
- Route only canonical authenticated actions with idempotency receipts and exact version lanes.
- Remove the live legacy server decoder/router after all canonical handlers exist.
- Persist separate aggregates and regenerate the pre-production fixture; do not store one opaque
  planner fingerprint as a substitute for the named state.

### LAI.50 / LAI.66–LAI.68 client boundary

- Make the canonical connection resource the only live snapshot/action transport.
- Convert remaining Plans, Cat Care, Progression, interaction, and live-render action builders to
  the restricted canonical action union or explicitly remove actions that the final plan no
  longer authorizes.
- Render task markers only from open canonical task geometry and never infer hidden sites,
  center-point shortcuts, stock, regeneration, or executor state.

### LAI.52 / LAI.70 final deletion

- Delete the unreachable legacy protocol modules, routes, client resource, compatibility tests,
  and stale documentation only after the canonical production path owns all consumers.
- Prove one protocol constant, one snapshot schema, one action schema, one server decoder, one
  client connection resource, and one report-safe projection.

## LAI.47 worker handoff boundary

The next protocol worker must start from the LAI.46 completion receipt, inspect current dependents,
and edit the protocol root only. It must not independently rewrite server, persistence, or client
roots. If a canonical field cannot be projected from authoritative LAI.46 state, the worker must
report that exact missing simulation interface instead of inventing a wire-only reconstruction.

