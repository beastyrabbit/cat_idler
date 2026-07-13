//! Production chains ported from `lib/game/production.ts`.

use crate::types::{BuildingType, CatSpecialization};

pub const WORKSHOP_MATERIALS_PER_CYCLE: f64 = 5.0;
pub const WORKSHOP_REFINED_PER_CYCLE: f64 = 1.0;
pub const WORKSHOP_CYCLE_SEC: f64 = 600.0;
/// Architects run workshops at double speed.
pub const ARCHITECT_SPEED: f64 = 2.0;

/// Passive food a single field grows per game-hour at full (1.0) crop fertility, before
/// the per-tile biome fertility multiplier (grass ~0.8, marsh ~1.5). Raised 2.0 → 3.0 as
/// part of the food-comfort fix: at the base grass fertility a field now yields ~2.4
/// food/hour, so the small, population-scaled clutch of fields the leader commissions
/// (see [`FIELD_CATS_PER_FIELD`]) is a meaningful passive floor under the lumpy hunt
/// sawtooth without needing so many plots that food goes trivial.
pub const FIELD_FOOD_PER_HOUR: f64 = 3.0;

pub const WORKSHOP_UNLOCK_LEVEL: u32 = 2;
pub const FIELD_UNLOCK_LEVEL: u32 = 4;

/// FOOD-SCALING food-comfort fix — a field is the intended food-scaling mechanism
/// (passive farm plot, `field_yield`), but it is gated behind [`FIELD_UNLOCK_LEVEL`]
/// (village level 4 = twelve completed non-shrine buildings), which a food-lean colony
/// never reaches. Left purely hunt-fed, food arrives in lumpy ~24-unit hunt deliveries
/// every 8 game-hours while consumption drains ~1/cat/hour continuously, so the larder
/// sawtooths from ~full to near-zero and rarely holds above the research-comfort bar
/// (`leader_director::is_research_comfortable`) — starving research and every
/// comfort-gated advance and still tripping `UnattendedCollapse` resets.
///
/// The leader therefore auto-commissions a small, population-scaled number of fields at
/// founding, ungated by the tree/village level — exactly mirroring how the research hut
/// was made founding-buildable (`world_tick::manage_field`). Fields are a passive floor
/// that lifts the sawtooth trough above comfort; the cap below keeps them a *supplement*
/// to hunting, never a replacement, so food never becomes trivial/infinite.
///
/// Essential passive-food base the leader builds toward regardless of food level (a cat
/// permitting), as a survival floor under the lumpy early hunt sawtooth. At the base grass
/// fertility (~0.8) a field yields ~2.4 food/hour, so two fields grow ~4.8 food/hour — a
/// meaningful cushion that (with the first hunts) keeps a food trough off zero, a mere five
/// game-minutes of which trips an `UnattendedCollapse` reset. Kept deliberately small (two,
/// not three) so it never poaches enough of the founding roster to suppress the breed/grow
/// bootstrap the long-horizon population-sustain proof guards. Over-building past the base
/// is fenced by the population-scaled cap.
pub const FIELD_MIN_COUNT: usize = 2;

/// Cats per field the leader targets beyond the essential base: the field cap is
/// `max(FIELD_MIN_COUNT, ceil(population / FIELD_CATS_PER_FIELD))`. At the base grass
/// fertility a field yields ~2.4 food/hour. Five cats/field leaves a fully-grown colony's
/// passive base still short of its consumption (~1 food/cat/hour) — hunts must carry the
/// remainder and build the comfort buffer, so food stays non-trivial while a housing-capped
/// colony can hold comfort.
pub const FIELD_CATS_PER_FIELD: f64 = 5.0;

/// Food fill-ratio at/above which the leader stops commissioning fields. Set comfortably
/// above the per-capita research-comfort bar (`leader_director::is_research_comfortable`;
/// e.g. a 15-cat colony is food-comfortable from 60/200 = 0.3) so fields accumulate until
/// the larder is comfortably — not just barely — stocked; once food is this full the
/// passive base is already doing its job and no new field is broken ground (standing
/// fields persist).
pub const FIELD_STOCK_TARGET_RATIO: f64 = 0.75;

