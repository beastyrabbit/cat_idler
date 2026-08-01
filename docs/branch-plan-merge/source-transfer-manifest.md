# Source-Branch Knowledge-Transfer Manifest

**Snapshot inspected:** 2026-07-25

**Target worktree:** `feature-new-leader-ai`

**Target head at inspection:** `25b5b27943d31f420984bde80faae57be1041706`

**Sources:** `the-shrine-upgrade` and `bug-gui-design`

**Related intent audit:** [thread-qa-audit.md](thread-qa-audit.md)

## Why this is a semantic transfer rather than a Git merge

The source branches contain functionality and design knowledge worth preserving, but neither can
safely own the target architecture:

- `the-shrine-upgrade` has the same committed head as the target inspection head and carries its
  implementation as a large uncommitted working-tree delta.
- `bug-gui-design` diverged at merge base
  `dfb5600da913e6e0db6bc55e733a6205cba320a6`, has four source-only commits, and also has a large
  dirty delta.
- Both sources and the target touch `world_tick`, actions, protocol/server roots, client roots,
  research catalogs, and tests.
- Both source designs predate some of the final report-limited Leader/Hole and two-lane research
  decisions.

A wholesale merge or cherry-pick would therefore select old authority accidentally and create
conflict-driven behavior loss. “Do not merge” does **not** mean “ignore the code.” It means:

1. freeze and identify the source state;
2. read the relevant source implementation, tests, docs, and assets;
3. extract its observable behavior, constants, state transitions, visuals, and failure cases;
4. reconcile conflicts against the exact plans and Q&A audit;
5. port bounded leaves or rewrite them against the target architecture;
6. prove the retained functionality with source-derived tests and end-to-end evidence;
7. record a disposition for every source path.

## Frozen source snapshots

The digest algorithm is SHA-256 over sorted lines of
`<working-file-sha256><two spaces><relative path>`. It detects source drift without copying dirty
hot roots into the target.

These digests identify the inspected state; they are not a backup. Do not clean, reset, delete,
archive, or reuse either source worktree while transfer receipts remain open. Before any such
operation, create a recoverable source-side commit/archive with explicit user authorization,
record its location and digest here, and verify that it includes untracked files and binary assets.
If a source digest changes legitimately, retain the old snapshot record and audit the delta rather
than silently replacing the value.

| Source | Git state | Changed working files | Snapshot digest |
|---|---|---:|---|
| `the-shrine-upgrade` | Head equals target inspection head; all source work is dirty/untracked | 13 tracked modifications + 69 untracked = 82; 53 are image assets | `b1bcc2433d29d23f10167de07465f4c39a7164bc782d9ec292fa8cafe3a4bdaf` |
| `bug-gui-design` committed source delta | Four commits after merge base; 26 paths | 26 committed paths | Source-only tree digest `4c47582234a231ae8c4818ed1ed6111f63903cd3df515faac7ea81d07990a85a`; binary diff digest `00bbec04c45e3725b8267bb4928f8f4cfb914026efe8d1647a2ac2378acf6812` |
| `bug-gui-design` dirty delta | Head `748db74a4d94b5b23d8be7fcd4642f4a63d45e94` | 15 tracked modifications + 5 untracked = 20 | `73ac0c009ec517e49b143819e9f7a809f95f18442fa8bff5c6e86cfbdba7e436` |

Tracked binary-diff digests are
`7edc993c965b53c898c779e9e93a400909e5455b9b26f32d2c2c4520aca8843f` for
`the-shrine-upgrade` and
`1105656232fa3de88fd3189be482190457d4c302809938bdb5b5880859823072` for the dirty
`bug-gui-design` delta. Untracked files are covered by the working-file snapshot digests.

### `the-shrine-upgrade` domain digests

| Prefix | Files | Digest |
|---|---:|---|
| `crates/cat-sim` | 13 | `2094dbb2095ced51d11c09c9771487518281825ae1b70e103701e6543a25c59a` |
| `crates/cat-protocol` | 5 | `19fde0aa36aa58282ba37e17e2385871ffbe2a7eed76261c651960fc7b38cffa` |
| `crates/cat-server` | 3 | `8c632c39eed574991089b54c15d3f420ccafd4c4dd7ae69c183441696e3134ec` |
| `crates/cat-client` | 7 | `7e47eaa7be3796ccc1f0185fca30ff7896be2a089de46b5f24fc8c14d4ab3de8` |
| `docs` | 1 | `1b8024bf436442dec5b6a19032d498fa1b2a7538d780aa421774f3aaba16fd58` |
| `public/images` | 53 | `f6752d02e2883aa4dc60d7a7483bea8c20e7e9368a52e3d001daccf6b9d5780b` |

### `bug-gui-design` dirty-domain digests

