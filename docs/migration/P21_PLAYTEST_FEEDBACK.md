# P21 Playtest Feedback Acceptance Checklist

Source: the user's 2026-07-22 playtest report. This file is the completion authority for the
active goal. An item is complete only when behavior is implemented, direct regression coverage
exists where practical, and the final live graphical audit confirms the player-visible result.

Status meanings: `open`, `red`, `dev`, `qa`, `done`.

## World representation and interaction

| ID | Status | Acceptance |
|---|---|---|
| P21-01 | qa | Dens occupy exactly 2x2 tiles and render as a compact cottage with no exposed wooden floor tiles. |
| P21-02 | qa | Workshops occupy and are hoverable/selectable across their entire 3x3 footprint. |
| P21-03 | qa | Storage is a designated world zone, not a boat/building prop; each ordinary tile holds four loose item stacks. |
| P21-04 | open | Containers such as barrels increase same-kind capacity without consuming one visible slot per contained item. |
| P21-05 | open | Specialized storage can be configured for workshop inputs and is used by hauling/production. |
| P21-06 | open | Walls occupy world tiles, are visibly marked, and are impassable; only gates permit crossing. |
| P21-07 | qa | Paved road hover identifies `street`; worn dirt-road hover identifies `path`. |
| P21-08 | qa | A rendered tree is hoverable across its complete canopy footprint. |
| P21-09 | qa | Fishing spots can be designated on valid shoreline tiles and immediately expose why invalid placement fails. |
| P21-10 | open | Farms are world tiles/plots with a leader-assigned crop; no abstract off-world farm representation remains. |
| P21-11 | open | Road authoring visibly marks its route, queues physical work, and produces completed street tiles. |
| P21-12 | qa | Open jobs have visible world markers (for example a red tree border) tied to the authoritative job queue. |

## Cats, jobs, needs, and autonomy

| ID | Status | Acceptance |
|---|---|---|
| P21-13 | qa | Cat walk cycles use stable per-actor phases and do not visibly march in lockstep. |
| P21-14 | qa | Every work-capable adult is assigned concrete work or renewable low-priority maintenance. |
| P21-15 | qa | Village demand outranks shrine demand; when village demand is satisfied the shrine always requests useful work. |
| P21-16 | qa | Shrine demand targets the highest-quality, most complex currently-producible items for score. |
| P21-17 | qa | Idle cats claim open jobs; renewable fallback includes hauling and cleaning without displacing real demand. |
| P21-18 | qa | Hungry/thirsty cats physically retrieve food/water from suitable storage. |
| P21-19 | qa | Cooked food is preferred and recipes provide a real yield advantage over eating raw resources. |
| P21-20 | qa | Cats do not materialize with food from fog; every carried item has an authoritative physical source. |
| P21-21 | open | The leader autonomously designates zones, crops, storage, roads, production, and other routine village needs. |
| P21-22 | open | Locked research does not deadlock the village; reachable prerequisites and productive work remain available. |

## Information architecture and records

| ID | Status | Acceptance |
|---|---|---|
| P21-23 | qa | Missing-glyph boxes are removed from generated names/copy or rendered by a font that contains the intended glyphs. |
| P21-24 | done | The separate map destination was removed by later player direction; **Center village** remains the direct navigation action. |
| P21-25 | dev | Stores is a full-screen management surface for zones, filters, containers, capacity, contents, and hauling state. |
| P21-26 | qa | Census is renamed Village and becomes a full-screen village-statistics and population surface. |
| P21-27 | done | Dispatches and the overflowing activity ticker were both removed by later player direction; the routed Log page owns event history. |
| P21-28 | qa | Log is full-screen with filters and a complete authoritative event history. |
| P21-29 | open | Each cat has a DF-style full record: stats, generated traits, family tree, and personal event history. |
| P21-30 | qa | Redundant Villages tab is removed; the session clearly represents either the global village or one local village. |
| P21-31 | open | Zones remain available but routine designation/assignment belongs to leader automation. |
| P21-32 | qa | Job queue is an accessible list of open/assigned jobs and connects selections to world markers. |
| P21-33 | qa | Primary management surfaces scale legibly at 1080p and 4K; tiny scattered panels do not carry full workflows. |