/// Acute-crisis floor for *discretionary* fields (those beyond [`FIELD_MIN_COUNT`]): the
/// leader will not break ground on an above-the-base field while food sits below this
/// fraction of capacity. In a genuine trough a cat is worth far more out hunting than tied
/// up on an 8-hour build, so discretionary expansion is confined to the band
/// `[FIELD_BUILD_MIN_RATIO, FIELD_STOCK_TARGET_RATIO)`. The essential base ignores this
/// floor — it is precisely what ends the trough — and is self-limited instead by builder
/// availability (`select_best_cat` yields nobody when every cat is on survival work).
pub const FIELD_BUILD_MIN_RATIO: f64 = 0.25;

/// Planks AND blocks the colony must already have banked before the leader breaks ground
/// on ANY field (essential base included). A field scaffold costs 2 planks + 2 blocks
/// (`SCAFFOLD_PLANK_COST`/`SCAFFOLD_BLOCK_COST`), and those build materials come from the
/// staffed wood-cutter/stone-prep chain the founding colony needs to fund its dens. Fields
/// are food *scaling*, and must be strictly ADDITIVE — never a tax on the critical
/// material economy — so a field is only commissioned out of a genuine build-material
/// surplus (buffer well above the scaffold cost, leaving plenty for the next den after the
/// field takes its share). Below this buffer the material chain keeps its cats and its
/// output, and no field is ordered.
pub const FIELD_MATERIAL_BUFFER: f64 = 4.0;

/// Passive per-cat fibre forage (P16/P19 clothing chain slice): cats picking up
/// wayside plant fibre while going about their day, independent of any building,
/// job, or worker assignment — mirrors `field_yield`'s "elapsed time -> yield"
/// shape but scales with living population instead of a building count. Kept
/// deliberately small and background; a dedicated fibre gather-spot/job is a
/// separate, larger card (P16 "Gather spots") and out of scope here.
pub const FIBRE_FORAGE_PER_CAT_PER_HOUR: f64 = 0.05;

/// Passive fibre gained this tick from `alive_cat_count` cats foraging in the
/// background over `elapsed_sec` seconds. Negative/NaN-safe like `field_yield`.
#[must_use]
pub fn fibre_forage_yield(alive_cat_count: f64, elapsed_sec: f64) -> f64 {
    js_max(0.0, alive_cat_count) * js_max(0.0, elapsed_sec) / 3600.0 * FIBRE_FORAGE_PER_CAT_PER_HOUR
}

// --- P12.4b raw-material refinement chains (P16 blueprint workshops) ---
//
// The wood-cutter and stone-prep shops share the refinement-workshop cadence
// (5 raw materials → 1 refined unit / 600s), so both reuse [`advance_workshop`]
// at the tick site — crediting planks and blocks respectively instead of the
// generic `refined` good. The woodworking shop is a two-input crafter
// (planks + blocks → tools) and has its own [`advance_woodworking`] cycle,
// mirroring the smithy's twin-input shape.

/// Raw materials one wood-cutter cycle consumes (aliased to the workshop rate).
pub const WOODCUTTER_MATERIALS_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Planks one wood-cutter cycle produces.
pub const WOODCUTTER_PLANKS_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;
/// Raw materials one stone-prep cycle consumes.
pub const STONEPREP_MATERIALS_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Blocks one stone-prep cycle produces.
pub const STONEPREP_BLOCKS_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;

/// Raw fibre one clothier refine cycle consumes (P16/P19 clothing chain slice,
/// aliased to the same refinement-workshop rate as the wood-cutter/stone-prep
/// benches — see the module doc above).
pub const CLOTHIER_FIBRE_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Cloth one clothier refine cycle produces.
pub const CLOTHIER_CLOTH_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;
/// Raw hide one tannery refine cycle consumes.
pub const TANNERY_HIDE_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Leather one tannery refine cycle produces.
pub const TANNERY_LEATHER_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;

/// Raw ore one smelter refine cycle consumes (P17/P19 ore→metal chain, aliased to the
/// same refinement-workshop rate as the wood-cutter/stone-prep/clothier/tannery
/// benches above). Ore only ever comes from mountain quarrying
/// (`world_tick::credit_quarry_ore`), so a colony that never reaches the mountains
/// simply never accumulates ore and this bench sits permanently idle — additive/inert.
pub const SMELTER_ORE_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Metal bars one smelter refine cycle produces.
pub const SMELTER_METAL_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;