| Prefix | Files | Digest |
|---|---:|---|
| `crates/cat-sim` | 10 | `616e050b875cd7c139393124b889606b18f2e231e1b48ac89cb3722227a4b639` |
| `crates/cat-protocol` | 1 | `08873958dd5946f30d2954f8fabc8ad85e36c46f646830f3987acf1574cd4d6b` |
| `crates/cat-server` | 1 | `6890833b6af9dfe5d725a9175415c6fe2c3bc5d639f667681d4fa37e5ff18020` |
| `crates/cat-client` | 4 | `768e59b27a258d884692e03daa733dfb490fd6776244dbb3a8bf2813a98e9425` |
| `docs` | 4 | `527dfa40a7d69cd12df5ca292dedfb04f2cba29a183cafc970dbbc7ea6e97457` |

## Exact source-path inventories

### `the-shrine-upgrade` tracked modifications

```text
crates/cat-client/src/lib.rs
crates/cat-client/src/research_ui.rs
crates/cat-client/src/station_layout.rs
crates/cat-protocol/src/lib.rs
crates/cat-server/src/main.rs
crates/cat-server/src/persistence.rs
crates/cat-sim/src/actions.rs
crates/cat-sim/src/lib.rs
crates/cat-sim/src/research_catalog.rs
crates/cat-sim/src/research_catalog_tracks.json
crates/cat-sim/src/upgrade_tree.rs
crates/cat-sim/src/world_tick.rs
crates/cat-sim/tests/player_action_campaign.rs
```

### `the-shrine-upgrade` untracked code, tests, and design

```text
crates/cat-client/src/layered_sprite.rs
crates/cat-client/tests/black_hole_art.rs
crates/cat-client/tests/layered_sprite.rs
crates/cat-client/tests/world_site_art.rs
crates/cat-protocol/src/black_hole.rs
crates/cat-protocol/src/hunting_lair.rs
crates/cat-protocol/tests/black_hole.rs
crates/cat-protocol/tests/hunting_lair.rs
crates/cat-server/src/persistence/black_hole.rs
crates/cat-sim/src/black_hole.rs
crates/cat-sim/src/hunting_lair.rs
crates/cat-sim/src/hunting_runtime.rs
crates/cat-sim/tests/black_hole.rs
crates/cat-sim/tests/hunting_lair.rs
crates/cat-sim/tests/hunting_runtime.rs
docs/migration/BLACK_HOLE_LEADER_AI_MERGE.md
```

### `the-shrine-upgrade` untracked assets

The exact 53-file asset set is:

- `public/images/game/buildings/black-hole.png`;
- `public/images/game/buildings/black-hole/base.png`;
- `public/images/game/buildings/black-hole/{width,depth,darkness}-01.png` through `-10.png`
  inclusive: thirty cumulative axis layers;
- `public/images/game/farm/dynamic/{catnip,grain,herb}-{sprout,growing,flowering,mature}.png`:
  twelve crop-stage sprites;
- `public/images/game/nature/tree_oak_apples_{low,mid,full}.png`;
- `public/images/game/sites/{lair,quarry}.png`;
- `public/images/game/transport/{boat,dock_land,dock_water,rail_cart}.png`.

The Plan 1 asset requirements are broader than this source set. Absence from this list means “must
author,” not “drop the requirement.”

### `bug-gui-design` source-only commits

```text
add6951 Rebuild client surfaces and research tree
640b769 Implement P21 playtest feedback slices
e230481 Merge P21 playtest feedback slices
748db74 chore: configure Orca dev workspace
```

Their 26 changed paths relative to merge base are:

```text
AGENTS.md
Cargo.lock
README.md
crates/cat-client/Cargo.toml
crates/cat-client/src/lib.rs
crates/cat-client/src/research_ui.rs
crates/cat-client/src/ui_shell.rs
crates/cat-protocol/src/lib.rs
crates/cat-server/src/main.rs
crates/cat-sim/src/actions.rs
crates/cat-sim/src/food_ecology.rs
crates/cat-sim/src/idle_engine.rs
crates/cat-sim/src/leader_ai.rs
crates/cat-sim/src/leader_director.rs
crates/cat-sim/src/lib.rs
crates/cat-sim/src/movement.rs
crates/cat-sim/src/skills.rs
crates/cat-sim/src/types.rs
crates/cat-sim/src/world_tick.rs
crates/cat-sim/tests/player_action_campaign.rs
docs/FIX_LOG.md
docs/UI_ARCHITECTURE.md
docs/migration/BOARD.md
docs/migration/P21_PLAYTEST_FEEDBACK.md
docs/migration/specs/p18-visual-polish.md
orca.yaml
```

### `bug-gui-design` dirty paths

```text
crates/cat-client/src/landing_showcase.rs
crates/cat-client/src/lib.rs
crates/cat-client/src/research_ui.rs
crates/cat-client/src/start_screen.rs
crates/cat-protocol/src/lib.rs
crates/cat-server/src/main.rs
crates/cat-sim/src/actions.rs
crates/cat-sim/src/lib.rs
crates/cat-sim/src/research_catalog.rs
crates/cat-sim/src/research_catalog_junctions.json
crates/cat-sim/src/research_catalog_legacy.json
crates/cat-sim/src/research_catalog_tracks.json
crates/cat-sim/src/research_tracks.rs
crates/cat-sim/src/upgrade_tree.rs
crates/cat-sim/src/world_tick.rs
crates/cat-sim/tests/player_action_campaign.rs
docs/FIX_LOG.md
docs/RESEARCH_ARCHITECTURE.md
docs/UI_ARCHITECTURE.md
docs/migration/specs/p18-visual-polish.md
```

