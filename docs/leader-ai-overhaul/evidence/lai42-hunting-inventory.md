# LAI.42 Hunting Inventory

Date: 2026-07-25

Scope: read-only implementation inventory for the twenty-species Hunting domain. I read `AGENTS.md`,
the two restored plans, the LAI.42/P1 board rows, thread Q&A audit, current target catalog/quality/
hunting/spatial/task/protocol/server/client/persistence leaves, LAI.36/37 evidence, and protected
`the-shrine-upgrade` hunting code/tests/assets. No code, tests, boards, plans, source manifests,
Cargo files, protected source worktrees, builds, tests, formatters, browser, image generation, or
live AI were touched or run; this evidence document is the only file produced.

## Source Receipts Read

| Source | Exact origins | Classification | LAI.42 disposition |
| --- | --- | --- | --- |
| Final Plan 1 Hunting spec | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:399-461` | Exact plan authority | Normative for the twenty species, level/yield/material/Hole table, encounter bands, thresholds, `hunting_bulk`, XP, injury/death, cache, respawn, keyed quality, and first-clear guarantee. |
| Final Plan 1 visuals | `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md:127-136`, `619-631`, `770-773` | Exact plan authority | Ten public world lair sprites, selected-lair-only twenty portraits, unique drop icons, exact level hidden until report, and `EnemyLair` distinct from Quarry `CaveEntrance`. |
| Board LAI.42 row | `docs/leader-ai-overhaul/BOARD.md:1183` | Task contract | LAI.42 is the Hunting-domain leaf owner and depends on LAI.37. |
| P1 Hunting rows | `docs/leader-ai-overhaul/BOARD.md:1229-1240`, `1246-1248` | Acceptance register | P1.26-P1.29/P1.33-P1.36/P1.43/P1.45 retain exact Hunting content, protocol, persistence, UI, art, and acceptance scope. |
| Q&A audit | `docs/branch-plan-merge/thread-qa-audit.md:84-89`, `112-121` | Plan reconciliation | Locks twenty creatures, unique named drops, quality by encounter, report-safe God/Leader state, real Lair tasks, and semantic source transfer. |
| Source transfer manifest | `docs/branch-plan-merge/source-transfer-manifest.md:52-73`, `105-124`, `126-141`, `208-218`, `227-239` | Receipt protocol | `the-shrine-upgrade` has source hunting leaves/tests/assets; transfer is semantic, not wholesale copy. |
| LAI.36 evidence | `docs/leader-ai-overhaul/evidence/lai36-source-catalog-inventory.md:43-132` | Upstream ID/catalog contract | Current untracked manifest candidate owns the twenty creature IDs/material IDs/art keys; behavior leaf must consume it. |
| LAI.37 evidence | `docs/leader-ai-overhaul/evidence/lai37-quality-lot-inventory.md:65-153`, `168-176` | Upstream quality/lot contract | LAI.42 must consume `QualityBand`, `BulkLotKey`, lot locations including `Cache`, and conservation rules; it must not duplicate quality. |

Source file hashes recorded during this pass:

| Path | SHA-256 | Disposition |
| --- | --- | --- |
| `crates/cat-sim/src/hunting_lair.rs` | `3eea03a1f0cdc124608873fd85046b10aad257c81a5fc583ebf73a13b04bb111` | Current untracked four-species leaf; partial prototype only. |
| `crates/cat-sim/tests/hunting_lair.rs` | `22b76d9de1bd0cefe121793337c85267e669c0fcbaa7fba5dcbce40cb9b99155` | Current untracked four-species focused tests; source-derived red cases only after expansion. |
| `the-shrine-upgrade/crates/cat-sim/src/hunting_lair.rs` | `3eea03a1f0cdc124608873fd85046b10aad257c81a5fc583ebf73a13b04bb111` | Byte-identical to current untracked prototype. |
| `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs` | `05985b2cd2d21903e18827fe8ac0c29816a916c59396b08e8b183e187167626d` | Source runtime receipt to rewrite/adapt. |
| `the-shrine-upgrade/crates/cat-sim/tests/hunting_runtime.rs` | `4b6e457d0964fdbeb622c33174675606ea87afee6d841b9df81df13898fb150b` | Source behavior red cases to preserve after expanding to twenty species/lots. |
| `the-shrine-upgrade/crates/cat-protocol/src/hunting_lair.rs` | `6c6d75d181260608e0983ccbaf8d1e6800ac4c29e4c6bdbfbdc659679123b67c` | Source DTO receipt; target protocol currently has no LAI.42 DTO. |
| `the-shrine-upgrade/crates/cat-protocol/tests/hunting_lair.rs` | `b94f15b53d01438b7d7248f6286d13648449ebd375677d78f6007c7ae5b21ddd` | Source report-safety/action red cases. |

Current target files `crates/cat-sim/src/content_manifest.rs`, `crates/cat-sim/src/content_manifest.json`,
`crates/cat-sim/src/hunting_lair.rs`, and `crates/cat-sim/tests/hunting_lair.rs` are untracked dirty-tree
candidates at inspection time. This document inventories them as current-tree evidence but does not treat
them as merged or completed implementation.

## Top Reconciliation Findings

1. The current target tree has the twenty-creature manifest data, but the current/source Hunting
   behavior leaf still has only `Fox`, `Badger`, `Bear`, and `RivalBeast` (`crates/cat-sim/src/hunting_lair.rs:17-83`; protected source same hash). LAI.42 must replace the four-species enum with LAI.36 `CreatureId`/manifest-driven records or it will create a second roster authority.
2. The inventory originally found a level-60 boundary conflict between the plan's 40–60 mixed / 61+ mandatory rule and an earlier band-wide boolean. P1-C04 and the settled catalog now close it without changing party-size bands: `mystic_required_from_level` is `None/None/None/61/80/95`, so level 60 remains mixed and level 61 becomes mandatory.
3. Current legacy runtime hunts are finite `CaveEntrance` food jobs with hide/bone side cargo, while LAI.42 requires creature-owned `EnemyLair` encounters with quality-bearing Meat/Hide/Bone/material lots and `CaveEntrance` reserved for Quarry (`crates/cat-sim/src/world_tick.rs:4178-4198`, `7197-7285`, `28160-28439`; `types.rs:142-160`).
4. The plan fixed rare-drop quality bands and the roll key but did not originally fix twenty-species probabilities. P1-C04 now supplies the deterministic level-band table `10/15/20/25/30/40%`; the four-species source table remains only provenance for that closure, not a second authority.

## Current Target Authorities

| Leaf | Origin | Classification | Existing behavior | LAI.42 risk/cutover |
| --- | --- | --- | --- | --- |
| LAI.36 content manifest module | `crates/cat-sim/src/content_manifest.rs:737-789`, `1988-2110` | Manifest data/validator | Defines `CreatureDescriptor`, `CreatureStats`, `LairBandDescriptor`, `LairVisualBandDescriptor`, and validates exact twenty Plan 1 creature identities. | This is the data source LAI.42 should consume; do not duplicate creature enums/stats in Hunting. |
| Embedded manifest JSON | `crates/cat-sim/src/content_manifest.json:2046-2891`, `8972-9152`, `9568-9746`, `10279-10474` | Manifest data | Contains twenty creatures, six encounter bands, ten visual bands, twenty portrait art keys, and twenty material icon keys. | Good ID/data seed, but assets are planned paths only and behavior is not wired. |
| Current `hunting_lair.rs` | `crates/cat-sim/src/hunting_lair.rs:1-481` | Partial handler/prototype | Four species, scalar `BaseLoot { food, hide, bone }`, danger-only roster sizing, `Hunter`, risk gates, XP, damage/death flag, first-clear guarantee, cooldown respawn. | Must be rewritten around twenty manifest creatures, LAI.37 quality/lots, exact roll key, and `EnemyLair` sites. |
| Current `hunting_lair` tests | `crates/cat-sim/tests/hunting_lair.rs:23-226` | Partial red tests | Verify four-species danger thresholds, authority gates, party cap, loot, XP, first clear, respawn, failure/death flag. | Useful characterization but insufficient for P1.26-P1.29. |
| LAI.37 quality lots | `crates/cat-sim/src/quality_lots.rs:53-222`, `224-240`; `crates/cat-sim/tests/lai37_quality_lots.rs:290-310` | Upstream authority | Owns `QualityBand`, exact multipliers/formula/thresholds, keyed variation, and `LotLocation::Cache`. | Hunting must consume this and emit/debit quality lots; no raw `u8` quality or scalar resources. |
| Tile taxonomy | `crates/cat-sim/src/types.rs:142-160`, `340-350`; `crates/cat-sim/src/biomes.rs:40-80`, `343-353` | Spatial enum/data | `EnemyLair` and `CaveEntrance` are distinct, but current legacy hunt sources still use `CaveEntrance`. | LAI.42 should attach hunting rosters only to `EnemyLair`; Quarry keeps `CaveEntrance`. |
| Legacy founding hunt runtime | `crates/cat-sim/src/world_tick.rs:4136-4265` | Legacy/obsolete survival adapter | Guarantees three rich finite `CaveEntrance` wildlife sources, not `EnemyLair` creature lairs. | LAI.38/40 founding food may retain shoreline/fish; LAI.42 must not inherit cave food as lair combat. |
| Legacy spatial hunt request | `crates/cat-sim/src/world_tick.rs:7197-7285` | Handler with scalar source claim | Pins a `ResourceSourceKind::Hunting` objective to a finite food tile and adjacent work tile. | Needs `EnemyLair` objective/site ID, party reservation, cargo/cache lots, and no fallback markers. |
| Legacy hunt completion/cargo | `crates/cat-sim/src/world_tick.rs:8044-8078`, `28160-28439`, `29103-29241` | Legacy/obsolete handler | Credits scalar `Food`, increments Hunt XP, carries Food/Hide/Bone across multiple loads, drains tile food. | Competes with Plan Meat/Hide/Bone/material lots, Fight XP, equipment wear, injury/death, cache overflow. |
| Injury leaf | `crates/cat-sim/src/injuries.rs:19-67`, `160-268`; `world_tick.rs:8761-8817` | Existing handler | `HazardousWorkUnit::Hunt` has 100 bp incident probability and deterministic injury/fatal resolution. | Can be a consumer, but source LAI.42 combat has direct damage/death; reconcile one death/injury path. |
| Death events | `crates/cat-sim/src/world_tick.rs:2019-2041` | Closed behavior enum | Current `DeathCause` has no `Hunt`; source runtime wants `DeathCause::Hunt`. | Add/cut over in later implementation; current target cannot report hunt deaths distinctly. |
| Equipment identity | `crates/cat-sim/src/items.rs:108-142`, `390-467`; `world_tick.rs:2883-2895` | Existing item handler | `ItemKind::Weapon/Armor`, exact `ItemInstance` location/durability, and `wear_equipped_item`. | Use LAI.37 item instance fields/quality; do not read scalar Weapons/Armor as equipped gear. |
| Current protocol/server/client | `crates/cat-protocol/src/lib.rs:1121-1139`, `2071-2085`; `crates/cat-server/src/main.rs:6227-6406`; `crates/cat-client/src/lib.rs:9083-9084` | Legacy wire/UI | Exposes `JobKind::HuntExpedition` and player job request, no `HuntingLairSnapshot`/`NudgeHuntingSite` in current target. | LAI.47+ must add strict report-safe LAI.42 DTO/action; LAI.42 should define sim boundary first. |

## Twenty-Species Source-to-Target Table

Common loot names below map Plan `Meat` to manifest `food_raw_meat`, not legacy generic `Food`.
`Hide` and `Bone` map to `resource_hide` and `resource_bone`. Stats are manifest `body/attack/defense/danger`.

| Species | Stable IDs | Levels / tier / stats | Common loot | Named drop and Hole gate/value | Exact origins |
| --- | --- | --- | --- | --- | --- |
| Cave Bat | `cave_bat`, `creature_cave_bat`, `art_creature_cave_bat` | 1-8 / normal / `1/3/2/2` | Meat 1, Hide 0, Bone 1 | `bat_wing`, D1 / 0.25, `art_material_bat_wing` | Plan `:405`; creature `content_manifest.json:2047-2079`; material `:1426-1455`; portrait `:9001-9007`; icon `:9586-9592`. |
| Red Fox | `red_fox`, `creature_red_fox`, `art_creature_red_fox` | 5-18 / normal / `12/8/5/7` | Meat 12, Hide 2, Bone 1 | `fox_pelt`, D2 / 0.5, `art_material_fox_pelt` | Plan `:406`; creature `:2080-2116`; material `:1457-1486`; portrait `:9118-9124`; icon `:9649-9655`. |
| Badger | `badger`, `creature_badger`, `art_creature_badger` | 10-24 / normal / `18/11/12/11` | Meat 18, Hide 3, Bone 2 | `badger_pelt`, D3 / 1, `art_material_badger_pelt` | Plan `:407`; creature `:2117-2153`; material `:1488-1517`; portrait `:8974-8980`; icon `:9568-9574`. |
| Wild Boar | `wild_boar`, `creature_wild_boar`, `art_creature_wild_boar` | 16-30 / normal / `24/17/15/16` | Meat 24, Hide 3, Bone 4 | `boar_tusk`, D3 / 0.8, `art_material_boar_tusk` | Plan `:408`; creature `:2154-2190`; material `:1519-1548`; portrait `:9136-9142`; icon `:9613-9619`. |
| Gray Wolf | `gray_wolf`, `creature_gray_wolf`, `art_creature_gray_wolf` | 22-36 / normal / `22/22/16/21` | Meat 22, Hide 3, Bone 3 | `wolf_pelt`, D4 / 1.2, `art_material_wolf_pelt` | Plan `:409`; creature `:2191-2227`; material `:1550-1579`; portrait `:9055-9061`; icon `:9730-9736`. |
| Lynx | `lynx`, `creature_lynx`, `art_creature_lynx` | 28-42 / normal / `20/27/19/26` | Meat 20, Hide 3, Bone 3 | `lynx_pelt`, D4 / 1.5, `art_material_lynx_pelt` | Plan `:410`; creature `:2228-2264`; material `:1581-1610`; portrait `:9091-9097`; icon `:9667-9673`. |
| Great Stag | `great_stag`, `creature_great_stag`, `art_creature_great_stag` | 32-46 / normal / `35/29/24/28` | Meat 35, Hide 4, Bone 5 | `stag_antler`, D4 / 1.2, `art_material_stag_antler` | Plan `:411`; creature `:2266-2301`; material `:1612-1641`; portrait `:9073-9079`; icon `:9703-9709`. |
| Giant Serpent | `giant_serpent`, `creature_giant_serpent`, `art_creature_giant_serpent` | 36-50 / normal / `18/34/25/34` | Meat 18, Hide 4, Bone 2 | `serpent_scale`, D5 / 1.5, `art_material_serpent_scale` | Plan `:412`; creature `:2302-2338`; material `:1643-1672`; portrait `:9046-9052`; icon `:9694-9700`. |
| Brown Bear | `brown_bear`, `creature_brown_bear`, `art_creature_brown_bear` | 40-54 / normal / `30/38/35/38` | Meat 30, Hide 6, Bone 4 | `bear_pelt`, D5 / 2, `art_material_bear_pelt` | Plan `:413`; creature `:2339-2375`; material `:1674-1703`; portrait `:8992-8998`; icon `:9595-9601`. |
| Great Eagle | `great_eagle`, `creature_great_eagle`, `art_creature_great_eagle` | 44-60 / normal / `16/42/29/41` | Meat 16, Hide 3, Bone 1 | `eagle_feather`, D5 / 1.8, `art_material_eagle_feather` | Plan `:414`; creature `:2376-2412`; material `:1705-1734`; portrait `:9064-9070`; icon `:9640-9646`. |
| Moon Stag | `moon_stag`, `creature_moon_stag`, `art_creature_moon_stag` | 40-60 / mystic / `40/44/37/44` | Meat 40, Hide 5, Bone 5 | `moon_antler`, D6 / 2.5, `art_material_moon_antler` | Plan `:415`; creature `:2413-2449`; material `:1736-1765`; portrait `:9109-9115`; icon `:9685-9691`. |
| Warg | `warg`, `creature_warg`, `art_creature_warg` | 46-66 / mystic / `35/49/39/49` | Meat 35, Hide 5, Bone 5 | `warg_fang`, D6 / 2.8, `art_material_warg_fang` | Plan `:416`; creature `:2450-2486`; material `:1767-1796`; portrait `:9127-9133`; icon `:9721-9727`. |
| Cockatrice | `cockatrice`, `creature_cockatrice`, `art_creature_cockatrice` | 50-70 / mystic / `24/53/43/54` | Meat 24, Hide 5, Bone 2 | `cockatrice_eye`, D6 / 3, `art_material_cockatrice_eye` | Plan `:417`; creature `:2487-2523`; material `:1798-1827`; portrait `:9019-9025`; icon `:9622-9628`. |
| Forest Troll | `forest_troll`, `creature_forest_troll`, `art_creature_forest_troll` | 56-76 / mystic / `50/59/55/59` | Meat 50, Hide 10, Bone 8 | `troll_hide`, D7 / 3.5, `art_material_troll_hide` | Plan `:418`; creature `:2524-2560`; material `:1829-1858`; portrait `:9037-9043`; icon `:9712-9718`. |
| Griffin | `griffin`, `creature_griffin`, `art_creature_griffin` | 62-82 / mystic / `45/65/58/65` | Meat 45, Hide 7, Bone 6 | `griffin_plume`, D7 / 4, `art_material_griffin_plume` | Plan `:419`; creature `:2561-2597`; material `:1860-1889`; portrait `:9082-9088`; icon `:9658-9664`. |
| Basilisk | `basilisk`, `creature_basilisk`, `art_creature_basilisk` | 68-88 / mystic / `35/71/62/72` | Meat 35, Hide 8, Bone 5 | `basilisk_scale`, D8 / 4.5, `art_material_basilisk_scale` | Plan `:420`; creature `:2598-2634`; material `:1891-1920`; portrait `:8983-8989`; icon `:9577-9583`. |
| Manticore | `manticore`, `creature_manticore`, `art_creature_manticore` | 74-92 / mystic / `55/78/67/79` | Meat 55, Hide 9, Bone 8 | `manticore_barb`, D8 / 5, `art_material_manticore_barb` | Plan `:421`; creature `:2635-2671`; material `:1922-1951`; portrait `:9100-9106`; icon `:9676-9682`. |
| Chimera | `chimera`, `creature_chimera`, `art_creature_chimera` | 80-96 / mystic / `70/84/73/85` | Meat 70, Hide 12, Bone 10 | `beast_core`, D8 / 5, `art_material_beast_core` | Plan `:422`; creature `:2672-2708`; material `:1953-1982`; portrait `:9010-9016`; icon `:9604-9610`. |
| Wyvern | `wyvern`, `creature_wyvern`, `art_creature_wyvern` | 86-99 / mystic / `80/91/79/92` | Meat 80, Hide 14, Bone 12 | `wyvern_membrane`, D9 / 7.5, `art_material_wyvern_membrane` | Plan `:423`; creature `:2709-2745`; material `:1984-2013`; portrait `:9145-9151`; icon `:9739-9745`. |
| Elder Dragon | `elder_dragon`, `creature_elder_dragon`, `art_creature_elder_dragon` | 95-100 / boss / `120/100/100/100` | Meat 120, Hide 30, Bone 20 | `dragon_heart`, D10 / 10, `art_material_dragon_heart` | Plan `:424-426`; creature `:2746-2782`; material `:2015-2044`; portrait `:9028-9034`; icon `:9631-9637`. |

## Encounter, Party, Quality, and Respawn Constants

| Topic | Exact required value | Current/source state | Risk |
| --- | --- | --- | --- |
| Encounter mixing | 1-39 normal only; 40-60 normal/mystic mixtures; 61-100 at least one mystic (`final-hole-hunting-content-plan.md:430-432`). | The settled six catalog bands carry `mystic_required_from_level=None/None/None/61/80/95`. The source prototype's danger 0-84/85-89/90-94/95+ four-species thresholds remain deletion evidence only. | Implement the settled threshold field; never infer mandatory mystic from the whole 60–79 party-size band. |
| Party-size bands | 1 at 1-19; 1-2 at 20-39; 2 at 40-59; 2-3 at 60-79; 3 at 80-94; boss plus two supporters at 95-100 (`final-hole-hunting-content-plan.md:433-440`). | Manifest records min/max sizes for the six bands (`content_manifest.json:2784-2839`) but no boss/supporter semantic; source prototype sizes by danger 0-84/85-94/95+ (`hunting_lair.rs:191-195`). | Add explicit boss-plus-two logic; do not infer it from max size alone. |
| Autonomous gate | predicted success >= 70% and every hunter health >= 70% (`final-hole-hunting-content-plan.md:442`). | Current/source leaf implements exactly `(70,70.0)` for `AutonomousLeader` (`hunting_lair.rs:293-308`); source runtime repeats it (`the-shrine-upgrade/.../hunting_runtime.rs:532-539`). | Keep, but success formula must consume twenty-species stats and equipment quality. |
| Player nudge gate | predicted success >= 45% and every hunter health >= 80%, still normal planner review (`final-hole-hunting-content-plan.md:443-444`). | Current/source leaf implements exactly `(45,80.0)` (`hunting_lair.rs:293-308`); source action stores a hint, not force-combat (`the-shrine-upgrade/.../hunting_runtime.rs:478-550`). | Preserve review-only semantics. |
| `hunting_bulk` | Stable study meaning is Hunting Parties, party cap three (`final-hole-hunting-content-plan.md:445`; `BOARD.md:1230`). | Current/source cap constants are 1 and 3 (`hunting_lair.rs:11-13`), source runtime checks `hunting_bulk` (`the-shrine-upgrade/.../hunting_runtime.rs:27`, `706-708`). | Retain ID, change display to Hunting Parties through LAI.44/50. |
| Equipment | Exact equipped items supply bonuses and receive durability wear (`final-hole-hunting-content-plan.md:446`). | Source runtime uses only equipped non-broken `Weapon`/`Armor`, ignores scalar resources, adds +25 weapon/+25 armor, and wears both (`the-shrine-upgrade/.../hunting_runtime.rs:27-30`, `326-329`, `674-704`); source test confirms aggregate resources do not count (`tests/hunting_runtime.rs:127-223`). | Reuse invariant, but bonuses must be quality/material-aware later; source constants are partial. |
| XP and injury/death | Hunts award Hunting and Fight XP; failure may injure or kill (`final-hole-hunting-content-plan.md:447-448`). | Current/source leaf returns Hunting/Fight XP and damage/death flag (`hunting_lair.rs:327-336`, `395-419`); source runtime applies XP/health/death and wants `DeathCause::Hunt` (`the-shrine-upgrade/.../hunting_runtime.rs:306-345`), but target `DeathCause` lacks Hunt (`world_tick.rs:2019-2041`). | Add single death/injury authority, no double rolling between hazardous-work and combat. |
| Cache overflow | Overflow creates visible one-tile lair cache (`final-hole-hunting-content-plan.md:449`). | LAI.37 has `LotLocation::Cache("lair_a")` representable (`tests/lai37_quality_lots.rs:290-310`); source runtime credits directly to stock/resources/material counters (`the-shrine-upgrade/.../hunting_runtime.rs:330-355`). | Implement cache as quality lots, not scalar overflow. |
| Respawn | Respawn stores one absolute game-time deadline (`final-hole-hunting-content-plan.md:450`). | Current/source leaf stores `respawn_ready_at_ms` and source runtime persists it in shared state (`hunting_lair.rs:115-121`, `465-468`; source persistence receipt below). | Need exact per-species/per-level respawn constants; source four-species cooldowns do not cover Plan 1 twenty rows. |
| Rare-drop quality | 1-24: 0; 25-49: 0-1; 50-69: 1-2; 70-84: 2-3; 85-94: 3-4; 95-100: 4 (`final-hole-hunting-content-plan.md:452-459`). | LAI.37 owns `QualityBand` 0-4 and value multipliers (`quality_lots.rs:53-107`); current/source Hunting has no quality. | LAI.42 should call LAI.37; no duplicate `QualityBand`. |
| Rare-drop RNG key | world seed + lair ID + generation + creature ID + clear index; first clear guarantees strongest creature primary drop at band floor if normal rolls yield none (`final-hole-hunting-content-plan.md:461`). | Current/source leaf guarantees strongest material but lacks quality and uses a combat seed plus per-monster rolls (`hunting_lair.rs:435-455`); source runtime seed uses site/generation/game hour/nonce (`the-shrine-upgrade/.../hunting_runtime.rs:716-727`). | Need exact key tuple and clear-index counter; source nonce/game-hour seed is not the plan key. |
| Drop probabilities | P1-C04 closes the plan gap at 10/15/20/25/30/40% for levels 1–24/25–49/50–69/70–84/85–94/95–100. | Source prototype had only Fox 10%, Badger 15%, Bear 25%, RivalBeast 40%; it remains provenance, not runtime authority. | Implement one level-band function plus separately keyed P1.28 quality and first-clear guarantee. |

## Protected Source Behavior Inventory

| Source leaf | Exact origins | Behavior to retain | Conflict/gap |
| --- | --- | --- | --- |
| Runtime state | `the-shrine-upgrade/crates/cat-sim/src/hunting_runtime.rs:34-96` | Active party, trophy claim, outcome record, Captain recommendation, attempt report, and errors for unrevealed, not-lair, dead/unavailable/duplicate cats, unsafe attempts, empty lairs. | Needs `CreatureId`, `PhysicalLotId`, quality lots, route/cargo reservations, and target protocol lane. |
| Lair reconciliation | `the-shrine-upgrade/.../hunting_runtime.rs:98-125` | Materializes rosters only for `TileType::EnemyLair`, respawns if ready, removes rosters for non-EnemyLair tiles. | Good `EnemyLair` vs Quarry invariant; expand roster rules. |
| Dispatch/attempt | `the-shrine-upgrade/.../hunting_runtime.rs:237-393` | Validates site/party/cap/safety, computes seed, resolves attempt, applies XP/health/deaths/equipment wear/loot/materials/trophy/outcomes. | Scalar resources/material counters and four species must be replaced. |
| Leader/player adapter | `the-shrine-upgrade/.../hunting_runtime.rs:405-610` | Delayed active party, one active party per colony, lean-food or player nudge review, health/success gates, reservation release on failure. | Adapter predates new Leader AI; LAI.45+ owns policy, LAI.42 owns pure attempt invariants. |
| Party assembly | `the-shrine-upgrade/.../hunting_runtime.rs:650-704` | Requires living, age-capable, idle/available cats, exact equipped non-broken weapon/armor. | P1-C04 resolves the formula boundary: full resolved weapon effect plus half resolved armor effect, with exact item/material/quality numbers supplied by LAI.37/43 rather than duplicated in Hunting. |
| Source sim tests | `the-shrine-upgrade/crates/cat-sim/tests/hunting_runtime.rs:85-125`, `127-223`, `226-297`, `300-362`, `364-450`, `452-548` | Lair-vs-quarry distinction, Captain/Leader authority, exact gear/wear, physical loot, respawn deadline/snapshot, `hunting_bulk`, snapshot/nudge only revealed EnemyLairs, Hole darkness gating, delayed reserved party. | Port as red cases but update resources to LAI.37 lots and twenty species. |
| Source protocol DTO | `the-shrine-upgrade/crates/cat-protocol/src/hunting_lair.rs:1-299` | Public snapshot hides seeds/rolls/stats/drop chances, uses visible sites, danger/risk/status bands, loot preview, parties/outcomes, and `NudgeHuntingSite`. | Exact level/report ecology and ten-band sprites need Plan 1 additions; target protocol lacks module. |
| Source protocol tests | `the-shrine-upgrade/crates/cat-protocol/tests/hunting_lair.rs:135-220` | Round-trip stable IDs/bands, hidden combat state rejected, targeted or general nudge. | Fixture species are non-Plan placeholders (`thorn_boar`, `stone_marten`); replace with Plan IDs. |
| Source snapshot builders | `the-shrine-upgrade/crates/cat-sim/src/actions.rs:4565-4840` | Report-safe lair projection, stable `enemy-lair:x:y` IDs, first-clear trophy, Captain advice, party/outcome snapshots, material IDs. | Four species only, no exact ten-level band art key, no selected-lair-only portrait inventory, scalar rewards. |
| Source persistence | `the-shrine-upgrade/crates/cat-server/src/persistence.rs:42-54`, `66-105`, `113-145`, `7993-8093` | Persists lairs, trophies, materials, nudges, active parties, attempt nonces, outcomes. | Persists source structs/counters; target LAI.48 must persist lots/quality/cache/clear counters/version lanes. |

## Visual and Asset Inventory

Manifest-planned assets:

| Required visual | Manifest origins | Required count/dimensions | Actual reusable protected assets | Missing for LAI.49 |
| --- | --- | --- | --- | --- |
| Public world lair sprites | `content_manifest.json:2840-2891`, `10387-10474` | Ten `art_lair_visual_01_10` through `91_100`, `80x80`, `world_base`, `lair_band`. | Only one source `public/images/game/sites/lair.png`, 32x32, SHA `7a329b100a2b72e60b15afc97bfc0b11ae242058449ebcd56d35176f725736d3`; loaded by source client `lib.rs:1218-1281`, drawn/tinted `:8233-8247`. | All ten band-specific 80x80 sprites are missing. |
| Quarry site sprite | Plan requires distinct `EnemyLair` vs Quarry (`final-hole-hunting-content-plan.md:135-136`). | Source/client expects distinct site asset. | `public/images/game/sites/quarry.png`, 32x32, SHA `a5044ef5f0bec606555081476883a580e1b276d1479e4437da0999aa0207aaf6`; source client loads it `lib.rs:1218-1281`. | Needs target asset receipt/bounds validation; not a lair band replacement. |
| Selected-lair creature portraits | `content_manifest.json:8974-9152`; Plan `final-hole-hunting-content-plan.md:132-134`, Board `:1248`. | Twenty `art_creature_*` portraits, `80x80`, `portrait`, accessibility `creature_name`, selected lair panel only. | Source has four tiny world enemy sprites under `public/images/game/enemies/{fox,badger,bear,rival_beast}.png` and old larger non-game enemy art; exact dimensions and hashes are in the protected asset receipt table below. | All twenty manifest portraits are missing as `art_creature_*`; source old fox/badger/bear may be style references/adaptation inputs only. |
| Named drop icons | Material icon registry `content_manifest.json:9568-9746`; material keys `:1431`, `1462`, `1493`, `1524`, `1555`, `1586`, `1617`, `1648`, `1679`, `1710`, `1741`, `1772`, `1803`, `1834`, `1865`, `1896`, `1927`, `1958`, `1989`, `2020`. | Twenty unique `art_material_*` icons, `16x16`, `item_material`, accessibility `content_name`. | No protected source named-drop icons found. Generic source icons exist for `bone`, `hide`, `food`, `materials`, etc., but they are not unique named drops. | All twenty named-drop icons are genuinely missing. |
| Hunter/party visualization | Source runtime plans active parties; client has hunter cat art. | Worker/rod/route/cargo visualization for LAI.40 plus hunt party display for LAI.42/50. | Source `hat-hunter.png` and `cat-hunter.png` exist; exact dimensions and hashes are in the protected asset receipt table below. Current client has hunter hat loading around `crates/cat-client/src/lib.rs:2217-2250`. | Need party/route/cache/task markers and selected-lair UI assets; no LAI.42-specific cargo/cache marker found. |

Protected asset receipt table:

| Protected source asset | Dimensions | SHA-256 | LAI.42 use |
| --- | ---: | --- | --- |
| `the-shrine-upgrade/public/images/game/sites/lair.png` | 32x32 | `7a329b100a2b72e60b15afc97bfc0b11ae242058449ebcd56d35176f725736d3` | Reusable/adaptable generic lair site, not ten-band replacement. |
| `the-shrine-upgrade/public/images/game/sites/quarry.png` | 32x32 | `a5044ef5f0bec606555081476883a580e1b276d1479e4437da0999aa0207aaf6` | Reusable/adaptable distinct Quarry receipt. |
| `the-shrine-upgrade/public/images/game/enemies/fox.png` | 16x16 | `fdd4a1c7d1372b7b1757d7820c3d3fb1e30a9e7ef790f9371bd14cef3335f290` | Source four-species tiny map/panel art; reference/adapt only. |
| `the-shrine-upgrade/public/images/game/enemies/badger.png` | 16x16 | `14e29e142ceff74e23974b5c326273dc877d9e7aff85ad911a985e923e3349ca` | Source four-species tiny map/panel art; reference/adapt only. |
| `the-shrine-upgrade/public/images/game/enemies/bear.png` | 16x16 | `92e2eaa38ccb2b8bf4d279175421e43ab2f0a8832e81d48fbb87b74e5adea61e` | Source four-species tiny map/panel art; reference/adapt only. |
| `the-shrine-upgrade/public/images/game/enemies/rival_beast.png` | 16x16 | `b457727913d38780adeaf7c78e5aec93e538d354f7d8bb28db426ad0be8c4dec` | Source non-Plan prototype species art; reference only. |
| `the-shrine-upgrade/public/images/enemies/fox.png` | 192x192 | `65b6494fdc33514eb5e8ecb20b24b616e6968ee1c07a78b07a1c4e1777c493b4` | Larger old portrait reference for Red Fox style. |
| `the-shrine-upgrade/public/images/enemies/badger.png` | 192x192 | `70d698a049ef502e85a3d053348adbe8a025d2844394e8156b1870b697d4f1f6` | Larger old portrait reference for Badger style. |
| `the-shrine-upgrade/public/images/enemies/bear.png` | 256x256 | `068ae009d9b6ab5feffc920f12429895dd7f2d6a3edfe2ee3105d2a8ab15501b` | Larger old portrait reference for Brown Bear style. |
| `the-shrine-upgrade/public/images/enemies/hawk.png` | 192x192 | `36c7f3bee0065f97c544b4ca6d78a8a920e02e4b92c9e10675de538031c604f2` | Possible Great Eagle style reference, not a species match. |
| `the-shrine-upgrade/public/images/enemies/rival_cat.png` | 128x128 | `d1b66bf408f2dbd9dd75b10f607cc41a213f20e9b3f94459c56fbd2d42e8db64` | Non-Plan enemy reference only. |
| `the-shrine-upgrade/public/images/ui/tasks/hunt.png` | 64x64 | `474960c71b0d94d823810ca316754fb6944cb4f70c8a7c29f2dd88b06bd85628` | Existing hunt task icon reference; not a lair/cache marker. |
| `the-shrine-upgrade/public/images/game/icons/bone.png` | 128x128 | `b149bb6ab6d368831d23f12340a7771ebbf61a9cd7c9684e3b7c50a4bb712892` | Generic Bone icon reference; not a named-drop icon. |
| `the-shrine-upgrade/public/images/game/icons/hide.png` | 145x147 | `0a262594d855d70e481d6df439e4234def9455e35b5166aa22ab4faba60c3fe6` | Generic Hide icon reference; not a named-drop icon. |
| `the-shrine-upgrade/public/images/cats/hat-hunter.png` | 32x32 | `4948c383791f756969f5b6d9b2b437459e83a07bd8f3816b3ebc3fb11150a881` | Hunter role/party visualization reference. |
| `the-shrine-upgrade/public/images/cats/cat-hunter.png` | 32x32 | `8d1f7f7cac83c31701adf7414419c26952726ecb5715d3ebb372bb2ebae540e9` | Hunter role/party visualization reference. |

LAI.49 missing art list after style inspection: ten world lair band sprites; twenty 80x80 selected-lair portraits for Cave Bat, Red Fox, Badger, Wild Boar, Gray Wolf, Lynx, Great Stag, Giant Serpent, Brown Bear, Great Eagle, Moon Stag, Warg, Cockatrice, Forest Troll, Griffin, Basilisk, Manticore, Chimera, Wyvern, Elder Dragon; twenty 16x16 named-drop icons; lair-cache marker; hunting party/route marker if not covered by general spatial task art.

## Report, Protocol, Persistence, UI, Diagnostics, and Cutover Consumers

| Consumer | Current/source origins | Required LAI.42 treatment |
| --- | --- | --- |
| God/Leader report parity | Plan `final-integrated-overhaul-plan.md:27-29`, `56-58`; Q&A `thread-qa-audit.md:112-114`; source protocol hides seeds/rolls/exact state `the-shrine-upgrade/crates/cat-protocol/src/hunting_lair.rs:1-7`. | Public snapshots show only report-authorized lair band/exact-level status, Captain advice, selected-lair portraits, parties, outcomes, loot lots, respawn as allowed; no hidden exact ecology or server RNG in labels/errors/logs. |
| Protocol v3 | Board `BOARD.md:1236-1238`; source DTO/action `the-shrine-upgrade/crates/cat-protocol/src/hunting_lair.rs:13-299`. | Add strict LAI.42 snapshots/actions after sim authority; reject old clients/header-first in LAI.47; action is nudge/hint only. |
| Persistence | Board `BOARD.md:1237`; source persistence receipt above; current target persistence normalizes item locations but has no lair state (`crates/cat-server/src/persistence.rs:1235-1258`). | Persist `EnemyLair` records by stable site ID, generation, level, roster, clear index, first-clear flag, absolute respawn deadline, active reservations, cache lot IDs, outcomes/report history, and no scalar material counter. |
| UI/client | Board `BOARD.md:1238`; source client `the-shrine-upgrade/crates/cat-client/src/lib.rs:8233-8250`, `11064-11076`. | Implement lair band/world sprite, selected-lair panel with twenty portraits only when selected, exact-level/report gate, party/equipment/loot quality/respawn fields, and visible distinction from quarry. |
| Diagnostics | Board `BOARD.md:1246-1247`; source tests show focused coverage. | Add bounded diagnostics for hunt gate decisions, candidate lairs, reservations, route/cargo/cache counts, respawn deadlines, and hidden-state redaction. |
| Legacy deletion/cutover | Current legacy `HuntExpedition`, `CaveEntrance` food drains, scalar `ResourceKind::Food`, source four-species `MonsterSpecies`, source scalar `SpeciesMaterial` counters. | Stage adapters until LAI.47/48/50 consume LAI.42; then delete legacy cave-hunt food path, four-species enum, scalar material inventory, generic Food/Hide/Bone rewards in hunting, and placeholder protocol/UI. |

## Smallest LAI.42 Authority Boundary

Smallest single authority: a pure `cat-sim` Hunting domain leaf that consumes LAI.36 `ContentManifest`
creature/material/art IDs and LAI.37 `QualityBand`/lot primitives, but owns only lair encounter state
and pure resolution. It should not own catalog parsing, material processing, food stats, item quality,
spatial geometry types, report projection, protocol wire structs, or persistence schema.

Minimum module surface:

- `CreatureEncounterSpec` resolved from `ContentManifest::creatures`, not duplicated as a hard-coded enum.
- `HuntingLairState { site_id, tile, level, generation, clear_index, roster, first_clear_claimed, respawn_ready_at_ms, cache_lot_ids }`.
- `HuntPartyInput { cat_id, health_percent, combat stats, exact equipped item instance refs, report authority }`.
- `plan_roster(level, world_seed, site_id, generation)` implementing six bands, 20-species eligibility, boss-plus-two, and deterministic ordering.
- `resolve_hunt(world_seed, lair_id, generation, clear_index, party, equipment, now_ms)` returning pure debits/credits: XP, injuries/deaths, equipment wear intents, quality `BulkLotKey` outputs, named material instance/lot outputs, first-clear trophy, cache requirement, absolute respawn deadline.
- Adapters later: world-tick/Leader policy, spatial route/reservation, protocol projection, persistence, UI.

No back-reference/duplicates:

- Consume `CreatureId`, `MaterialId`, `ContentId`, `ArtKey` from LAI.36.
- Consume `QualityBand`, `PhysicalLotId`, `MaterialInstanceId`, `LotLocation::Cache`, and multipliers from LAI.37.
- Consume spatial `TileType::EnemyLair`, site/route/work-slot types from spatial leaves.
- Report/UI/protocol convert from domain outputs only; they do not recalculate success, drops, quality, or respawn.

## Exhaustive Red Matrix

1. Manifest roster rejects missing, duplicate, out-of-order, or non-Plan creature IDs; every primary material must exist.
2. Species table exactly matches Plan rows for levels, common yields, named material, and Hole gate/value.
3. Level 1, 19, 20, 39, 40, 59, 60, 61, 79, 80, 94, 95, 100 exercise encounter/party bands and resolve the level-60 conflict explicitly.
4. Levels 1-39 never include mystic; 40-60 mixture permits both normal and mystic; 61-100 includes at least one mystic; 95-100 includes boss plus two supporters.
5. Autonomous attempts fail below 70% or with any hunter below 70 health; nudge fails below 45% or any hunter below 80 health; nudge never forces combat.
6. `hunting_bulk` absent caps party at one; present caps at three; extra party members reject without silent dropping.
7. Exact equipped weapon/armor instances affect success and wear; scalar Weapons/Armor resources do not count; broken/reserved/wrong-location items do not count.
8. Victory credits Hunting and Fight XP, wears equipment, creates quality Meat/Hide/Bone/material lots, sets first-clear/trophy, increments clear index, and schedules one absolute respawn deadline.
9. Failure awards failure XP, applies injury/death exactly once, creates no loot, keeps lair roster alive, releases reservations, and preserves carried/equipped items through death/cancel/restart.
10. Rare-drop quality bands produce only allowed `QualityBand` ranges; first clear guarantees strongest creature primary drop at band floor when ordinary rolls produce none.
11. Drop RNG key changes with world seed, lair ID, generation, creature ID, and clear index; unchanged inputs replay exactly.
12. Overflow creates a visible one-tile `LotLocation::Cache(site_id)` and no scalar stock laundering.
13. Respawn before deadline does nothing; at deadline creates a new deterministic generation and clears deadline; exact replenishment remains report-limited.
14. `EnemyLair` gets rosters; `CaveEntrance` remains Quarry only; non-lair and unrevealed sites reject.
15. Protocol/report projection omits server seed, RNG state, combat roll, exact hidden stats, exact drop chance, hidden exact ecology, and unrevealed sites.
16. UI/world visual shows ten-level band sprite, not exact level; selected lair shows portraits; map never roams creature sprites.
17. Persistence restart preserves roster, generation, clear index, first-clear, respawn deadline, active party, lot/cache IDs, equipment wear links, and recent report-safe outcomes.
18. Legacy `CaveEntrance` hunt food path, four-species `MonsterSpecies`, scalar material counters, and generic `Food` reward aliases are removed only after LAI.42/47/48/50 consumers pass.

## Staged Consumer Order

1. LAI.42 pure domain red tests and implementation against LAI.36/37 contracts.
2. LAI.46 spatial/world-tick integration: `EnemyLair` sites, reservations, real routes, cargo/cache conservation, death/cancel/restart.
3. LAI.45 Leader/Captain policy: report-only candidates, 70/70 and 45/80 gates, review cadence, recovery work.
4. LAI.47 protocol v3: report-safe snapshots/actions/version rejection.
5. LAI.48 persistence reset schema: lairs, rosters, clear indexes, respawn, active parties, lots/cache, outcomes.
6. LAI.49 assets: copy/adapt/generate validated band sprites, portraits, icons, cache/task markers with source hashes.
7. LAI.50 UI/accessibility: selected-lair panel, band/world visuals, party/equipment/loot/quality/respawn fields.
8. LAI.51/52 diagnostics, acceptance, and deletion receipts for old cave-hunt/scalar paths.

## Source gaps found during inventory, now closed additively

- The exact plan/source did not specify twenty-species named-drop probabilities; P1-C04 closes them with the six level-band percentages below.
- The exact plan/source did not specify a complete respawn table; P1-C04 closes it with six absolute party-band deadlines below.
- The source success formula did not resolve LAI.37/43 item effects; P1-C04 keeps the formula and delegates exact weapon/armor effects to those authorities.
- The source had two possible injury paths; P1-C04 makes the combat result the sole hunt injury/death roll.
- The exact plan did not prescribe a generated portrait/icon style; the maintained art-style inspection plus LAI.49/51 acceptance closes that delivery rule.

## Coordinator Reconciliation After Inventory

The preceding list truthfully records what the source and exact plan did not specify. It is now
closed by append-only board decision P1-C04 rather than left for an implementation worker to
invent:

- the catalog expresses the level-60/61 boundary with
  `mystic_required_from_level = None/None/None/61/80/95`;
- named-drop chance is 10/15/20/25/30/40 percent across the exact six level ranges;
- absolute respawn is 6/8/12/14/18/24 game-hours across the six party-size bands;
- roster eligibility, clamped creature levels, the mandatory mystic, and Elder Dragon
  boss-plus-two behavior are deterministic and keyed;
- fixed-point success preserves the protected `50 + party power - danger` rule while exact
  LAI.37/43 item effects supply weapon/armor power;
- the protected XP and failure-damage formulas remain, one durability wears per eligible equipped
  item per accepted attempt, and combat is the sole injury/death roll;
- Meat/Hide/Bone are quality lots, named drops retain instance identity, and overflow is a
  one-tile physical cache;
- the visual style is no longer unspecified: generation must follow
  [art-style-inspection.md](art-style-inspection.md), inspect existing same-class sprites first,
  and pass the LAI.49/51 native-dimension, transparency, palette, accessibility, and in-world
  screenshot checks.