## Renewable food ecology and expedition safety (follow-up playtest)

| ID | Status | Acceptance |
|---|---|---|
| P21-34 | qa | Cleaning is invisible ambient idle movement, never an active/queued job, job-count entry, world marker, or cat task. Real work immediately preempts it. |
| P21-35 | qa | Persistent caves are visible hunting sites with deterministic levels 1-100, varied finite food capacity, and site-specific regeneration. |
| P21-36 | qa | Sparse apple trees and berry bushes are visible renewable gathering sites with distinct capacities/regeneration; gathering raises Farming skill. |
| P21-37 | qa | Fishing habitats have varied finite capacities and regeneration rates rather than one global identical ecology. |
| P21-38 | qa | Cave hunts raise both Hunting and Fighting and have deterministic level-scaled injury risk. |
| P21-39 | qa | Hard caves require research-unlocked groups; groups can bring physical weapons and no solo cat may enter above the safe threshold. |
| P21-40 | qa | The responsible officer/leader selects hunting/gathering sites and cats using skill, site level, equipment, known injuries, and leader intelligence. |
| P21-41 | qa | A capable leader stops sending underqualified cats to a cave after injuries; a low-intelligence leader may make poorer but deterministic risk decisions. |

## Final gates

- [ ] Every table row is `done`.
- [ ] Focused regression suites pass for every slice.
- [ ] `cargo nextest run --workspace --profile smoke` passes.
- [ ] Touched-crate Clippy passes with `-D warnings` and rustfmt is clean.
- [ ] Deterministic multi-hour campaigns show no unexplained item creation or idle work-capable adults.
- [ ] A clean temporary-database native playtest checks every player-visible row at 1080p and 4K.
- [ ] Final audit cites screenshots and test names; only then may the active goal and P21 board card be complete.

## QA evidence in progress

The later GUI correction supersedes the earlier proposed Map, Help, Dispatches, ticker, and
screen-opening keyboard-shortcut treatments. The P21 gameplay data and world interactions remain;
primary-screen ownership, scrolling, and buttons follow `docs/UI_ARCHITECTURE.md`.

- 1080p clean-save world/HUD: `/tmp/cat-p21-start-selection.png` (`Working 30/30`, `idle 0`, cursor-aligned street border and tooltip).
- 1080p Village page: `/tmp/cat-p21-1080-village-30of30.png` (30/30 employed; no cleaning jobs).
- 4K full-screen Village: `/tmp/cat-p21-4k-village.png`; 4K full-screen Research: `/tmp/cat-p21-4k-ui2x.png` (2x responsive UI scale, legible controls and text).
- Renewable ecology: `food_ecology::*` (5 tests), `fruit_harvest_is_finite_and_depletes_only_the_persisted_source`.
- Cave safety: `cave_hunt_trains_fighting_and_can_cause_a_persisted_injury_warning`, `difficult_caves_require_researched_groups_and_smart_leaders_learn_from_injuries`, `researched_cave_group_shares_one_site_and_physically_retrieves_weapons`.
- Evergreen work: `one_authoritative_tick_assigns_every_work_capable_founding_cat`, `end_of_tick_release_is_immediately_absorbed_by_shrine_work`, `vacancy_cleanup_preserves_only_baseline_leader_jobs`.
- Client truthfulness: `village_employment_counts_cats_not_job_records_and_ignores_ambient_motion`, `village_job_queue_excludes_ambient_cleaning_and_closed_history`, `only_open_physical_jobs_receive_world_markers`.
- Current quality checkpoint: touched-crate Clippy with `-D warnings` passes; workspace smoke profile passes 75/75 (Nextest run `ed266e74-fddf-443c-b673-e520be0ba378`).