/// Planks one woodworking cycle consumes.
pub const WOODWORKING_PLANKS_PER_CYCLE: f64 = 2.0;
/// Blocks one woodworking cycle consumes.
pub const WOODWORKING_BLOCKS_PER_CYCLE: f64 = 2.0;
/// Tools one woodworking cycle forges.
pub const WOODWORKING_TOOLS_PER_CYCLE: f64 = 1.0;
/// Seconds of work one full woodworking cycle takes.
pub const WOODWORKING_CYCLE_SEC: f64 = 600.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkshopOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub materials_available: f64,
}

impl WorkshopOptions {
    #[must_use]
    pub const fn new(
        has_worker: bool,
        worker_is_architect: bool,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker,
            worker_is_architect,
            materials_available,
        }
    }

    #[must_use]
    pub const fn from_worker(
        worker_specialization: Option<CatSpecialization>,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker: worker_specialization.is_some(),
            worker_is_architect: worker_is_architect(worker_specialization),
            materials_available,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkshopStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Materials consumed this tick.
    pub materials_used: f64,
    /// Refined goods produced this tick.
    pub refined_produced: f64,
}

#[must_use]
pub const fn workshop_unlocked(village_level: u32) -> bool {
    village_level >= WORKSHOP_UNLOCK_LEVEL
}

#[must_use]
pub const fn field_unlocked(village_level: u32) -> bool {
    village_level >= FIELD_UNLOCK_LEVEL
}

#[must_use]
pub const fn building_unlocked(building_type: BuildingType, village_level: u32) -> bool {
    match building_type {
        BuildingType::Workshop => workshop_unlocked(village_level),
        BuildingType::Field => field_unlocked(village_level),
        _ => true,
    }
}

#[must_use]
pub const fn worker_is_architect(worker_specialization: Option<CatSpecialization>) -> bool {
    matches!(worker_specialization, Some(CatSpecialization::Architect))
}

#[must_use]
pub fn advance_workshop(
    progress_sec: f64,
    elapsed_sec: f64,
    options: WorkshopOptions,
) -> WorkshopStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return WorkshopStep {
            next_progress: progress_sec,
            materials_used: 0.0,
            refined_produced: 0.0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    let cycles_by_time = (progress / WORKSHOP_CYCLE_SEC).floor();
    let cycles_by_materials = (options.materials_available / WORKSHOP_MATERIALS_PER_CYCLE).floor();
    let cycles = js_max(0.0, js_min(cycles_by_time, cycles_by_materials));

    progress -= cycles * WORKSHOP_CYCLE_SEC;
    progress = js_min(progress, WORKSHOP_CYCLE_SEC);

    WorkshopStep {
        next_progress: progress,
        materials_used: cycles * WORKSHOP_MATERIALS_PER_CYCLE,
        refined_produced: cycles * WORKSHOP_REFINED_PER_CYCLE,
    }
}

#[must_use]
pub fn field_yield(elapsed_sec: f64) -> f64 {
    js_max(0.0, elapsed_sec / 3600.0 * FIELD_FOOD_PER_HOUR)
}

/// Inputs for one woodworking tick: whether a worker is present/fast and how many
/// planks and blocks are on hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodworkingOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub planks_available: f64,
    pub blocks_available: f64,
}

impl WoodworkingOptions {
    #[must_use]
    pub const fn new(
        has_worker: bool,
        worker_is_architect: bool,
        planks_available: f64,
        blocks_available: f64,
    ) -> Self {
        Self {
            has_worker,
            worker_is_architect,
            planks_available,
            blocks_available,
        }
    }

