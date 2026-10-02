using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Construction: founding layout, roads, doors, enclosures, expansion and placement.</summary>
    public static class ConstructionScenarios
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
            foreach (string obstruction in new[] { "field", "entrance", "gather" })
            {
                string kind = obstruction;
                yield return new Scenario("regression.review_expansion_preserves_" + kind, () => ExpansionFootprint(kind, false));
            }
            yield return new Scenario("regression.review_expansion_rechecks_field", () => ExpansionFootprint("field", true));
            yield return new Scenario("regression.review_expansion_resumes_after_gather_removal", () => ExpansionFootprint("gather", true));
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
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.review_expansion_respects_linear_" + (track ? "rail" : "road"), () => PlacementScenarios.PendingExpansionLinear(track, false));
            }
            yield return new Scenario("regression.layout_shrine_ring_reserved", ShrineRingReserved);
            yield return new Scenario("regression.layout_new_building_uses_door", BuildingDoor);
            yield return new Scenario("regression.layout_isolated_road_cannot_extend", IsolatedRoad);
            yield return new Scenario("regression.layout_paid_road_opens_building", PaidBuildingRoad);
            yield return new Scenario("regression.layout_legacy_geometry_and_roads_preserved", LegacyLayout);
            yield return new Scenario("regression.layout_avoid_cannot_close_shrine_access", AvoidShrineAccess);
            yield return new Scenario("regression.buildable_founding_services", BuildableServices);
            yield return new Scenario("regression.communal_and_personal_enclosures", Enclosures);
            yield return new Scenario("regression.expansion_interruption_preserves_perimeter", ExpansionInterruption);
            yield return new Scenario("construction.demo_plan_queues_without_builder", DemoPlanQueuesWithoutBuilder);
            yield return new Scenario("construction.demo_plan_queues_without_materials", DemoPlanQueuesWithoutMaterials);
            yield return new Scenario("construction.classic_plan_names_missing_material", ClassicPlanNamesMaterial);
            yield return new Scenario("construction.demo_cost_counts_player_built_only", DemoCostCountsPlayerBuilt);
            foreach (bool demo in new[] { false, true })
            {
                bool profile = demo;
                yield return new Scenario("construction.cancel_refunds_and_releases_" + (profile ? "demo" : "classic"), () => CancelConstruction(profile));
            }
            yield return new Scenario("construction.demolish_refunds_half_and_guards", Demolish);
            yield return new Scenario("construction.demo_orphaned_scaffold_gets_a_new_job", DemoOrphanedScaffold);
            yield return new Scenario("construction.demo_two_builders_share_a_site", DemoTwoBuilders);
            yield return new Scenario("construction.demo_walls_and_roads_carry_full_loads", DemoFullLoads);
            yield return new Scenario("construction.demo_build_rate_ignores_station_cycle_time", DemoBuildRate);
            yield return new Scenario("construction.completion_logs_milestones_only", CompletionNotes);
        }

        /// <summary>Calls a private World member by name (rules the scenarios exercise before they are wired in).</summary>
        internal static object Invoke(World w, string name, params object[] args)
        {
            var method = typeof(World).GetMethod(name, System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            Check(method != null, "World has no member " + name);
            try
            {
                return method.Invoke(w, args);
            }
            catch (System.Reflection.TargetInvocationException e) when (e.InnerException != null)
            {
                throw e.InnerException;
            }
        }

        /// <summary>
        /// Applies CancelConstruction or Demolish through the router. Until W2-INT-a adds the two router cases
        /// (ActionRouter belongs to W2-06) the router answers "Unknown action"; then the handler is called
        /// directly. Remove the fallback once the cases are routed.
        /// </summary>
        internal static ActionResult Construct(World w, Village v, GameAction a)
        {
            var r = w.Apply(Context(v), a);
            if (r.Success || !r.Error.StartsWith("Unknown action", StringComparison.Ordinal))
                return r;
            return (ActionResult)Invoke(w, a.Kind == "CancelConstruction" ? "HandleCancelConstruction" : "HandleDemolish", v, a);
        }

        /// <summary>The safety net W2-INT-a calls from TickVillage on planning ticks; scenarios call it directly.</summary>
        static void SafetyNet(World w, Village v) => Invoke(w, "EnsureScaffoldJobs", v);

        static World DemoFixture(out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            w.PacingProfile = "demo";
            c.BuildingId = "";
            return w;
        }

        static void DemoPlanQueuesWithoutBuilder()
        {
            var w = DemoFixture(out var v, out var c);
            c.ControlledBy = "fixture-held";
            c.ControlLeaseUntil = 10000000;
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1) });
            Check(result.Success && result.Code == ResultCode.Queued && result.Args["reason"] == Blocked.WorkerRequired, "A plan without a free builder was not queued: " + result.Code + " " + result.Error);
            var site = v.Buildings.Single(b => b.Id == result.EntityId);
            var job = v.Jobs.Single(j => j.TargetId == site.Id);
            Check(!site.Completed && job.CatId == "" && !job.Completed && job.BlockedReason == Blocked.WorkerRequired, "Queued plan has no visible unowned build job");
            Check(w.Reservations.Where(r => r.OwnerId == site.Id).Sum(r => r.Amount) == 6, "Queued plan did not hold its bill");
            c.ControlledBy = "";
            for (int tick = 0; tick < 600 && !site.Completed; tick++)
                w.Step(1);
            Check(site.Completed && job.Completed && job.CatId == c.Id, "A freed builder did not pick up the queued plan");
            Near(Goods(w, v, "planks"), 0, "Queued plan changed its timber bill");
            Valid(w);
        }

        static void DemoPlanQueuesWithoutMaterials()
        {
            var w = DemoFixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "planks", 3);
            var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1) });
            Check(result.Success && result.Code == ResultCode.Queued && result.Args["reason"] == Blocked.MissingScaffoldInput, "A plan without materials was not queued: " + result.Code + " " + result.Error);
            var site = v.Buildings.Single(b => b.Id == result.EntityId);
            Check(site.BlockedReason == Blocked.MissingScaffoldInput && !v.Jobs.Any(j => j.TargetId == site.Id && !j.Completed) && c.JobId == "", "A site waiting for materials took a builder");
            Check(w.Reservations.Where(r => r.OwnerId == site.Id && r.Resource == "planks").Sum(r => r.Amount) == 3, "A waiting site did not claim the materials that exist");
            w.Step(5);
            SafetyNet(w, v);
            Check(!v.Jobs.Any(j => j.TargetId == site.Id && !j.Completed), "A site short of materials started work");
            World.Add(v.Stockpiles[0].Goods, "planks", 1);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            SafetyNet(w, v);
            var job = v.Jobs.Single(j => j.TargetId == site.Id && !j.Completed);
            Check(job.CatId == c.Id && site.BlockedReason == "", "Arriving materials did not start the waiting site with the free builder");
            for (int tick = 0; tick < 600 && !site.Completed; tick++)
                w.Step(1);
            Check(site.Completed, "The waiting site did not complete once materials arrived");
            Near(Goods(w, v, "planks"), 0, "Waiting site changed its timber bill");
            Near(Goods(w, v, "blocks"), 0, "Waiting site changed its block bill");
            Valid(w);
        }

        static void ClassicPlanNamesMaterial()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 1);
            int buildings = v.Buildings.Count;
            var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
            Check(!result.Success && result.Code == ResultCode.MissingMaterial && result.Args["resource"] == "planks" && result.Args["need"] == "4" && result.Args["have"] == "1", "Classic plan did not name the missing material: " + result.Code + " " + result.Error);
            Check(v.Buildings.Count == buildings && w.Reservations.Count == 0, "A refused classic plan left a scaffold or claims");
            World.Add(v.Stockpiles[0].Goods, "planks", 3);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            c.ControlledBy = "fixture-held";
            var busy = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1) });
            Check(!busy.Success && busy.Code == PlacementCode.BuilderBusy, "Classic plan without a builder was not refused as today");
        }

        static void DemoCostCountsPlayerBuilt()
        {
            foreach (bool demo in new[] { false, true })
            {
                var w = World.Create(41);
                if (demo)
                    w.PacingProfile = "demo";
                var v = w.Villages[0];
                Check(v.Buildings.Count(b => b.Kind == "den") == 6, "Fixture must start with six founding dens");
                var first = w.BuildCost(v, "den");
                Check(World.Amount(first.Inputs, "planks") == (demo ? 4 : 16) && World.Amount(first.Inputs, "blocks") == (demo ? 2 : 8), "Den price counted the wrong buildings: " + string.Join(",", first.Inputs.Select(s => s.Resource + ":" + s.Amount)));
                if (!demo)
                    continue;
                var check = w.CheckPlacement(v, "den", new Int2(-1, -6));
                Check(World.Amount(check.Cost, "planks") == 4, "The placement check showed a different price than BuildCost");
                var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "den", Position = check.Ok ? new Int2(-1, -6) : FirstLegalDen(w, v) });
                Check(result.Success, "Demo den plan failed: " + result.Code);
                var second = w.BuildCost(v, "den");
                Check(World.Amount(second.Inputs, "planks") == 6 && World.Amount(second.Inputs, "blocks") == 3, "The second player den did not cost more than the first");
                foreach (var pile in v.Stockpiles)
                    pile.Goods.RemoveAll(s => s.Resource == "planks" || s.Resource == "lumber");
                var shortage = w.Shortfalls(v, w.BuildCost(v, "den").Inputs).Single();
                Check(shortage.Resource == "planks" && shortage.Need == 6 && shortage.Have == 0, "Missing material was not named with need and have");
            }
        }

        static Int2 FirstLegalDen(World w, Village v)
        {
            for (int z = -9; z <= 9; z++)
                for (int x = -9; x <= 9; x++)
                    if (w.CheckPlacement(v, "den", new Int2(x, z)).Ok)
                        return new Int2(x, z);
            throw new InvalidOperationException("No legal den site in the founding layout");
        }

        static void CancelConstruction(bool demo)
        {
            var w = Fixture(out var v, out var c);
            if (demo)
                w.PacingProfile = "demo";
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 10);
            World.Add(v.Stockpiles[0].Goods, "blocks", 10);
            double planks = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
            var at = new Int2(-5, 1);
            var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id });
            var site = v.Buildings.Single(b => b.Id == planned.EntityId);
            var job = v.Jobs.Single(j => j.TargetId == site.Id);
            for (int tick = 0; tick < 400 && (site.Inputs.Count == 0 || c.Cargo.Count == 0); tick++)
                w.Step(0.25);
            Check(site.Inputs.Count > 0 || c.Cargo.Count > 0, "Fixture must cancel while materials are staged or carried");
            var result = Construct(w, v, new GameAction { Kind = "CancelConstruction", TargetId = site.Id });
            Check(result.Success && result.Code == ResultCode.Cancelled, "Cancel construction failed: " + result.Code + " " + result.Error);
            Check(!v.Buildings.Contains(site) && job.Completed && c.JobId == "" && c.Cargo.Count == 0 && w.Reservations.Count == 0, "Cancel left the scaffold, the job, the builder or claims");
            Near(Goods(w, v, "planks"), planks, "Cancel lost or created planks");
            Near(Goods(w, v, "blocks"), blocks, "Cancel lost or created blocks");
            Near(World.Amount(v.Stockpiles[0].Goods, "planks"), planks, "Cancel did not return the planks to storage");
            Check(w.CheckPlacement(v, "wood_cutter", at).Ok, "The cancelled site is still blocked");
            var again = Construct(w, v, new GameAction { Kind = "CancelConstruction", TargetId = site.Id });
            Check(!again.Success && again.Code == ResultCode.UnknownTarget, "A second cancel did not report the unknown site");
            var den = v.Buildings.First(b => b.Kind == "den");
            Check(Construct(w, v, new GameAction { Kind = "CancelConstruction", TargetId = den.Id }).Code == PlacementCode.NotUnderConstruction, "A finished building could be cancelled");
            Valid(w);
        }

        static void Demolish()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            Check(Construct(w, v, new GameAction { Kind = "Demolish", TargetId = shrine.Id }).Code == PlacementCode.ProtectedBuilding, "The shrine could be demolished");
            var homeDen = v.Buildings.First(b => b.Kind == "den" && v.Cats.Any(x => x.BedId == b.Id));
            Check(Construct(w, v, new GameAction { Kind = "Demolish", TargetId = homeDen.Id }).Code == PlacementCode.OccupiedDen, "An occupied den could be demolished");
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-5, 1);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "food_storage", Position = at, CatId = c.Id }).EntityId;
            var store = v.Buildings.Single(b => b.Id == id);
            Check(Construct(w, v, new GameAction { Kind = "Demolish", TargetId = store.Id }).Code == PlacementCode.NotComplete, "A scaffold could be demolished instead of cancelled");
            for (int tick = 0; tick < 600 && !store.Completed; tick++)
                w.Step(1);
            Check(store.Completed, "Fixture food store did not complete");
            var container = v.Stockpiles.Single(s => s.ManagedBy == store.Id);
            World.Add(container.Goods, "food", 5);
            double food = Goods(w, v, "food");
            var result = Construct(w, v, new GameAction { Kind = "Demolish", TargetId = store.Id });
            Check(result.Success && result.Code == ResultCode.Demolished && result.Args["planks"] == "2" && result.Args["blocks"] == "1", "Demolish did not return half the bill: " + result.Code + " " + result.Error);
            Check(!v.Buildings.Contains(store) && container.Kind == "spill" && container.ManagedBy == "", "Demolish left the building or lost its stored goods");
            Near(Goods(w, v, "food"), food, "Demolish destroyed stored food");
            Near(Goods(w, v, "planks"), 2, "Demolish refund is not half the planks");
            Near(Goods(w, v, "blocks"), 1, "Demolish refund is not half the blocks");
            Check(w.CheckPlacement(v, "food_storage", at).Code != PlacementCode.Occupied || v.Stockpiles.Any(s => s.Kind == "spill" && s.Position.Equals(at)), "Demolished footprint is still occupied by the building");
            Valid(w);
        }

        static void DemoOrphanedScaffold()
        {
            var w = DemoFixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var site = v.Buildings.Single(b => b.Id == id);
            w.Step(2);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            c.BuildingId = "";
            Check(!site.Completed && c.JobId == "", "Fixture must leave the scaffold without its builder");
            SafetyNet(w, v);
            var active = v.Jobs.Where(j => j.TargetId == site.Id && !j.Completed).ToList();
            Check(active.Count == 1, "The safety net did not give the orphaned scaffold exactly one build job: " + active.Count);
            SafetyNet(w, v);
            Check(v.Jobs.Count(j => j.TargetId == site.Id && !j.Completed) == 1, "The safety net duplicated a build job");
            for (int tick = 0; tick < 600 && !site.Completed; tick++)
            {
                w.Step(1);
                SafetyNet(w, v);
            }
            Check(site.Completed, "The orphaned scaffold never completed");
            Near(Goods(w, v, "planks"), 0, "Orphan recovery changed the timber bill");
            Near(Goods(w, v, "blocks"), 0, "Orphan recovery changed the block bill");
            Check(w.Reservations.Count == 0, "Recovered scaffold kept claims");
            Valid(w);
        }

        static double BuildTime(bool helper, out int peakBuilders)
        {
            var w = DemoFixture(out var v, out var c);
            var second = v.Cats[1];
            second.ControlledBy = "";
            second.BuildingId = helper ? "" : "fixture-held";
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var site = v.Buildings.Single(b => b.Id == id);
            peakBuilders = 0;
            for (int tick = 0; tick < 1200 && !site.Completed; tick++)
            {
                w.Step(0.5);
                if ((tick & 1) == 1)
                    SafetyNet(w, v);
                peakBuilders = Math.Max(peakBuilders, v.Jobs.Count(j => j.TargetId == site.Id && !j.Completed && j.CatId != ""));
            }
            Check(site.Completed, "Fixture site did not complete");
            Near(Goods(w, v, "planks"), 0, "Shared site changed its timber bill");
            Near(Goods(w, v, "blocks"), 0, "Shared site changed its block bill");
            Check(v.Jobs.All(j => j.TargetId != site.Id || j.Completed) && w.Reservations.Count == 0 && v.Cats.All(x => x.Cargo.Count == 0), "Shared site left work, claims or cargo behind");
            Valid(w);
            return w.TimeSeconds;
        }

        static void DemoTwoBuilders()
        {
            double alone = BuildTime(false, out int single), shared = BuildTime(true, out int pair);
            Check(single == 1 && pair == 2, "Builders per site: alone=" + single + " shared=" + pair);
            Check(shared < alone, "A second builder did not speed up the site: alone=" + alone + " shared=" + shared);
        }

        static void DemoFullLoads()
        {
            foreach (bool demo in new[] { false, true })
            {
                var w = Fixture(out var v, out var c);
                if (demo)
                    w.PacingProfile = "demo";
                c.BuildingId = "";
                PrepareExpansion(w, v);
                World.Add(v.Stockpiles[0].Goods, "materials", 1000);
                string expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId;
                var expansion = v.Jobs.Single(j => j.Id == expansionId);
                double firstLoad = 0;
                for (int tick = 0; tick < 4000 && firstLoad == 0; tick++)
                {
                    w.Step(0.25);
                    if (expansion.Phase == "working")
                        firstLoad = World.Amount(c.Cargo, "materials");
                }
                Check(firstLoad == (demo ? 8 : 1), (demo ? "Demo" : "Classic") + " wall builder carried " + firstLoad + " per trip");
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                    w.Step(1);
                Check(expansion.Completed && expansion.Path.All(p => w.TileAt(p).Wall), "Expansion with full loads did not complete every segment");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Full loads changed the wall bill");
                Valid(w);
            }
            foreach (bool demo in new[] { false, true })
            {
                var w = Fixture(out var v, out var c);
                if (demo)
                    w.PacingProfile = "demo";
                c.BuildingId = "";
                foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                    tile.Road = false;
                World.Add(v.Stockpiles[0].Goods, "materials", 4);
                string roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(-3, -2), End = new Int2(-6, -2), CatId = c.Id }).EntityId;
                var road = v.Jobs.Single(j => j.Id == roadId);
                double firstLoad = 0;
                for (int tick = 0; tick < 4000 && firstLoad == 0; tick++)
                {
                    w.Step(0.25);
                    if (road.Phase == "working")
                        firstLoad = World.Amount(c.Cargo, "materials");
                }
                Check(firstLoad == (demo ? 4 : 1), (demo ? "Demo" : "Classic") + " road builder carried " + firstLoad + " per trip");
                for (int tick = 0; tick < 600 && !road.Completed; tick++)
                    w.Step(1);
                Check(road.Completed && road.Path.All(p => w.TileAt(p).Road), "Road with full loads did not pave every tile");
                Near(Goods(w, v, "materials"), 0, "Full loads changed the road bill");
                Valid(w);
            }
        }

        static void DemoBuildRate()
        {
            foreach (bool demo in new[] { false, true })
            {
                double Rate(bool owned)
                {
                    var w = StudyFixture("workshop_timing", owned, out var v, out var c);
                    if (demo)
                        w.PacingProfile = "demo";
                    return (double)Invoke(w, "WorkRate", v, c, "build", "workshop");
                }
                double without = Rate(false), with = Rate(true);
                if (demo)
                    Near(with, without, "A workshop production upgrade changed how fast workshops are built");
                else
                    Check(with > without, "Classic build rate no longer follows the station cycle time (today's rule)");
            }
        }

        static void CompletionNotes()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            for (int tick = 0; tick < 600 && !v.Buildings.Single(b => b.Id == id).Completed; tick++)
                w.Step(1);
            Check(v.Events.Count(e => e.Kind == EventKind.BuildingCompleted && e.EntityId == id) == 1, "Building completion was not logged once as a milestone");
            int before = v.Events.Count;
            Invoke(w, "NoteJobCompleted", v, c, new Job { Id = "road-job", Kind = JobKind.Road });
            Invoke(w, "NoteJobCompleted", v, c, new Job { Id = "offering-job", Kind = JobKind.Offering });
            Invoke(w, "NoteJobCompleted", v, c, new Job { Id = "haul-job", Kind = JobKind.Haul });
            Invoke(w, "NoteJobCompleted", v, c, new Job { Id = "hunt-job", Kind = JobKind.Hunt });
            Invoke(w, "NoteJobCompleted", v, c, new Job { Id = "build-job", Kind = JobKind.Build });
            var logged = v.Events.Skip(before).ToList();
            Check(logged.Count == 2 && logged.All(e => e.Kind == EventKind.Completed) && logged[0].EntityId == "road-job" && logged[1].EntityId == "offering-job", "Routine jobs were logged or milestone jobs were not: " + string.Join(",", logged.Select(e => e.EntityId)));
            Invoke(w, "NoteScoutReport", v, c, 12);
            var report = v.Events.Last();
            Check(report.Kind == EventKind.Discovery && report.EntityId == c.Id && report.Text == c.Name + " mapped 12 tiles", "Scout report does not say how many tiles were mapped: " + report.Text);
        }
        static HashSet<Int2> LayoutRoads(World w, Village v)
        {
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            bool Inside(Building b, Int2 p) => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth;
            var seen = new HashSet<Int2>();
            var pending = new Queue<Int2>();
            for (int x = 0; x < shrine.Width; x++)
                for (int z = 0; z < shrine.Depth; z++)
                    pending.Enqueue(new Int2(shrine.Position.X + x, shrine.Position.Z + z));
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                {
                    var next = new Int2(at.X + offset.X, at.Z + offset.Z);
                    var tile = w.GetTile(next);
                    if (tile != null && tile.Road && w.Walkable(v, next) && !v.Buildings.Any(b => Inside(b, next)) && !seen.Contains(next))
                    {
                        seen.Add(next);
                        pending.Enqueue(next);
                    }
                }
            }
            return seen;
        }
        static void FoundingLayout(bool personal, bool roads)
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            if (personal)
                v = w.Village(Act(w, v, new GameAction { Kind = "FoundVillage", Name = "Layout home" }).EntityId);
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            if (!roads)
            {
                Check(shrine.Width == 3 && shrine.Depth == 3 && shrine.Position.Equals(new Int2(v.Center.X - 1, v.Center.Z - 1)), "Shrine must occupy exactly the centered3×3tiles");
                for (int z = -2; z <= 2; z++)
                    for (int x = -2; x <= 2; x++)
                    {
                        var p = new Int2(v.Center.X + x, v.Center.Z + z);
                        var t = w.TileAt(p);
                        Check(t.Road == (Math.Max(Math.Abs(x), Math.Abs(z)) == 2), "Shrine road ring is incomplete or crosses the shrine at " + p);
                        if (t.Road)
                            Check(!v.Buildings.Any(b => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth) && !v.Stockpiles.Any(s => p.X >= s.Position.X && p.X < s.Position.X + s.Width && p.Z >= s.Position.Z && p.Z < s.Position.Z + s.Depth), "Shrine road overlaps a physical footprint");
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
                var w = World.Create(seed);
                Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Exit home" });
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
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var at = new Int2(-6, -5);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= -2))
                tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            int buildings = v.Buildings.Count, claims = w.Reservations.Count;
            double timber = Goods(w, v, "planks");
            var denied = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id });
            Check(!denied.Success, "Disconnected paving allowed a new building without a shrine-connected entrance");
            Check(v.Buildings.Count == buildings && w.Reservations.Count == claims && Goods(w, v, "planks") == timber, "Rejected layout changed property or claims");
        }
        static void ExpansionFootprint(string kind, bool pending)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", kind == "entrance" ? 1002 : 1001);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int radius = v.Radius, outer = radius + 2;
            var at = new Int2(outer + (kind == "entrance" ? 1 : 0), 1);
            var entry = kind == "entrance" ? new Int2(outer, 1) : new Int2(outer, 0);
            for (int x = outer; x <= outer + 2; x++)
                for (int z = 0; z <= 2; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            for (int x = radius; x < outer; x++)
                w.TileAt(new Int2(x, 0)).Road = true;
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer, 0), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId);
            for (int tick = 0; tick < 600 && !road.Completed; tick++)
                w.Step(1);
            Check(road.Completed && w.TileAt(entry).Road, "Public road did not connect the exterior work site");
            Near(Goods(w, v, "materials"), 1000, "Exterior access road did not consume its exact bill");
            Job expansion = null;
            Cat builder = c;
            if (pending)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                for (int tick = 0; tick < 1800 && expansion.PathIndex == 0; tick++)
                    w.Step(1);
                Check(expansion.PathIndex > 0, "Expansion must have paid perimeter work before the competing building");
                builder = v.Cats[1];
                builder.ControlledBy = "";
            }
            string obstacleId;
            if (pending)
            {
                // Explicit loaded-state fault: older saves could already contain work on the reserved perimeter.
                if (kind == "gather")
                {
                    var pile = new Stockpile { Id = w.Id("loaded-gather"), Kind = "gather", Position = at, Width = 1, Depth = 1 };
                    v.Stockpiles.Add(pile);
                    obstacleId = pile.Id;
                }
                else
                {
                    var field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                    v.Buildings.Add(field);
                    obstacleId = field.Id;
                }
            }
            else if (kind == "gather")
                obstacleId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = at, End = at }).EntityId;
            else
            {
                obstacleId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id }).EntityId;
                var field = v.Buildings.Single(b => b.Id == obstacleId);
                Check(field.Entrance.Equals(entry), "Field did not use the intended exterior entrance");
                if (!pending)
                {
                    for (int tick = 0; tick < 600 && !field.Completed; tick++)
                        w.Step(1);
                    Check(field.Completed && w.Path(v.Center, field.Position, v) != null, "Public exterior Field did not complete with reachable work position");
                }
            }
            int claims = w.Reservations.Count, jobs = v.Jobs.Count;
            double materials = Goods(w, v, "materials");
            string terrain = TerrainState(w.TileAt(at)), entryTerrain = TerrainState(w.TileAt(entry));
            if (pending)
            {
                int paid = expansion.PathIndex;
                double progress = expansion.Progress;
                string held = string.Join("|", w.Reservations.Where(r => r.OwnerId.StartsWith(expansion.Id, StringComparison.Ordinal)).Select(r => r.OwnerId + ":" + r.Amount));
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
            Near(Goods(w, v, "materials"), materials, "Conflicting expansion consumed materials");
            Valid(w);
            if (pending && kind == "gather")
            {
                Act(w, v, new GameAction { Kind = "RemoveGatherSpot", TargetId = obstacleId });
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                    w.Step(1);
                Check(expansion.Completed && v.Radius == outer && expansion.BlockedReason == "", "Removing the obstruction did not resume the same expansion");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) })
                    Check(!w.TileAt(gate).Wall, "Resumed expansion closed a required cardinal gate");
                Check(expansion.Path.All(p => w.TileAt(p).Wall), "Resumed expansion lost a paid perimeter segment");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Resumed expansion consumed more than its finite perimeter bill");
                Check(w.Reservations.Count == 0, "Completed expansion retained claims");
                Valid(w);
            }
        }
        static void ExpansionRoadConnection(string order)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1003);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2;
            var at = new Int2(outer + 1, 2);
            var entry = new Int2(outer + 1, 1);
            var crossing = new Int2(outer, 1);
            for (int x = outer - 1; x <= outer + 2; x++)
                for (int z = 0; z <= 3; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(outer - 1, 0)).Road = true;
            if (order == "alternate_route")
            {
                w.TileAt(new Int2(outer, 0)).Road = true;
                w.TileAt(new Int2(outer + 1, 0)).Road = true;
            }
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer - 1, 1), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId);
            for (int tick = 0; tick < 600 && !road.Completed; tick++)
                w.Step(1);
            Check(road.Completed && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Public road did not connect the distant Field entrance");
            Near(Goods(w, v, "materials"), 1000, "Access road changed its finite material bill");
            bool expansionFirst = order == "expansion_first" || order == "loaded_conflict";
            Job expansion = null;
            Building field = null;
            Cat builder = c;
            if (expansionFirst)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                Check(expansion.Path.Contains(crossing) && !expansion.Path.Contains(entry), "Fixture must reserve an upstream road, not the doorway");
                builder = v.Cats[1];
                builder.ControlledBy = "";
            }
            if (order == "loaded_conflict")
            {
                // Existing save conflict: a completed Field whose road crosses already planned wall work.
                field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                v.Buildings.Add(field);
                double materials = Goods(w, v, "materials"), progress = expansion.Progress;
                int claims = w.Reservations.Count;
                w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field's upstream road did not pause the existing expansion before material movement");
                Near(Goods(w, v, "materials"), materials, "Blocked upstream road consumed retained expansion materials");
                Check(!w.TileAt(crossing).Wall, "Blocked expansion walled the Field's upstream road");
                Valid(w);
                return;
            }
            int reservations = w.Reservations.Count, jobs = v.Jobs.Count, buildings = v.Buildings.Count;
            var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
            if (order == "expansion_first")
            {
                Check(!placement.Success, "Field placement accepted a road that pending expansion will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected disconnected Field changed structures, jobs or claims");
                Near(Goods(w, v, "planks"), 4, "Rejected Field consumed timber");
                Near(Goods(w, v, "blocks"), 2, "Rejected Field consumed blocks");
                Valid(w);
                return;
            }
            Check(placement.Success, "Initial exterior Field was not connected: " + placement.Error);
            field = v.Buildings.Single(b => b.Id == placement.EntityId);
            for (int tick = 0; tick < 600 && !field.Completed; tick++)
                w.Step(1);
            Check(field.Completed, "Public exterior Field construction did not complete");
            reservations = w.Reservations.Count;
            jobs = v.Jobs.Count;
            var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted an upstream wall that disconnects an existing Field");
                Check(v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected expansion created jobs or claims");
                Near(Goods(w, v, "materials"), 1000, "Rejected expansion consumed materials");
            }
            else
            {
                Check(expanded.Success, "Alternate road through the required gate did not allow expansion");
                expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                    w.Step(1);
                Check(expansion.Completed && w.TileAt(crossing).Wall && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Completed expansion lost the Field's alternate shrine-connected road");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) })
                    Check(!w.TileAt(gate).Wall, "Alternate access created a missing cardinal gate");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Expansion with alternate access changed its wall bill");
            }
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Existing Field lost its shrine-connected doorway");
            Valid(w);
        }
        static void ExpansionFarmEdge(string order)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1020);
            World.Add(v.Stockpiles[0].Goods, "planks", 10);
            World.Add(v.Stockpiles[0].Goods, "blocks", 5);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            for (int x = 10; x <= 15; x++)
                for (int z = 0; z <= 6; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(10, 0)).Road = true;
            void Road(Int2 from, Int2 to)
            {
                var id = Act(w, v, new GameAction { Kind = "BuildRoad", Position = from, End = to, CatId = c.Id }).EntityId;
                var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < 1000 && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Farm-edge fixture road did not complete");
            }
            Building Field(Int2 position)
            {
                var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = position, CatId = c.Id }).EntityId;
                var building = v.Buildings.Single(b => b.Id == id);
                for (int tick = 0; tick < 1000 && !building.Completed; tick++)
                    w.Step(1);
                Check(building.Completed, "Farm-edge fixture Field did not complete");
                return building;
            }
            Road(new Int2(11, 0), new Int2(13, 0));
            var existing = Field(new Int2(13, 1));
            Road(new Int2(11, 1), new Int2(11, 3));
            Road(new Int2(12, 3), new Int2(12, 3));
            if (order == "alternate_route")
            {
                Road(new Int2(12, 1), new Int2(12, 1));
                Road(new Int2(12, 2), new Int2(12, 2));
            }
            var at = new Int2(12, 4);
            var entry = new Int2(12, 3);
            var edgeFrom = new Int2(11, 0);
            var edgeTo = new Int2(11, 1);
            Building target = null;
            if (order == "field_first" || order == "alternate_route")
                target = Field(at);
            Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 1), End = new Int2(10, 3) });
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Crossable(edgeFrom, edgeTo), "Farm-edge fixture must start with a connected road across the future fence");
            int claims = w.Reservations.Count, jobs = v.Jobs.Count, radius = v.Radius;
            double materials = Goods(w, v, "materials");
            var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted a future farm fence that disconnects an existing Field");
                Check(v.Jobs.Count == jobs && w.Reservations.Count == claims && v.Radius == radius && v.BoundaryEdges.Count == 0, "Rejected farm-edge expansion changed work, claims or fences");
                Near(Goods(w, v, "materials"), materials, "Rejected farm-edge expansion consumed materials");
                Valid(w);
                return;
            }
            Check(expanded.Success, "Clear farm-edge expansion was rejected: " + expanded.Error);
            var expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
            Check(!expansion.Path.Contains(edgeFrom) && !expansion.Path.Contains(edgeTo) && !expansion.Path.Contains(entry), "Farm-edge fixture accidentally relies on a future wall tile");
            if (order == "expansion_first")
            {
                var builder = v.Cats[1];
                builder.ControlledBy = "";
                claims = w.Reservations.Count;
                jobs = v.Jobs.Count;
                int buildings = v.Buildings.Count;
                double timber = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
                var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
                Check(!placement.Success, "New Field accepted a road that the pending farm fence will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected Field changed structures, jobs or claims");
                Near(Goods(w, v, "planks"), timber, "Rejected Field consumed timber");
                Near(Goods(w, v, "blocks"), blocks, "Rejected Field consumed blocks");
                Valid(w);
                return;
            }
            if (order == "loaded_conflict")
            {
                // Saved-state conflict from a prior version: the later Field already exists.
                target = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                v.Buildings.Add(target);
                claims = w.Reservations.Count;
                double progress = expansion.Progress;
                w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field did not pause expansion before its future farm fence cut the road");
                Near(Goods(w, v, "materials"), materials, "Blocked farm-edge expansion consumed materials");
                Check(w.Crossable(edgeFrom, edgeTo), "Blocked expansion changed the farm fence");
                Valid(w);
                return;
            }
            for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                w.Step(1);
            Check(expansion.Completed && v.Radius == radius + 2 && !w.Crossable(edgeFrom, edgeTo) && w.TileAt(edgeFrom).Road && w.TileAt(edgeTo).Road && !w.TileAt(edgeTo).Wall, "Expansion did not create the expected edge fence while retaining its road tiles");
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Path(v.Center, target.Position, v) != null && w.BuildingEntrance(v, new Building { Position = existing.Position }).Equals(existing.Entrance), "Alternate road did not preserve both Fields across the completed farm fence");
            Near(Goods(w, v, "materials"), materials - expansion.Path.Count, "Farm-edge expansion changed its exact wall bill");
            Check(w.Reservations.Count == 0, "Completed farm-edge expansion retained claims");
            Valid(w);
        }
        static void ShrineRingReserved()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-3, -3), CatId = c.Id }).Success, "Construction occupied the shrine's required road ring");
        }
        static void BuildingDoor()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var building = v.Buildings.Single(b => b.Id == id);
            Check(building.HasEntrance, "New construction did not preserve its selected doorway");
            var path = w.Path(v.Center, building.Position, v);
            Check(path != null && path.Contains(building.Entrance), "Physical worker path bypassed the selected doorway");
            var closed = new Int2(building.Position.X - 1, building.Position.Z);
            Check(!closed.Equals(building.Entrance) && !w.Crossable(closed, building.Position), "Building exterior allowed a wall crossing away from its door");
        }
        static void IsolatedRoad()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "materials", 100);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            Check(!LayoutRoads(w, v).Contains(new Int2(-6, -3)), "Test paving is connected instead of isolated");
            Check(!w.Apply(Context(v), new GameAction { Kind = "BuildRoad", Position = new Int2(-6, -2), End = new Int2(-5, -2), CatId = c.Id }).Success, "Isolated paving bypassed shrine-road connectivity");
        }
        static void PaidBuildingRoad()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                tile.Road = false;
            World.Add(v.Stockpiles[0].Goods, "materials", 4);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-6, -4);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).Success, "Unpaved site was accepted before its access road existed");
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(-3, -2), End = new Int2(-6, -2), CatId = c.Id }).EntityId;
            Check(!w.TileAt(new Int2(-6, -2)).Road, "Road appeared before physical work");
            for (int tick = 0; tick < 600 && !v.Jobs.Single(j => j.Id == roadId).Completed; tick++)
                w.Step(1);
            Check(v.Jobs.Single(j => j.Id == roadId).Completed, "Paid access road did not finish");
            Near(Goods(w, v, "materials"), 0, "Road did not consume its exact four materials");
            var buildingId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).EntityId;
            w.Step(600);
            var building = v.Buildings.Single(b => b.Id == buildingId);
            Check(building.Completed && LayoutRoads(w, v).Contains(building.Entrance), "Road did not enable a connected completed building");
            Near(Goods(w, v, "planks"), 0, "Building timber bill changed");
            Near(Goods(w, v, "blocks"), 0, "Building block bill changed");
            Valid(w);
        }
        static void LegacyLayout()
        {
            var w = new World { Seed = 41 };
            var v = new Village { Id = "legacy-layout", Communal = true, Center = new Int2(0, 0), Radius = 12 };
            w.Villages.Add(v);
            var shrine = new Building { Id = "saved-shrine", Kind = "shrine", Position = new Int2(3, 3), Width = 3, Depth = 3, Completed = true };
            var old = new Building { Id = "saved-workshop", Kind = "wood_cutter", Position = new Int2(8, 4), Width = 2, Depth = 2, Completed = true };
            v.Buildings.AddRange(new[] { shrine, old });
            for (int z = 0; z <= 12; z++)
                for (int x = 0; x <= 12; x++)
                {
                    var at = new Int2(x, z);
                    v.Known.Add(at);
                    v.ClaimedTiles.Add(at);
                    w.Tiles.Add(new Tile { Position = at, ClaimId = v.Id, Biome = "meadow" });
                }
            foreach (var at in new[] { new Int2(6, 4), new Int2(7, 4), new Int2(6, 3), new Int2(7, 3), new Int2(8, 3) })
                w.TileAt(at).Overlay = "road_built";
            var cat = new Cat { Id = "saved-cat", VillageId = v.Id, Position = new Int2(6, 3), X = 6, Z = 3 };
            v.Cats.Add(cat);
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
            var w = World.Create(7);
            var v = w.Villages[0];
            foreach (var at in new[] { v.Center, new Int2(v.Center.X, v.Center.Z + 2), v.Buildings.First(b => b.HasEntrance).Entrance })
                Check(!w.Apply(Context(v), new GameAction { Kind = "CreateZone", Resource = "avoid", Position = at, End = at }).Success, "Avoid zone severed shrine or building access");
        }
        static void BuildableServices()
        {
            foreach (var kind in new[] { "den", "workshop", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "mouse_farm" })
            {
                var w = Fixture(out var v, out var c);
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                c.BuildingId = "";
                Act(w, v, new GameAction { Kind = "PlanBuilding", Name = kind, Position = new Int2(-5, 1), CatId = c.Id });
                w.Step(500);
                Check(v.Buildings.Any(b => b.Kind == kind && b.Position.Equals(new Int2(-5, 1)) && b.Completed), kind + " cannot complete physical construction");
                Valid(w);
            }
        }
        static void Enclosures()
        {
            var w = World.Create(7);
            Check(w.Villages[0].Radius == 9, "Communal enclosure must fit its30cat/6Den blueprint at radius9");
            var made = Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Personal camp" });
            var v = w.Village(made.EntityId);
            Check(v.Radius == 6 && v.Cats.Count(c => c.Alive) == 15, "Personal founding footprint/population changed");
        }
        static void ExpansionInterruption()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            int radius = v.Radius, newRadius = radius + 2;
            foreach (var t in w.Tiles.Where(t => Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius))
            {
                t.Wall = t.Water = t.Mountain = false;
                t.Resource = "";
                t.Amount = 0;
            }
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            for (int i = 0; i < 1200 && !w.Tiles.Any(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)); i++)
                w.Step(1);
            var built = w.Tiles.Where(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)).Select(t => t.Position).ToArray();
            Check(built.Length > 0, "Expansion never built its first real segment");
            Check(v.Radius == radius, "Expansion cut over before perimeter completion");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            w.Step(12000);
            Check(v.Radius == newRadius, "Interrupted expansion cannot resume to complete its wall");
            Check(built.All(p => w.TileAt(p).Wall), "Expansion interruption lost completed segments");
            Check(!w.TileAt(new Int2(0, newRadius)).Wall, "Expansion closed the new south gate");
            Valid(w);
        }
    }
}