## Source-to-destination transfer matrix

| Source knowledge | Required treatment | Destination |
|---|---|---|
| Shrine `black_hole.rs`, protocol types/tests, persistence leaf, and related actions | Extract feed/axis/state/validation/idempotency behavior and tests; rename to target Hole types; replace old Leader adapters and currency assumptions | LAI.41, LAI.44–48 |
| Shrine Hunting leaves and focused tests | Port encounter/roster/cache/respawn/injury/equipment behavior as bounded leaves; expand to the exact twenty-creature plan and universal quality | LAI.36–37, LAI.42–48 |
| Shrine catalog/upgrade changes | Extract useful IDs, gates, and relationships; rebuild through the validated unified manifest rather than copying fixed counts or obsolete nodes | LAI.36, LAI.43–44, LAI.58 |
| Shrine `actions.rs`, `world_tick.rs`, client/protocol/server roots | Never copy wholesale; inspect every relevant hunk, extract state transitions and missing cases, then integrate once through the target owners | LAI.45–48, LAI.50, LAI.63–65 |
| Shrine layered-sprite implementation and art tests | Adapt deterministic layering, bounds, visibility, and state-change tests to Bevy target rendering | LAI.49, LAI.68 |
| Shrine Hole/crop/Apple/site/transport images | Copy only through the asset owner, preserve source hash/provenance, validate transparency/native bounds, and record whether reused, adapted, or replaced | LAI.49, LAI.68 |
| Shrine merge design document and campaign additions | Treat explanations and scenarios as evidence input; reconcile every assertion against Plan 1/Q&A rather than assuming code is final | LAI.35, LAI.51–52, LAI.69–70 |
| Bug committed UI shell and dirty start/showcase work | Adapt navigation, layout, visual language, responsiveness, focus/Escape behavior, and off-map showcase; do not restore obsolete authority or routes | LAI.54, LAI.66–68 |
| Bug committed and dirty research graph/catalog/UI | Preserve graph layout, junctions, track/effect knowledge, overview/focus UI, and useful tests; rebuild around the two authoritative lanes and unified capability manifest | LAI.36, LAI.44, LAI.58, LAI.64–65, LAI.67–68 |
| Bug skills, food ecology, idle/Leader/director, movement, actions, and world tick | Extract catalog ideas, playtest fixes, constants, and scenarios; do not restore the old planner, false task geometry, direct God micromanagement, or duplicate mutation roots | LAI.55–63 |
| Bug protocol/server roots | Inventory every new action/snapshot field and error path, then implement only through the single versioned report-safe contract | LAI.64–65 |
| Bug campaign test changes | Preserve scenarios and regressions as source-derived acceptance, updated for Hole, physical lots, families, two research lanes, barter, and clean reset | LAI.69–70 |
| Bug UI/research/fix-log/visual-polish docs | Retain rationale and screenshots/checkpoint intent, reconcile conflicts explicitly, and update maintained docs instead of copying stale claims | LAI.53–54, LAI.66–70 |
| Bug manifests, lockfile, workspace configuration, and board metadata | Reference only. Do not import dependency/workspace/agent configuration unless a destination card independently requires and verifies it | LAI.53, LAI.69–70 |

## Required per-file transfer receipt

Before a destination card can claim source functionality, its evidence must record:

1. source branch, source head, relative path, state (`committed`, `modified`, or `untracked`), and
   source file hash;
2. relevant functions/types/assets/tests/docs and the behavior or visual knowledge extracted;
3. disposition: `reuse`, `adapt`, `rewrite`, `reference-only`, `superseded`, or `reject`;
4. exact Plan/Q&A/conflict IDs that authorize the disposition;
5. target files and sole hot-root owner;
6. source-derived red test or characterization evidence before implementation where practical;
7. green focused evidence, then final integration/browser evidence;
8. any deliberate difference and why the later authority requires it.

`reference-only`, `superseded`, and `reject` are valid only with a written reason. “Large diff,”
“old branch,” “not merged,” or “newer target code exists” is not sufficient.

## Final reconciliation gate

LAI.70 remains open until:

- the source worktrees remain recoverable or an authorized verified archive/commit replaces them;
- all 82 `the-shrine-upgrade` working files have receipts, including all 53 assets;
- all 26 committed and all 20 dirty `bug-gui-design` paths have receipts;
- overlapping paths are audited independently for committed and dirty knowledge;
- every source test is ported, replaced by stronger target evidence, or rejected with a recorded
  superseding rule;
- every useful source asset is reused/adapted or explicitly replaced;
- hot roots have one target implementation rather than textual branch merges;
- the resulting game demonstrates the retained functionality through report-safe simulation,
  real persistence/server paths, shipped UI/world rendering, and serialized browser evidence.
