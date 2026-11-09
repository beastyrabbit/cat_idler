using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class EquipmentAccessScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.equipment_access_retrieval_blocked_anchor", () => BlockedAnchor(false));
            yield return new Scenario("regression.equipment_access_return_blocked_anchor", () => BlockedAnchor(true));
            yield return new Scenario("regression.equipment_access_retrieval_fractional_closure_recovers", () => FractionalClosure(false));
            yield return new Scenario("regression.equipment_access_return_fractional_closure_recovers", () => FractionalClosure(true));
            yield return new Scenario("regression.equipment_access_retrieval_closed_refusal_is_atomic", () => ClosedRefusal(false));
            yield return new Scenario("regression.equipment_access_return_closed_refusal_is_atomic", () => ClosedRefusal(true));
            yield return new Scenario("regression.equipment_access_retrieval_closed_retry_is_bounded", () => ClosedRetry(false));
            yield return new Scenario("regression.equipment_access_return_closed_retry_is_bounded", () => ClosedRetry(true));
            yield return new Scenario("regression.equipment_access_retrieval_cached_intermediate_edge_closure", () => CachedEdgeClosure(false));
            yield return new Scenario("regression.equipment_access_return_cached_intermediate_edge_closure", () => CachedEdgeClosure(true));
            yield return new Scenario("regression.equipment_access_retrieval_needs_resume_exact_item", () => Needs(false, false));
            yield return new Scenario("regression.equipment_access_return_needs_resume_exact_item", () => Needs(true, false));
            yield return new Scenario("regression.equipment_access_retrieval_cancel_during_needs", () => Needs(false, true));
            yield return new Scenario("regression.equipment_access_return_cancel_during_needs", () => Needs(true, true));
            yield return new Scenario("regression.equipment_access_source_loss_spills_at_original_anchor", MissingSource);
            yield return new Scenario("regression.equipment_access_return_rechecks_capacity_and_filter", ChangedStorage);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, string kind, Cat cat = null, string target = "", Int2? position = null) => world.Apply(
            new PlayerContext { PlayerId = "equipment-access-player", VillageId = village.Id },
            new GameAction { Kind = kind, CatId = cat?.Id ?? "", TargetId = target, Resource = "avoid", Position = position ?? default, End = position ?? default });
        static Int2 Alternate(Stockpile pile) => new Int2(pile.Position.X + 1, pile.Position.Z, pile.Position.Level);
        static bool Contains(Int2 anchor, int width, int depth, Int2 at) => at.Level == anchor.Level && at.X >= anchor.X && at.X < anchor.X + width && at.Z >= anchor.Z && at.Z < anchor.Z + depth;
        static string Avoid(World world, Village village, Int2 position)
        {
            var result = Act(world, village, "CreateZone", position: position);
            Check(result.Success, "Avoid fixture refused: " + result.Error);
            return result.EntityId;
        }
        static World Fixture(bool returning, out Village village, out Cat cat, out Stockpile pile, out Item item)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.LastLeaderResearch = 0;
            village.Buildings.RemoveAll(building => building.Kind != "den" && building.Kind != "shrine");
            foreach (var storage in village.Stockpiles.Where(storage => storage.Kind == "storage"))
                storage.Accepts = new List<string> { "food", "water" };
            foreach (var other in village.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
                other.Hunger = other.Thirst = other.Rest = 100;
            }
            cat = village.Cats[1];
            cat.ControlledBy = "";
            cat.Position = new Int2(1, 5); cat.X = 1; cat.Z = 5;
            cat.Skills.Clear();
            pile = new Stockpile { Id = world.Id("equipment-access-store"), Position = new Int2(3, 5), Width = 2, Depth = 1, Capacity = 2, Accepts = new List<string> { "weapons" } };
            foreach (var at in new[] { pile.Position, Alternate(pile) })
                Check(!village.Buildings.Any(building => Contains(building.Position, building.Width, building.Depth, at)) && !village.Stockpiles.Any(storage => Contains(storage.Position, storage.Width, storage.Depth, at)), "Equipment footprint must be disjoint from existing buildings and stockpiles");
            village.Stockpiles.Add(pile);
            foreach (var position in new[] { cat.Position, pile.Position, Alternate(pile) })
            {
                World.ClearSurface(world.TileAt(position), 0);
                world.TileAt(position).Wall = false;
                if (!village.Known.Contains(position)) village.Known.Add(position);
            }
            item = new Item { Id = world.Id("equipment-access-weapon"), Kind = "weapon", VillageId = village.Id, LocationId = returning ? cat.Id : pile.Id, Material = "metal", Quality = 3, Condition = 37.5, MaxCondition = 73 };
            village.Items.Add(item);
            if (returning) cat.Equipment.Add(item.Id);
            return world;
        }
        static Job Order(World world, Village village, Cat cat, Item item, bool returning)
        {
            var result = Act(world, village, returning ? "UnequipItem" : "EquipItem", cat, item.Id);
            Check(result.Success, (returning ? "Return" : "Retrieval") + " refused a reachable stockpile footprint: " + result.Error);
            return village.Jobs.Single(job => job.Id == result.EntityId);
        }
        static void Exact(Village village, Item item) => Check(village.Items.Count(candidate => candidate.Id == item.Id) == 1 && item.VillageId == village.Id && item.Kind == "weapon" && item.Material == "metal" && item.Quality == 3 && item.Condition == 37.5 && item.MaxCondition == 73, "Footprint equipment travel changed exact item identity, condition or ownership");
        static void Held(World world, Village village, Cat cat, Stockpile pile, Item item, Job job, bool returning)
        {
            Exact(village, item);
            Check(!job.Completed && job.Position.Equals(pile.Position) && cat.JobId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Blocked equipment travel lost its same anchored order");
            Check(returning ? item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") > 1 : item.LocationId == job.Id && !cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") == 1, "Equipment moved or changed benefit before physical arrival");
        }
        static void Finish(World world, Village village, Cat cat, Stockpile pile, Item item, Job job, bool returning)
        {
            for (int step = 0; step < 2400 && !job.Completed; step++)
            {
                world.Step(.05);
                if (!job.Completed) Held(world, village, cat, pile, item, job, returning);
            }
            var access = Alternate(pile);
            Check(job.Completed && cat.Position.Equals(access) && Math.Abs(cat.X - access.X) < 1e-9 && Math.Abs(cat.Z - access.Z) < 1e-9 && !world.Walkable(village, pile.Position), "Equipment did not finish at the open physical footprint tile while its anchor stayed closed");
            Check(returning ? item.LocationId == pile.Id && !cat.Equipment.Contains(item.Id) : item.LocationId == cat.Id && cat.Equipment.Contains(item.Id), "Equipment transfer used the wrong physical owner");
            Exact(village, item);
            Check(job.ItemIds.Count == 0 && cat.JobId == "" && cat.ResumeJobId == "" && job.BlockedReason == "" && cat.BlockedReason == "" && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Equipment completion retained ownership or warnings, or completed twice");
            world.Step(1);
            Check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1 && village.Items.Count(candidate => candidate.Id == item.Id) == 1, "Footprint equipment completion replayed");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
        static void BlockedAnchor(bool returning)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            Check(world.Path(cat.Position, pile.Position, village) == null && world.Path(cat.Position, Alternate(pile), village) != null, "Equipment fixture must close the anchor and retain alternate access");
            var job = Order(world, village, cat, item, returning);
            Check(job.Position.Equals(pile.Position) && job.ItemIds.SequenceEqual(new[] { item.Id }) && !job.Completed && cat.JobId == job.Id, "Equipment dispatch changed anchored order identity");
            Check(returning ? item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") > 1 : item.LocationId == job.Id && !cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") == 1, "Footprint dispatch transferred equipment before physical arrival");
            var repeat = Act(world, village, returning ? "UnequipItem" : "EquipItem", cat, item.Id);
            Check(repeat.Success && repeat.EntityId == job.Id && village.Jobs.Count(candidate => !candidate.Completed && candidate.ItemIds.Contains(item.Id)) == 1, "Repeating the same exact equipment order duplicated its claim");
            if (!returning)
            {
                var other = village.Cats[2]; other.ControlledBy = "";
                Check(!Act(world, village, "EquipItem", other, item.Id).Success && !Act(world, village, "HaulGatherSpot", other, pile.Id).Success && !Act(world, village, "RemoveStockpile", target: pile.Id).Success, "Another action stole or removed the exact footprint equipment claim");
                other.ControlledBy = "fixture-held";
                pile.Capacity = 1;
                Check(!world.HasRoom(village, pile, "weapons", 1), "Uncollected equipment no longer occupies its physical source");
            }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            Finish(world, village, cat, pile, item, job, returning);
        }
        static void FractionalClosure(bool returning)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            var job = Order(world, village, cat, item, returning);
            world.Step(.05);
            Check(Math.Abs(cat.X - cat.Position.X) + Math.Abs(cat.Z - cat.Position.Z) > 1e-9, "Equipment closure fixture did not enter a physical fractional edge");
            Avoid(world, village, pile.Position);
            world.Step(.05);
            Held(world, village, cat, pile, item, job, returning);
            string closedAlternate = Avoid(world, village, Alternate(pile));
            world.Step(.05);
            Held(world, village, cat, pile, item, job, returning);
            Check(job.BlockedReason == "blocked_route" && cat.BlockedReason == "blocked_route" && job.NextStorageAttemptAt > world.TimeSeconds, "Full equipment closure did not hold the same order and schedule a retry");
            double x = cat.X, z = cat.Z;
            world.Step(.1);
            Held(world, village, cat, pile, item, job, returning);
            Check(cat.X == x && cat.Z == z && job.BlockedReason == "blocked_route", "A closed footprint moved or cleared its real route warning before retry");
            Check(Act(world, village, "RemoveZone", target: closedAlternate).Success, "Equipment reopening refused");
            Finish(world, village, cat, pile, item, job, returning);
        }
        static void ClosedRefusal(bool returning)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            Avoid(world, village, Alternate(pile));
            long next = world.NextId;
            uint entropy = world.RandomState;
            double time = world.TimeSeconds;
            int jobs = village.Jobs.Count;
            var result = Act(world, village, returning ? "UnequipItem" : "EquipItem", cat, item.Id);
            Check(!result.Success && world.NextId == next && world.RandomState == entropy && world.TimeSeconds == time && village.Jobs.Count == jobs && cat.JobId == "" && cat.ResumeJobId == "" && cat.Cargo.Count == 0 && village.Items.Count == 1 && item.LocationId == (returning ? cat.Id : pile.Id) && cat.Equipment.Contains(item.Id) == returning, "Fully closed equipment refusal changed work, exact ownership, identity, entropy or time");
            Exact(village, item);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
        static void ClosedRetry(bool returning)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var job = Order(world, village, cat, item, returning);
            var at = Alternate(pile);
            foreach (var delta in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = at, To = new Int2(at.X + delta.X, at.Z + delta.Z) });
            world.Step(.05);
            Held(world, village, cat, pile, item, job, returning);
            Check(job.NextStorageAttemptAt > world.TimeSeconds && job.BlockedReason == "blocked_route" && world.Walkable(village, at), "Walkable but unreachable equipment footprint did not enter a bounded route hold");
            var counter = typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            long searches = (long)counter.GetValue(world);
            double next = job.NextStorageAttemptAt;
            world.Step(.2);
            Check((long)counter.GetValue(world) == searches && job.NextStorageAttemptAt == next && world.TimeSeconds < next, "Closed equipment footprint repeated path searches before its retry boundary");
            Held(world, village, cat, pile, item, job, returning);
            village.BoundaryEdges.Clear();
            Finish(world, village, cat, pile, item, job, returning);
        }
        static void Needs(bool returning, bool cancel)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var job = Order(world, village, cat, item, returning);
            cat.Thirst = 34;
            world.Step(.05);
            Check(cat.Goal == "need_drink" && cat.ResumeJobId == job.Id && !job.Completed && item.LocationId == (returning ? cat.Id : job.Id) && cat.Equipment.Contains(item.Id) == returning, "Needs lost exact footprint equipment ownership");
            if (cancel)
            {
                string goal = cat.Goal;
                var path = cat.Path.ToArray();
                var cargo = cat.Cargo.Select(stack => (stack.Resource, stack.Amount)).ToArray();
                Check(Act(world, village, "CancelEquipment", cat, job.Id).Success && job.Completed && cat.ResumeJobId == "" && cat.Goal == goal && cat.Path.SequenceEqual(path) && cat.Cargo.Select(stack => (stack.Resource, stack.Amount)).SequenceEqual(cargo), "Equipment cancellation changed need ownership, route or cargo");
                Check(item.LocationId == (returning ? cat.Id : pile.Id) && cat.Equipment.Contains(item.Id) == returning && job.Position.Equals(pile.Position), "Need cancellation moved exact equipment away from its original physical owner");
                Exact(village, item);
                Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
                return;
            }
            for (int step = 0; step < 2400 && cat.Goal.StartsWith("need_", StringComparison.Ordinal); step++) world.Step(.05);
            Check(cat.Thirst > 34 && cat.ResumeJobId == "" && !job.Completed && cat.JobId == job.Id, "Needs did not resume the same physical equipment order");
            Finish(world, village, cat, pile, item, job, returning);
        }
        static void CachedEdgeClosure(bool returning)
        {
            var world = Fixture(returning, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var job = Order(world, village, cat, item, returning);
            world.Step(.05);
            var origin = cat.Position;
            var endpoint = Alternate(pile);
            Check(cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(endpoint) && Math.Abs(cat.X - origin.X) + Math.Abs(cat.Z - origin.Z) > 1e-9 && world.Walkable(village, endpoint), "Cached equipment fixture did not begin a fractional route to its open endpoint");
            var closedEdges = new List<BoundaryEdge>();
            foreach (var delta in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                closedEdges.Add(new BoundaryEdge { From = origin, To = new Int2(origin.X + delta.X, origin.Z + delta.Z) });
            village.BoundaryEdges.AddRange(closedEdges);
            Check(!world.Crossable(origin, cat.Path[0]) && world.Walkable(village, endpoint), "Equipment closure must block its next cached edge while leaving the endpoint walkable");
            world.Step(.05);
            Held(world, village, cat, pile, item, job, returning);
            Check(cat.Path.Count == 0 && cat.Position.Equals(origin) && cat.X == origin.X && cat.Z == origin.Z && job.BlockedReason == "blocked_route" && cat.BlockedReason == "blocked_route", "Equipment bypassed a closed cached edge instead of physically retracing and rejecting its route");
            world.Step(.05);
            Held(world, village, cat, pile, item, job, returning);
            Check(job.NextStorageAttemptAt > world.TimeSeconds && job.BlockedReason == "blocked_route", "Unreachable cached equipment fallback did not retain its bounded route hold");
            var counter = typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            long searches = (long)counter.GetValue(world);
            double next = job.NextStorageAttemptAt;
            world.Step(.2);
            Check((long)counter.GetValue(world) == searches && job.NextStorageAttemptAt == next && world.TimeSeconds < next, "Cached equipment fallback repeated route searches within its retry interval");
            Held(world, village, cat, pile, item, job, returning);
            foreach (var edge in closedEdges) village.BoundaryEdges.Remove(edge);
            Finish(world, village, cat, pile, item, job, returning);
        }
        static void MissingSource()
        {
            var world = Fixture(false, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var job = Order(world, village, cat, item, false);
            world.Step(.05);
            Check(job.Position.Equals(pile.Position) && !cat.Position.Equals(pile.Position), "Equipment source-loss fixture replaced the anchor with an access endpoint");
            village.Stockpiles.Remove(pile);
            world.Step(.05);
            var spill = village.Stockpiles.SingleOrDefault(candidate => candidate.Id == item.LocationId);
            Check(job.Completed && spill != null && spill.Kind == "spill" && spill.Position.Equals(pile.Position) && !spill.Position.Equals(Alternate(pile)) && cat.Equipment.Count == 0 && cat.JobId == "" && cat.ResumeJobId == "", "Lost footprint source did not release its exact item at the original anchor");
            Exact(village, item);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
        static void ChangedStorage()
        {
            var world = Fixture(true, out var village, out var cat, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var job = Order(world, village, cat, item, true);
            world.Step(.05);
            pile.Capacity = 0;
            world.Step(.05);
            Held(world, village, cat, pile, item, job, true);
            Check(job.BlockedReason == "storage_full", "Equipment return ignored reduced capacity");
            pile.Capacity = 2;
            pile.Accepts = new List<string> { "food" };
            world.Step(.05);
            Held(world, village, cat, pile, item, job, true);
            Check(job.BlockedReason == "storage_full", "Equipment return ignored its changed resource filter");
            pile.Accepts = new List<string> { "weapons" };
            Finish(world, village, cat, pile, item, job, true);
        }
    }
}