    #[must_use]
    pub const fn from_worker(
        worker_specialization: Option<CatSpecialization>,
        planks_available: f64,
        blocks_available: f64,
    ) -> Self {
        Self {
            has_worker: worker_specialization.is_some(),
            worker_is_architect: worker_is_architect(worker_specialization),
            planks_available,
            blocks_available,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodworkingStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Planks consumed this tick.
    pub planks_used: f64,
    /// Blocks consumed this tick.
    pub blocks_used: f64,
    /// Tools produced this tick.
    pub tools_produced: f64,
}

/// Advance a staffed woodworking shop: planks + blocks → tools, floored by
/// elapsed time and the scarcer of the two inputs. Mirrors the smithy's
/// twin-input cadence; architects work the bench at double speed.
#[must_use]
pub fn advance_woodworking(
    progress_sec: f64,
    elapsed_sec: f64,
    options: WoodworkingOptions,
) -> WoodworkingStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return WoodworkingStep {
            next_progress: progress_sec,
            planks_used: 0.0,
            blocks_used: 0.0,
            tools_produced: 0.0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    let cycles_by_time = (progress / WOODWORKING_CYCLE_SEC).floor();
    let cycles_by_planks = (options.planks_available / WOODWORKING_PLANKS_PER_CYCLE).floor();
    let cycles_by_blocks = (options.blocks_available / WOODWORKING_BLOCKS_PER_CYCLE).floor();
    let cycles = js_max(
        0.0,
        js_min(cycles_by_time, js_min(cycles_by_planks, cycles_by_blocks)),
    );

    progress -= cycles * WOODWORKING_CYCLE_SEC;
    progress = js_min(progress, WOODWORKING_CYCLE_SEC);

    WoodworkingStep {
        next_progress: progress,
        planks_used: cycles * WOODWORKING_PLANKS_PER_CYCLE,
        blocks_used: cycles * WOODWORKING_BLOCKS_PER_CYCLE,
        tools_produced: cycles * WOODWORKING_TOOLS_PER_CYCLE,
    }
}

/// Max worker occupancy for `building_type` (its worker slots), for the client
/// building-inspector panel (`BuildingSnapshot::staff_cap`).
///
/// The sim currently models a single worker slot per production building —
/// `BuildingRuntime::assigned_cat` is an `Option<CatId>`, not a list — so this is 1 for
/// exactly the building types staffed through `buildings_needing_workers` in
/// `world_tick.rs`: the auto-staff mop-up for the generic workshop and the P16
/// raw-material benches (wood-cutter, stone-prep, woodworking), plus the
/// leader-directed smithy queue (`smithy_queue` in `phase_20`-era planning).
/// Every other building type has no worker-slot concept: dens/storage/walls/etc.
/// never take an `assigned_cat`, and fields produce passively via `field_yield`
/// with no worker check at all (see `phase_23_production`'s `BuildingType::Field`
/// arm, which adds yield unconditionally).
#[must_use]
pub const fn building_staff_cap(building_type: BuildingType) -> u32 {
    match building_type {
        BuildingType::Workshop
        | BuildingType::WoodCutter
        | BuildingType::StonePrep
        | BuildingType::Woodworking
        | BuildingType::Smithy
        | BuildingType::Clothier
        | BuildingType::Tannery
        // A research hut seats one scholar; it accrues upgrade-tree research points rather
        // than a stockpile resource, so it has no `building_cycle_sec`/output label. The
        // school (unlocked by the "school" upgrade node) is a second research building
        // and shares the same one-scholar staffing shape.
        | BuildingType::ResearchHut
        | BuildingType::School
        | BuildingType::Smelter => 1,
        BuildingType::Den
        | BuildingType::FoodStorage
        | BuildingType::WaterBowl
        | BuildingType::Beds
        | BuildingType::HerbGarden
        | BuildingType::Nursery
        | BuildingType::ElderCorner
        | BuildingType::Walls
        | BuildingType::MouseFarm
        | BuildingType::Shrine
        | BuildingType::Field
        | BuildingType::Barracks
        | BuildingType::AccountingTent => 0,
    }
}

/// Length of one production cycle, in seconds, for building types that craft on a
/// timer — `BuildingRuntime::production_progress` accumulates elapsed seconds toward
/// this in `phase_23_production` (see `advance_workshop`/`advance_woodworking` above
/// and `smithy::advance_smithy`). `None` for building types with no timed cycle,
/// including fields: `field_yield` adds food continuously every tick, with no cycle
/// to complete (`production_progress` is simply never touched for a field).
#[must_use]
pub const fn building_cycle_sec(building_type: BuildingType) -> Option<f64> {
    match building_type {
        BuildingType::Workshop
        | BuildingType::WoodCutter
        | BuildingType::StonePrep
        | BuildingType::Clothier
        | BuildingType::Tannery
        | BuildingType::Smelter => Some(WORKSHOP_CYCLE_SEC),
        BuildingType::Woodworking => Some(WOODWORKING_CYCLE_SEC),
        BuildingType::Smithy => Some(crate::smithy::SMITHY_CYCLE_SEC),
        _ => None,
    }
}

/// Short, stable, lowercase label of what `building_type` produces, for the client
/// building-inspector panel (`BuildingSnapshot::production_output`). `None` if the
/// building type doesn't craft a resource.
///
/// Verified against the actual `phase_23_production` recipes in `world_tick.rs`:
/// workshop → refined (materials → refined, this module's `advance_workshop`),
/// wood-cutter → plank (materials → planks, same `advance_workshop` cadence credited
/// to `planks`), stone-prep → block (materials → blocks, credited to `blocks`),
/// woodworking → tool (planks + blocks → tools, `advance_woodworking`), smithy →
/// weapon+armor (refined + materials → 1 weapon *and* 1 armor per cycle,
/// `smithy::advance_smithy`/`SMITHY_WEAPONS_PER_CYCLE`/`SMITHY_ARMOR_PER_CYCLE`, both
/// 1.0), field → food (`field_yield`, passive, no worker/cycle). Every other building
/// type (den, storage, walls, mouse farm, shrine, barracks, accounting tent, etc.)
/// crafts nothing and reports `None`.
#[must_use]
pub const fn building_output_label(building_type: BuildingType) -> Option<&'static str> {
    match building_type {
        BuildingType::Workshop => Some("refined"),
        BuildingType::WoodCutter => Some("plank"),
        BuildingType::StonePrep => Some("block"),
        BuildingType::Woodworking => Some("tool"),
        BuildingType::Smithy => Some("weapon+armor"),
        BuildingType::Field => Some("food"),
        BuildingType::Clothier => Some("cloth"),
        BuildingType::Tannery => Some("leather"),
        BuildingType::Smelter => Some("metal"),
        BuildingType::Den
        | BuildingType::FoodStorage
        | BuildingType::WaterBowl
        | BuildingType::Beds
        | BuildingType::HerbGarden
        | BuildingType::Nursery
        | BuildingType::ElderCorner
        | BuildingType::Walls
        | BuildingType::MouseFarm
        | BuildingType::Shrine
        | BuildingType::Barracks
        | BuildingType::ResearchHut
        | BuildingType::School
        | BuildingType::AccountingTent => None,
    }
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left <= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FIELD_FOOD_PER_HOUR, FIELD_UNLOCK_LEVEL, WORKSHOP_CYCLE_SEC, WORKSHOP_MATERIALS_PER_CYCLE,
        WORKSHOP_REFINED_PER_CYCLE, WORKSHOP_UNLOCK_LEVEL, WorkshopOptions, WorkshopStep,
        advance_workshop, building_unlocked, field_unlocked, field_yield, worker_is_architect,
        workshop_unlocked,
    };
    use crate::types::{BuildingType, CatSpecialization};

