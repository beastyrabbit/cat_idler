using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public sealed class Scenario
    {
        public string Name; public Action Run;
        public Scenario(string name, Action run) { Name = name; Run = run; }
    }
    /// <summary>Shared executable acceptance: all fixture credit is established before actions/time begin.</summary>
    public static class AcceptanceScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (bool personal in new[] { false, true })
            {
                bool own = personal;
                yield return new Scenario("regression.layout_shrine_ring_" + (own ? "personal" : "communal"), () => FoundingLayout(own, false));
                yield return new Scenario("regression.layout_building_roads_" + (own ? "personal" : "communal"), () => FoundingLayout(own, true));
            }
            yield return new Scenario("regression.layout_four_exits_reach_finite_sources", FoundingExits);
            yield return new Scenario("regression.layout_disconnected_building_rejected", DisconnectedBuilding);
            foreach (string housing in new[] { "automatic", "leader_plan_house", "build_house" })
            {
                string mode = housing;
                yield return new Scenario("regression.review_housing_skips_disconnected_" + mode, () => HousingConnectedSite(mode));
            }
            foreach (string obstruction in new[] { "field", "entrance", "gather" })
            {
                string kind = obstruction;
                yield return new Scenario("regression.review_expansion_preserves_" + kind, () => ExpansionFootprint(kind, false));
            }
            yield return new Scenario("regression.review_expansion_rechecks_field", () => ExpansionFootprint("field", true));
            yield return new Scenario("regression.review_expansion_resumes_after_gather_removal", () => ExpansionFootprint("gather", true));
            foreach (string site in new[] { "footprint", "entrance", "alternate_entrance", "gather" })
            {
                string placement = site;
                yield return new Scenario("regression.review_expansion_reserves_" + placement, () => PendingExpansionPlacement(placement));
            }
            foreach (string order in new[] { "field_first", "expansion_first", "alternate_route", "loaded_conflict" })
            {
                string sequence = order;
                yield return new Scenario("regression.review_expansion_upstream_road_" + sequence, () => ExpansionRoadConnection(sequence));
            }
            foreach (string order in new[] { "field_first", "expansion_first", "alternate_route", "loaded_conflict" })
            {
                string sequence = order;
                yield return new Scenario("regression.review_expansion_farm_edge_" + sequence, () => ExpansionFarmEdge(sequence));
            }
            foreach (string shore in new[] { "road", "entrance", "pending_wall", "duplicate", "unmapped", "legal_owned", "legal_exterior" })
            {
                string placement = shore;
                yield return new Scenario("regression.review_fishing_placement_" + placement, () => FishingPlacement(placement));
            }
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.review_expansion_reserves_linear_" + (track ? "rail" : "road"), () => PendingExpansionLinear(track, true));
                yield return new Scenario("regression.review_expansion_respects_linear_" + (track ? "rail" : "road"), () => PendingExpansionLinear(track, false));
            }
            yield return new Scenario("regression.layout_shrine_ring_reserved", ShrineRingReserved);
            yield return new Scenario("regression.layout_pending_entrance_preserves_work", PendingEntrance);
            yield return new Scenario("regression.layout_new_building_uses_door", BuildingDoor);
            yield return new Scenario("regression.layout_isolated_road_cannot_extend", IsolatedRoad);
            yield return new Scenario("regression.layout_paid_road_opens_building", PaidBuildingRoad);
            yield return new Scenario("regression.layout_legacy_geometry_and_roads_preserved", LegacyLayout);
            yield return new Scenario("regression.layout_avoid_cannot_close_shrine_access", AvoidShrineAccess);
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_reserves_footprint", () => PendingRoadFootprint(track, false));
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_rechecks_footprint", () => PendingRoadFootprint(track, true));
            }
            yield return new Scenario("regression.buildable_founding_services", BuildableServices);
            yield return new Scenario("regression.cargo_critical_need_conservation", CargoNeed);
            yield return new Scenario("regression.busy_cat_job_ownership", BusyCat);
            yield return new Scenario("regression.direct_control_route_ownership", RouteControl);
            yield return new Scenario("regression.item_only_stockpile_removal", ItemPile);
            yield return new Scenario("regression.farm_worker_replacement", FarmReplacement);
            yield return new Scenario("regression.preserved_food_is_edible", PreservedFood);
            yield return new Scenario("regression.emergency_staffed_water", EmergencyWater);
            yield return new Scenario("regression.undefended_raid_has_consequences", RaidConsequences);
            yield return new Scenario("regression.production_expertise_affects_progress", Expertise);
            yield return new Scenario("regression.accountant_skips_unreachable_pile", Accounting);
            yield return new Scenario("regression.mountain_research_opens_route", Mountain);
            yield return new Scenario("regression.station_capacity_study_changes_headroom", StationCapacity);
            yield return new Scenario("regression.pregnancy_beds_no_overbooking", Beds);
            yield return new Scenario("regression.functional_weapon_and_armor", Equipment);
            yield return new Scenario("regression.gather_and_avoid_zones", Zones);
            yield return new Scenario("regression.resource_targeted_scout", TargetedScout);
            yield return new Scenario("regression.selected_quarry_resource", QuarryResource);
            yield return new Scenario("regression.medicine_and_brew_consumption", ProcessedNeeds);
            yield return new Scenario("regression.direct_control_leaves_recoverable_cargo", ControlledCargo);
            yield return new Scenario("regression.review_scaffold_preserves_controlled_food", () => ControlledScaffoldCargo("food"));
            yield return new Scenario("regression.review_scaffold_preserves_surplus_planks", () => ControlledScaffoldCargo("planks"));
            yield return new Scenario("regression.review_new_building_preserves_controlled_food", () => ControlledScaffoldCargo("food", true));
            yield return new Scenario("regression.review_new_building_preserves_surplus_planks", () => ControlledScaffoldCargo("planks", true));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_offering", () => ControlledNewJobCargo("offering"));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_scouting", () => ControlledNewJobCargo("scout"));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_hauling", () => ControlledNewJobCargo("haul"));
            yield return new Scenario("regression.review_resume_road_preserves_controlled_food", () => ControlledResumeCargo("road"));
            yield return new Scenario("regression.review_resume_rail_preserves_controlled_food", () => ControlledResumeCargo("rail"));
            yield return new Scenario("regression.review_resume_expansion_preserves_controlled_food", () => ControlledResumeCargo("expand"));
            yield return new Scenario("regression.review_armor_production_accepting_storage", ArmorProductionStorage);
            yield return new Scenario("regression.review_armor_unequip_accepting_storage", ArmorUnequipStorage);
            yield return new Scenario("regression.exact_haul_produced_mug_remove_recover_sell", () => ProducedItemRecovery(false));
            yield return new Scenario("regression.exact_haul_cancelled_production_recover_sell", () => ProducedItemRecovery(true));
            yield return new Scenario("regression.exact_haul_cancel_before_pickup", () => ExactHaulInterruption(false, false));
            yield return new Scenario("regression.exact_haul_cancel_after_pickup", () => ExactHaulInterruption(true, false));
            yield return new Scenario("regression.exact_haul_death_before_pickup", () => ExactHaulInterruption(false, true));
            yield return new Scenario("regression.exact_haul_death_after_pickup", () => ExactHaulInterruption(true, true));
            yield return new Scenario("regression.exact_haul_claims_and_full_storage", ExactHaulClaims);
            yield return new Scenario("regression.exact_haul_steward_recovers_item_only_spill", ExactHaulSteward);
            yield return new Scenario("regression.exact_haul_source_capacity_until_pickup", ExactHaulSourceCapacity);
            yield return new Scenario("regression.exact_haul_between_existing_stores", ExactHaulStoredTransfer);
            foreach (var infrastructure in new[] { "road", "rail", "bridge", "dock", "wagon", "vessel" })
            {
                string kind = infrastructure;
                yield return new Scenario("regression.territory_rejects_foreign_" + kind, () => ForeignInfrastructure(kind, false));
                yield return new Scenario("regression.territory_rejects_stale_building_" + kind, () => ForeignInfrastructure(kind, true));
                yield return new Scenario("regression.territory_rechecks_pending_" + kind, () => PendingForeignInfrastructure(kind));
            }
            yield return new Scenario("regression.territory_expansion_rejects_foreign_ring", () => ForeignExpansion(false, false));
            yield return new Scenario("regression.territory_expansion_rejects_foreign_interior", () => ForeignExpansion(true, false));
            yield return new Scenario("regression.territory_expansion_rejects_stale_building", () => ForeignExpansion(true, true));
            yield return new Scenario("regression.territory_expansion_rechecks_claims", PendingForeignExpansion);
            yield return new Scenario("regression.territory_legacy_tile_rechecks_claims", () => LegacyForeignExpansion(false));
            yield return new Scenario("regression.territory_legacy_tile_rechecks_stale_building", () => LegacyForeignExpansion(true));
            yield return new Scenario("regression.territory_founding_skips_foreign_deposit_site", ForeignFounding);
            yield return new Scenario("regression.territory_recovery_preserves_foreign_land_and_pending_state", ForeignRecovery);
            yield return new Scenario("regression.territory_zone_rejects_foreign_footprint", () => ForeignDesignation(false));
            yield return new Scenario("regression.territory_fishing_rejects_foreign_claim", () => ForeignDesignation(true));
            yield return new Scenario("regression.territory_scaffold_rejects_foreign_footprint", () => ForeignScaffold(false));
            yield return new Scenario("regression.territory_scaffold_rechecks_claims", () => ForeignScaffold(true));
            yield return new Scenario("catalog.research_487_public_purchase", ResearchGraph);
            yield return new Scenario("catalog.all_buildings_public_construction", AllBuildings);
            yield return new Scenario("capability.rail_public_construction", RailCapability);
            yield return new Scenario("capability.shipping_public_construction", ShippingCapability);
            yield return new Scenario("regression.repeat_queue_reaches_second_recipe", RepeatedQueue);
            yield return new Scenario("regression.pause_preserves_active_recipe", PauseQueue);
            yield return new Scenario("regression.direct_control_flood_cannot_advance_time", ControlFlood);
            yield return new Scenario("regression.death_preserves_cargo_and_exact_items", DeathCargo);
            yield return new Scenario("regression.extinction_restores_atomic_founding", Extinction);
            yield return new Scenario("regression.unhoused_migrant_physically_leaves", MigrantDeparture);
            yield return new Scenario("regression.communal_and_personal_enclosures", Enclosures);
            yield return new Scenario("regression.officer_vacancy_stops_automatic_staffing", OfficerVacancy);
            yield return new Scenario("regression.resource_capacity_is_resource_specific", ResourceSpecificCapacity);
            yield return new Scenario("regression.expansion_interruption_preserves_perimeter", ExpansionInterruption);
            yield return new Scenario("regression.rail_transfers_exact_equipment_identity", EquipmentRail);
            yield return new Scenario("regression.review_rail_stops_at_expanded_wall_and_recovers", RailExpandedWall);
            yield return new Scenario("regression.review_transport_rail_drink_reboards_physically", () => TransportNeed(false));
            yield return new Scenario("regression.review_transport_rail_drink_blocked_reboarding", () => TransportNeed(false, true));
            yield return new Scenario("regression.review_transport_shipping_sleep_docks_and_reboards", () => TransportNeed(true));
            yield return new Scenario("regression.review_transport_caravan_expansion_fence_and_recovery", CaravanExpansionFence);
            foreach (string mover in new[] { "merchant", "raid" }) foreach (bool departing in new[] { false, true })
            {
                string kind = mover; bool returning = departing;
                yield return new Scenario("regression.review_land_mover_" + kind + "_" + (returning ? "departing" : "arriving"), () => CachedLandMover(kind, returning));
            }
            yield return new Scenario("regression.review_land_mover_public_expansion_reroutes_merchant_and_raid", MerchantExpansionReroute);
            foreach (string obstruction in new[] { "water", "boundary", "exact_boundary", "return_boundary" })
            {
                string barrier = obstruction;
                yield return new Scenario("regression.review_rail_passability_" + barrier, () => RailPassability(barrier));
            }
            yield return new Scenario("regression.shipping_cancel_returns_loaded_cargo", () => ShippingCancel(false));
            yield return new Scenario("regression.shipping_cancel_full_source_retains_cargo", () => ShippingCancel(true));
            yield return new Scenario("regression.shipping_driver_death_bridge_salvage", () => ShippingCancel(false, true));
            yield return new Scenario("chain.field_to_handoff_to_mill_to_food", FarmFoodChain);
            foreach (var entry in new[] { ("clickPower", 2, 20), ("supplySpeed", 3, 10), ("huntMastery", 5, 10), ("buildMastery", 5, 10), ("ritualMastery", 6, 10), ("resilience", 7, 10) })
            { var e = entry; yield return new Scenario("legacy_upgrade." + e.Item1, () => LegacyUpgrade(e.Item1, e.Item2, e.Item3)); yield return new Scenario("legacy_effect." + e.Item1, () => LegacyEffect(e.Item1)); }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify_building")))
            { var id = node.Id; yield return new Scenario("building_effect." + id, () => BuildingStudy(id)); }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify")))
            { var id = node.Id; yield return new Scenario("service_effect." + id, () => ServiceStudy(id)); }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_resource")))
            { var id = node.Id; yield return new Scenario("resource_effect." + id, () => ResourceStudy(id)); }
            foreach (var recipe in Catalog.Recipes) { var id = recipe.Id; yield return new Scenario("recipe." + id, () => RecipeChain(id)); }
            foreach (int seed in new[] { 7, 41, 127 }) { int captured = seed; yield return new Scenario("campaign.fresh_48h_seed_" + seed, () => Campaign(captured, 48, false)); yield return new Scenario("campaign.established_72h_seed_" + seed, () => Campaign(captured, 72, true)); yield return new Scenario("campaign.shared_personal_48h_seed_" + seed, () => Campaign(captured, 48, false, true)); }
        }
        public static void Run(string name) => Cases().Single(c => c.Name == name).Run();
        static HashSet<Int2> LayoutRoads(World w, Village v)
        {
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            bool Inside(Building b, Int2 p) => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth;
            var seen = new HashSet<Int2>(); var pending = new Queue<Int2>();
            for (int x = 0; x < shrine.Width; x++) for (int z = 0; z < shrine.Depth; z++)
                pending.Enqueue(new Int2(shrine.Position.X + x, shrine.Position.Z + z));
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                {
                    var next = new Int2(at.X + offset.X, at.Z + offset.Z); var tile = w.GetTile(next);
                    if (tile != null && tile.Road && w.Walkable(v, next) && !v.Buildings.Any(b => Inside(b, next)) && !seen.Contains(next))
                    { seen.Add(next); pending.Enqueue(next); }
                }
            }
            return seen;
        }
        static void FoundingLayout(bool personal, bool roads)
        {
            var w = World.Create(41); var v = w.Villages[0];
            if (personal) v = w.Village(Act(w, v, new GameAction { Kind = "FoundVillage", Name = "Layout home" }).EntityId);
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            if (!roads)
            {
                Check(shrine.Width == 3 && shrine.Depth == 3 && shrine.Position.Equals(new Int2(v.Center.X - 1, v.Center.Z - 1)), "Shrine must occupy exactly the centered3×3tiles");
                for (int z = -2; z <= 2; z++) for (int x = -2; x <= 2; x++)
                {
                    var p = new Int2(v.Center.X + x, v.Center.Z + z); var t = w.TileAt(p);
                    Check(t.Road == (Math.Max(Math.Abs(x), Math.Abs(z)) == 2), "Shrine road ring is incomplete or crosses the shrine at " + p);
                    if (t.Road) Check(!v.Buildings.Any(b => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth) && !v.Stockpiles.Any(s => p.X >= s.Position.X && p.X < s.Position.X + s.Width && p.Z >= s.Position.Z && p.Z < s.Position.Z + s.Depth), "Shrine road overlaps a physical footprint");
                }
                Check(v.Cats.Count(c => c.Alive) == (personal ? 15 : 30) && v.Buildings.Count(b => b.Kind == "den") == (personal ? 3 : 6), "Founding population or housing changed");
                return;
            }
            var network = LayoutRoads(w, v);
            foreach (var b in v.Buildings.Where(b => b.Kind != "shrine"))
            {
                bool adjacent = network.Any(p => p.X >= b.Position.X && p.X < b.Position.X + b.Width && (p.Z == b.Position.Z - 1 || p.Z == b.Position.Z + b.Depth) || p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth && (p.X == b.Position.X - 1 || p.X == b.Position.X + b.Width));
                Check(adjacent, "Founding building lacks an entrance on shrine-connected road: " + b.Kind + " " + b.Position);
                Check(w.Path(v.Center, b.Position, v) != null, "Founding building is physically unreachable: " + b.Kind);
            }
        }
        static void FoundingExits()
        {
            foreach (int seed in new[] { 7, 41, 127 })
            {
                var w = World.Create(seed); Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Exit home" });
                foreach (var v in w.Villages)
                {
                    foreach (var d in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                    {
                        var gate = new Int2(v.Center.X + d.X * v.Radius, v.Center.Z + d.Z * v.Radius);
                        var outside = new Int2(gate.X + d.X, gate.Z + d.Z);
                        Check(!w.TileAt(gate).Wall && w.TileAt(gate).Road && w.Walkable(v, outside) && w.Path(v.Center, outside, v) != null, "Founding gate is not a real traversable exit: " + gate);
                    }
                    foreach (string resource in new[] { "food", "logs", "stone", "fibre", "ore", "clay", "sand", "gem" })
                        Check(w.Tiles.Where(t => t.Resource == resource && t.Amount == 200 && Int2.Distance(t.Position, v.Center) < 30).Any(t => w.Path(v.Center, t.Position, v) != null), "Finite founding source is isolated: " + resource + " seed=" + seed);
                }
            }
        }
        static void DisconnectedBuilding()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var at = new Int2(-6, -5); foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= -2)) tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            int buildings = v.Buildings.Count, claims = w.Reservations.Count; double timber = Goods(w, v, "planks");
            var denied = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id });
            Check(!denied.Success, "Disconnected paving allowed a new building without a shrine-connected entrance");
            Check(v.Buildings.Count == buildings && w.Reservations.Count == claims && Goods(w, v, "planks") == timber, "Rejected layout changed property or claims");
        }
        static void HousingConnectedSite(string mode)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.FoundedAt = -6 * 3600;
            World.Add(v.Stockpiles[0].Goods, "planks", 16); World.Add(v.Stockpiles[0].Goods, "blocks", 8);
            var disconnected = new Int2(-4, -4); var connected = new Int2(4, -4);
            v.Known.Clear();
            v.Known.Add(new Int2(-11, 4));
            foreach (var tile in w.Tiles) tile.Road = Math.Max(Math.Abs(tile.Position.X), Math.Abs(tile.Position.Z)) == 2;
            foreach (var origin in new[] { disconnected, connected })
                for (int x = 0; x < 2; x++) for (int z = 0; z < 2; z++) v.Known.Add(new Int2(origin.X + x, origin.Z + z));
            w.TileAt(new Int2(3, -2)).Road = w.TileAt(new Int2(4, -2)).Road = true;
            Check(!w.BuildingEntrance(v, new Building { Position = disconnected }).HasValue && w.BuildingEntrance(v, new Building { Position = connected }).HasValue, "Housing fixture must offer a disconnected site before a connected site");
            var denied = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "den", Position = disconnected, CatId = c.Id });
            Check(!denied.Success && denied.Error.Contains("entrance"), "First housing site is not a clear but disconnected footprint");
            if (mode == "automatic") w.Step(10);
            else Act(w, v, new GameAction { Kind = "RequestJob", Name = mode, CatId = c.Id });
            var den = v.Buildings.SingleOrDefault(b => b.Kind == "den" && !b.Completed);
            Check(den != null && den.Position.Equals(connected), "Housing stopped at the disconnected site instead of selecting the later connected site");
            var job = v.Jobs.Single(j => j.TargetId == den.Id);
            if (mode == "automatic") Check(job.AutomatedBy == "leader" && job.Requester == "leader", "Automatic housing lost its Leader work owner");
            w.Step(600); Check(den.Completed && job.Completed, "Selected connected housing did not complete through physical delivery and construction");
            Near(Goods(w, v, "planks"), 0, "Housing did not consume its exact Plank bill"); Near(Goods(w, v, "blocks"), 0, "Housing did not consume its exact Block bill");
            Check(w.Reservations.Count == 0, "Completed housing retained claims"); Valid(w);
        }
        static void ExpansionFootprint(string kind, bool pending)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", kind == "entrance" ? 1002 : 1001); World.Add(v.Stockpiles[0].Goods, "planks", 4); World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList(); int radius = v.Radius, outer = radius + 2;
            var at = new Int2(outer + (kind == "entrance" ? 1 : 0), 1); var entry = kind == "entrance" ? new Int2(outer, 1) : new Int2(outer, 0);
            for (int x = outer; x <= outer + 2; x++) for (int z = 0; z <= 2; z++)
            { var p = new Int2(x, z); var tile = w.TileAt(p); tile.Wall = tile.Water = tile.Mountain = tile.Road = false; tile.ClaimId = ""; if (!v.Known.Contains(p)) v.Known.Add(p); }
            for (int x = radius; x < outer; x++) w.TileAt(new Int2(x, 0)).Road = true;
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer, 0), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId); for (int tick = 0; tick < 600 && !road.Completed; tick++) w.Step(1);
            Check(road.Completed && w.TileAt(entry).Road, "Public road did not connect the exterior work site"); Near(Goods(w, v, "materials"), 1000, "Exterior access road did not consume its exact bill");
            Job expansion = null; Cat builder = c;
            if (pending)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }); expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                for (int tick = 0; tick < 1800 && expansion.PathIndex == 0; tick++) w.Step(1);
                Check(expansion.PathIndex > 0, "Expansion must have paid perimeter work before the competing building");
                builder = v.Cats[1]; builder.ControlledBy = "";
            }
            string obstacleId;
            if (pending)
            {
                // Explicit loaded-state fault: older saves could already contain work on the reserved perimeter.
                if (kind == "gather") { var pile = new Stockpile { Id = w.Id("loaded-gather"), Kind = "gather", Position = at, Width = 1, Depth = 1 }; v.Stockpiles.Add(pile); obstacleId = pile.Id; }
                else { var field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true }; v.Buildings.Add(field); obstacleId = field.Id; }
            }
            else if (kind == "gather") obstacleId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = at, End = at }).EntityId;
            else
            {
                obstacleId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id }).EntityId;
                var field = v.Buildings.Single(b => b.Id == obstacleId); Check(field.Entrance.Equals(entry), "Field did not use the intended exterior entrance");
                if (!pending) { for (int tick = 0; tick < 600 && !field.Completed; tick++) w.Step(1); Check(field.Completed && w.Path(v.Center, field.Position, v) != null, "Public exterior Field did not complete with reachable work position"); }
            }
            int claims = w.Reservations.Count, jobs = v.Jobs.Count; double materials = Goods(w, v, "materials"); string terrain = TerrainState(w.TileAt(at)), entryTerrain = TerrainState(w.TileAt(entry));
            if (pending)
            {
                int paid = expansion.PathIndex; double progress = expansion.Progress; string held = string.Join("|", w.Reservations.Where(r => r.OwnerId.StartsWith(expansion.Id, StringComparison.Ordinal)).Select(r => r.OwnerId + ":" + r.Amount));
                w.Step(3);
                Check(!expansion.Completed && expansion.BlockedReason == "expansion_footprint_blocked" && expansion.PathIndex == paid && expansion.Progress == progress, "Pending expansion advanced after an exterior Field occupied its wall route");
                Check(expansion.Path.Take(paid).All(p => w.TileAt(p).Wall), "Blocked expansion discarded paid wall segments");
                Check(held == string.Join("|", w.Reservations.Where(r => r.OwnerId.StartsWith(expansion.Id, StringComparison.Ordinal)).Select(r => r.OwnerId + ":" + r.Amount)), "Blocked expansion changed its held input claims");
            }
            else
            {
                var denied = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                Check(!denied.Success && denied.Error.Contains("footprint"), "Expansion accepted a wall through an existing " + kind);
                Check(v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected expansion created work or reserved materials");
            }
            Check(v.Radius == radius && TerrainState(w.TileAt(at)) == terrain && TerrainState(w.TileAt(entry)) == entryTerrain, "Expansion changed the protected work position or entrance");
            Near(Goods(w, v, "materials"), materials, "Conflicting expansion consumed materials"); Valid(w);
            if (pending && kind == "gather")
            {
                Act(w, v, new GameAction { Kind = "RemoveGatherSpot", TargetId = obstacleId });
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++) w.Step(1);
                Check(expansion.Completed && v.Radius == outer && expansion.BlockedReason == "", "Removing the obstruction did not resume the same expansion");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) }) Check(!w.TileAt(gate).Wall, "Resumed expansion closed a required cardinal gate");
                Check(expansion.Path.All(p => w.TileAt(p).Wall), "Resumed expansion lost a paid perimeter segment");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Resumed expansion consumed more than its finite perimeter bill"); Check(w.Reservations.Count == 0, "Completed expansion retained claims"); Valid(w);
            }
        }
        static void PendingExpansionPlacement(string site)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000); World.Add(v.Stockpiles[0].Goods, "planks", 4); World.Add(v.Stockpiles[0].Goods, "blocks", 2); v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2; bool doorway = site == "entrance" || site == "alternate_entrance";
            int row = site == "alternate_entrance" ? 2 : 1;
            var at = new Int2(outer + (doorway ? 1 : 0), row); var blockedEntry = new Int2(outer, row); var alternate = new Int2(outer + 3, row);
            for (int x = outer; x <= outer + 3; x++) for (int z = 0; z <= row + 1; z++)
            { var p = new Int2(x, z); var tile = w.TileAt(p); tile.Wall = tile.Water = tile.Mountain = tile.Road = false; tile.ClaimId = ""; if (!v.Known.Contains(p)) v.Known.Add(p); }
            for (int x = v.Radius; x <= outer; x++) w.TileAt(new Int2(x, 0)).Road = true;
            if (doorway) for (int z = 1; z <= row; z++) w.TileAt(new Int2(outer, z)).Road = true;
            if (site == "alternate_entrance")
            {
                for (int x = outer + 1; x <= alternate.X; x++) w.TileAt(new Int2(x, 0)).Road = true;
                for (int z = 1; z <= row; z++) w.TileAt(new Int2(alternate.X, z)).Road = true;
                Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(blockedEntry), "Fixture must prefer the doorway that expansion reserves");
            }
            var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }); var expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
            var builder = v.Cats[1]; builder.ControlledBy = ""; int claims = w.Reservations.Count, buildings = v.Buildings.Count, piles = v.Stockpiles.Count, jobs = v.Jobs.Count;
            double materials = Goods(w, v, "materials"), timber = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
            var action = site == "gather" ? new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = at, End = at } : new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id };
            var result = w.Apply(Context(v), action);
            if (site == "alternate_entrance")
            {
                Check(result.Success, "Reserved first doorway prevented selection of another connected entrance"); var field = v.Buildings.Single(b => b.Id == result.EntityId);
                Check(field.Entrance.Equals(alternate) && !expansion.Path.Contains(field.Entrance), "Field chose the reserved expansion wall instead of the alternate entrance");
                for (int tick = 0; tick < 12000 && (!expansion.Completed || !field.Completed); tick++) w.Step(1);
                Check(field.Completed && expansion.Completed && w.Path(v.Center, field.Position, v) != null, "Field with an alternate doorway did not remain reachable through completed expansion");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Concurrent expansion changed its exact material bill"); Near(Goods(w, v, "planks"), 0, "Concurrent Field changed its timber bill"); Near(Goods(w, v, "blocks"), 0, "Concurrent Field changed its block bill");
            }
            else
            {
                Check(!result.Success, "Public " + site + " placement occupied a pending expansion wall");
                Check(v.Buildings.Count == buildings && v.Stockpiles.Count == piles && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected placement changed structures, work or claims");
                Near(Goods(w, v, "materials"), materials, "Rejected placement consumed expansion materials"); Near(Goods(w, v, "planks"), timber, "Rejected placement consumed timber"); Near(Goods(w, v, "blocks"), blocks, "Rejected placement consumed blocks");
            }
            Valid(w);
        }
        static void ExpansionRoadConnection(string order)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1003); World.Add(v.Stockpiles[0].Goods, "planks", 4); World.Add(v.Stockpiles[0].Goods, "blocks", 2); v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2; var at = new Int2(outer + 1, 2); var entry = new Int2(outer + 1, 1); var crossing = new Int2(outer, 1);
            for (int x = outer - 1; x <= outer + 2; x++) for (int z = 0; z <= 3; z++)
            { var p = new Int2(x, z); var tile = w.TileAt(p); tile.Wall = tile.Water = tile.Mountain = tile.Road = false; tile.ClaimId = ""; if (!v.Known.Contains(p)) v.Known.Add(p); }
            w.TileAt(new Int2(outer - 1, 0)).Road = true;
            if (order == "alternate_route") { w.TileAt(new Int2(outer, 0)).Road = true; w.TileAt(new Int2(outer + 1, 0)).Road = true; }
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer - 1, 1), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId); for (int tick = 0; tick < 600 && !road.Completed; tick++) w.Step(1);
            Check(road.Completed && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Public road did not connect the distant Field entrance"); Near(Goods(w, v, "materials"), 1000, "Access road changed its finite material bill");
            bool expansionFirst = order == "expansion_first" || order == "loaded_conflict"; Job expansion = null; Building field = null; Cat builder = c;
            if (expansionFirst)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }); expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                Check(expansion.Path.Contains(crossing) && !expansion.Path.Contains(entry), "Fixture must reserve an upstream road, not the doorway");
                builder = v.Cats[1]; builder.ControlledBy = "";
            }
            if (order == "loaded_conflict")
            {
                // Existing save conflict: a completed Field whose road crosses already planned wall work.
                field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true }; v.Buildings.Add(field);
                double materials = Goods(w, v, "materials"), progress = expansion.Progress; int claims = w.Reservations.Count; w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field's upstream road did not pause the existing expansion before material movement");
                Near(Goods(w, v, "materials"), materials, "Blocked upstream road consumed retained expansion materials"); Check(!w.TileAt(crossing).Wall, "Blocked expansion walled the Field's upstream road"); Valid(w); return;
            }
            int reservations = w.Reservations.Count, jobs = v.Jobs.Count, buildings = v.Buildings.Count;
            var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
            if (order == "expansion_first")
            {
                Check(!placement.Success, "Field placement accepted a road that pending expansion will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected disconnected Field changed structures, jobs or claims"); Near(Goods(w, v, "planks"), 4, "Rejected Field consumed timber"); Near(Goods(w, v, "blocks"), 2, "Rejected Field consumed blocks"); Valid(w); return;
            }
            Check(placement.Success, "Initial exterior Field was not connected: " + placement.Error); field = v.Buildings.Single(b => b.Id == placement.EntityId);
            for (int tick = 0; tick < 600 && !field.Completed; tick++) w.Step(1); Check(field.Completed, "Public exterior Field construction did not complete");
            reservations = w.Reservations.Count; jobs = v.Jobs.Count; var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted an upstream wall that disconnects an existing Field"); Check(v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected expansion created jobs or claims"); Near(Goods(w, v, "materials"), 1000, "Rejected expansion consumed materials");
            }
            else
            {
                Check(expanded.Success, "Alternate road through the required gate did not allow expansion"); expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++) w.Step(1);
                Check(expansion.Completed && w.TileAt(crossing).Wall && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Completed expansion lost the Field's alternate shrine-connected road");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) }) Check(!w.TileAt(gate).Wall, "Alternate access created a missing cardinal gate");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Expansion with alternate access changed its wall bill");
            }
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Existing Field lost its shrine-connected doorway"); Valid(w);
        }
        static void ExpansionFarmEdge(string order)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1020); World.Add(v.Stockpiles[0].Goods, "planks", 10); World.Add(v.Stockpiles[0].Goods, "blocks", 5); v.Research = Catalog.Research.Select(n => n.Id).ToList();
            for (int x = 10; x <= 15; x++) for (int z = 0; z <= 6; z++)
            { var p = new Int2(x, z); var tile = w.TileAt(p); tile.Wall = tile.Water = tile.Mountain = tile.Road = false; tile.ClaimId = ""; if (!v.Known.Contains(p)) v.Known.Add(p); }
            w.TileAt(new Int2(10, 0)).Road = true;
            void Road(Int2 from, Int2 to)
            {
                var id = Act(w, v, new GameAction { Kind = "BuildRoad", Position = from, End = to, CatId = c.Id }).EntityId; var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < 1000 && !job.Completed; tick++) w.Step(1); Check(job.Completed, "Farm-edge fixture road did not complete");
            }
            Building Field(Int2 position)
            {
                var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = position, CatId = c.Id }).EntityId; var building = v.Buildings.Single(b => b.Id == id);
                for (int tick = 0; tick < 1000 && !building.Completed; tick++) w.Step(1); Check(building.Completed, "Farm-edge fixture Field did not complete"); return building;
            }
            Road(new Int2(11, 0), new Int2(13, 0)); var existing = Field(new Int2(13, 1));
            Road(new Int2(11, 1), new Int2(11, 3)); Road(new Int2(12, 3), new Int2(12, 3));
            if (order == "alternate_route") { Road(new Int2(12, 1), new Int2(12, 1)); Road(new Int2(12, 2), new Int2(12, 2)); }
            var at = new Int2(12, 4); var entry = new Int2(12, 3); var edgeFrom = new Int2(11, 0); var edgeTo = new Int2(11, 1); Building target = null;
            if (order == "field_first" || order == "alternate_route") target = Field(at);
            Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 1), End = new Int2(10, 3) });
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Crossable(edgeFrom, edgeTo), "Farm-edge fixture must start with a connected road across the future fence");
            int claims = w.Reservations.Count, jobs = v.Jobs.Count, radius = v.Radius; double materials = Goods(w, v, "materials");
            var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted a future farm fence that disconnects an existing Field");
                Check(v.Jobs.Count == jobs && w.Reservations.Count == claims && v.Radius == radius && v.BoundaryEdges.Count == 0, "Rejected farm-edge expansion changed work, claims or fences"); Near(Goods(w, v, "materials"), materials, "Rejected farm-edge expansion consumed materials"); Valid(w); return;
            }
            Check(expanded.Success, "Clear farm-edge expansion was rejected: " + expanded.Error); var expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
            Check(!expansion.Path.Contains(edgeFrom) && !expansion.Path.Contains(edgeTo) && !expansion.Path.Contains(entry), "Farm-edge fixture accidentally relies on a future wall tile");
            if (order == "expansion_first")
            {
                var builder = v.Cats[1]; builder.ControlledBy = ""; claims = w.Reservations.Count; jobs = v.Jobs.Count; int buildings = v.Buildings.Count; double timber = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
                var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
                Check(!placement.Success, "New Field accepted a road that the pending farm fence will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected Field changed structures, jobs or claims"); Near(Goods(w, v, "planks"), timber, "Rejected Field consumed timber"); Near(Goods(w, v, "blocks"), blocks, "Rejected Field consumed blocks"); Valid(w); return;
            }
            if (order == "loaded_conflict")
            {
                // Saved-state conflict from a prior version: the later Field already exists.
                target = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true }; v.Buildings.Add(target);
                claims = w.Reservations.Count; double progress = expansion.Progress; w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field did not pause expansion before its future farm fence cut the road"); Near(Goods(w, v, "materials"), materials, "Blocked farm-edge expansion consumed materials"); Check(w.Crossable(edgeFrom, edgeTo), "Blocked expansion changed the farm fence"); Valid(w); return;
            }
            for (int tick = 0; tick < 12000 && !expansion.Completed; tick++) w.Step(1);
            Check(expansion.Completed && v.Radius == radius + 2 && !w.Crossable(edgeFrom, edgeTo) && w.TileAt(edgeFrom).Road && w.TileAt(edgeTo).Road && !w.TileAt(edgeTo).Wall, "Expansion did not create the expected edge fence while retaining its road tiles");
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Path(v.Center, target.Position, v) != null && w.BuildingEntrance(v, new Building { Position = existing.Position }).Equals(existing.Entrance), "Alternate road did not preserve both Fields across the completed farm fence");
            Near(Goods(w, v, "materials"), materials - expansion.Path.Count, "Farm-edge expansion changed its exact wall bill"); Check(w.Reservations.Count == 0, "Completed farm-edge expansion retained claims"); Valid(w);
        }
        static void FishingPlacement(string placement)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v); World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            var at = placement == "road" ? new Int2(2, 1) : placement == "entrance" ? v.Buildings.First(b => b.Kind == "den").Entrance : placement == "pending_wall" ? new Int2(v.Radius + 2, 1) : placement == "legal_owned" ? new Int2(4, -5) : new Int2(12, 5);
            var tile = w.TileAt(at); tile.Wall = tile.Water = tile.Mountain = false;
            if (placement != "road" && placement != "entrance") tile.Road = tile.Rail = false;
            if (!v.Known.Contains(at)) v.Known.Add(at); if (placement == "unmapped") v.Known.Remove(at);
            var water = w.TileAt(new Int2(at.X + 1, at.Z)); water.Wall = water.Mountain = false; water.Water = true; water.Resource = "fish"; water.Amount = water.FishCapacity = 24;
            if (placement == "pending_wall") Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            var action = new GameAction { Kind = "DesignateFishingSpot", Position = at, End = at };
            if (placement == "duplicate") Act(w, v, action);
            int piles = v.Stockpiles.Count, jobs = v.Jobs.Count, claims = w.Reservations.Count; string terrain = TerrainState(tile); double materials = Goods(w, v, "materials");
            var result = w.Apply(Context(v), action); bool legal = placement == "legal_owned" || placement == "legal_exterior";
            if (legal)
            {
                Check(result.Success, "Legal " + placement + " fishing shore was rejected: " + result.Error);
                Check(v.Stockpiles.Count == piles + 1 && v.Stockpiles.Single(p => p.Id == result.EntityId).Kind == "fishing", "Legal fishing designation did not create its physical shore pile");
            }
            else
            {
                Check(!result.Success, "Fishing designation accepted " + placement + " shore");
                Check(v.Stockpiles.Count == piles && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected fishing designation changed piles, work or claims");
            }
            Check(TerrainState(tile) == terrain, "Fishing designation changed its underlying terrain"); Near(Goods(w, v, "materials"), materials, "Fishing designation changed held materials"); Valid(w);
        }
        static void PendingExpansionLinear(bool rail, bool expansionFirst)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000); World.Add(v.Stockpiles[0].Goods, "metal", 10); v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2; var at = new Int2(outer, 1); var tile = w.TileAt(at); tile.Road = tile.Rail = false; w.TileAt(new Int2(outer, 0)).Road = true;
            var wall = new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }; var line = new GameAction { Kind = rail ? "DesignateRail" : "BuildRoad", Position = at, End = at, CatId = c.Id };
            Act(w, v, expansionFirst ? wall : line); var builder = v.Cats[1]; builder.ControlledBy = "";
            int jobs = v.Jobs.Count, claims = w.Reservations.Count; double materials = Goods(w, v, "materials"), metal = Goods(w, v, "metal");
            var second = expansionFirst ? line : wall; second.CatId = builder.Id; var result = w.Apply(Context(v), second);
            Check(!result.Success, expansionFirst ? "Linear infrastructure accepted a reserved expansion wall" : "Expansion accepted a reserved linear infrastructure path");
            Check(v.Jobs.Count == jobs && w.Reservations.Count == claims && !tile.Road && !tile.Rail, "Rejected linear work changed jobs, claims or terrain");
            Near(Goods(w, v, "materials"), materials, "Rejected road consumed expansion materials"); Near(Goods(w, v, "metal"), metal, "Rejected rail consumed metal"); Valid(w);
        }
        static void ShrineRingReserved()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-3, -3), CatId = c.Id }).Success, "Construction occupied the shrine's required road ring");
        }
        static void PendingEntrance()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var building = v.Buildings.Single(b => b.Id == id); var entrance = w.BuildingEntrance(v, building); Check(entrance.HasValue, "Test scaffold has no connected entry");
            var tile = w.TileAt(entrance.Value); tile.Road = false;
            var job = v.Jobs.Single(j => j.TargetId == id); double goods = Goods(w, v, "planks"), progress = building.Progress; string claims = string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Amount));
            w.Step(3);
            Check(job.BlockedReason == "building_entrance_disconnected" && !job.Completed && building.Progress == progress, "Scaffold progressed without its connected entrance");
            Near(Goods(w, v, "planks"), goods, "Blocked entry consumed construction goods"); Check(string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Amount)) == claims, "Blocked entry changed held claims");
            tile.Road = true; w.Step(600); Check(building.Completed && job.Completed, "Repairing the existing road did not resume the same scaffold");
        }
        static void BuildingDoor()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var building = v.Buildings.Single(b => b.Id == id); Check(building.HasEntrance, "New construction did not preserve its selected doorway");
            var path = w.Path(v.Center, building.Position, v); Check(path != null && path.Contains(building.Entrance), "Physical worker path bypassed the selected doorway");
            var closed = new Int2(building.Position.X - 1, building.Position.Z); Check(!closed.Equals(building.Entrance) && !w.Crossable(closed, building.Position), "Building exterior allowed a wall crossing away from its door");
        }
        static void IsolatedRoad()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "materials", 100);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0)) tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            Check(!LayoutRoads(w, v).Contains(new Int2(-6, -3)), "Test paving is connected instead of isolated");
            Check(!w.Apply(Context(v), new GameAction { Kind = "BuildRoad", Position = new Int2(-6, -2), End = new Int2(-5, -2), CatId = c.Id }).Success, "Isolated paving bypassed shrine-road connectivity");
        }
        static void PaidBuildingRoad()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = "";
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0)) tile.Road = false;
            World.Add(v.Stockpiles[0].Goods, "materials", 4); World.Add(v.Stockpiles[0].Goods, "planks", 4); World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-6, -4);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).Success, "Unpaved site was accepted before its access road existed");
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(-3, -2), End = new Int2(-6, -2), CatId = c.Id }).EntityId;
            Check(!w.TileAt(new Int2(-6, -2)).Road, "Road appeared before physical work");
            for (int tick = 0; tick < 600 && !v.Jobs.Single(j => j.Id == roadId).Completed; tick++) w.Step(1);
            Check(v.Jobs.Single(j => j.Id == roadId).Completed, "Paid access road did not finish"); Near(Goods(w, v, "materials"), 0, "Road did not consume its exact four materials");
            var buildingId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).EntityId;
            w.Step(600); var building = v.Buildings.Single(b => b.Id == buildingId);
            Check(building.Completed && LayoutRoads(w, v).Contains(building.Entrance), "Road did not enable a connected completed building");
            Near(Goods(w, v, "planks"), 0, "Building timber bill changed"); Near(Goods(w, v, "blocks"), 0, "Building block bill changed"); Valid(w);
        }
        static void LegacyLayout()
        {
            var w = new World { Seed = 41 }; var v = new Village { Id = "legacy-layout", Communal = true, Center = new Int2(0, 0), Radius = 12 }; w.Villages.Add(v);
            var shrine = new Building { Id = "saved-shrine", Kind = "shrine", Position = new Int2(3, 3), Width = 3, Depth = 3, Completed = true };
            var old = new Building { Id = "saved-workshop", Kind = "wood_cutter", Position = new Int2(8, 4), Width = 2, Depth = 2, Completed = true }; v.Buildings.AddRange(new[] { shrine, old });
            for (int z = 0; z <= 12; z++) for (int x = 0; x <= 12; x++) { var at = new Int2(x, z); v.Known.Add(at); v.ClaimedTiles.Add(at); w.Tiles.Add(new Tile { Position = at, ClaimId = v.Id, Biome = "meadow" }); }
            foreach (var at in new[] { new Int2(6, 4), new Int2(7, 4), new Int2(6, 3), new Int2(7, 3), new Int2(8, 3) }) w.TileAt(at).Overlay = "road_built";
            var cat = new Cat { Id = "saved-cat", VillageId = v.Id, Position = new Int2(6, 3), X = 6, Z = 3 }; v.Cats.Add(cat);
            v.Stockpiles.Add(new Stockpile { Id = "saved-store", Position = new Int2(1, 1), Goods = new List<Stack> { new Stack("planks", 20), new Stack("blocks", 20) } });
            var terrain = string.Join("|", w.Tiles.Select(t => t.Position + ":" + t.Overlay + ":" + TerrainState(t)));
            Check(w.BuildingEntrance(v, old).Equals(new Int2(8, 3)), "Imported road_built network was not rooted at the actual saved shrine footprint");
            var result = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(8, 1), CatId = cat.Id });
            Check(v.LayoutVersion == 0 && !old.HasEntrance && old.Position.Equals(new Int2(8, 4)) && shrine.Position.Equals(new Int2(3, 3)), "Inspecting or extending a legacy layout relocated saved property");
            Check(w.Tiles.Count == 169 && terrain == string.Join("|", w.Tiles.Select(t => t.Position + ":" + t.Overlay + ":" + TerrainState(t))), "Legacy compatibility silently paved or rewrote saved terrain");
            Check(v.Buildings.Single(b => b.Id == result.EntityId).HasEntrance && v.Cats.Single().Id == "saved-cat", "New connected extension lost persisted layout or cat identity");
        }
        static void AvoidShrineAccess()
        {
            var w = World.Create(7); var v = w.Villages[0];
            foreach (var at in new[] { v.Center, new Int2(v.Center.X, v.Center.Z + 2), v.Buildings.First(b => b.HasEntrance).Entrance })
                Check(!w.Apply(Context(v), new GameAction { Kind = "CreateZone", Resource = "avoid", Position = at, End = at }).Success, "Avoid zone severed shrine or building access");
        }
        static void PendingRoadFootprint(bool rail, bool changed)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var builder = v.Cats[1]; builder.ControlledBy = ""; builder.BuildingId = "";
            string resource = rail ? "metal" : "materials";
            World.Add(v.Stockpiles[0].Goods, resource, 2); World.Add(v.Stockpiles[0].Goods, "planks", 4); World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-5, 1); var roadId = Act(w, v, new GameAction { Kind = rail ? "DesignateRail" : "BuildRoad", Position = at, End = new Int2(-4, 1), CatId = c.Id }).EntityId;
            var job = v.Jobs.Single(j => j.Id == roadId); int count = v.Buildings.Count; double goods = Goods(w, v, resource), timber = Goods(w, v, "planks");
            var claims = string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount));
            if (!changed)
            {
                var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = builder.Id });
                Check(!result.Success, "Second builder occupied a reserved " + job.Kind + " path before paving completed");
                Check(v.Buildings.Count == count && builder.JobId == "" && claims == string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount)), "Rejected footprint changed ownership or claims");
                Near(Goods(w, v, resource), goods, "Rejected footprint consumed road inputs"); Near(Goods(w, v, "planks"), timber, "Rejected footprint consumed timber");
                return;
            }
            builder.ControlledBy = "fixture-held";
            // Explicit imported-state fault: old saves can contain work under a later structure.
            var blocking = new Building { Id = "imported-overlap", Kind = "den", Position = at, Completed = true }; v.Buildings.Add(blocking);
            w.Step(3);
            Check(job.BlockedReason == "infrastructure_footprint_blocked" && !job.Completed && job.Progress == 0 && job.PathIndex == 0, "Pending " + job.Kind + " advanced through an occupied footprint");
            Check(claims == string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount)), "Blocked infrastructure changed held input claims"); Near(Goods(w, v, resource), goods, "Blocked infrastructure consumed inputs");
            v.Buildings.Remove(blocking); for (int tick = 0; tick < 600 && !job.Completed; tick++) w.Step(1);
            Check(job.Completed && job.Path.All(p => rail ? w.TileAt(p).Rail : w.TileAt(p).Road), "Cleared footprint did not resume the same infrastructure work"); Near(Goods(w, v, resource), 0, "Resumed infrastructure did not consume exact finite inputs"); Valid(w);
        }
        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Near(double actual, double expected, string message, double tolerance = 0.00001) => Check(Math.Abs(actual - expected) <= tolerance, message + " expected=" + expected + " actual=" + actual);
        static PlayerContext Context(Village v) => new PlayerContext { PlayerId = "acceptance-player", VillageId = v.Id };
        static ActionResult Act(World w, Village v, GameAction a) { var r = w.Apply(Context(v), a); Check(r.Success, a.Kind + ": " + r.Error); return r; }
        static World Fixture(out Village v, out Cat c)
        {
            var w = World.Create(41); v = w.Villages[0]; c = v.Cats[0];
            // Isolated rule fixtures retain founding beds/shrine; full blueprints run in campaigns.
            v.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine"); v.Stockpiles.RemoveRange(1, v.Stockpiles.Count - 1);
            foreach (var other in v.Cats) { other.ControlledBy = "fixture-held"; other.ControlLeaseUntil = 10000000; }
            c.ControlledBy = ""; c.BuildingId = "fixture-held"; c.Position = new Int2(1, 2); c.X = 1; c.Z = 2;
            v.ResearchPoints = 1000000; v.LastLeaderResearch = 0; v.Stockpiles[0].Goods.Clear(); v.Stockpiles[0].Capacity = 100000;
            v.Stockpiles[0].Position = new Int2(3, 2); World.Add(v.Stockpiles[0].Goods, "food", 1000); World.Add(v.Stockpiles[0].Goods, "water", 1000); return w;
        }
        static Building Station(World w, Village v, string kind, Int2? at = null)
        { var b = new Building { Id = w.Id("fixture-station"), Kind = kind, Position = at ?? new Int2(1, 3), Completed = true }; v.Buildings.Add(b); return b; }
        static double Goods(World w, Village v, string resource) => v.Stockpiles.Sum(p => World.Amount(p.Goods, resource)) + v.Cats.Sum(c => World.Amount(c.Cargo, resource)) + v.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, resource)) + v.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource)) + v.Farms.Where(f => f.Crop == resource).Sum(f => f.Harvest) + v.Vehicles.Sum(x => World.Amount(x.Cargo, resource));
        static void Valid(World w)
        {
            Check(w.Validate().Count == 0, string.Join("; ", w.Validate()));
            foreach (var v in w.Villages)
            {
                foreach (var item in v.Items) Check(v.Stockpiles.Any(p => p.Id == item.LocationId) || v.Cats.Any(c => c.Alive && c.Id == item.LocationId) || v.Jobs.Any(j => !j.Completed && j.Id == item.LocationId) || v.Vehicles.Any(vehicle => vehicle.Id == item.LocationId), "Orphan exact item " + item.Id + " at " + item.LocationId);
                int beds = v.Buildings.Count(b => b.Completed && b.Kind == "den") * w.DenCapacity(v); Check(v.Cats.Count(c => c.Alive && c.BedId != "") + v.Cats.Count(c => c.Alive && c.PregnantUntil > 0) <= beds, "Permanent beds plus pregnancy claims overbooked");
                foreach (var cargo in v.Jobs.Where(j => !j.Completed).SelectMany(j => j.Local).Concat(v.Vehicles.SelectMany(x => x.Cargo))) Check(World.Finite(cargo.Amount) && cargo.Amount >= 0, "Invalid work/vehicle cargo");
            }
            foreach (var claim in w.Reservations) { var pile = w.Villages.SelectMany(v => v.Stockpiles).FirstOrDefault(p => p.Id == claim.PileId); Check(pile != null && World.Finite(claim.Amount) && claim.Amount > 0, "Orphan/nonfinite reservation " + claim.OwnerId); Check(w.Villages.Any(v => v.Jobs.Any(j => !j.Completed && (j.Id == claim.OwnerId || j.Id + ":resume" == claim.OwnerId)) || v.Buildings.Any(b => !b.Completed && b.Id == claim.OwnerId)), "Reservation has no live work owner " + claim.OwnerId); }
        }
        static void BuildableServices()
        {
            foreach (var kind in new[] { "den", "workshop", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "mouse_farm" })
            { var w = Fixture(out var v, out var c); World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); c.BuildingId = ""; Act(w, v, new GameAction { Kind = "PlanBuilding", Name = kind, Position = new Int2(-5, 1), CatId = c.Id }); w.Step(500); Check(v.Buildings.Any(b => b.Kind == kind && b.Position.Equals(new Int2(-5, 1)) && b.Completed), kind + " cannot complete physical construction"); Valid(w); }
        }
        static void CargoNeed()
        {
            var w = Fixture(out var v, out var c); c.Thirst = 1; c.Cargo.Add(new Stack("logs", 8)); v.Stockpiles[0].Accepts.Add("water"); World.Add(v.Stockpiles[0].Goods, "water", 10);
            var j = new Job { Id = w.Id("job"), Kind = "logs", CatId = c.Id, Resource = "logs", Phase = "output_delivery", Position = c.Position }; v.Jobs.Add(j); c.JobId = j.Id;
            w.Step(60); Check(c.Alive && c.Thirst > 35, "Cargo prevented drinking available water"); Near(Goods(w, v, "logs"), 8, "Interrupted cargo lost/duplicated"); Valid(w);
        }
        static void BusyCat()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); var spill = new Stockpile { Id = w.Id("spill"), Kind = "spill", Position = new Int2(2, 2), Goods = new List<Stack> { new Stack("logs", 8) } }; v.Stockpiles.Add(spill);
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }); Valid(w);
            Check(v.Jobs.Count(j => !j.Completed && j.CatId == c.Id) <= 1, "Builder replaced job without releasing owner");
            w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", TargetId = spill.Id, CatId = c.Id }); Valid(w);
        }
        static void RouteControl()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; var source = v.Stockpiles[0]; source.Position = new Int2(1, 2); source.Goods.Add(new Stack("logs", 20));
            var destination = new Stockpile { Id = w.Id("destination"), Position = new Int2(3, 2) }; v.Stockpiles.Add(destination); var path = new List<Int2> { new Int2(1, 2), new Int2(2, 2), new Int2(3, 2) }; foreach (var p in path) w.TileAt(p).Rail = true;
            var vehicle = new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position }; v.Vehicles.Add(vehicle);
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = "logs", Amount = 8, Path = path, Repeat = true }); w.Step(3);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            var station = Station(w, v, "research_hut"); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }); w.Step(10);
            Check(!v.Routes.Any(r => r.CatId == c.Id), "Route retained reassigned cat ownership"); Near(Goods(w, v, "logs"), 20, "Direct-control transport conservation"); Valid(w);
        }
        static void ItemPile()
        {
            var w = Fixture(out var v, out var c); var p = v.Stockpiles[0]; p.Goods.Clear(); var item = new Item { Id = w.Id("tool"), Kind = "tool", Material = "wood", VillageId = v.Id, LocationId = p.Id }; v.Items.Add(item);
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = p.Id }); Check(v.Items.Single().Id == item.Id && v.Stockpiles.Any(x => x.Id == item.LocationId), "Removing item-only pile orphaned item");
        }
        static void FarmReplacement()
        {
            var w = Fixture(out var v, out var a); var b = v.Cats[1]; b.ControlledBy = ""; var f = new Farm { Id = w.Id("farm"), Position = new Int2(8, 1), Handoff = new Int2(7, 1) }; v.Farms.Add(f);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = a.Id, TargetId = f.Id }); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = b.Id, TargetId = f.Id }); Check(a.BuildingId == "", "Replaced farm worker still assigned"); Check(f.WorkerId == b.Id, "New worker absent");
        }
        static void PreservedFood()
        { var w = Fixture(out var v, out var c); c.Hunger = 1; v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "food"); World.Add(v.Stockpiles[0].Goods, "preserves", 8); w.Step(60); Check(c.Hunger > 35, "Preserved food cannot satisfy hunger"); Check(Goods(w, v, "preserves") < 8, "Eating did not debit finite serving"); }
        static void EmergencyWater()
        {
            var w = Fixture(out var v, out var c); v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water"); var station = Station(w, v, "research_hut"); c.Thirst = 1; Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }); w.Step(240);
            Check(c.Alive && c.Thirst > 15, "No emergency fetch from staffed colony"); Check(Goods(w, v, "water") > 0 || c.Thirst > 35, "No physical water recovery");
        }
        static void RaidConsequences()
        {
            var w = Fixture(out var v, out var c); World.Add(v.Stockpiles[0].Goods, "materials", 500); v.Raids.Add(new Raid { Id = w.Id("raid"), Position = new Int2(0, 5), Strength = 100, Health = 100 }); var health = v.Cats.Sum(x => x.Health); w.Step(1800);
            Check(v.Raids.Count == 0 || v.Cats.Sum(x => x.Health) < health - 1 || Goods(w, v, "materials") < 500, "Undefended raid remains inert without consequences");
        }
        static double ProductionProgress(double skill)
        {
            var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList(); var recipe = Catalog.Recipes.First(r => r.Labor == "mill"); var b = Station(w, v, recipe.Building); c.Skills.Add(new Stack(recipe.Labor, skill)); foreach (var s in recipe.Inputs) World.Add(v.Stockpiles[0].Goods, s.Resource, 100); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe.Id }); w.Step(100); return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
        }
        static void Expertise() => Check(ProductionProgress(100) > ProductionProgress(0) * 1.05, "Earned milling skill does not improve milling work");
        static void Accounting()
        {
            var w = Fixture(out var v, out var c); var b = Station(w, v, "accounting_tent"); v.Research.Add("basic_tools"); Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = c.Id }); Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            var unreachable = new Stockpile { Id = "aaa-unreachable", Position = new Int2(-3, 2) }; v.Stockpiles.Insert(0, unreachable); w.TileAt(unreachable.Position).Wall = true; w.Step(90);
            Check(v.Stockpiles.Any(p => p.Id != "aaa-unreachable" && p.CountedAt >= 0), "Unreachable pile starves all accounting rounds"); Check(unreachable.CountedAt < 0, "Unreachable pile falsely counted");
        }
        static void Mountain()
        {
            var w = Fixture(out var v, out var c); foreach (var n in Catalog.Research) v.Research.Add(n.Id); var p = new Int2(1, 2); w.TileAt(p).Mountain = true; c.Position = new Int2(1, 1); c.X = 1; c.Z = 1;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) }); w.Step(2); Check(c.Position.Equals(p), "Mountaineering did not open mountain route");
        }
        static void StationCapacity()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Research = Catalog.Research.Where(n => n.Id != "clothier_stores").Select(n => n.Id).ToList(); World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var made = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "clothier", Position = new Int2(-5, 1), CatId = c.Id }); w.Step(500); var piles = v.Stockpiles.Where(p => p.ManagedBy == made.EntityId).ToArray(); Check(piles.Length > 0, "Completed Clothier has no physical station reserve"); double before = piles.Sum(p => w.Capacity(v, p)); Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "clothier_stores" }); Check(piles.Sum(p => w.Capacity(v, p)) > before, "Clothier stores research has no finite headroom effect");
        }
        static void Beds()
        {
            var w = Fixture(out var v, out var c); v.Buildings.Add(new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(-4, 1), Completed = true }); foreach (var mother in v.Cats.Take(5)) mother.PregnantUntil = 59;
            var migrant = new Cat { Id = w.Id("migrant"), Name = "Probationer", VillageId = v.Id, Position = v.Center, Migration = "probationary", ProbationUntil = 100000 }; v.Cats.Add(migrant); w.Step(60); Valid(w);
            Check(v.Cats.Count(x => x.Alive && x.AgeHours < 1) == 5, "Reserved pregnancies failed to receive beds"); Check(migrant.BedId == "", "Migrant took reserved bed");
        }
        static (double damage, double health) Battle(bool equipped)
        {
            var w = Fixture(out var v, out var c); c.Position = v.Center; c.X = 0; c.Z = 0; c.Skills.Add(new Stack("fight", 10)); c.BuildingId = "";
            var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Strength = 50, Health = 1000 }; v.Raids.Add(raid);
            foreach (var kind in new[] { "weapon", "armor" }) v.Items.Add(new Item { Id = w.Id(kind), Kind = kind, Material = "metal", Quality = 4, VillageId = v.Id, LocationId = v.Stockpiles[0].Id });
            if (equipped) foreach (var item in v.Items.ToArray()) Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = item.Id });
            w.Step(30); return (1000 - raid.Health, c.Health);
        }
        static void Equipment() { var plain = Battle(false); var equipped = Battle(true); Check(equipped.damage > plain.damage * 1.1, "Weapon does not improve actual combat damage"); Check(equipped.health > plain.health, "Armor does not reduce actual combat injury"); }
        static void Exterior(World w, Village v)
        {
            for (int x = 7; x <= 18; x++) for (int z = 7; z <= 18; z++) { var p = new Int2(x, z); var t = w.TileAt(p); t.Water = t.Mountain = t.Wall = false; t.Road = t.Rail = false; t.Resource = ""; t.Amount = 0; t.ClaimId = ""; if (!v.Known.Contains(p)) v.Known.Add(p); }
        }
        static void FixtureAccessRoad(World w, Village v, Int2 destination)
        {
            Check(w.TimeSeconds == 0, "Fixture roads must be established before play");
            var at = new Int2(v.Center.X, v.Center.Z + v.Radius + 1);
            while (true)
            {
                var tile = w.TileAt(at); tile.Road = true; tile.Water = tile.Mountain = tile.Wall = false; tile.Resource = ""; tile.Amount = 0;
                if (!v.Known.Contains(at)) v.Known.Add(at);
                if (at.Equals(destination)) break;
                if (at.Z != destination.Z) at.Z += Math.Sign(destination.Z - at.Z); else at.X += Math.Sign(destination.X - at.X);
            }
        }
        static void Zones()
        {
            var w = Fixture(out var v, out var c); Exterior(w, v); c.BuildingId = ""; c.Position = new Int2(8, 8); c.X = 8; c.Z = 8;
            foreach (var t in w.Tiles.Where(t => t.Resource == "logs")) { t.Resource = ""; t.Amount = 0; }
            var near = new Int2(9, 8); var preferred = new Int2(12, 8); w.TileAt(near).Resource = w.TileAt(preferred).Resource = "logs"; w.TileAt(near).Amount = w.TileAt(preferred).Amount = 20;
            Act(w, v, new GameAction { Kind = "CreateZone", Resource = "gather", Position = preferred, End = preferred });
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "logs", CatId = c.Id }); Check(v.Jobs.Single(j => j.CatId == c.Id && !j.Completed).Position.Equals(preferred), "Gather zone does not influence source selection");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id }); Act(w, v, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = near, End = near });
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); Check(!w.Apply(Context(v), new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) }).Success, "Avoid zone did not constrain ordinary route");
        }
        static void TargetedScout()
        {
            var w = Fixture(out var v, out var c); Exterior(w, v); c.BuildingId = ""; c.Position = new Int2(8, 8); c.X = 8; c.Z = 8; var target = new Int2(11, 10);
            foreach (var t in w.Tiles) { if (t.Resource == "ore") { t.Resource = ""; t.Amount = 0; } }
            w.TileAt(target).Resource = "ore"; w.TileAt(target).Amount = 30; v.Known.RemoveAll(p => p.X >= 9 && p.Z >= 9);
            Act(w, v, new GameAction { Kind = "DispatchScout", CatId = c.Id, Resource = "ore" }); w.Step(240); Check(v.Known.Contains(target), "Resource-targeted scout failed to return nearby ore discovery to shrine"); Check(c.ScoutNotes.Count == 0, "Scout discovery not committed by shrine return");
        }
        static void QuarryResource()
        {
            var w = Fixture(out var v, out var c); Exterior(w, v); c.BuildingId = ""; c.Position = new Int2(8, 8); c.X = 8; c.Z = 8; var at = new Int2(9, 8); w.TileAt(at).Resource = "gem"; w.TileAt(at).Amount = 30;
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "quarry", Resource = "gem", CatId = c.Id }); w.Step(240); Check(w.Total(v, "gem") > 0, "Selected Gem quarry did not deliver Gem"); Near(w.TileAt(at).Amount + Goods(w, v, "gem"), 30, "Quarry source-to-storage Gem conservation");
        }
        static void ProcessedNeeds()
        {
            var w = Fixture(out var v, out var c); c.Health = 10; c.Hunger = 100; c.Thirst = 100; World.Add(v.Stockpiles[0].Goods, "medicine", 5); w.Step(30); Check(c.Health > 30 && Goods(w, v, "medicine") < 5, "Medicine has no finite healing consumer");
            w = Fixture(out v, out c); c.Thirst = 1; World.Add(v.Stockpiles[0].Goods, "brew", 5); w.Step(30); Check(c.Thirst > 35, "Brew followed by clean water did not satisfy thirst"); Near(Goods(w, v, "brew"), 4.75, "One drink must consume exactly one quarter Brew serving"); Near(Goods(w, v, "water"), 999.25, "Brew drink must finish with three quarters clean Water");
        }
        static void ControlledCargo()
        {
            var w = Fixture(out var v, out var c); c.Thirst = 1; c.Cargo.Add(new Stack("logs", 8)); Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id }); w.Step(60);
            Check(c.Alive && c.Thirst > 35, "Leaving direct control with cargo prevents AI need recovery"); Near(Goods(w, v, "logs"), 8, "Direct-control cargo disappeared");
        }
        static void ControlledScaffoldCargo(string resource, bool planAfterControl = false)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; c.Position = new Int2(2, 2); c.X = 2; c.Z = 2;
            var source = v.Stockpiles[0]; World.Add(source.Goods, "planks", 12); World.Add(source.Goods, "blocks", 2);
            Building scaffold = null;
            if (!planAfterControl)
            {
                var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                scaffold = v.Buildings.Single(b => b.Id == planned.EntityId);
            }
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = source.Id, Resource = resource, Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Check(c.JobId == "" && World.Amount(c.Cargo, resource) == 8, "Fixture must leave direct control with cargo and no active job");
            double initialFood = Goods(w, v, "food"), initialPlanks = Goods(w, v, "planks"), initialBlocks = Goods(w, v, "blocks");
            if (planAfterControl)
            {
                var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                scaffold = v.Buildings.Single(b => b.Id == planned.EntityId);
            }
            else Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = scaffold.Id });
            bool preservedAtHandoff = c.Cargo.Count == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, resource) == 8);
            for (int tick = 0; tick < 600 && !scaffold.Completed; tick++) w.Step(1);
            Check(scaffold.Completed, "Reassigned scaffold did not complete with its finite material bill");
            Near(Goods(w, v, "food"), initialFood * Math.Pow(1 - 0.0002, Math.Floor(w.TimeSeconds / 60)), "Construction destroyed unrelated food beyond ordinary spoilage");
            Near(Goods(w, v, "planks"), initialPlanks - World.Amount(scaffold.Required, "planks"), "Construction consumed surplus planks beyond its material bill");
            Near(Goods(w, v, "blocks"), initialBlocks - World.Amount(scaffold.Required, "blocks"), "Construction block bill was not conserved");
            Check(preservedAtHandoff, "New work must leave prior cargo in a physical spill before fetching construction materials");
            Check(c.Cargo.Count == 0 && scaffold.Inputs.Count == 0 && w.Reservations.Count == 0, "Completed construction left cargo or input claims"); Valid(w);
        }
        static void ControlledNewJobCargo(string kind)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; c.Position = new Int2(2, 2); c.X = 2; c.Z = 2;
            var storage = v.Stockpiles[0]; storage.Accepts.AddRange(new[] { "food", "water", "materials", "logs" });
            World.Add(storage.Goods, "stone", 8); World.Add(storage.Goods, "materials", 15);
            var haulSource = new Stockpile { Id = w.Id("haul-source"), Kind = "spill", Position = new Int2(-2, 1), Goods = new List<Stack> { new Stack("logs", 8) } }; v.Stockpiles.Add(haulSource);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = storage.Id, Resource = "stone", Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Check(c.JobId == "" && World.Amount(c.Cargo, "stone") == 8, "Fixture must start new work with unassigned controlled cargo");
            var action = kind == "offering" ? new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id } : kind == "scout" ? new GameAction { Kind = "DispatchScout", CatId = c.Id } : new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = haulSource.Id };
            Act(w, v, action); var job = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
            bool preservedAtStart = c.Cargo.Count == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, "stone") == 8);
            bool carriedClaimedInput = false;
            for (int tick = 0; tick < 600 && !job.Completed; tick++)
            {
                w.Step(1); carriedClaimedInput |= World.Amount(c.Cargo, kind == "offering" ? "materials" : "logs") > 0;
            }
            Check(job.Completed, "Public " + kind + " failed to complete while preserving prior cargo");
            Near(Goods(w, v, "stone"), 8, "New " + kind + " consumed prior cargo");
            Check(preservedAtStart, "New " + kind + " must preserve prior cargo before claiming work ownership");
            Check(c.Cargo.Count == 0 && w.Reservations.Count == 0, "New " + kind + " left cargo or reservations after completion");
            if (kind == "offering")
            {
                Check(carriedClaimedInput, "Offering bypassed physical claimed input pickup"); Near(Goods(w, v, "materials"), 10, "Offering consumed the wrong finite bill"); Near(v.Blessings, 1, "Offering converted unrelated cargo into blessings");
            }
            if (kind == "haul")
            {
                Check(carriedClaimedInput, "Haul bypassed physical claimed source pickup"); Near(World.Amount(storage.Goods, "logs"), 8, "Haul did not deliver its claimed logs to accepting storage"); Near(World.Amount(haulSource.Goods, "logs"), 0, "Haul duplicated its finite source");
            }
            if (kind == "scout") Check(c.ScoutNotes.Count == 0 && c.Position.Equals(v.Center), "Scout failed to return its knowledge to the shrine");
            Valid(w);
        }
        static void ControlledResumeCargo(string kind)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; string material = kind == "rail" ? "metal" : "materials";
            var replacement = v.Cats[1]; replacement.ControlledBy = ""; replacement.BuildingId = "fixture-held"; replacement.Position = new Int2(2, 2); replacement.X = 2; replacement.Z = 2;
            var holdStation = Station(w, v, "wood_cutter", new Int2(5, 1)); World.Add(v.Stockpiles[0].Goods, material, 1000);
            var first = new Int2(1, 2); var last = new Int2(1, 3);
            foreach (var at in new[] { first, last }) { var tile = w.TileAt(at); tile.Road = tile.Rail = tile.Wall = tile.Water = tile.Mountain = false; }
            w.TileAt(new Int2(0, 2)).Road = true;
            if (kind == "rail") v.Research.Add("rail");
            int newRadius = v.Radius + 2;
            if (kind == "expand") foreach (var tile in w.Tiles.Where(t => Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)) { tile.Wall = tile.Water = tile.Mountain = false; tile.Resource = ""; tile.Amount = 0; }
            var action = kind == "expand" ? new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id } : new GameAction { Kind = kind == "road" ? "BuildRoad" : "DesignateRail", Position = first, End = last, CatId = c.Id };
            Act(w, v, action); var pending = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
            for (int tick = 0; tick < 1800 && !(pending.Phase == "working" && pending.Progress > 0 && World.Amount(c.Cargo, material) == 1); tick++) w.Step(1);
            Check(pending.Phase == "working" && pending.Progress > 0 && World.Amount(c.Cargo, material) == 1, "Fixture must interrupt active " + kind + " work with its real segment material in cargo");
            int builtBefore = pending.PathIndex;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Check(pending.CatId == "" && w.Reservations.Any(r => r.OwnerId == pending.Id + ":resume" && r.Resource == material && r.Amount == 1), "Interruption did not reserve the carried segment material for resumption");
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = holdStation.Id });
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = replacement.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = replacement.Id, TargetId = v.Stockpiles[0].Id, Resource = "food", Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = replacement.Id });
            Check(replacement.JobId == "" && World.Amount(replacement.Cargo, "food") == 8, "Replacement must carry unrelated food before pending work adoption");
            if (kind == "expand") Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = replacement.Id });
            else w.Step(1);
            bool preservedAtAdoption = World.Amount(replacement.Cargo, "food") == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, "food") == 8);
            Check(pending.CatId == replacement.Id && replacement.JobId == pending.Id, "Pending " + kind + " was not assigned to its replacement worker");
            for (int tick = 0; tick < 12000 && !pending.Completed; tick++) w.Step(1);
            Check(pending.Completed && pending.PathIndex == pending.Path.Count && pending.PathIndex > builtBefore, "Resumed " + kind + " failed to finish its physical path");
            Near(Goods(w, v, material), 1000 - pending.Path.Count, "Resumed " + kind + " failed to consume exactly one real material per completed segment");
            Check(preservedAtAdoption, "Pending " + kind + " adoption must preserve unrelated cargo before claiming the worker");
            Check(w.Reservations.Count == 0 && replacement.Cargo.Count == 0, "Resumed " + kind + " retained cargo or suspended claims after completion");
            Check(pending.Path.All(at => kind == "road" ? w.TileAt(at).Road : kind == "rail" ? w.TileAt(at).Rail : w.TileAt(at).Wall), "Resumed " + kind + " lost a completed physical segment");
            if (kind == "expand")
            {
                Check(v.Radius == newRadius, "Resumed expansion never committed its complete perimeter");
                Check(!w.TileAt(new Int2(0, newRadius)).Wall, "Expansion closed the required south gate");
                Check(!w.Tiles.Any(tile => Math.Abs(tile.Position.X) < newRadius && Math.Abs(tile.Position.Z) < newRadius && tile.Wall), "Expansion retained an obsolete interior wall");
            }
            Valid(w);
        }
        static Stockpile ArmorPile(World w, Village v)
        {
            var at = new Int2(-5, 1);
            var result = Act(w, v, new GameAction { Kind = "DesignateStockpile", Position = at, End = at, Accepts = new List<string> { "armor" } });
            return v.Stockpiles.Single(p => p.Id == result.EntityId);
        }
        static void ArmorProductionStorage()
        {
            var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var recipe = Catalog.Recipe("smithy_armor"); var station = Station(w, v, recipe.Building); var source = v.Stockpiles[0];
            source.Accepts.AddRange(new[] { "food", "water", "metal" }); World.Add(source.Goods, "metal", w.RecipeInput(v, recipe, recipe.Inputs.Single().Amount));
            var destination = ArmorPile(w, v);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = recipe.Id });
            for (int tick = 0; tick < 1800 && v.Items.Count == 0; tick++) w.Step(1);
            Check(v.Items.Count == 1, "Armor recipe did not create exactly one physical item");
            var item = v.Items.Single(); string id = item.Id; double condition = item.Condition, maximum = item.MaxCondition;
            Check(item.Kind == "armor" && item.LocationId != destination.Id, "Fixture must observe produced armor before physical delivery");
            for (int tick = 0; tick < 100 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); tick++) w.Step(1);
            Check(v.Jobs.Any(j => j.Kind == "production" && j.Completed), "Armor production is blocked despite reachable armor-only storage");
            Check(v.Items.Count == 1 && v.Items.Single().Id == id && item.LocationId == destination.Id, "Armor delivery changed identity or missed accepting storage");
            Near(item.Condition, condition, "Armor delivery changed condition"); Near(item.MaxCondition, maximum, "Armor delivery changed maximum condition");
            Near(Goods(w, v, "armor"), 0, "Armor delivery duplicated an exact item as scalar goods"); Near(Goods(w, v, "metal"), 0, "Armor production did not consume its exact material bill"); Valid(w);
        }
        static void ArmorUnequipStorage()
        {
            var w = Fixture(out var v, out var c); v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" }); var destination = ArmorPile(w, v);
            var item = new Item { Id = w.Id("armor"), Kind = "armor", Material = "metal", Quality = 2, VillageId = v.Id, LocationId = destination.Id, Condition = 37.5, MaxCondition = 73 };
            v.Items.Add(item); string id = item.Id;
            Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = id });
            Check(c.Equipment.Contains(id) && item.LocationId == c.Id, "Armor did not enter the selected cat's equipment slot");
            Act(w, v, new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id });
            Check(v.Items.Count == 1 && v.Items.Single().Id == id && item.LocationId == destination.Id && !c.Equipment.Contains(id), "Unequipping lost exact armor identity or failed to use accepting storage");
            Near(item.Condition, 37.5, "Unequipping changed armor condition"); Near(item.MaxCondition, 73, "Unequipping changed armor maximum condition");
            Near(Goods(w, v, "armor"), 0, "Unequipping duplicated an exact item as scalar goods"); Valid(w);
        }
        static Stockpile MugPile(World w, Village v, Int2 at)
        {
            var result = Act(w, v, new GameAction { Kind = "DesignateStockpile", Position = at, End = at });
            return v.Stockpiles.Single(p => p.Id == result.EntityId);
        }
        static World ExactHaulFixture(out Village v, out Cat c, out Item item, out Stockpile source, out Stockpile destination)
        {
            var w = Fixture(out v, out c); c.BuildingId = ""; v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            source = MugPile(w, v, new Int2(-5, 1)); destination = MugPile(w, v, new Int2(5, 1));
            item = new Item { Id = w.Id("mug"), Kind = "mug", Material = "clay", Quality = 2, Condition = 54.25, MaxCondition = 120, VillageId = v.Id, LocationId = source.Id };
            v.Items.Add(item); return w;
        }
        static void FinishExactHaul(World w, Village v, Cat c, Job job, Item item, Stockpile destination)
        {
            string id = item.Id, material = item.Material; double condition = item.Condition, maximum = item.MaxCondition; int quality = item.Quality;
            bool carried = false;
            for (int tick = 0; tick < 300 && !job.Completed; tick++)
            {
                w.Step(1); carried |= job.Phase == "output_delivery" && item.LocationId == job.Id && job.ItemIds.Contains(id);
            }
            Check(job.Completed && carried, "Exact haul did not physically pick up and carry its item before completing");
            Check(v.Items.Count(i => i.Id == id) == 1 && item.LocationId == destination.Id && job.ItemIds.Count == 0, "Exact haul did not release one unchanged identity to its destination");
            Check(item.Material == material && item.Quality == quality, "Exact haul changed item material or quality"); Near(item.Condition, condition, "Exact haul changed item condition"); Near(item.MaxCondition, maximum, "Exact haul changed maximum condition");
            Near(Goods(w, v, "mugs"), 0, "Exact haul created scalar item copies"); Valid(w);
        }
        static void ProducedItemRecovery(bool interruptProduction)
        {
            var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList(); v.Trader.Phase = "trading"; v.Trader.Until = 100000;
            var recipe = Catalog.Recipe("clay_mug"); var station = Station(w, v, recipe.Building); var source = MugPile(w, v, new Int2(-5, 1));
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water", "clay" }); World.Add(v.Stockpiles[0].Goods, "clay", w.RecipeInput(v, recipe, recipe.Inputs.Single().Amount));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = recipe.Id });
            for (int tick = 0; tick < 1800 && !(interruptProduction ? v.Items.Count > 0 : v.Jobs.Any(j => j.Kind == "production" && j.Completed)); tick++) w.Step(1);
            var item = v.Items.Single(); string id = item.Id; Check(item.Kind == "mug", "Production did not create a mug");
            Check(interruptProduction ? v.Jobs.Any(j => !j.Completed && j.ItemIds.Contains(id) && item.LocationId == j.Id) : item.LocationId == source.Id, "Produced mug did not reach the requested interruption stage");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id }); Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            if (interruptProduction) source = v.Stockpiles.Single(p => p.Id == item.LocationId);
            Check(source.Kind == "spill" && source.Goods.Count == 0 && item.LocationId == source.Id, "Removing mug-only storage did not preserve the exact item in a spill");
            var destination = MugPile(w, v, new Int2(5, 1));
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }); var haul = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            FinishExactHaul(w, v, c, haul, item, destination);
            double coins = v.Coins; Act(w, v, new GameAction { Kind = "SellGoods", TargetId = id });
            Check(v.Items.All(i => i.Id != id) && v.Trader.Items.Count(i => i.Id == id) == 1 && v.Coins > coins, "Recovered produced mug could not be sold under its original identity");
        }
        static void ExactHaulInterruption(bool pickedUp, bool death)
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            var helper = v.Cats[1]; helper.ControlledBy = ""; helper.BuildingId = "fixture-held";
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }); var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            if (pickedUp) for (int tick = 0; tick < 100 && job.Phase != "output_delivery"; tick++) w.Step(1);
            if (pickedUp) for (int tick = 0; tick < 10 && c.Position.Equals(source.Position) && !job.Completed; tick++) w.Step(1);
            Check(pickedUp ? job.Phase == "output_delivery" && !job.Completed && !c.Position.Equals(source.Position) : !c.Position.Equals(source.Position), "Exact haul fixture did not reach requested interruption stage");
            var carrierPosition = c.Position;
            if (death) { c.Health = 0; w.Step(1); Check(!c.Alive, "Exact carrier death did not use the normal lifecycle"); }
            else Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Check(job.Completed && job.ItemIds.Count == 0 && v.Items.Count(i => i.Id == item.Id) == 1, "Interrupted exact haul retained a claim or lost its item");
            var recovered = v.Stockpiles.Single(p => p.Id == item.LocationId);
            Check(pickedUp ? recovered.Position.Equals(carrierPosition) : recovered.Id == source.Id, "Interruption teleported exact cargo before pickup or lost its physical carrier position");
            Near(item.Condition, 54.25, "Interruption changed exact item condition"); Near(item.MaxCondition, 120, "Interruption changed maximum condition");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id });
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = recovered.Id }); var resumed = v.Jobs.Single(j => !j.Completed && j.CatId == helper.Id);
            FinishExactHaul(w, v, helper, resumed, item, destination);
        }
        static void ExactHaulClaims()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            var helper = v.Cats[1]; helper.ControlledBy = ""; helper.BuildingId = "";
            var filler = new Item { Id = w.Id("mug"), Kind = "mug", VillageId = v.Id, LocationId = destination.Id }; v.Items.Add(filler); destination.Capacity = 1;
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            Check(!w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }).Success && item.LocationId == source.Id, "Full destination accepted or claimed an undeliverable item");
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = destination.Id }); destination = MugPile(w, v, new Int2(5, 4));
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }); var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            Check(item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Exact haul did not claim one identity exclusively");
            v.Trader.Phase = "trading"; v.Trader.Until = 100000;
            Check(!w.Apply(Context(v), new GameAction { Kind = "SellGoods", TargetId = item.Id }).Success && item.LocationId == job.Id, "Trader sold an exact item already claimed for physical hauling");
            Check(!w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = source.Id }).Success, "Second carrier double-claimed an exact item");
            Check(!w.Apply(Context(v), new GameAction { Kind = "RemoveGatherSpot", TargetId = source.Id }).Success, "Removing a source discarded a pending exact pickup");
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = destination.Id });
            for (int tick = 0; tick < 100 && job.Phase != "output_delivery"; tick++) w.Step(1);
            w.Step(10); Check(!job.Completed && item.LocationId == job.Id && job.ItemIds.Count == 1 && c.BlockedReason == "output_storage_full_or_unreachable", "Losing accepting storage discarded carried exact cargo");
            destination = MugPile(w, v, new Int2(5, 6)); FinishExactHaul(w, v, c, job, item, destination);
            Check(filler.LocationId != destination.Id, "Recovery silently moved an unrelated exact item");
        }
        static void ExactHaulSteward()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            Station(w, v, "workshop", new Int2(5, 6)); v.Research.Add("basic_tools");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id }); w.Step(10);
            var job = v.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id);
            Check(job != null && job.AutomatedBy == "steward" && item.LocationId == job.Id, "Steward did not claim a recoverable item-only spill");
            FinishExactHaul(w, v, c, job, item, destination);
        }
        static void ExactHaulSourceCapacity()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination); source.Capacity = 1;
            Check(!w.HasRoom(v, source, "logs", 1), "Fixture item did not occupy its physical source capacity");
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }); var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            Check(job.Phase == "item_fetch" && item.LocationId == job.Id && !c.Position.Equals(source.Position), "Stored item fixture did not reserve before physical pickup");
            Check(!w.HasRoom(v, source, "logs", 1), "Exact item claim freed source capacity before physical pickup");
            for (int tick = 0; tick < 100 && job.Phase == "item_fetch"; tick++) w.Step(1);
            Check(job.Phase == "output_delivery" && c.Position.Equals(source.Position) && w.HasRoom(v, source, "logs", 1), "Physical pickup did not free the source capacity");
            FinishExactHaul(w, v, c, job, item, destination);
        }
        static void ExactHaulStoredTransfer()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            Check(w.HasRoom(v, source, "logs", 1), "Fixture source must retain spare storage capacity");
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }); var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            FinishExactHaul(w, v, c, job, item, destination);
            Check(source.Kind == "storage" && item.LocationId != source.Id, "Exact haul returned its cargo to the original source instead of moving it");
        }
        static World TerritoryFixture(out Village v, out Cat c, out Village foreign, out PlayerContext actor)
        {
            var w = Fixture(out v, out c); c.BuildingId = ""; v.Research = Catalog.Research.Select(n => n.Id).ToList();
            foreach (var resource in new[] { "materials", "metal", "lumber", "planks", "blocks" }) World.Add(v.Stockpiles[0].Goods, resource, 1000);
            var owner = new PlayerContext { PlayerId = "terrain-owner-b" };
            var founded = w.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Private terrain fixture" }); Check(founded.Success, "Second identity could not found its private village");
            foreign = w.Village(founded.EntityId); foreach (var cat in foreign.Cats) { cat.ControlledBy = "fixture-held"; cat.ControlLeaseUntil = 10000000; }
            foreign.LastLeaderResearch = w.TimeSeconds; actor = new PlayerContext { PlayerId = "terrain-owner-a", VillageId = v.Id };
            Check(w.CanControl(owner, foreign) && !w.CanControl(actor, foreign), "Fixture identities do not establish private-village ownership"); return w;
        }
        static GameAction InfrastructureAction(string kind, string catId, Int2 at) => kind == "road" || kind == "rail" ? new GameAction { Kind = kind == "road" ? "BuildRoad" : "DesignateRail", CatId = catId, Position = at, End = at } : new GameAction { Kind = kind == "bridge" ? "BuildBridge" : kind == "dock" ? "BuildDock" : "BuildTransportVehicle", CatId = catId, Position = at, Mode = kind == "vessel" ? "shipping" : "rail" };
        static void InfrastructureSite(World w, Village v, string kind, Int2 at)
        {
            foreach (var p in new[] { at, new Int2(at.X - 1, at.Z), new Int2(at.X + 1, at.Z), new Int2(at.X, at.Z - 1), new Int2(at.X, at.Z + 1) })
            { var t = w.TileAt(p); t.Wall = t.Water = t.Mountain = t.Road = t.Rail = t.Dock = t.Bridge = false; if (!v.Known.Contains(p)) v.Known.Add(p); }
            if (kind == "road") w.TileAt(new Int2(at.X - 1, at.Z)).Road = true;
            if (kind == "bridge") w.TileAt(at).Water = true;
            if (kind == "dock") w.TileAt(new Int2(at.X + 1, at.Z)).Water = true;
            if (kind == "wagon") w.TileAt(at).Rail = true;
            if (kind == "vessel") w.TileAt(at).Dock = true;
        }
        static string TerrainState(Tile t) => t.ClaimId + ":" + t.Wall + ":" + t.Water + ":" + t.Mountain + ":" + t.Road + ":" + t.Rail + ":" + t.Bridge + ":" + t.Dock + ":" + t.Resource + ":" + t.Amount;
        static void ForeignInfrastructure(string kind, bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(1, 2); InfrastructureSite(w, v, kind, at);
            w.TileAt(at).ClaimId = staleBuilding ? v.Id : foreign.Id;
            if (staleBuilding) Station(w, foreign, "den", at);
            string terrain = TerrainState(w.TileAt(at)); int jobs = v.Jobs.Count, reservations = w.Reservations.Count;
            var result = w.Apply(actor, InfrastructureAction(kind, c.Id, at));
            Check(!result.Success, "Public " + kind + " accepted another village's " + (staleBuilding ? "building footprint with stale ClaimId" : "claimed terrain"));
            Check(TerrainState(w.TileAt(at)) == terrain && v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected foreign construction changed terrain or reserved inputs");
            if (!staleBuilding)
            {
                w.TileAt(at).ClaimId = "";
                foreign.Stockpiles.Add(new Stockpile { Id = w.Id("foreign-zone-hint"), Kind = "zone_gather", Position = at, Width = 1, Depth = 1 });
                result = w.Apply(actor, InfrastructureAction(kind, c.Id, at)); Check(result.Success, "Unclaimed exterior " + kind + " became unavailable: " + result.Error);
                var job = v.Jobs.Single(j => j.Id == result.EntityId); for (int tick = 0; tick < 500 && !job.Completed; tick++) w.Step(1);
                Check(job.Completed, "Allowed unclaimed " + kind + " did not complete physically"); Valid(w);
            }
        }
        static void PendingForeignInfrastructure(string kind)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(1, 2); InfrastructureSite(w, v, kind, at);
            var result = w.Apply(actor, InfrastructureAction(kind, c.Id, at)); Check(result.Success, "Could not plan own terrain before ownership change");
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            for (int tick = 0; tick < 200 && job.Phase != "working"; tick++) w.Step(1);
            Check(job.Phase == "working" && !job.Completed, "Fixture must change ownership during funded construction");
            w.TileAt(at).ClaimId = foreign.Id; string terrain = TerrainState(w.TileAt(at)); double progress = job.Progress;
            var quantities = new[] { "materials", "metal", "lumber" }.Select(r => Goods(w, v, r)).ToArray(); w.Step(180);
            Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.Progress == progress, "Pending " + kind + " advanced after its site became foreign");
            Check(TerrainState(w.TileAt(at)) == terrain && v.Vehicles.Count == 0, "Pending " + kind + " changed foreign terrain or created a vehicle there");
            for (int i = 0; i < quantities.Length; i++) Near(Goods(w, v, new[] { "materials", "metal", "lumber" }[i]), quantities[i], "Blocked foreign construction consumed its retained inputs"); Valid(w);
            w.TileAt(at).ClaimId = v.Id;
            for (int tick = 0; tick < 500 && !job.Completed; tick++) w.Step(1);
            Check(job.Completed && job.BlockedReason != "foreign_territory", "Construction could not resume when its own site became available"); Valid(w);
        }
        static void PrepareExpansion(World w, Village v)
        {
            int radius = v.Radius + 2;
            for (int x = -radius; x <= radius; x++) for (int z = -radius; z <= radius; z++)
            { var at = new Int2(v.Center.X + x, v.Center.Z + z); var t = w.TileAt(at); if (Math.Abs(x) == radius || Math.Abs(z) == radius) { t.Wall = t.Water = t.Mountain = false; } if (!v.Known.Contains(at)) v.Known.Add(at); }
        }
        static void ForeignExpansion(bool interior, bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); PrepareExpansion(w, v);
            var at = new Int2(v.Radius + (interior ? 1 : 2), 1); w.TileAt(at).ClaimId = staleBuilding ? "" : foreign.Id;
            if (staleBuilding) Station(w, foreign, "den", at);
            string terrain = TerrainState(w.TileAt(at)); int radius = v.Radius;
            var result = w.Apply(actor, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            Check(!result.Success, "Expansion accepted a foreign " + (staleBuilding ? "building" : interior ? "interior claim" : "ring claim"));
            Check(v.Radius == radius && TerrainState(w.TileAt(at)) == terrain && v.Jobs.Count == 0 && w.Reservations.Count == 0, "Rejected expansion changed ownership or reserved inputs"); Valid(w);
        }
        static void PendingForeignExpansion()
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); PrepareExpansion(w, v);
            var result = w.Apply(actor, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }); Check(result.Success, "Could not plan clear expansion"); var job = v.Jobs.Single(j => j.Id == result.EntityId);
            for (int tick = 0; tick < 1800 && job.PathIndex == 0; tick++) w.Step(1); Check(job.PathIndex > 0, "Expansion never paid for a physical perimeter segment");
            var at = new Int2(v.Radius + 1, 1); w.TileAt(at).ClaimId = foreign.Id; string terrain = TerrainState(w.TileAt(at)); int radius = v.Radius, completed = job.PathIndex; double goods = Goods(w, v, "materials");
            w.Step(180); Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.PathIndex == completed && v.Radius == radius, "Pending expansion advanced across a newly foreign interior claim");
            Check(TerrainState(w.TileAt(at)) == terrain && job.Path.Take(completed).All(p => w.TileAt(p).Wall), "Blocked expansion overwrote foreign ownership or lost paid segments"); Near(Goods(w, v, "materials"), goods, "Blocked expansion consumed retained materials"); Valid(w);
        }
        static void LegacyForeignExpansion(bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(1, 2); w.TileAt(at).ClaimId = staleBuilding ? v.Id : foreign.Id;
            if (staleBuilding) Station(w, foreign, "den", at);
            var job = new Job { Id = w.Id("imported-expand"), Kind = "expand", OriginalKind = "expand_village", Phase = "working", CatId = c.Id, Position = at, Progress = 9, RequiredWork = 10 }; v.Jobs.Add(job); c.JobId = job.Id;
            string terrain = TerrainState(w.TileAt(at)); int radius = v.Radius, claimed = v.ClaimedTiles.Count; w.Step(5);
            Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.Progress == 9, "Imported single-tile expansion advanced on foreign property");
            Check(TerrainState(w.TileAt(at)) == terrain && v.Radius == radius && v.ClaimedTiles.Count == claimed, "Imported single-tile expansion rewrote foreign terrain or boundaries"); Valid(w);
        }
        static void ForeignFounding()
        {
            var w = World.Create(41); var existing = w.Villages[0]; var owner = new PlayerContext { PlayerId = "terrain-founding-owner" }; uint hash = World.Hash(owner.PlayerId);
            var first = new Int2(80 + (int)(hash % 8) * 80, 80 + (int)((hash / 8) % 8) * 80); var deposit = new Int2(first.X - 12, first.Z - 4);
            var tile = w.TileAt(deposit); tile.ClaimId = existing.Id; tile.Resource = "gem"; tile.Amount = 17; string before = TerrainState(tile);
            var result = w.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Protected founding" }); Check(result.Success, "Could not choose a safe alternative founding site");
            Check(w.Village(result.EntityId).Center.Equals(new Int2(first.X + 80, first.Z)), "Founding did not skip the deterministic candidate whose starter deposit crosses foreign land");
            Check(TerrainState(tile) == before && w.GetTile(first) == null, "Founding overwrote foreign land or generated tiles while checking a rejected candidate"); Valid(w);
        }
        static void ForeignRecovery()
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(v.Center.X + 2, v.Center.Z + v.Radius + 2); w.TileAt(at).ClaimId = foreign.Id;
            foreach (var cat in v.Cats) cat.Alive = false;
            var scaffold = new Building { Id = w.Id("pending-scaffold"), Kind = "wood_cutter", Position = new Int2(-5, 1) }; v.Buildings.Add(scaffold);
            w.Reservations.Add(new Reservation { OwnerId = scaffold.Id, VillageId = v.Id, PileId = v.Stockpiles[0].Id, Resource = "materials", Amount = 1 });
            var trade = new TradeOffer { Id = w.Id("pending-trade"), FromVillageId = v.Id, ToVillageId = foreign.Id, Status = "offered" }; w.TradeOffers.Add(trade);
            string terrain = TerrainState(w.TileAt(at)); int run = v.Run; w.Step(120);
            Check(ReferenceEquals(w.Village(v.Id), v) && v.Run == run && !v.Cats.Any(cat => cat.Alive), "Conflicting extinction recovery silently replaced the colony");
            Check(TerrainState(w.TileAt(at)) == terrain && w.Reservations.Any(r => r.OwnerId == scaffold.Id) && trade.Status == "offered", "Blocked recovery changed foreign terrain or destroyed pending ownership");
            Check(v.Events.Count(e => e.Kind == "recovery_blocked") == 1, "Blocked recovery must report the conflict once rather than repeat every check"); Valid(w);
        }
        static void ForeignDesignation(bool fishing)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(1, 2); InfrastructureSite(w, v, "dock", at);
            if (fishing) w.TileAt(at).ClaimId = foreign.Id; else Station(w, foreign, "den", at);
            int piles = v.Stockpiles.Count;
            var result = w.Apply(actor, new GameAction { Kind = fishing ? "DesignateFishingSpot" : "CreateZone", Resource = "gather", Position = at, End = at });
            Check(!result.Success && v.Stockpiles.Count == piles, "Placed work site bypassed foreign terrain ownership"); Valid(w);
        }
        static void ForeignScaffold(bool pending)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor); var at = new Int2(-5, 1);
            if (!pending) Station(w, foreign, "den", at);
            var result = w.Apply(actor, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", CatId = c.Id, Position = at });
            if (!pending) { Check(!result.Success && w.Reservations.Count == 0, "New scaffold occupied a foreign building footprint under stale ClaimId"); return; }
            Check(result.Success, "Could not plan an own-territory scaffold"); var scaffold = v.Buildings.Single(b => b.Id == result.EntityId); var job = v.Jobs.Single(j => j.TargetId == scaffold.Id);
            w.TileAt(at).ClaimId = foreign.Id; double timber = Goods(w, v, "lumber"), blocks = Goods(w, v, "blocks"); int claims = w.Reservations.Count;
            w.Step(180); Check(!scaffold.Completed && scaffold.Progress == 0 && scaffold.Inputs.Count == 0 && job.BlockedReason == "foreign_territory", "Scaffold consumed inputs or advanced on newly foreign terrain");
            Check(w.Reservations.Count == claims, "Blocked scaffold discarded its finite material claims"); Near(Goods(w, v, "lumber"), timber, "Blocked scaffold consumed timber"); Near(Goods(w, v, "blocks"), blocks, "Blocked scaffold consumed blocks"); Valid(w);
        }
        static List<string> Ancestors(Study node)
        { var result = new HashSet<string>(); void Visit(string id) { if (!result.Add(id)) return; foreach (var p in Catalog.Study(id).Prerequisites) Visit(p); } foreach (var p in node.Prerequisites) Visit(p); return result.OrderBy(x => x, StringComparer.Ordinal).ToList(); }
        static World StudyFixture(string id, bool own, out Village v, out Cat c)
        { var w = Fixture(out v, out c); var node = Catalog.Study(id); v.Research = Ancestors(node); if (own) v.Research.Add(id); return w; }
        static void BuildingStudy(string id)
        {
            var node = Catalog.Study(id); foreach (var payload in node.Payloads.Where(p => p.Kind == "modify_building"))
            { double before = BuildingMeasure(id, payload, false), after = BuildingMeasure(id, payload, true); Check(after > before + 1e-8, "No functioning " + payload.Id + " " + payload.Attribute + " improvement: " + id + " before=" + before + " after=" + after); }
        }
        static double BuildingMeasure(string id, ResearchPayload p, bool own)
        {
            var w = StudyFixture(id, own, out var v, out var c); var b = Station(w, v, p.Id); c.Position = b.Position; c.X = b.Position.X; c.Z = b.Position.Z;
            if (p.Attribute == "capacity")
            { var pile = new Stockpile { Id = w.Id("station-store"), ManagedBy = b.Id, Position = b.Position, Capacity = 100 }; v.Stockpiles.Add(pile); return w.Capacity(v, pile); }
            if (p.Attribute == "worker_slots")
            { int assigned = 0; foreach (var worker in v.Cats.Take(8)) { worker.ControlledBy = ""; if (w.Apply(Context(v), new GameAction { Kind = "AssignWorker", CatId = worker.Id, BuildingId = b.Id }).Success) assigned++; } return assigned; }
            if (p.Id == "field")
            { var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = p.Attribute == "output" ? 7199 : 0 }; v.Farms.Add(f); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id }); w.Step(1); return p.Attribute == "output" ? f.Harvest : f.Growth; }
            if (p.Id == "school" || p.Id == "research_hut")
            { double before = v.ResearchPoints; Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id }); w.Step(10); return v.ResearchPoints - before; }
            var recipe = Catalog.Recipes.FirstOrDefault(r => r.Building == p.Id && (p.Attribute == "durability" ? r.ItemKind != "" : p.Attribute == "output" ? r.Outputs.Count > 0 : true)) ?? Catalog.Recipes.First(r => r.Building == p.Id);
            foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(e => e.Kind == "unlock_recipe" && e.Id == recipe.Id))) if (!v.Research.Contains(unlock.Id)) v.Research.Add(unlock.Id);
            var tracked = recipe.Inputs.Concat(recipe.Outputs).Select(s => s.Resource).ToArray(); foreach (var s in recipe.Inputs) World.Add(v.Stockpiles[0].Goods, s.Resource, 100); var initial = recipe.Outputs.Sum(s => Goods(w, v, s.Resource));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = recipe.Id, Edit = "add" });
            if (p.Attribute == "cycle_time") { w.Step(100); return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress); }
            for (int i = 0; i < 1800 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); i++) w.Step(1);
            Check(v.Jobs.Any(j => j.Kind == "production" && j.Completed), "Building effect probe recipe failed " + recipe.Id);
            // Exact goods stay whole; an output study may improve finite ingredient efficiency.
            return p.Attribute == "durability" ? v.Items.Sum(x => x.MaxCondition) : recipe.Outputs.Count > 0 ? recipe.Outputs.Sum(s => Goods(w, v, s.Resource)) - initial : recipe.Inputs.Sum(s => Goods(w, v, s.Resource));
        }
        static void ServiceStudy(string id)
        { var p = Catalog.Study(id).Payloads.First(p => p.Kind == "modify"); double before = ServiceMeasure(id, p.Id, false), after = ServiceMeasure(id, p.Id, true); Check(after > before + 1e-9, "No physical service effect " + id + " / " + p.Id + " before=" + before + " after=" + after); }
        static double ServiceMeasure(string id, string effect, bool own)
        {
            var w = StudyFixture(id, own, out var v, out var c);
            if (effect == "storageCapacity" || effect == "storagePerLevelMult") return w.Capacity(v, v.Stockpiles[0]);
            if (effect == "housingPerDen")
            { for (int i = 0; i < 100; i++) v.Cats.Add(new Cat { Id = w.Id("arrival"), Name = "Bed candidate", VillageId = v.Id, Position = v.Center, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000, Migration = "probationary", ProbationUntil = 100000 }); w.Step(60); return v.Cats.Count(cat => cat.Alive && cat.BedId != ""); }
            if (effect == "moveSpeedMult" || effect == "movementSpeed")
            { Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); double x = c.X; Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) }); w.Step(0.1); return c.X - x; }
            if (effect == "tradeValue")
            { v.Trader.Phase = "trading"; v.Trader.Until = 10000; var item = new Item { Id = w.Id("ware"), Kind = "mug", Material = "wood", VillageId = v.Id, LocationId = v.Stockpiles[0].Id }; v.Items.Add(item); Act(w, v, new GameAction { Kind = "SellGoods", TargetId = item.Id }); return v.Coins; }
            if (new[] { "combatPower", "combatPowerMult", "barracksReadiness", "defensePower", "defenseMult", "wallDefense" }.Contains(effect))
            { Station(w, v, "barracks"); Station(w, v, "walls"); c.Skills.Add(new Stack("fight", 10)); c.Position = v.Center; c.X = 0; c.Z = 0; c.Health = 60; var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Health = 1000 }; v.Raids.Add(raid); w.Step(30); return effect == "defensePower" || effect == "defenseMult" || effect == "wallDefense" ? c.Health : 1000 - raid.Health; }
            if (effect == "constructionSpeed")
            { c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }); w.Step(80); return v.Buildings.First(b => b.Id == planned.EntityId).Progress; }
            if (effect == "productionRate")
            { var b = Station(w, v, "wood_cutter"); World.Add(v.Stockpiles[0].Goods, "logs", 100); Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = "logs_to_planks" }); w.Step(100); return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress); }
            if (new[] { "huntYieldMult", "gatherYieldMult", "materialYieldMult", "haulCapacity", "waterCarryCapacity" }.Contains(effect))
            { string kind = effect == "huntYieldMult" ? "hunt" : effect == "gatherYieldMult" ? "fibre" : effect == "waterCarryCapacity" ? "water" : "logs"; string resource = kind == "hunt" ? "food" : kind; double initial = Goods(w, v, resource); c.BuildingId = ""; Act(w, v, new GameAction { Kind = "RequestJob", Name = kind, CatId = c.Id }); var job = v.Jobs.First(j => j.CatId == c.Id && !j.Completed); for (int t = 0; t < 300 && job.Phase != "output_delivery" && !job.Completed; t++) w.Step(1); Check(job.Phase == "output_delivery" || job.Completed, "Gather effect never reached finite output " + effect); return Goods(w, v, resource) - initial; }
            if (effect == "denStewardship") { w.Step(60); return c.Rest; }
            if (effect == "waterStewardship" || effect == "waterEfficiency") { Station(w, v, "water_bowl"); w.Step(60); return c.Thirst; }
            if (effect == "foodStorekeeping" || effect == "spoilageResistance") { Station(w, v, "food_storage"); w.Step(60); return Goods(w, v, "food"); }
            if (effect == "healthRecovery") { c.Health = 60; w.Step(60); return c.Health; }
            if (effect == "kittenGrowth") { Station(w, v, "nursery"); c.AgeHours = 1; w.Step(60); return c.AgeHours; }
            if (effect == "restRecovery") { Station(w, v, "beds"); c.Rest = 1; c.Goal = "need_sleep"; var den = v.Buildings.First(b => b.Id == c.BedId); c.Position = den.Position; c.X = c.Position.X; c.Z = c.Position.Z; w.Step(20); return c.Rest; }
            if (effect == "herbMedicineEfficacy") { Station(w, v, "herb_garden"); World.Add(v.Stockpiles[0].Goods, "medicine", 5); c.Health = 10; w.Step(30); return c.Health; }
            if (effect == "researchRate" || effect == "researchRateMult") { var b = Station(w, v, "research_hut"); double before = v.ResearchPoints; Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id }); w.Step(30); return v.ResearchPoints - before; }
            if (effect == "accountingSpeed") { var b = Station(w, v, "accounting_tent"); if (!v.Research.Contains("basic_tools")) v.Research.Add("basic_tools"); c.Position = v.Stockpiles[0].Position; c.X = c.Position.X; c.Z = c.Position.Z; Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = c.Id }); Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id }); for (int i = 0; i < 60 && b.Progress == 0; i++) w.Step(1); Check(b.Progress > 0, "Accountant never arrived and began counting"); return b.Progress; }
            if (effect == "fieldStewardship" || effect == "farmYield" || effect == "farmYieldMult") { Station(w, v, "field"); var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = effect == "fieldStewardship" ? 0 : 7199 }; v.Farms.Add(f); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id }); w.Step(1); return effect == "fieldStewardship" ? f.Growth : f.Harvest; }
            if (effect == "shrineBlessingYield") { World.Add(v.Stockpiles[0].Goods, "materials", 30); c.BuildingId = ""; Act(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id }); w.Step(100); return v.Blessings; }
            if (effect == "mouseFarmFood") { var b = Station(w, v, "mouse_farm"); World.Add(v.Stockpiles[0].Goods, "grain", 1); double before = Goods(w, v, "food"); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id }); w.Step(660); return Goods(w, v, "food") - before; }
            if (effect == "elderProtection")
            {
                Station(w, v, "elder_corner"); c.AgeHours = 300; c.ControlledBy = "fixture-held"; c.ControlLeaseUntil = 10000000; var node = Catalog.Study(id); double baseline = 1; foreach (var ancestor in Ancestors(node)) foreach (var p in Catalog.Study(ancestor).Payloads.Where(p => p.Id == effect)) baseline = p.Operation == "add" ? baseline + p.Value : baseline * p.Value; var delta = node.Payloads.First(p => p.Id == effect); double upgraded = delta.Operation == "add" ? baseline + delta.Value : baseline * delta.Value;
                double low = 0.0002 / upgraded, high = 0.0002 / baseline; bool found = false; for (uint state = 1; state < 10000000; state++) { double roll = unchecked(state * 1664525u + 1013904223u) / 4294967296.0; if (roll >= low && roll < high) { w.RandomState = state; found = true; break; } }
                Check(found, "No deterministic old-age boundary seed"); w.Step(60); return c.Alive ? 1 : 0;
            }
            throw new InvalidOperationException("Missing concrete service measurement " + effect);
        }
        static void ResourceStudy(string id)
        {
            double Measure(bool own)
            {
                var w = StudyFixture(id, own, out var v, out var c); var split = id.LastIndexOf('_'); string family = id.Substring(0, split), stage = id.Substring(split + 1); var recipe = Catalog.Recipes.FirstOrDefault(r => World.RecipeFamily(r) == family);
                if (stage == "reserves") { string resource = family == "foraging" ? "fibre" : family == "grain_milling" ? "grain" : "food"; return w.ResourceCapacity(v, v.Stockpiles[0], resource); }
                Check(recipe != null, "Resource family has no executable recipe " + family);
                if (stage == "preservation" && recipe.ItemKind == "") return w.ResourceCapacity(v, v.Stockpiles[0], recipe.Outputs[0].Resource);
                var station = Station(w, v, recipe.Building); foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == recipe.Id))) if (!v.Research.Contains(unlock.Id)) v.Research.Add(unlock.Id);
                foreach (var s in recipe.Inputs) World.Add(v.Stockpiles[0].Goods, s.Resource, 100); double initialInput = recipe.Inputs.Sum(s => Goods(w, v, s.Resource)); double initialOutput = recipe.Outputs.Sum(s => Goods(w, v, s.Resource));
                Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = c.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, RecipeId = recipe.Id, Edit = "add" }); int elapsed = 0;
                for (; elapsed < 1800 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); elapsed++) w.Step(1); Check(elapsed < 1800, "Resource-effect production failed " + id);
                if (stage == "bulk") return -elapsed; if (stage == "preservation") return v.Items.Sum(i => i.MaxCondition);
                Check(stage == "sources", "Unknown resource effect stage " + id); return recipe.ItemKind != "" ? recipe.Inputs.Sum(s => Goods(w, v, s.Resource)) - initialInput : recipe.Outputs.Sum(s => Goods(w, v, s.Resource)) - initialOutput;
            }
            double before = Measure(false), after = Measure(true); Check(after > before + 1e-8, "No finite resource effect " + id + " before=" + before + " after=" + after);
        }
        static void AllBuildings()
        {
            foreach (string kind in Catalog.Buildings.Where(k => k != "shrine"))
            {
                var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList(); c.BuildingId = ""; Exterior(w, v); World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); var at = kind == "field" ? new Int2(14, 14) : new Int2(-5, 1);
                if (kind == "field") FixtureAccessRoad(w, v, new Int2(at.X, at.Z - 1));
                var made = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = kind, Position = at, CatId = c.Id }); w.Step(600); var built = v.Buildings.Single(b => b.Id == made.EntityId); Check(built.Completed, "Public construction failed " + kind); Check(built.Inputs.Count == 0, "Completed construction retained spendable inputs " + kind); Valid(w);
            }
        }
        static void RailCapability()
        {
            var w = StudyFixture("rail", false, out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "metal", 100); var path = new GameAction { Kind = "DesignateRail", Position = new Int2(-6, 1), End = new Int2(-4, 1), CatId = c.Id };
            Check(!w.Apply(Context(v), path).Success, "Rail construction works without blueprint"); Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "rail" }); Act(w, v, path); w.Step(200); Check(Enumerable.Range(-6, 3).All(x => w.TileAt(new Int2(x, 1)).Rail), "Researched physical track did not finish");
            Act(w, v, new GameAction { Kind = "BuildTransportVehicle", Mode = "rail", Position = new Int2(-6, 1), CatId = c.Id }); w.Step(180); Check(v.Vehicles.Any(x => x.Mode == "rail"), "Rail vehicle remains unreachable"); Valid(w);
        }
        static void ShippingCapability()
        {
            var study = Catalog.Research.Single(n => n.Payloads.Any(p => p.Kind == "unlock_capability" && p.Id == "water_travel")); var w = StudyFixture(study.Id, false, out var v, out var c); Exterior(w, v); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "lumber", 100); w.TileAt(new Int2(8, 9)).Water = true; var dock = new GameAction { Kind = "BuildDock", Position = new Int2(8, 8), CatId = c.Id };
            Check(!w.Apply(Context(v), dock).Success, "Dock construction works without Shipping"); Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = study.Id }); Act(w, v, dock); w.Step(200); Check(w.TileAt(new Int2(8, 8)).Dock, "Shipping dock failed physical build"); Act(w, v, new GameAction { Kind = "BuildTransportVehicle", Mode = "shipping", Position = new Int2(8, 8), CatId = c.Id }); w.Step(200); Check(v.Vehicles.Any(x => x.Mode == "shipping"), "Shipping vessel unreachable"); Valid(w);
        }
        static void RepeatedQueue()
        {
            var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList(); World.Add(v.Stockpiles[0].Goods, "fibre", 100); var b = Station(w, v, "clothier"); Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id }); foreach (var id in new[] { "fibre_to_thread", "fibre_to_cloth" }) Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = id, Edit = "add", Repeat = true }); w.Step(1500);
            Check(w.Total(v, "cloth") > 0, "Repeating first queue entry starves downstream recipe"); Check(v.Jobs.Where(j => j.Kind == "production" && j.Completed).Select(j => j.RecipeId).Distinct().Count() == 2, "Both linked recipes were not completed"); Valid(w);
        }
        static void PauseQueue()
        {
            var w = Fixture(out var v, out var c); World.Add(v.Stockpiles[0].Goods, "logs", 50); var b = Station(w, v, "wood_cutter"); Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = "logs_to_planks", Edit = "add" }); w.Step(100); var job = v.Jobs.Single(j => j.Kind == "production" && !j.Completed); double progress = job.Progress, goods = Goods(w, v, "logs"); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "pause", Enabled = true }); w.Step(60); Near(job.Progress, progress, "Paused job kept working"); Near(Goods(w, v, "logs"), goods, "Paused inputs lost"); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "pause", Enabled = false }); w.Step(700); Check(job.Completed, "Paused recipe did not resume"); Valid(w);
        }
        static void ControlFlood()
        {
            var w = Fixture(out var v, out var c); Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); double time = w.TimeSeconds, x = c.X, z = c.Z; for (int i = 0; i < 1000; i++) Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) }); Near(w.TimeSeconds, time, "Action flood advanced authority clock"); Near(c.X, x, "Action flood directly moved cat"); Near(c.Z, z, "Action flood directly moved cat"); w.Step(0.1); Check(c.X > x && c.X - x < 1, "Continuous movement lacks bounded explicit-time integration");
        }
        static void DeathCargo()
        {
            var w = Fixture(out var v, out var c); c.Health = 0; c.Cargo.Add(new Stack("logs", 8)); var job = new Job { Id = w.Id("job"), Kind = "production", RecipeId = "logs_to_planks", CatId = c.Id, Phase = "working", Position = c.Position, Local = new List<Stack> { new Stack("logs", 2) } }; v.Jobs.Add(job); c.JobId = job.Id; var item = new Item { Id = w.Id("tool"), Kind = "tool", VillageId = v.Id, LocationId = c.Id }; v.Items.Add(item); c.Equipment.Add(item.Id); w.Step(1); Check(!c.Alive, "Zero-health worker did not die"); Near(Goods(w, v, "logs"), 10, "Death lost/duplicated raw cargo and staged inputs"); Check(v.Items.Count == 1 && v.Stockpiles.Any(p => p.Id == item.LocationId), "Death lost exact equipped item"); Valid(w);
        }
        static void Extinction()
        {
            var w = Fixture(out var v, out var c); var oldIds = v.Cats.Select(x => x.Id).ToArray(); foreach (var cat in v.Cats) cat.Health = 0; v.Blessings = 7; var pending = new Job { Id = w.Id("job"), Kind = "haul", CatId = c.Id, Local = new List<Stack> { new Stack("logs", 2) } }; v.Jobs.Add(pending); c.JobId = pending.Id; w.Step(60); v = w.Villages[0]; Check(v.Run == 2 && v.Cats.Count(x => x.Alive) == 30, "Extinction did not atomically restore communal founding"); Check(v.Buildings.Count(b => b.Kind == "den" && b.Completed) == 6, "Extinction did not restore complete housing"); Check(!v.Cats.Any(x => oldIds.Contains(x.Id)), "Extinction reused cat identities"); Check(v.Jobs.Count == 0 && w.Reservations.Count == 0, "Failed-run work survived reset"); Near(v.Blessings, 7, "Blessing remainder lost during reset"); Valid(w);
        }
        static void MigrantDeparture()
        {
            var w = Fixture(out var v, out var c); var migrant = new Cat { Id = w.Id("arrival"), Name = "Unhoused visitor", VillageId = v.Id, Migration = "probationary", ProbationUntil = 1, Position = v.Center }; v.Cats.Add(migrant); w.Step(100); Check(!migrant.Alive && Int2.Distance(migrant.Position, v.Center) > v.Radius, "Unhoused probationer did not physically leave after deadline"); Check(v.Run == 1, "Departure caused colony reset");
        }
        static void Enclosures()
        {
            var w = World.Create(7); Check(w.Villages[0].Radius == 9, "Communal enclosure must fit its30cat/6Den blueprint at radius9"); var made = Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Personal camp" }); var v = w.Village(made.EntityId); Check(v.Radius == 6 && v.Cats.Count(c => c.Alive) == 15, "Personal founding footprint/population changed");
        }
        static void OfficerVacancy()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; foreach (var worker in v.Cats.Take(6)) worker.ControlledBy = ""; v.Research = Catalog.Research.Select(n => n.Id).ToList(); World.Add(v.Stockpiles[0].Goods, "logs", 100); World.Add(v.Stockpiles[0].Goods, "stone", 100); var station = Station(w, v, "wood_cutter"); Station(w, v, "sawmill"); Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = c.Id }); w.Step(12); var slot = station.Slots.FirstOrDefault(s => s.CatId != ""); Check(slot != null, "Forester did not start automatic station staffing while retaining an idle reserve"); string workerId = slot.CatId; Act(w, v, new GameAction { Kind = "UnassignOfficer", Role = "forester" }); w.Step(1200); Check(!station.Slots.Any(s => s.CatId == workerId) && v.Cats.Single(x => x.Id == workerId).BuildingId == "", "Vacant Forester retained automatic production worker"); Check(!v.Jobs.Any(j => j.Kind == "production" && !j.Completed), "Vacant Forester kept creating production jobs"); Valid(w);
        }
        static void LegacyUpgrade(string name, int baseCost, int maximum)
        {
            var w = Fixture(out var v, out var c); v.Blessings = 1000000; double spent = 0, points = v.ResearchPoints; for (int level = 0; level < maximum; level++) { Act(w, v, new GameAction { Kind = "PurchaseUpgrade", Name = name }); spent += baseCost * (level + 1); Near(v.Blessings, 1000000 - spent, "Wrong escalating legacy upgrade cost " + name); }
            Check(!w.Apply(Context(v), new GameAction { Kind = "PurchaseUpgrade", Name = name }).Success, "Legacy upgrade exceeds maximum " + name); Near(v.ResearchPoints, points, "Legacy blessing upgrade consumed research points"); Near(v.Blessings, 1000000 - spent, "Rejected max upgrade charged again");
        }
        static void ResourceSpecificCapacity()
        {
            var w = StudyFixture("grain_milling_preservation", false, out var v, out var c); double logs = w.ResourceCapacity(v, v.Stockpiles[0], "logs"), flour = w.ResourceCapacity(v, v.Stockpiles[0], "flour"); Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "grain_milling_preservation" }); Near(w.ResourceCapacity(v, v.Stockpiles[0], "logs"), logs, "Grain preservation incorrectly increases Logs capacity"); Check(w.ResourceCapacity(v, v.Stockpiles[0], "flour") > flour, "Grain preservation does not increase Flour capacity");
        }
        static void ExpansionInterruption()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; World.Add(v.Stockpiles[0].Goods, "materials", 1000); int radius = v.Radius, newRadius = radius + 2;
            foreach (var t in w.Tiles.Where(t => Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)) { t.Wall = t.Water = t.Mountain = false; t.Resource = ""; t.Amount = 0; }
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }); for (int i = 0; i < 1200 && !w.Tiles.Any(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)); i++) w.Step(1);
            var built = w.Tiles.Where(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)).Select(t => t.Position).ToArray(); Check(built.Length > 0, "Expansion never built its first real segment"); Check(v.Radius == radius, "Expansion cut over before perimeter completion"); Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id }); Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id }); w.Step(12000); Check(v.Radius == newRadius, "Interrupted expansion cannot resume to complete its wall"); Check(built.All(p => w.TileAt(p).Wall), "Expansion interruption lost completed segments"); Check(!w.TileAt(new Int2(0, newRadius)).Wall, "Expansion closed the new south gate"); Valid(w);
        }
        static void EquipmentRail()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; var source = v.Stockpiles[0]; source.Position = new Int2(1, 2); var destination = new Stockpile { Id = w.Id("destination"), Position = new Int2(3, 2), Accepts = new List<string> { "tools" } }; v.Stockpiles.Add(destination); var path = new List<Int2> { new Int2(1, 2), new Int2(2, 2), new Int2(3, 2) }; foreach (var p in path) w.TileAt(p).Rail = true;
            v.Vehicles.Add(new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position }); var tool = new Item { Id = w.Id("tool"), Kind = "tool", Material = "metal", Quality = 3, Condition = 72, MaxCondition = 125, VillageId = v.Id, LocationId = source.Id }; v.Items.Add(tool);
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = "tools", Amount = 1, Path = path }); w.Step(30); Check(v.Items.Count == 1 && tool.LocationId == destination.Id, "Rail route failed to transport exact equipment"); Near(tool.Condition, 72, "Rail transfer changed exact item condition"); Near(World.Amount(source.Goods, "tools") + World.Amount(destination.Goods, "tools"), 0, "Rail invented scalar equipment copies"); Valid(w);
        }
        static void RailExpandedWall()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "materials", 1000); World.Add(v.Stockpiles[0].Goods, "metal", 11);
            var helper = v.Cats[1]; helper.ControlledBy = ""; helper.BuildingId = "fixture-held"; helper.Position = new Int2(10, 0); helper.X = 10; helper.Z = 0; helper.Cargo.Add(new Stack("logs", 8));
            for (int x = -13; x <= 13; x++) for (int z = -13; z <= 13; z++)
            { var p = new Int2(x, z); var t = w.TileAt(p); if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || Math.Abs(x) == 13 || Math.Abs(z) == 13 || x >= 10 && z == 1) t.Wall = t.Water = t.Mountain = false; if (x >= 10 && z == 1) t.Road = t.Rail = false; if (!v.Known.Contains(p)) v.Known.Add(p); }
            var sourceAt = new Int2(10, 1); var destinationAt = new Int2(12, 1); var wall = w.TileAt(new Int2(11, 1));
            var sourceId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = sourceAt, End = sourceAt }).EntityId;
            var destinationId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = destinationAt, End = destinationAt }).EntityId;
            var source = v.Stockpiles.Single(p => p.Id == sourceId); var destination = v.Stockpiles.Single(p => p.Id == destinationId);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id }); Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = sourceId }); Act(w, v, new GameAction { Kind = "LeaveCat", CatId = helper.Id }); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id, BuildingId = v.Buildings.First(b => b.Kind == "den").Id });
            void Complete(GameAction action, int limit)
            {
                var id = Act(w, v, action).EntityId; var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < limit && !job.Completed; tick++) w.Step(1); Check(job.Completed, "Public rail fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "DesignateRail", CatId = c.Id, Position = sourceAt, End = destinationAt }, 1000);
            Complete(new GameAction { Kind = "BuildTransportVehicle", CatId = c.Id, Position = sourceAt, Mode = "rail" }, 1000);
            Complete(new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }, 12000);
            Check(v.Radius == 11 && wall.Rail && wall.Wall, "Public expansion did not retain completed track beneath its new wall");
            var path = new List<Int2> { sourceAt, wall.Position, destinationAt }; var vehicle = v.Vehicles.Single(); string vehicleId = vehicle.Id, driverId = c.Id;
            var routeId = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = sourceId, BuildingId = destinationId, Mode = "rail", Resource = "logs", Amount = 8, Path = path }).EntityId;
            var route = v.Routes.Single(r => r.Id == routeId); for (int tick = 0; tick < 1000 && route.Phase != "outbound"; tick++) w.Step(1);
            Check(route.Phase == "outbound" && World.Amount(vehicle.Cargo, "logs") == 8 && vehicle.Position.Equals(sourceAt), "Route did not board and load the physical wagon");
            w.Step(3);
            Check(route.BlockedReason == "route_blocked" && route.PathIndex == 0 && vehicle.Position.Equals(sourceAt) && c.Position.Equals(sourceAt), "Loaded rail vehicle or driver crossed the completed expansion wall");
            Check(v.Routes.Single().Id == routeId && vehicle.RouteId == routeId && c.BuildingId == routeId && route.VehicleId == vehicleId && route.CatId == driverId, "Blocked rail lost vehicle or driver ownership");
            Near(World.Amount(vehicle.Cargo, "logs"), 8, "Blocked wagon lost cargo"); Near(World.Amount(destination.Goods, "logs"), 0, "Blocked wagon delivered through the wall"); Near(Goods(w, v, "logs"), 8, "Blocked rail duplicated or consumed logs");
            // The next public expansion removes the obsolete interior wall; no save or terrain reset is needed.
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id }); Complete(new GameAction { Kind = "RequestJob", Name = "expand", CatId = helper.Id }, 12000);
            for (int tick = 0; tick < 30 && v.Routes.Any(r => r.Id == routeId); tick++) w.Step(1);
            Check(v.Radius == 13 && !wall.Wall && wall.Rail && v.Vehicles.Single().Id == vehicleId && vehicle.Position.Equals(sourceAt), "Rail did not recover on the same physical wagon after expansion removed the wall");
            Check(!v.Routes.Any(r => r.Id == routeId) && vehicle.RouteId == "" && c.BuildingId == "" && vehicle.Cargo.Count == 0, "Recovered rail retained cargo or route ownership");
            Near(World.Amount(destination.Goods, "logs"), 8, "Recovered wagon did not deliver its held cargo"); Near(Goods(w, v, "logs"), 8, "Rail recovery lost or duplicated cargo"); Valid(w);
        }
        static void RailPassability(string obstruction)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; var source = v.Stockpiles[0]; source.Position = new Int2(-6, 1);
            var destination = new Stockpile { Id = w.Id("rail-destination"), Position = new Int2(-3, 1), Width = 1, Depth = 1 }; v.Stockpiles.Add(destination);
            var path = Enumerable.Range(-6, 4).Select(x => new Int2(x, 1)).ToList(); foreach (var p in path) { var t = w.TileAt(p); t.Wall = t.Water = t.Mountain = t.Road = t.Bridge = false; t.Rail = true; }
            var vehicle = new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position }; v.Vehicles.Add(vehicle); bool exact = obstruction == "exact_boundary", returning = obstruction == "return_boundary";
            var item = new Item { Id = w.Id("tool"), Kind = "tool", Material = "metal", Quality = 3, Condition = 57, MaxCondition = 88, VillageId = v.Id, LocationId = source.Id };
            if (exact) v.Items.Add(item); else World.Add(source.Goods, "logs", 8);
            var id = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = exact ? "tools" : "logs", Amount = exact ? 1 : 8, Path = path }).EntityId;
            var route = v.Routes.Single(r => r.Id == id); for (int tick = 0; tick < 100 && route.Phase != (returning ? "returning" : "outbound"); tick++) w.Step(1);
            Check(route.Phase == (returning ? "returning" : "outbound"), "Rail fixture did not reach its physical interruption stage");
            int index = route.PathIndex; var before = vehicle.Position; var next = path[index + (returning ? -1 : 1)]; var tile = w.TileAt(next); var edge = new BoundaryEdge { From = before, To = next };
            if (obstruction == "water") tile.Water = true; else v.BoundaryEdges.Add(edge);
            Check(!w.Walkable(v, next) || !w.Crossable(before, next), "Rail fault did not obstruct authoritative movement"); w.Step(3);
            Check(route.BlockedReason == "route_blocked" && route.PathIndex == index && vehicle.Position.Equals(before) && c.Position.Equals(before) && c.X == before.X && c.Z == before.Z, "Rail bypassed authoritative " + obstruction + " passability");
            Check(v.Routes.Single().Id == id && vehicle.RouteId == id && c.BuildingId == id && route.VehicleId == vehicle.Id && route.CatId == c.Id, "Blocked rail changed route, wagon or driver identity");
            if (exact) Check(v.Items.Count == 1 && item.LocationId == vehicle.Id && vehicle.ItemIds.SequenceEqual(new[] { item.Id }) && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Blocked rail changed exact cargo identity or condition");
            else { Near(World.Amount(vehicle.Cargo, "logs"), returning ? 0 : 8, "Blocked rail changed held cargo"); Near(World.Amount(destination.Goods, "logs"), returning ? 8 : 0, "Blocked rail delivered across the obstruction"); Near(Goods(w, v, "logs"), 8, "Blocked rail lost or duplicated cargo"); }
            // Remove only the injected obstruction; the saved route, wagon and cargo are reused.
            if (obstruction == "water") tile.Water = false; else v.BoundaryEdges.Remove(edge);
            for (int tick = 0; tick < 30 && v.Routes.Any(r => r.Id == id); tick++) w.Step(1);
            Check(!v.Routes.Any(r => r.Id == id) && vehicle.RouteId == "" && vehicle.Position.Equals(source.Position) && c.BuildingId == "" && vehicle.Cargo.Count == 0 && vehicle.ItemIds.Count == 0, "Cleared rail obstruction did not resume delivery and release the same route");
            if (exact) Check(v.Items.Count == 1 && item.LocationId == destination.Id && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Recovered rail changed exact delivered cargo"); else Near(World.Amount(destination.Goods, "logs"), 8, "Recovered rail did not deliver its conserved cargo"); Valid(w);
        }
        static void MerchantExpansionReroute()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; c.Skills.RemoveAll(s => s.Resource == "fight"); v.Research = Catalog.Research.Select(n => n.Id).ToList(); v.Coins = 10;
            World.Add(v.Stockpiles[0].Goods, "materials", 1000); World.Add(v.Stockpiles[0].Goods, "planks", 10); World.Add(v.Stockpiles[0].Goods, "blocks", 5);
            for (int x = -13; x <= 13; x++) for (int z = -13; z <= 16; z++)
            {
                var p = new Int2(x, z); var tile = w.TileAt(p);
                if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || z >= 10) tile.Wall = tile.Water = tile.Mountain = false;
                if (z >= 10) { tile.Road = false; tile.ClaimId = ""; }
                if (!v.Known.Contains(p)) v.Known.Add(p);
            }
            w.TileAt(new Int2(0, 10)).Road = true;
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(0, 11), To = new Int2(0, 12) }); v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(1, 12), To = new Int2(1, 13) });
            void Complete(GameAction action)
            {
                var id = Act(w, v, action).EntityId; var job = v.Jobs.Single(j => j.Id == id); for (int tick = 0; tick < 1000 && !job.Completed; tick++) w.Step(1); Check(job.Completed, "Public merchant fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(0, 11), End = new Int2(0, 15), CatId = c.Id }); Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(1, 11), End = new Int2(1, 12), CatId = c.Id });
            var fieldId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(-2, 15), CatId = c.Id }).EntityId; var field = v.Buildings.Single(b => b.Id == fieldId); for (int tick = 0; tick < 1000 && !field.Completed; tick++) w.Step(1); Check(field.Completed, "Public merchant Field did not complete");
            Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(1, 10), End = new Int2(3, 10) });
            double materials = Goods(w, v, "materials"); var expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId; var expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 && expansion.PathIndex < expansion.Path.Count; tick++) w.Step(1);
            var trader = v.Trader; Check(!expansion.Completed && expansion.PathIndex == expansion.Path.Count && w.TimeSeconds < trader.NextAt, "Merchant fixture did not reach the pre-cutover checkpoint before its natural visit");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            while (w.TimeSeconds < trader.NextAt) { Act(w, v, new GameAction { Kind = "KeepCatControl", CatId = c.Id }); w.Step(Math.Min(20, trader.NextAt - w.TimeSeconds)); }
            var from = new Int2(1, 10); var to = new Int2(0, 10); int crossing = trader.Path.FindIndex(p => p.Equals(from));
            Check(trader.Phase == "arriving" && trader.Position.Equals(new Int2(0, 12)) && crossing >= 0 && crossing + 1 < trader.Path.Count && trader.Path[crossing + 1].Equals(to), "Automatic merchant did not cache the future public farm-fence crossing: phase=" + trader.Phase + " position=" + trader.Position + " path=" + string.Join(";", trader.Path) + " wall=" + w.TileAt(new Int2(-1, 11)).Wall + " radius=" + v.Radius);
            string identity = trader.Id; long visits = v.TraderVisitCount; double coins = trader.Coins; var cargo = trader.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToArray(); var originalPath = trader.Path;
            // Inject a finite active threat before the public cutover, using ordinary pathfinding.
            var raid = new Raid { Id = w.Id("farm-fence-raid"), Position = trader.Position, Path = w.Path(trader.Position, v.Center), Health = 100, Loot = new List<Stack> { new Stack("logs", 8) } }; v.Raids.Add(raid); var raidPath = raid.Path;
            Check(raidPath != null && raidPath.Contains(from) && raidPath[raidPath.IndexOf(from) + 1].Equals(to), "Active raid did not cache the same future farm-fence edge"); double food = Goods(w, v, "food");
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id }); w.Step(1);
            Check(expansion.Completed && v.Radius == 11 && w.Walkable(from) && w.Walkable(to) && !w.Crossable(from, to), "Public expansion did not close the cached walkable merchant edge");
            bool stopped = false, rerouted = false, raidRerouted = false;
            for (int tick = 0; tick < 100 && (trader.Phase != "trading" || raid.Phase != "departing"); tick++)
            {
                var before = trader.Position; var raidBefore = raid.Position; double progress = trader.Progress; w.Step(1);
                Check(before.Equals(trader.Position) || Int2.Distance(before, trader.Position) == 1 && w.Crossable(before, trader.Position), "Automatic merchant crossed the completed public farm fence");
                Check(raidBefore.Equals(raid.Position) || Int2.Distance(raidBefore, raid.Position) == 1 && w.Crossable(raidBefore, raid.Position), "Active raid crossed the completed public farm fence");
                if (before.Equals(from) && before.Equals(trader.Position) && trader.BlockedReason == "blocked_route") { stopped = true; Check(trader.Progress == progress, "Blocked merchant consumed movement progress before replanning"); }
                rerouted |= !ReferenceEquals(originalPath, trader.Path);
                raidRerouted |= !ReferenceEquals(raidPath, raid.Path); Check(v.Raids.Contains(raid), "Farm fence removed the active raid identity"); Near(World.Amount(raid.Loot, "logs"), 8, "Farm-fence raid reroute changed its finite loot");
                Check(trader.Id == identity && v.TraderVisitCount == visits && trader.Coins == coins && trader.Goods.Count == cargo.Length, "Merchant reroute replaced its visit, purse or finite cargo"); foreach (var stack in cargo) Near(World.Amount(trader.Goods, stack.Resource), stack.Amount, "Public fence changed merchant cargo");
            }
            Check(stopped && rerouted && trader.Phase == "trading" && trader.Position.Equals(v.Center) && trader.Exterior.Value.Equals(new Int2(0, 12)), "Blocked merchant did not lawfully reroute the same visit to its shrine"); Near(Goods(w, v, "materials"), materials - expansion.Path.Count, "Merchant interruption changed the public expansion bill");
            Check(raidRerouted && raid.Phase == "departing" && raid.Position.Equals(v.Center) && v.Events.Count(e => e.EntityId == raid.Id && e.Kind == "raid_breach") == 1, "Active raid did not lawfully reroute to one physical shrine breach"); Near(Goods(w, v, "food"), food - 20, "Rerouted public-fence raid did not debit exactly one finite theft"); Near(World.Amount(raid.Loot, "food"), 20, "Rerouted public-fence raid did not retain the stolen food");
            double logs = Goods(w, v, "logs"), stock = World.Amount(trader.Goods, "logs"); Act(w, v, new GameAction { Kind = "BuyResource", Resource = "logs", Amount = 1 }); Near(Goods(w, v, "logs"), logs + 1, "Recovered merchant could not supply a public purchase"); Near(World.Amount(trader.Goods, "logs"), stock - 1, "Recovered merchant purchase did not debit finite stock"); Near(v.Coins, 8, "Recovered merchant purchase charged the wrong coin"); Near(trader.Coins, coins + 2, "Recovered merchant purchase did not credit its original purse"); Valid(w);
        }
        static void CachedLandMover(string kind, bool departing)
        {
            var w = Fixture(out var v, out var c); c.ControlledBy = "fixture-held"; c.ControlLeaseUntil = 10000000;
            var start = new Int2(-6, 1); var exit = new Int2(0, v.Radius + 3); var destination = departing ? exit : v.Center;
            foreach (var p in new[] { new Int2(-7, 1), new Int2(-6, 0), new Int2(-6, 2) }) w.TileAt(p).Wall = true;
            var path = w.Path(start, destination); Check(path != null && path.Count > 1 && path[0].Equals(new Int2(-5, 1)), "Land mover fixture did not cache its only physical exit");
            var edge = new BoundaryEdge { From = start, To = path[0] }; double food = Goods(w, v, "food");
            var trader = v.Trader; var item = new Item { Id = w.Id("merchant-tool"), Kind = "tool", LocationId = "trader", Quality = 3, Condition = 57, MaxCondition = 88 };
            var raid = new Raid { Id = w.Id("cached-raid"), Phase = departing ? "departing" : "approaching", Position = start, Path = new List<Int2>(path), Health = 100, Loot = new List<Stack> { new Stack("logs", 8) } };
            if (kind == "merchant")
            {
                trader.Id = w.Id("cached-merchant"); trader.Phase = departing ? "departing" : "arriving"; trader.Position = start; trader.Exterior = exit; trader.VisitDestination = v.Center; trader.Path = new List<Int2>(path); trader.Goods.Add(new Stack("logs", 8)); trader.Items.Add(item); trader.Coins = 17;
            }
            else v.Raids.Add(raid);
            w.Step(1); Check((kind == "merchant" ? trader.Position : raid.Position).Equals(start), "Land mover advanced before its movement budget was ready");
            v.BoundaryEdges.Add(edge); double progress = trader.Progress; string identity = kind == "merchant" ? trader.Id : raid.Id; w.Step(3);
            Check((kind == "merchant" ? trader.Position : raid.Position).Equals(start), "Cached " + kind + " " + (departing ? "departure" : "arrival") + " crossed a newly blocked edge");
            if (kind == "merchant")
            {
                Check(trader.BlockedReason == "blocked_route" && trader.Progress == progress && trader.PathIndex == 0 && trader.Phase == (departing ? "departing" : "arriving") && trader.Id == identity, "Blocked merchant changed its phase, progress or visit identity");
                Check(trader.Items.Count == 1 && trader.Items[0] == item && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Blocked merchant changed exact cargo identity or condition"); Near(World.Amount(trader.Goods, "logs"), 8, "Blocked merchant lost its finite goods"); Near(trader.Coins, 17, "Blocked merchant changed its purse"); Check(trader.LastDepartedAt < 0, "Blocked merchant completed its departure");
            }
            else { Check(v.Raids.Single().Id == identity && raid.Phase == (departing ? "departing" : "approaching"), "Blocked raid changed identity or phase"); Near(World.Amount(raid.Loot, "logs"), 8, "Blocked raid lost finite loot"); Near(Goods(w, v, "food"), food, "Blocked raid looted through the edge"); }
            v.BoundaryEdges.Remove(edge);
            bool Finished() => kind == "merchant" ? trader.Phase == (departing ? "absent" : "trading") : departing ? !v.Raids.Contains(raid) : raid.Phase == "departing";
            for (int tick = 0; tick < 100 && !Finished(); tick++)
            {
                var before = kind == "merchant" ? trader.Position : raid.Position; w.Step(1); var after = kind == "merchant" ? trader.Position : raid.Position;
                Check(before.Equals(after) || Int2.Distance(before, after) == 1 && w.Crossable(before, after), "Recovered land mover skipped a physical edge");
            }
            Check(Finished(), "Cleared edge did not resume the original " + kind + " journey");
            if (kind == "merchant")
            {
                Check(trader.Id == identity && trader.Position.Equals(destination), "Recovered merchant changed visit identity or missed its destination"); Near(World.Amount(trader.Goods, "logs"), 8, "Recovered merchant changed finite goods"); Near(trader.Coins, 17, "Recovered merchant changed its purse");
                if (departing) Check(trader.Items.Count == 0 && trader.LastDepartedAt > 4 && trader.NextAt > w.TimeSeconds && !v.Items.Any(i => i.Id == item.Id), "Departing merchant did not remove its external cargo once at the exit"); else Check(trader.Items.Single() == item && item.Condition == 57 && trader.Until > w.TimeSeconds, "Arriving merchant did not retain its exact cargo for trade");
            }
            else if (departing) Check(v.Events.Count(e => e.EntityId == identity && e.Kind == "raid_loss") == 1 && Goods(w, v, "logs") == 0, "Escaping raid duplicated its finite stolen cargo or departure");
            else { Near(Goods(w, v, "food"), food - 20, "Recovered raid did not debit one finite shrine theft"); Near(World.Amount(raid.Loot, "food"), 20, "Recovered raid did not retain the stolen goods"); Check(v.Events.Count(e => e.EntityId == identity && e.Kind == "raid_breach") == 1, "Recovered raid looted more than once"); }
            Valid(w);
        }
        static void CaravanExpansionFence()
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "materials", 2000); World.Add(v.Stockpiles[0].Goods, "planks", 10); World.Add(v.Stockpiles[0].Goods, "blocks", 5); World.Add(v.Stockpiles[0].Goods, "logs", 8);
            for (int x = -13; x <= 22; x++) for (int z = -13; z <= 13; z++)
            {
                var p = new Int2(x, z); var tile = w.TileAt(p);
                if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || Math.Abs(x) == 13 || Math.Abs(z) == 13 || x >= 10) tile.Wall = tile.Water = tile.Mountain = false;
                if (x >= 10) { tile.Road = false; tile.ClaimId = ""; }
                if (!v.Known.Contains(p)) v.Known.Add(p);
            }
            w.TileAt(new Int2(10, 0)).Road = true;
            // Preexisting fence geometry selects a detour that both public expansions keep dry.
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(11, 0), To = new Int2(12, 0) }); v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(12, 1), To = new Int2(13, 1) });
            var other = new Village { Id = w.Id("trade-village"), Name = "Fence trade partner", OwnerId = Context(v).PlayerId, Center = new Int2(20, 0), Radius = 1, LastLeaderResearch = 0 }; w.Villages.Add(other);
            var den = new Building { Id = w.Id("trade-den"), Kind = "den", Position = new Int2(20, -2), Completed = true }; other.Buildings.Add(den); other.Buildings.Add(new Building { Id = w.Id("trade-shrine"), Kind = "shrine", Position = other.Center, Completed = true });
            var resident = new Cat { Id = w.Id("trade-resident"), VillageId = other.Id, BedId = den.Id, Position = other.Center, X = 20, Z = 0, AgeHours = 24, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000 }; other.Cats.Add(resident); other.LeaderId = resident.Id;
            var receiver = new Stockpile { Id = w.Id("trade-store"), Position = new Int2(20, 2), Capacity = 1000, Goods = new List<Stack> { new Stack("stone", 5) } }; other.Stockpiles.Add(receiver); v.Contacts.Add(other.Id); other.Contacts.Add(v.Id);
            void Complete(GameAction action)
            {
                var id = Act(w, v, action).EntityId; var job = v.Jobs.Single(j => j.Id == id); for (int tick = 0; tick < 1000 && !job.Completed; tick++) w.Step(1); Check(job.Completed, "Public caravan fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(11, 0), End = new Int2(15, 0), CatId = c.Id });
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(11, 1), End = new Int2(12, 1), CatId = c.Id });
            var fieldId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(15, -2), CatId = c.Id }).EntityId; var field = v.Buildings.Single(b => b.Id == fieldId); for (int tick = 0; tick < 1000 && !field.Completed; tick++) w.Step(1); Check(field.Completed, "Public caravan Field did not complete");
            var farmId = Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 1), End = new Int2(10, 3) }).EntityId;
            var expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId; var expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 && expansion.PathIndex < expansion.Path.Count; tick++) w.Step(1);
            var from = new Int2(11, 0); var to = new Int2(11, 1); Check(!expansion.Completed && expansion.PathIndex == expansion.Path.Count && w.Crossable(from, to), "Caravan was not accepted immediately before public farm-fence completion");
            var tradeId = Act(w, v, new GameAction { Kind = "OfferVillageTrade", OtherVillageId = other.Id, Resource = "logs", Amount = 8, OtherResource = "stone", OtherAmount = 5 }).EntityId; Act(w, other, new GameAction { Kind = "AcceptVillageTrade", TargetId = tradeId }); var trade = w.TradeOffers.Single(t => t.Id == tradeId);
            int crossing = trade.Path.FindIndex(p => p.Equals(from)); Check(crossing >= 0 && crossing + 1 < trade.Path.Count && trade.Path[crossing + 1].Equals(to), "Public accepted caravan did not cache the future farm-fence edge");
            w.Step(1); Check(expansion.Completed && v.Radius == 11 && w.Walkable(from) && w.Walkable(to) && !w.Crossable(from, to), "Public expansion did not create the walkable-tile fence");
            for (int tick = 0; tick < 100 && !(trade.X == from.X && trade.Z == from.Z); tick++) w.Step(1);
            Check(trade.X == from.X && trade.Z == from.Z && trade.PathIndex == crossing + 1, "Caravan did not reach the new fence physically"); double progress = trade.Progress; int index = trade.PathIndex; w.Step(3);
            Check(trade.Status == "outbound" && trade.PathIndex == index && trade.X == from.X && trade.Z == from.Z && trade.Progress == progress, "Cached caravan path crossed the completed farm boundary");
            Near(Goods(w, v, "logs") + Goods(w, other, "logs"), 0, "Blocked caravan released its offered escrow"); Near(Goods(w, v, "stone") + Goods(w, other, "stone"), 0, "Blocked caravan released its payment escrow"); Near(trade.Offered.Amount, 8, "Blocked trade changed offered cargo"); Near(trade.Requested.Amount, 5, "Blocked trade changed payment cargo"); Check(!trade.OfferedDelivered, "Blocked caravan credited an outward delivery");
            Act(w, v, new GameAction { Kind = "ClearFarm", TargetId = farmId }); expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId; expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 && !expansion.Completed; tick++) w.Step(1); Check(expansion.Completed && v.Radius == 13 && w.Crossable(from, to), "Public expansion did not reopen the obsolete farm fence");
            for (int tick = 0; tick < 500 && trade.Status != "completed"; tick++) w.Step(1); Check(trade.Status == "completed" && trade.OfferedDelivered, "Reopened caravan did not resume its original exchange");
            w.Step(10); Near(Goods(w, other, "logs"), 8, "Resumed caravan did not deliver offered goods exactly once"); Near(Goods(w, v, "stone"), 5, "Resumed caravan did not deliver payment exactly once"); Near(Goods(w, v, "logs"), 0, "Resumed caravan returned its spent offer"); Near(Goods(w, other, "stone"), 0, "Resumed caravan duplicated payment"); Check(w.TradeOffers.Count(t => t.Id == tradeId) == 1, "Caravan recovery replaced its exchange identity"); Valid(w);
        }
        static void TransportNeed(bool shipping, bool blockReturn = false)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; string mode = shipping ? "shipping" : "rail";
            var path = Enumerable.Range(shipping ? 1 : -6, 6).Select(x => new Int2(x, shipping ? 5 : 1)).ToList();
            foreach (var p in path) { var tile = w.TileAt(p); tile.Wall = tile.Mountain = tile.Road = tile.Dirt = tile.Bridge = false; tile.Water = shipping && !p.Equals(path[0]) && !p.Equals(path[path.Count - 1]); tile.Dock = shipping && !tile.Water; tile.Rail = !shipping; }
            if (shipping) for (int x = 2; x <= 5; x++) foreach (int z in new[] { 4, 6 }) { var tile = w.TileAt(new Int2(x, z)); tile.Wall = tile.Mountain = tile.Bridge = false; tile.Water = true; }
            var source = new Stockpile { Id = w.Id("need-source"), Position = path[0], Width = 1, Depth = 1, Goods = new List<Stack> { new Stack("logs", 8) } }; var destination = new Stockpile { Id = w.Id("need-destination"), Position = path[path.Count - 1], Width = 1, Depth = 1 }; v.Stockpiles.Add(source); v.Stockpiles.Add(destination);
            var vehicle = new Vehicle { Id = w.Id("need-vehicle"), Mode = mode, Position = source.Position }; v.Vehicles.Add(vehicle);
            var id = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = mode, Resource = "logs", Amount = 8, Path = path }).EntityId; var route = v.Routes.Single(r => r.Id == id);
            for (int tick = 0; tick < 100 && !(route.Phase == "outbound" && route.PathIndex == 2); tick++) w.Step(1);
            Check(route.Phase == "outbound" && route.PathIndex == 2 && c.Position.Equals(vehicle.Position) && World.Amount(vehicle.Cargo, "logs") == 8, "Needs fixture did not load and physically depart");
            if (shipping) c.Rest = 19; else c.Thirst = 34;
            double water = Goods(w, v, "water"), age = c.AgeHours, health = c.Health; var parked = vehicle.Position; int parkedIndex = route.PathIndex; bool departed = false, satisfied = false, reboarded = false; var edges = new List<BoundaryEdge>();
            for (int tick = 0; tick < (shipping ? 900 : 150) && !satisfied; tick++)
            {
                var before = vehicle.Position; double x = c.X, z = c.Z, rest = c.Rest, hunger = c.Hunger, thirst = c.Thirst, offshoreAge = c.AgeHours, offshoreHealth = c.Health; bool aboard = c.Position.Equals(before) && c.X == before.X && c.Z == before.Z;
                w.Step(1);
                Check(Math.Sqrt((c.X - x) * (c.X - x) + (c.Z - z) * (c.Z - z)) <= 2.80001, "Need interruption teleported the transport driver");
                if (!aboard) Check(vehicle.Position.Equals(before), "Vehicle moved before its driver physically reboarded");
                if (shipping && !w.Walkable(v, before))
                {
                    Check(c.Position.Equals(vehicle.Position) && c.Rest <= rest, "Offshore sleep left the vessel or restored rest without reaching its bed");
                    Check(c.Hunger < hunger && c.Thirst < thirst && c.AgeHours > offshoreAge && c.Health <= offshoreHealth, "Offshore need deferral stopped aging or restored needs without physical service");
                    Check(!vehicle.Position.Equals(before), "Sleep need stranded the loaded vessel offshore");
                }
                if (!c.Position.Equals(vehicle.Position))
                {
                    departed = true;
                    if (blockReturn && edges.Count == 0)
                        foreach (var adjacent in new[] { new Int2(parked.X - 1, parked.Z), new Int2(parked.X + 1, parked.Z), new Int2(parked.X, parked.Z - 1), new Int2(parked.X, parked.Z + 1) }) { var edge = new BoundaryEdge { From = parked, To = adjacent }; edges.Add(edge); v.BoundaryEdges.Add(edge); }
                }
                Check(v.Routes.Single().Id == id && vehicle.RouteId == id && c.BuildingId == id && route.CatId == c.Id && route.VehicleId == vehicle.Id, "Need interruption lost transport identities or ownership");
                Near(World.Amount(vehicle.Cargo, "logs"), 8, "Need interruption released loaded cargo before reboarding"); Near(World.Amount(destination.Goods, "logs"), 0, "Needs delivered cargo without its driver"); Near(Goods(w, v, "logs"), 8, "Need interruption lost or duplicated cargo");
                satisfied = shipping ? c.Rest >= 95 : c.Thirst >= 80;
            }
            Check(departed && satisfied && c.AgeHours > age && c.Health <= health, "Transport need never reached and used its physical supplies or bed");
            if (!shipping)
            {
                Near(Goods(w, v, "water"), water - 1, "Rail drink did not consume exactly one finite serving");
                if (blockReturn)
                {
                    w.Step(5); Check(vehicle.Position.Equals(parked) && route.PathIndex == parkedIndex && !c.Position.Equals(parked) && c.BlockedReason == "blocked_route", "New boundary did not block the driver's physical return to its wagon");
                    foreach (var edge in edges) v.BoundaryEdges.Remove(edge);
                }
            }
            for (int tick = 0; tick < 100 && v.Routes.Any(r => r.Id == id); tick++)
            {
                var before = vehicle.Position; bool aboard = c.Position.Equals(before) && c.X == before.X && c.Z == before.Z; double x = c.X, z = c.Z; w.Step(1);
                Check(Math.Sqrt((c.X - x) * (c.X - x) + (c.Z - z) * (c.Z - z)) <= 2.80001, "Reboarding teleported the transport driver");
                if (!aboard) Check(vehicle.Position.Equals(before), "Transport resumed before its driver's return completed");
                if (departed && c.Position.Equals(vehicle.Position) && c.X == vehicle.Position.X && c.Z == vehicle.Position.Z) reboarded = true;
            }
            Check(reboarded && !v.Routes.Any(r => r.Id == id) && vehicle.RouteId == "" && c.BuildingId == "" && vehicle.Position.Equals(source.Position) && vehicle.Cargo.Count == 0, "Satisfied driver did not reboard, finish and release the same transport"); Near(World.Amount(destination.Goods, "logs"), 8, "Resumed needs route did not deliver exactly once"); Near(Goods(w, v, "logs"), 8, "Resumed needs route changed cargo totals"); Valid(w);
        }
        static void ShippingCancel(bool fullSource, bool driverDeath = false)
        {
            var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Stockpiles[0].Position = new Int2(1, 0); if (driverDeath) World.Add(v.Stockpiles[0].Goods, "lumber", 4);
            var source = new Stockpile { Id = w.Id("port"), Position = new Int2(1, 2), Capacity = 8, Goods = new List<Stack> { new Stack("logs", 8) } };
            var destination = new Stockpile { Id = w.Id("port"), Position = new Int2(6, 2), Capacity = 8 }; v.Stockpiles.Add(source); v.Stockpiles.Add(destination);
            var path = Enumerable.Range(1, 6).Select(x => new Int2(x, 2)).ToList();
            foreach (var p in path) { var tile = w.TileAt(p); tile.Wall = tile.Mountain = false; tile.Water = p.X > 1 && p.X < 6; tile.Dock = !tile.Water; }
            var vehicle = new Vehicle { Id = w.Id("vessel"), Mode = "shipping", Position = source.Position }; v.Vehicles.Add(vehicle);
            var helper = v.Cats[1]; helper.ControlledBy = ""; helper.BuildingId = driverDeath ? "" : "fixture-held"; helper.Position = new Int2(1, 1); helper.X = 1; helper.Z = 1; if (fullSource) helper.Cargo.Add(new Stack("stone", 8));
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "shipping", Resource = "logs", Amount = 8, Path = path, Repeat = true });
            w.Step(3); Check(w.TileAt(vehicle.Position).Water && World.Amount(vehicle.Cargo, "logs") == 8, "Shipping fixture never loaded and departed over water");
            if (fullSource) { Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id }); Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = source.Id }); }
            var position = vehicle.Position; var route = v.Routes.Single();
            if (driverDeath) { c.Health = 0; w.Step(1); Check(!c.Alive, "Driver death did not pass through simulation lifecycle"); }
            else Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(vehicle.Position.Equals(position) && World.Amount(vehicle.Cargo, "logs") == 8, "Cancelling at sea spilled or teleported loaded cargo");
            if (driverDeath)
            {
                w.Step(20); Check(vehicle.Position.Equals(position) && World.Amount(vehicle.Cargo, "logs") == 8, "Crewless vessel must retain cargo at its actual position until physical recovery");
                Act(w, v, new GameAction { Kind = "BuildBridge", CatId = helper.Id, Position = position }); w.Step(300); Check(w.TileAt(position).Bridge, "Physical bridge did not reach the stranded vessel"); Near(Goods(w, v, "lumber"), 0, "Bridge recovery did not consume its finite lumber cost");
                if (v.Routes.Any(r => r.Id == route.Id)) Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id }); var salvage = v.Stockpiles.SingleOrDefault(p => p.Kind == "spill" && p.Position.Equals(position) && World.Amount(p.Goods, "logs") == 8); Check(salvage != null && w.Path(helper.Position, salvage.Position, v) != null, "Bridge-accessible vessel cargo did not become recoverable salvage");
                Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = salvage.Id }); w.Step(120); Near(v.Stockpiles.Where(p => p.Kind == "storage").Sum(p => World.Amount(p.Goods, "logs")), 8, "Salvaged vessel cargo failed physical delivery to storage"); Near(Goods(w, v, "logs"), 8, "Driver death and salvage lost or duplicated cargo"); Check(vehicle.Cargo.Count == 0 && vehicle.RouteId == "" && !v.Routes.Any(r => r.Id == route.Id), "Salvaged wreck retained route ownership"); Valid(w); return;
            }
            w.Step(20); Check(vehicle.Position.Equals(source.Position), "Cancelled vessel did not physically return to its source port");
            Check(!v.Stockpiles.Any(p => p.Kind == "spill" && w.TileAt(p.Position).Water && p.Goods.Count > 0), "Cancelled cargo stranded in a water spill"); Near(Goods(w, v, "logs"), 8, "Shipping cancellation lost or duplicated cargo");
            if (fullSource)
            {
                Near(World.Amount(vehicle.Cargo, "logs"), 8, "Full source discarded returning vessel cargo"); Check(v.Routes.Any(r => r.Id == route.Id), "Full source released route ownership before safe unloading");
                Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = source.Id, Resource = "stone", Amount = 8 }); Act(w, v, new GameAction { Kind = "LeaveCat", CatId = helper.Id }); w.Step(20);
            }
            Near(World.Amount(source.Goods, "logs"), 8, "Cancelled cargo did not return to source storage"); Check(vehicle.Cargo.Count == 0 && vehicle.RouteId == "" && c.BuildingId == "" && !v.Routes.Any(r => r.Id == route.Id), "Docked cancellation left cargo or route ownership behind"); Valid(w);
        }
        static void LegacyEffect(string name)
        {
            double Measure(bool owned)
            {
                var w = Fixture(out var v, out var c); c.BuildingId = ""; v.Blessings = 10000; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); World.Add(v.Stockpiles[0].Goods, "materials", 100); c.Hunger = 80;
                if (owned) for (int i = 0; i < 5; i++) Act(w, v, new GameAction { Kind = "PurchaseUpgrade", Name = name });
                if (name == "resilience") { w.Step(600); return c.Hunger; }
                if (name == "buildMastery")
                { var build = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }); int ticks = 0; for (; ticks < 1000 && !v.Buildings.Single(b => b.Id == build.EntityId).Completed; ticks++) w.Step(1); Check(ticks < 1000, "Legacy build effect never completed construction"); return -ticks; }
                if (name == "ritualMastery") Act(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id }); else Act(w, v, new GameAction { Kind = "RequestJob", Name = name == "supplySpeed" ? "water" : "hunt", CatId = c.Id });
                var job = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
                if (name == "clickPower") { double before = job.RequiredWork; Act(w, v, new GameAction { Kind = "Boost", TargetId = job.Id }); return before - job.RequiredWork; }
                int elapsed = 0; for (; elapsed < 1000 && !job.Completed; elapsed++) w.Step(1); Check(elapsed < 1000, "Legacy effect job never completed " + name); return -elapsed;
            }
            double baseline = Measure(false), upgraded = Measure(true); Check(upgraded > baseline, "Purchased legacy upgrade has no gameplay effect " + name + " baseline=" + baseline + " upgraded=" + upgraded);
        }
        static void FarmFoodChain()
        {
            var w = Fixture(out var v, out var c); Exterior(w, v); v.Research = Catalog.Research.Select(n => n.Id).ToList(); World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); var mill = Station(w, v, "mill"); c.BuildingId = ""; double foodBefore = Goods(w, v, "food");
            FixtureAccessRoad(w, v, new Int2(12, 11));
            Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(12, 12), CatId = c.Id }); w.Step(600); var farm = Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(14, 12), End = new Int2(15, 13) }); Act(w, v, new GameAction { Kind = "AssignWorker", TargetId = farm.EntityId, CatId = c.Id }); w.Step(7800);
            var output = v.Stockpiles.SingleOrDefault(p => p.ManagedBy == farm.EntityId && p.Kind == "farm_output"); Check(output != null && World.Amount(output.Goods, "grain") > 0, "Farm did not carry grain to its exterior handoff"); Near(w.Total(v, "grain"), 0, "Harvest teleported into general storage without hauling"); double grain = Goods(w, v, "grain");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id }); Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = output.Id, CatId = c.Id }); w.Step(200); Check(w.Total(v, "grain") > 0, "Manual handoff haul never reached storehouse"); Check(Goods(w, v, "grain") <= grain && Goods(w, v, "grain") >= grain * 0.98, "Harvest/haul duplicated or discarded grain");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = c.Id }); foreach (string recipe in new[] { "grain_to_flour", "flour_to_food" }) Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = mill.Id, RecipeId = recipe, Edit = "add" }); bool completed = false; for (int i = 0; i < 1500 && !completed; i++) { w.Step(1); completed = v.Jobs.Any(j => j.RecipeId == "flour_to_food" && j.Completed); }
            Check(completed, "Farm grain did not complete flour-to-food production"); Check(Goods(w, v, "food") > foodBefore * 0.90, "Food chain consumed unexplained survival supply"); Valid(w);
        }
        static void ResearchGraph()
        {
            var w = Fixture(out var v, out var c); double initial = v.ResearchPoints, spent = 0; var pending = Catalog.Research.ToList(); Check(pending.Count == 487, "Maintained research count changed");
            while (pending.Count > 0) { var ready = pending.Where(n => n.Prerequisites.All(v.Research.Contains)).ToArray(); Check(ready.Length > 0, "Unreachable/cyclic research: " + string.Join(",", pending.Select(n => n.Id))); foreach (var n in ready) { Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = n.Id }); spent += n.Cost; pending.Remove(n); Check(!w.Apply(Context(v), new GameAction { Kind = "ResearchNode", NodeId = n.Id }).Success, "Duplicate research charged again"); } }
            Near(v.ResearchPoints, initial - spent, "Catalog exact research cost"); Check(Catalog.Recipes.All(r => Catalog.RecipeAvailable(v, r)), "Recipe lacks reachable unlock"); Check(Catalog.Buildings.All(b => Catalog.BuildingAvailable(v, b)), "Building lacks reachable unlock");
        }
        static void RecipeChain(string id)
        {
            var w = Fixture(out var v, out var c); v.Research = Catalog.Research.Select(n => n.Id).ToList(); var r = Catalog.Recipe(id); var station = Station(w, v, r.Building); var input = new Dictionary<string, double>(); foreach (var s in r.Inputs) { double amount = w.RecipeInput(v, r, s.Amount); World.Add(v.Stockpiles[0].Goods, s.Resource, amount); input[s.Resource] = Goods(w, v, s.Resource); }
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = id }); bool carried = false, worked = false, completed = false;
            for (int tick = 0; tick < 3600; tick++) { w.Step(1); carried |= c.Cargo.Count > 0; worked |= v.Jobs.Any(j => j.Kind == "production" && j.Progress > 0 && c.Position.Equals(station.Position)); completed = v.Jobs.Any(j => j.Kind == "production" && j.Completed); if (completed) break; }
            Check(completed, "Recipe never completed: " + id + " blocked=" + c.BlockedReason + " / " + station.BlockedReason); Check(carried && worked, "Recipe skipped physical input carry/work: " + id); Check(w.Reservations.Count == 0, "Recipe left orphan input claim: " + id);
            foreach (var s in r.Inputs.Where(s => !r.Outputs.Any(o => o.Resource == s.Resource))) { double remaining = input[s.Resource] - w.RecipeInput(v, r, s.Amount); double actual = Goods(w, v, s.Resource); Check(actual <= remaining + 0.0001 && actual >= remaining * 0.98 - 0.0001, "Wrong conserved input after recipe/spoilage " + id + " / " + s.Resource + " expected approximately=" + remaining + " actual=" + actual); }
            foreach (var s in r.Outputs) { double expected = w.RecipeOutput(v, r, s.Amount); Check(w.Total(v, s.Resource) >= expected * 0.98, "Output not physically stored: " + id + " / " + s.Resource); }
            if (r.ItemKind != "") { var items = v.Items.Where(i => i.Kind == r.ItemKind).ToArray(); Check(items.Length == 1 && items[0].Material == r.Material && items[0].Quality == r.Quality, "Wrong exact item identity/material/quality " + id); Check(v.Stockpiles.Any(p => p.Id == items[0].LocationId), "Item never reached pile " + id); }
            Valid(w);
        }
        static string Fingerprint(World w)
        {
            var result = new System.Text.StringBuilder();
            void Append(object value)
            {
                if (value == null) { result.Append("null;"); return; }
                if (value is string text) { result.Append(text.Length).Append(':').Append(text).Append(';'); return; }
                if (value is double number) { result.Append(Math.Round(number, 9).ToString("R", System.Globalization.CultureInfo.InvariantCulture)).Append(';'); return; }
                if (value.GetType().IsPrimitive) { result.Append(Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture)).Append(';'); return; }
                if (value is System.Collections.IEnumerable sequence) { result.Append('['); foreach (var entry in sequence) Append(entry); result.Append(']'); return; }
                result.Append('{'); foreach (var field in value.GetType().GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public).OrderBy(f => f.Name, StringComparer.Ordinal)) { result.Append(field.Name).Append('='); Append(field.GetValue(value)); }
                result.Append('}');
            }
            Append(w); return result.ToString();
        }
        static World CampaignWorld(int seed, bool established, bool personal)
        {
            var w = World.Create(seed); if (personal) Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Personal acceptance camp" }); if (established) { var v = w.Villages[0]; v.ResearchPoints = 1000; World.Add(v.Stockpiles[0].Goods, "planks", 100); World.Add(v.Stockpiles[0].Goods, "blocks", 100); World.Add(v.Stockpiles[0].Goods, "logs", 100); var b = Station(w, v, "wood_cutter", new Int2(-4, 1)); Act(w, v, new GameAction { Kind = "AssignWorker", CatId = v.Cats[20].Id, BuildingId = b.Id }); var recipe = Catalog.Recipes.First(r => r.Building == b.Kind && r.Founding); Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe.Id, Repeat = true }); }
            return w;
        }
        static void Campaign(int seed, int hours, bool established, bool personal = false)
        {
            var a = CampaignWorld(seed, established, personal); var b = CampaignWorld(seed, established, personal);
            var founding = a.Villages.ToDictionary(v => v.Id, v => v.Cats.Select(c => c.Id).ToArray());
            for (int hour = 0; hour < hours; hour++)
            {
                a.Step(3600); for (int minute = 0; minute < 60; minute++) b.Step(60); Valid(a); Valid(b);
                Check(Fingerprint(a) == Fingerprint(b), "Partition/twin divergence seed=" + seed + " hour=" + (hour + 1));
                foreach (var v in a.Villages)
                {
                    if (v.Run != 1) throw new InvalidOperationException("Accidental extinction reset village=" + v.Id + " seed=" + seed + " hour=" + (hour + 1) + CampaignState(v));
                    if (!founding[v.Id].All(id => v.Cats.Any(c => c.Id == id && c.Alive))) throw new InvalidOperationException("Founding adult lost village=" + v.Id + " seed=" + seed + " hour=" + (hour + 1) + CampaignState(v));
                }
            }
        }
        static string CampaignState(Village v)
        {
            string StackText(IEnumerable<Stack> goods) => string.Join(",", goods.Select(s => s.Resource + "=" + s.Amount.ToString("F2", System.Globalization.CultureInfo.InvariantCulture)));
            return " piles=[" + string.Join(";", v.Stockpiles.Select(p => p.Id + "@" + p.Position.X + "," + p.Position.Z + ":" + StackText(p.Goods))) + "] cats=[" + string.Join(";", v.Cats.Where(c => !c.Alive || c.Health < 50 || c.Thirst < 15).Select(c => c.Id + " alive=" + c.Alive + " hp=" + c.Health + " thirst=" + c.Thirst + " hunger=" + c.Hunger + " pos=" + c.Position.X + "," + c.Position.Z + " goal=" + c.Goal + " blocked=" + c.BlockedReason + " path=" + c.Path.Count + " job=" + c.JobId + " cargo=" + StackText(c.Cargo))) + "] events=[" + string.Join(";", v.Events.Where(e => e.Kind == "death" || e.Kind == "recovery").Select(e => e.Time + ":" + e.Text)) + "]";
        }
    }
}