    fn options(
        has_worker: bool,
        worker_is_architect: bool,
        materials_available: f64,
    ) -> WorkshopOptions {
        WorkshopOptions {
            has_worker,
            worker_is_architect,
            materials_available,
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_step_bits(actual: WorkshopStep, expected: WorkshopStep, label: &str) {
        assert_f64_bits(
            actual.next_progress,
            expected.next_progress,
            &format!("{label} next_progress"),
        );
        assert_f64_bits(
            actual.materials_used,
            expected.materials_used,
            &format!("{label} materials_used"),
        );
        assert_f64_bits(
            actual.refined_produced,
            expected.refined_produced,
            &format!("{label} refined_produced"),
        );
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_f64_bits(WORKSHOP_MATERIALS_PER_CYCLE, 5.0, "materials per cycle");
        assert_f64_bits(WORKSHOP_REFINED_PER_CYCLE, 1.0, "refined per cycle");
        assert_f64_bits(WORKSHOP_CYCLE_SEC, 600.0, "workshop cycle seconds");
        // Diverged from the TS export (2.0) by the food-comfort fix — a field now grows
        // 3.0 food/game-hour at full fertility so the leader's small clutch of passive
        // plots is a meaningful floor under the hunt sawtooth.
        assert_f64_bits(FIELD_FOOD_PER_HOUR, 3.0, "field food per hour");
        assert_eq!(WORKSHOP_UNLOCK_LEVEL, 2);
        assert_eq!(FIELD_UNLOCK_LEVEL, 4);
    }

    #[test]
    fn unlocks_use_inclusive_village_level_thresholds() {
        assert!(!workshop_unlocked(0));
        assert!(!workshop_unlocked(1));
        assert!(workshop_unlocked(2));
        assert!(workshop_unlocked(3));

        assert!(!field_unlocked(3));
        assert!(field_unlocked(4));
        assert!(field_unlocked(5));
    }

    #[test]
    fn typed_unlock_helper_gates_only_production_buildings() {
        assert!(!building_unlocked(BuildingType::Workshop, 1));
        assert!(building_unlocked(BuildingType::Workshop, 2));
        assert!(!building_unlocked(BuildingType::Field, 3));
        assert!(building_unlocked(BuildingType::Field, 4));
        assert!(building_unlocked(BuildingType::Den, 0));
    }

    #[test]
    fn typed_worker_helper_identifies_architects() {
        assert!(!worker_is_architect(None));
        assert!(!worker_is_architect(Some(CatSpecialization::Hunter)));
        assert!(worker_is_architect(Some(CatSpecialization::Architect)));

        assert_eq!(
            WorkshopOptions::from_worker(Some(CatSpecialization::Architect), 25.0),
            options(true, true, 25.0)
        );
        assert_eq!(
            WorkshopOptions::from_worker(None, 25.0),
            options(false, false, 25.0)
        );
    }

    #[test]
    fn workshop_does_not_progress_without_worker_or_positive_elapsed_time() {
        assert_step_bits(
            advance_workshop(123.0, 600.0, options(false, false, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "without worker",
        );

        assert_step_bits(
            advance_workshop(123.0, 0.0, options(true, true, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "zero elapsed",
        );

        assert_step_bits(
            advance_workshop(123.0, -1.0, options(true, true, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative elapsed",
        );
    }

    #[test]
    fn workshop_accumulates_short_ticks_and_converts_complete_cycles() {
        assert_step_bits(
            advance_workshop(590.0, 30.0, options(true, false, 10.0)),
            WorkshopStep {
                next_progress: 20.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "one completed cycle",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 15.0)),
            WorkshopStep {
                next_progress: 0.0,
                materials_used: 15.0,
                refined_produced: 3.0,
            },
            "three completed cycles",
        );
    }

    #[test]
    fn architect_worker_advances_workshop_at_double_speed() {
        assert_step_bits(
            advance_workshop(0.0, 300.0, options(true, true, 5.0)),
            WorkshopStep {
                next_progress: 0.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "architect cycle",
        );

        assert_step_bits(
            advance_workshop(100.0, 125.0, options(true, true, 0.0)),
            WorkshopStep {
                next_progress: 350.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "architect partial progress",
        );
    }

    #[test]
    fn workshop_cycles_are_limited_by_available_materials() {
        assert_step_bits(
            advance_workshop(590.0, 30.0, options(true, false, 0.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "stalls at one full cycle without materials",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 5.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "insufficient materials cap banked progress",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 14.9)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 10.0,
                refined_produced: 2.0,
            },
            "fractional materials are floored to complete cycles",
        );
    }

    #[test]
    fn workshop_preserves_typescript_edge_cases_for_negative_progress_and_materials() {
        assert_step_bits(
            advance_workshop(-700.0, 100.0, options(true, false, 50.0)),
            WorkshopStep {
                next_progress: -600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative progress is not clamped upward",
        );

        assert_step_bits(
            advance_workshop(700.0, 0.1, options(true, false, -1.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative materials cannot create negative cycles",
        );
    }

    #[test]
    fn field_yield_is_passive_food_per_elapsed_window() {
        assert_f64_bits(field_yield(0.0), 0.0, "zero elapsed");
        assert_f64_bits(field_yield(-60.0), 0.0, "negative elapsed");
        // 3.0 food/game-hour at full fertility (food-comfort fix).
        assert_f64_bits(field_yield(1_800.0), 1.5, "half hour");
        assert_f64_bits(field_yield(3_600.0), 3.0, "one hour");
        assert_f64_bits(field_yield(5_400.0), 4.5, "one and a half hours");
    }

    #[test]
    fn fibre_forage_yield_scales_with_population_and_elapsed_time() {
        assert_f64_bits(super::fibre_forage_yield(5.0, 0.0), 0.0, "zero elapsed");
        assert_f64_bits(
            super::fibre_forage_yield(0.0, 3_600.0),
            0.0,
            "no living cats",
        );
        assert_f64_bits(
            super::fibre_forage_yield(-3.0, 3_600.0),
            0.0,
            "negative population clamps to zero",
        );
        assert_f64_bits(
            super::fibre_forage_yield(5.0, -3_600.0),
            0.0,
            "negative elapsed clamps to zero",
        );
        // 5 cats * 0.05 fibre/cat/hour * 1 hour = 0.25.
        assert_f64_bits(super::fibre_forage_yield(5.0, 3_600.0), 0.25, "one hour");
        // Doubling either population or elapsed time doubles the yield.
        assert_f64_bits(super::fibre_forage_yield(10.0, 3_600.0), 0.5, "double pop");
        assert_f64_bits(super::fibre_forage_yield(5.0, 7_200.0), 0.5, "double time");
    }

    #[test]
    fn nan_inputs_match_math_max_min_propagation() {
        let workshop_step = advance_workshop(f64::NAN, 1.0, options(true, false, 5.0));
        assert!(workshop_step.next_progress.is_nan());
        assert!(workshop_step.materials_used.is_nan());
        assert!(workshop_step.refined_produced.is_nan());

        assert!(field_yield(f64::NAN).is_nan());
    }

    fn wood_options(
        has_worker: bool,
        worker_is_architect: bool,
        planks_available: f64,
        blocks_available: f64,
    ) -> super::WoodworkingOptions {
        super::WoodworkingOptions {
            has_worker,
            worker_is_architect,
            planks_available,
            blocks_available,
        }
    }

    fn assert_wood_step_bits(
        actual: super::WoodworkingStep,
        expected: super::WoodworkingStep,
        label: &str,
    ) {
        assert_f64_bits(
            actual.next_progress,
            expected.next_progress,
            &format!("{label} next_progress"),
        );
        assert_f64_bits(
            actual.planks_used,
            expected.planks_used,
            &format!("{label} planks_used"),
        );
        assert_f64_bits(
            actual.blocks_used,
            expected.blocks_used,
            &format!("{label} blocks_used"),
        );
        assert_f64_bits(
            actual.tools_produced,
            expected.tools_produced,
            &format!("{label} tools_produced"),
        );
    }

    #[test]
    fn woodworking_chain_constants_match_the_two_plus_two_recipe() {
        assert_f64_bits(super::WOODWORKING_PLANKS_PER_CYCLE, 2.0, "planks per cycle");
        assert_f64_bits(super::WOODWORKING_BLOCKS_PER_CYCLE, 2.0, "blocks per cycle");
        assert_f64_bits(super::WOODWORKING_TOOLS_PER_CYCLE, 1.0, "tools per cycle");
        assert_f64_bits(super::WOODWORKING_CYCLE_SEC, 600.0, "cycle seconds");
        // The wood-cutter / stone-prep aliases inherit the refinement-workshop rate.
        assert_f64_bits(super::WOODCUTTER_MATERIALS_PER_CYCLE, 5.0, "woodcutter in");
        assert_f64_bits(super::WOODCUTTER_PLANKS_PER_CYCLE, 1.0, "woodcutter out");
        assert_f64_bits(super::STONEPREP_MATERIALS_PER_CYCLE, 5.0, "stoneprep in");
        assert_f64_bits(super::STONEPREP_BLOCKS_PER_CYCLE, 1.0, "stoneprep out");
    }

    #[test]
    fn woodworking_only_crafts_with_a_worker_and_positive_time() {
        assert_wood_step_bits(
            super::advance_woodworking(123.0, 600.0, wood_options(false, false, 50.0, 50.0)),
            super::WoodworkingStep {
                next_progress: 123.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "without worker",
        );
        assert_wood_step_bits(
            super::advance_woodworking(123.0, 0.0, wood_options(true, false, 50.0, 50.0)),
            super::WoodworkingStep {
                next_progress: 123.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "zero elapsed",
        );
    }

    #[test]
    fn woodworking_converts_complete_cycles_and_floors_by_scarcer_input() {
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 4.0, 4.0)),
            super::WoodworkingStep {
                next_progress: 20.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "one completed cycle",
        );
        // Blocks are the bottleneck: only one cycle's worth on hand.
        assert_wood_step_bits(
            super::advance_woodworking(0.0, 1_800.0, wood_options(true, false, 20.0, 2.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "blocks cap banked progress",
        );
        // Architects run the bench at double speed.
        assert_wood_step_bits(
            super::advance_woodworking(0.0, 300.0, wood_options(true, true, 2.0, 2.0)),
            super::WoodworkingStep {
                next_progress: 0.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "architect cycle",
        );
    }

    #[test]
    fn woodworking_stalls_without_inputs() {
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 0.0, 5.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "no planks",
        );
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 5.0, 0.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "no blocks",
        );
    }

    #[test]
    fn building_output_label_matches_every_verified_recipe() {
        use BuildingType::{
            AccountingTent, Barracks, Beds, Clothier, Den, ElderCorner, Field, FoodStorage,
            HerbGarden, MouseFarm, Nursery, Shrine, Smelter, Smithy, StonePrep, Tannery, Walls,
            WaterBowl, WoodCutter, Woodworking, Workshop,
        };

        // Producing types: label matches the resource actually credited in
        // `phase_23_production` (workshop -> refined, wood-cutter -> planks,
        // stone-prep -> blocks, woodworking -> tools, smithy -> 1 weapon + 1 armor,
        // field -> food, clothier -> cloth, tannery -> leather, smelter -> metal).
        assert_eq!(super::building_output_label(Workshop), Some("refined"));
        assert_eq!(super::building_output_label(WoodCutter), Some("plank"));
        assert_eq!(super::building_output_label(StonePrep), Some("block"));
        assert_eq!(super::building_output_label(Woodworking), Some("tool"));
        assert_eq!(super::building_output_label(Smithy), Some("weapon+armor"));
        assert_eq!(super::building_output_label(Field), Some("food"));
        assert_eq!(super::building_output_label(Clothier), Some("cloth"));
        assert_eq!(super::building_output_label(Tannery), Some("leather"));
        assert_eq!(super::building_output_label(Smelter), Some("metal"));

        // Non-producing types: no phase_23 arm credits a resource for these.
        for building_type in [
            Den,
            FoodStorage,
            WaterBowl,
            Beds,
            HerbGarden,
            Nursery,
            ElderCorner,
            Walls,
            MouseFarm,
            Shrine,
            Barracks,
            AccountingTent,
        ] {
            assert_eq!(
                super::building_output_label(building_type),
                None,
                "{building_type:?} should not report a production output"
            );
        }
    }

    #[test]
    fn building_staff_cap_is_one_only_for_the_single_worker_slot_benches() {
        for building_type in [
            BuildingType::Workshop,
            BuildingType::WoodCutter,
            BuildingType::StonePrep,
            BuildingType::Woodworking,
            BuildingType::Smithy,
            BuildingType::Clothier,
            BuildingType::Tannery,
            BuildingType::Smelter,
        ] {
            assert_eq!(
                super::building_staff_cap(building_type),
                1,
                "{building_type:?} should have a 1-cat worker slot"
            );
        }

        // No worker-slot concept, including `Field` (passive, unstaffed yield).
        for building_type in [
            BuildingType::Den,
            BuildingType::FoodStorage,
            BuildingType::WaterBowl,
            BuildingType::Beds,
            BuildingType::HerbGarden,
            BuildingType::Nursery,
            BuildingType::ElderCorner,
            BuildingType::Walls,
            BuildingType::MouseFarm,
            BuildingType::Shrine,
            BuildingType::Field,
            BuildingType::Barracks,
            BuildingType::AccountingTent,
        ] {
            assert_eq!(
                super::building_staff_cap(building_type),
                0,
                "{building_type:?} should have no worker slot"
            );
        }
    }

    #[test]
    fn building_cycle_sec_matches_each_chains_configured_cadence() {
        assert_eq!(
            super::building_cycle_sec(BuildingType::Workshop),
            Some(600.0)
        );
        assert_eq!(
            super::building_cycle_sec(BuildingType::WoodCutter),
            Some(600.0)
        );
        assert_eq!(
            super::building_cycle_sec(BuildingType::StonePrep),
            Some(600.0)
        );
        assert_eq!(
            super::building_cycle_sec(BuildingType::Woodworking),
            Some(600.0)
        );
        assert_eq!(super::building_cycle_sec(BuildingType::Smithy), Some(900.0));
        assert_eq!(
            super::building_cycle_sec(BuildingType::Smelter),
            Some(600.0)
        );
        // Field has no timed cycle: yield is continuous (`field_yield`).
        assert_eq!(super::building_cycle_sec(BuildingType::Field), None);
        assert_eq!(super::building_cycle_sec(BuildingType::Shrine), None);
    }

    #[test]
    fn smelter_constants_mirror_the_workshop_refine_rate() {
        // P17/P19 ore→metal chain: the smelter reuses `advance_workshop` at the same
        // 5:1/600s rate as the wood-cutter/stone-prep benches.
        assert_f64_bits(super::SMELTER_ORE_PER_CYCLE, 5.0, "ore per cycle");
        assert_f64_bits(super::SMELTER_METAL_PER_CYCLE, 1.0, "metal per cycle");
    }
}
