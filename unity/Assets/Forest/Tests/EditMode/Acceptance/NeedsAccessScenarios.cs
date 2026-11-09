using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class NeedsAccessScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (var resource in new[] { "water", "food", "medicine" })
            {
                string serving = resource;
                yield return new Scenario("regression.needs_access_" + serving + "_blocked_anchor_and_manual_resume", () => BlockedAnchor(serving));
            }
            yield return new Scenario("regression.needs_access_whole_closure_bounded_retry_and_reopen", ClosedRetry);
            yield return new Scenario("regression.needs_access_cached_footprint_closure_retargets", () => Retarget(false));
            yield return new Scenario("regression.needs_access_removed_cached_source_retargets", () => Retarget(true));
            yield return new Scenario("regression.needs_access_cached_intermediate_edge_retraces", CachedEdge);
            yield return new Scenario("regression.needs_access_reopened_anchor_keeps_cached_alternate", AnchorReopens);
            yield return new Scenario("regression.needs_access_competing_consumers_share_one_finite_serving", Competition);
            yield return new Scenario("regression.needs_access_reserved_source_before_selection", () => Reserved(false));
            yield return new Scenario("regression.needs_access_reserved_source_before_arrival", () => Reserved(true));
            yield return new Scenario("regression.needs_access_brew_then_fractional_water", Brew);
            yield return new Scenario("regression.needs_access_fish_preferred_over_preserves_and_food", Fish);
            yield return new Scenario("regression.needs_access_medicine_preferred_over_herbs", Medicine);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "needs-access-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }
        static Int2 Alternate(Stockpile pile) => new Int2(pile.Position.X + 1, pile.Position.Z, pile.Position.Level);
        static bool Contains(Int2 anchor, int width, int depth, Int2 at) => at.Level == anchor.Level && at.X >= anchor.X && at.X < anchor.X + width && at.Z >= anchor.Z && at.Z < anchor.Z + depth;
        static string Avoid(World world, Village village, Int2 at) => Act(world, village, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = at, End = at }).EntityId;
        static double Need(Cat cat, string resource) => resource == "water" ? cat.Thirst : resource == "medicine" ? cat.Health : cat.Hunger;
        static string Goal(string resource) => resource == "water" ? "need_drink" : resource == "medicine" ? "need_heal" : "need_eat";
        static void Valid(World world) => Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        static long Searches(World world) => (long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(world);
        static void Wait(World world, Func<bool> ready, string message)
        {
            for (int tick = 0; tick < 1200 && !ready(); tick++) world.Step(.05);
            Check(ready(), message);
        }
        static Stockpile Spare(World world, Village village, Int2 at, string resource, double amount)
        {
            Check(!village.Buildings.Any(building => Contains(building.Position, building.Width, building.Depth, at)) && !village.Stockpiles.Any(pile => Contains(pile.Position, pile.Width, pile.Depth, at)), "Needs spare footprint must be disjoint from homes, shrine and other stockpiles");
            World.ClearSurface(world.TileAt(at), 0);
            world.TileAt(at).Wall = false;
            if (!village.Known.Contains(at)) village.Known.Add(at);
            var spare = new Stockpile { Id = world.Id("needs-access-spare"), Position = at, Width = 1, Depth = 1, Capacity = 100, Goods = amount > 0 ? new List<Stack> { new Stack(resource, amount) } : new List<Stack>() };
            village.Stockpiles.Add(spare);
            return spare;
        }
        static void Recovered(World world, Village village, Cat cat, Item item, Job job, string resource)
        {
            Wait(world, () => Need(cat, resource) > 54 && cat.ResumeJobId == "", "Reopened or replacement source failed to physically satisfy the suspended need");
            Check(cat.Position.Equals(village.Center) && cat.X == village.Center.X && cat.Z == village.Center.Z && cat.Cargo.Count == 0 && !job.Completed && item.LocationId == job.Id && cat.JobId == job.Id, "Replacement need recovered away from the shrine or replaced its original finite work");
        }

        static World Fixture(string resource, out Village village, out Cat cat, out Stockpile pantry, out Item item, out Job job)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.LastLeaderResearch = 0;
            village.Buildings.RemoveAll(building => building.Kind != "den" && building.Kind != "shrine");
            foreach (var pile in village.Stockpiles) pile.Goods.Clear();
            // Keep the founding director's independent survival floor funded,
            // without offering another physically accessible needs supplier.
            var reserve = village.Stockpiles[0];
            reserve.Goods.AddRange(new[] { new Stack("food", 1000), new Stack("water", 1000) });
            for (int z = 0; z < reserve.Depth; z++)
                for (int x = 0; x < reserve.Width; x++)
                    Avoid(world, village, new Int2(reserve.Position.X + x, reserve.Position.Z + z));
            foreach (var other in village.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
                other.Hunger = other.Thirst = other.Rest = other.Health = 100;
            }
            pantry = new Stockpile { Id = world.Id("needs-access-pantry"), Position = new Int2(3, 5), Width = 2, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack(resource, 3) } };
            var equipment = new Stockpile { Id = world.Id("needs-access-equipment"), Position = new Int2(-3, 5), Width = 1, Depth = 1, Capacity = 10 };
            foreach (var created in new[] { pantry, equipment })
            {
                for (int x = 0; x < created.Width; x++)
                {
                    var at = new Int2(created.Position.X + x, created.Position.Z);
                    Check(!village.Buildings.Any(building => Contains(building.Position, building.Width, building.Depth, at)) && !village.Stockpiles.Any(pile => Contains(pile.Position, pile.Width, pile.Depth, at)), "Needs fixture footprint must be disjoint from homes, shrine and other stockpiles");
                    World.ClearSurface(world.TileAt(at), 0);
                    world.TileAt(at).Wall = false;
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
                village.Stockpiles.Add(created);
            }
            cat = village.Cats[1];
            cat.ControlledBy = "";
            cat.Position = new Int2(1, 5); cat.X = 1; cat.Z = 5;
            cat.Path.Clear();
            cat.Skills.Clear();
            World.ClearSurface(world.TileAt(cat.Position), 0);
            world.TileAt(cat.Position).Wall = false;
            if (!village.Known.Contains(cat.Position)) village.Known.Add(cat.Position);
            item = new Item { Id = world.Id("needs-access-tool"), Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = equipment.Id, Quality = 3, Condition = 17, MaxCondition = 42 };
            village.Items.Add(item);
            var order = Act(world, village, new GameAction { Kind = "EquipItem", CatId = cat.Id, TargetId = item.Id });
            job = village.Jobs.Single(candidate => candidate.Id == order.EntityId);
            Check(job.Manual && job.ItemIds.SequenceEqual(new[] { item.Id }) && item.LocationId == job.Id && cat.JobId == job.Id, "Needs fixture must start the exact finite manual retrieval through its public action");
            if (resource == "water") cat.Thirst = 34;
            else if (resource == "medicine") cat.Health = 34;
            else cat.Hunger = 34;
            return world;
        }

        static void Pending(Village village, Cat cat, Item item, Job job)
        {
            Check(!job.Completed && cat.JobId == job.Id && cat.ResumeJobId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }) && item.LocationId == job.Id && !cat.Equipment.Contains(item.Id), "Needs changed or prematurely completed the same finite manual retrieval");
            Check(village.Items.Count(candidate => candidate.Id == item.Id) == 1 && item.Kind == "tool" && item.Material == "wood" && item.Quality == 3 && item.Condition == 17 && item.MaxCondition == 42, "Needs changed the finite exact item's identity or condition");
        }

        static void Finish(World world, Village village, Cat cat, Item item, Job job)
        {
            Check(cat.ResumeJobId == "" && cat.JobId == job.Id && !job.Completed && cat.Cargo.Count == 0, "Shrine recovery failed to resume the same real finite manual order");
            for (int tick = 0; tick < 1200 && !job.Completed; tick++) world.Step(.05);
            Check(job.Completed && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && cat.JobId == "" && cat.ResumeJobId == "" && job.ItemIds.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Recovered cat failed to physically complete its original exact retrieval once");
            world.Step(1);
            Check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1 && village.Items.Count(candidate => candidate.Id == item.Id) == 1 && item.Condition == 17 && item.MaxCondition == 42, "Resumed finite retrieval replayed or changed its exact item");
            Valid(world);
        }

        static void BlockedAnchor(string resource)
        {
            var world = Fixture(resource, out var village, out var cat, out var pantry, out var item, out var job);
            Avoid(world, village, pantry.Position);
            Check(world.Path(cat.Position, pantry.Position, village) == null && world.Path(cat.Position, Alternate(pantry), village) != null && world.Path(Alternate(pantry), village.Center, village) != null, "Needs fixture must close only the pantry anchor and retain a physical supply-and-shrine route");
            world.Step(.05);
            Check(cat.Goal == Goal(resource) && cat.ResumeJobId == job.Id, "Low need failed to suspend the accepted finite manual retrieval");
            bool sawPickup = false, sawLoadedTravel = false, recovered = false;
            for (int tick = 0; tick < 1200 && !recovered; tick++)
            {
                double beforeCargo = World.Amount(cat.Cargo, resource);
                world.Step(.05);
                double cargo = World.Amount(cat.Cargo, resource);
                recovered = Need(cat, resource) > 54;
                if (!sawPickup && cargo > 0)
                {
                    sawPickup = true;
                    var access = Alternate(pantry);
                    Check(beforeCargo == 0 && cargo == 1 && cat.Position.Equals(access) && cat.X == access.X && cat.Z == access.Z && World.Amount(pantry.Goods, resource) == 2, "Needs serving transferred before physical alternate-tile arrival or changed its finite dose");
                }
                if (!recovered)
                {
                    Pending(village, cat, item, job);
                    Check(Need(cat, resource) <= 34 + world.TimeSeconds * .002 + 1e-9, "Needs recovered before carrying its real serving home to the shrine");
                    Check(World.Amount(pantry.Goods, resource) + cargo == 3, "Needs travel lost or duplicated a finite serving");
                    if (sawPickup && !cat.Position.Equals(village.Center) && cargo == 1 && (cat.X != Alternate(pantry).X || cat.Z != Alternate(pantry).Z)) sawLoadedTravel = true;
                }
            }
            Check(sawPickup && sawLoadedTravel && recovered, "Needs never physically used the open pantry footprint and returned with its serving");
            Check(cat.Position.Equals(village.Center) && cat.X == village.Center.X && cat.Z == village.Center.Z && World.Amount(pantry.Goods, resource) == 2 && cat.Cargo.Count == 0 && !world.Walkable(village, pantry.Position), "Needs recovered away from the shrine, consumed twice, retained duplicate cargo or reopened the closed anchor");
            Finish(world, village, cat, item, job);
        }

        static void ClosedRetry()
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            Avoid(world, village, pantry.Position);
            string alternateZone = Avoid(world, village, Alternate(pantry));
            world.Step(.1);
            Check(cat.NeedSourceId == "" && cat.NextNeedAttemptAt > world.TimeSeconds && World.Amount(pantry.Goods, "water") == 3 && cat.Cargo.Count == 0 && !job.Completed && item.LocationId == job.Id, "Fully closed needs supply consumed remotely or lost the finite work instead of entering its bounded retry");
            long searches = Searches(world);
            double retry = cat.NextNeedAttemptAt;
            double x = cat.X, z = cat.Z;
            world.Step(.2);
            Check(world.TimeSeconds < retry && cat.NextNeedAttemptAt == retry && Searches(world) == searches && (cat.X != x || cat.Z != z) && cat.JobId == job.Id && item.LocationId == job.Id, "Unavailable needs repeated source searches before retry or stopped its recoverable work route");
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = alternateZone });
            Recovered(world, village, cat, item, job, "water");
            Check(World.Amount(pantry.Goods, "water") == 2 && !world.Walkable(village, pantry.Position), "Reopened needs access changed the closed anchor or consumed more than one serving");
            Finish(world, village, cat, item, job);
        }

        static void Retarget(bool remove)
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            Avoid(world, village, pantry.Position);
            var spare = Spare(world, village, new Int2(3, 6), "water", 1);
            world.Step(.1);
            Check(cat.NeedSourceId == pantry.Id && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(Alternate(pantry)) && cat.X > cat.Position.X, "Cached needs fixture failed to begin fractional travel to the alternate tile");
            if (remove) Act(world, village, new GameAction { Kind = "RemoveStockpile", TargetId = pantry.Id });
            else Avoid(world, village, Alternate(pantry));
            world.Step(.05);
            Check(cat.NeedSourceId == spare.Id && cat.Cargo.Count == 0 && World.Amount(pantry.Goods, "water") == 3 && cat.ResumeJobId == job.Id && cat.X == cat.Position.X && cat.Z == cat.Position.Z, "Cached closed or removed source stayed eligible, consumed remotely or bypassed its fractional retrace");
            Recovered(world, village, cat, item, job, "water");
            Check(World.Amount(spare.Goods, "water") == 0 && World.Amount(pantry.Goods, "water") == 3 && (!remove || pantry.Kind == "spill"), "Needs retargeting changed the ineligible pile or duplicated its replacement serving");
            Finish(world, village, cat, item, job);
        }

        static void CachedEdge()
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            Avoid(world, village, pantry.Position);
            world.Step(.2);
            var origin = cat.Position;
            Check(cat.NeedSourceId == pantry.Id && cat.X != origin.X && cat.Path.Count > 0 && world.Walkable(village, Alternate(pantry)), "Needs edge fixture did not enter its cached fractional route");
            var edges = new List<BoundaryEdge>();
            foreach (var delta in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                edges.Add(new BoundaryEdge { From = origin, To = new Int2(origin.X + delta.X, origin.Z + delta.Z) });
            village.BoundaryEdges.AddRange(edges);
            double priorX = cat.X;
            world.Step(.05);
            Pending(village, cat, item, job);
            Check(cat.Position.Equals(origin) && cat.X > origin.X && cat.X < priorX && cat.NeedSourceId == pantry.Id && cat.Cargo.Count == 0 && World.Amount(pantry.Goods, "water") == 3, "Closed needs edge snapped its fractional position home or consumed before its physical retrace finished");
            world.Step(.1);
            Pending(village, cat, item, job);
            Check(cat.Position.Equals(origin) && cat.X == origin.X && cat.Z == origin.Z && cat.Path.Count == 0 && cat.NeedSourceId == "" && cat.BlockedReason == "blocked_route" && cat.NextNeedAttemptAt > world.TimeSeconds && World.Amount(pantry.Goods, "water") == 3 && cat.Cargo.Count == 0, "Needs bypassed its closed intermediate edge or failed to physically retrace and retain the finite order");
            long searches = Searches(world);
            world.Step(.2);
            Check(Searches(world) == searches && cat.Path.Count == 0 && cat.Cargo.Count == 0 && cat.BlockedReason == "blocked_route", "Cached needs failure repeated path searches or cleared its real blocker before retry");
            foreach (var edge in edges) village.BoundaryEdges.Remove(edge);
            Recovered(world, village, cat, item, job, "water");
            Check(World.Amount(pantry.Goods, "water") == 2, "Reopened intermediate route duplicated or lost its finite need serving");
            Finish(world, village, cat, item, job);
        }

        static void AnchorReopens()
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            string anchorZone = Avoid(world, village, pantry.Position);
            world.Step(.1);
            var endpoint = Alternate(pantry);
            Check(cat.NeedSourceId == pantry.Id && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(endpoint) && cat.X > cat.Position.X, "Reopening fixture must begin fractional travel to the cached alternate");
            double priorX = cat.X;
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = anchorZone });
            world.Step(.05);
            Pending(village, cat, item, job);
            Check(world.Walkable(village, pantry.Position) && cat.NeedSourceId == pantry.Id && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(endpoint) && cat.X > priorX && cat.Cargo.Count == 0, "Reopened anchor replaced the cached alternate destination or reversed ongoing physical travel");
            Wait(world, () => cat.Cargo.Count > 0, "Reopened anchor prevented pickup at the retained alternate endpoint");
            Check(cat.Position.Equals(endpoint) && cat.X == endpoint.X && cat.Z == endpoint.Z && World.Amount(cat.Cargo, "water") == 1 && World.Amount(pantry.Goods, "water") == 2 && cat.Thirst < 35, "Reopened anchor caused remote pickup, early recovery or a different endpoint");
            Recovered(world, village, cat, item, job, "water");
            Check(World.Amount(pantry.Goods, "water") == 2, "Anchor reopening consumed the same need serving more than once");
            Finish(world, village, cat, item, job);
        }

        static void Competition()
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            World.Add(pantry.Goods, "water", -2);
            Avoid(world, village, pantry.Position);
            var other = village.Cats[2];
            other.ControlledBy = "";
            other.Position = cat.Position; other.X = cat.X; other.Z = cat.Z; other.Thirst = 34;
            other.Path.Clear(); other.Skills.Clear();
            world.Step(.1);
            Check(cat.NeedSourceId == pantry.Id && other.NeedSourceId == pantry.Id && World.Amount(pantry.Goods, "water") == 1, "Competing needs must both approach the same last finite serving without reserving or consuming it remotely");
            Wait(world, () => cat.Cargo.Count > 0 || other.Cargo.Count > 0, "Neither competing cat reached the real last serving");
            Check(World.Amount(pantry.Goods, "water") == 0 && World.Amount(cat.Cargo, "water") + World.Amount(other.Cargo, "water") == 1 && cat.Thirst < 35 && other.Thirst < 35, "Competing footprint arrivals duplicated the last serving or recovered before returning home");
            // Freeze the losing observer after its arrival recheck, so the
            // founding director cannot assign unrelated work during this check.
            Check(World.Amount(cat.Cargo, "water") == 1 && other.Cargo.Count == 0, "Stable actor order must award the last serving exactly once");
            other.ControlledBy = "fixture-held";
            Recovered(world, village, cat, item, job, "water");
            Check(other.Thirst < 35 && World.Amount(pantry.Goods, "water") == 0 && other.Cargo.Count == 0, "The second consumer recovered from a serving already carried and consumed by another cat");
            Finish(world, village, cat, item, job);
        }

        static void Reserved(bool afterSelection)
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            Avoid(world, village, pantry.Position);
            var spare = Spare(world, village, new Int2(3, 6), "water", 1);
            Spare(world, village, new Int2(5, 5), "water", 0);
            if (afterSelection)
            {
                world.Step(.1);
                Check(cat.NeedSourceId == pantry.Id && cat.Cargo.Count == 0, "Reserved-arrival fixture did not first cache an unclaimed real supply");
            }
            var hauler = village.Cats[2];
            hauler.ControlledBy = "";
            var haulAction = Act(world, village, new GameAction { Kind = "HaulGatherSpot", CatId = hauler.Id, TargetId = pantry.Id });
            var haul = village.Jobs.Single(candidate => candidate.Id == haulAction.EntityId);
            hauler.ControlledBy = "fixture-held";
            Check(haul.Amount == 3 && world.Reservations.Any(claim => claim.OwnerId == haul.Id && claim.PileId == pantry.Id && claim.Resource == "water" && claim.Amount == 3), "Needs reservation guard requires a real competing finite hauling claim");
            world.Step(afterSelection ? .05 : .1);
            Check(cat.NeedSourceId == spare.Id && cat.Cargo.Count == 0 && World.Amount(pantry.Goods, "water") == 3, "Needs selection or cached-arrival check stole a serving reserved by another real job");
            Recovered(world, village, cat, item, job, "water");
            Check(World.Amount(spare.Goods, "water") == 0 && World.Amount(pantry.Goods, "water") == 3 && world.Reservations.Any(claim => claim.OwnerId == haul.Id && claim.PileId == pantry.Id && claim.Amount == 3) && !haul.Completed, "Needs retargeting consumed or released another job's finite reservation");
            Finish(world, village, cat, item, job);
        }

        static void Brew()
        {
            var world = Fixture("water", out var village, out var cat, out var pantry, out var item, out var job);
            World.Add(pantry.Goods, "brew", .25);
            Avoid(world, village, pantry.Position);
            Wait(world, () => cat.Cargo.Count > 0, "Brew need failed to reach its open footprint");
            Check(World.Amount(cat.Cargo, "brew") == .25 && World.Amount(pantry.Goods, "brew") == 0 && World.Amount(pantry.Goods, "water") == 3 && cat.Position.Equals(Alternate(pantry)) && cat.Thirst < 35 && !cat.PersonalBrewUsed, "Brew preference or fractional serving changed before physical home consumption");
            Wait(world, () => cat.PersonalBrewUsed, "Brew was never physically returned and consumed");
            Pending(village, cat, item, job);
            Check(cat.Position.Equals(village.Center) && cat.Cargo.Count == 0 && cat.Thirst > 45 && cat.Thirst < 47 && World.Amount(pantry.Goods, "water") == 3, "Brew skipped shrine arrival, completed the whole need early or consumed water remotely");
            Wait(world, () => World.Amount(cat.Cargo, "water") > 0, "Brew need did not return for its remaining water serving");
            Check(World.Amount(cat.Cargo, "water") == .75 && World.Amount(pantry.Goods, "water") == 2.25 && cat.Position.Equals(Alternate(pantry)) && cat.Thirst < 47, "Post-brew water retrieval changed the remaining finite .75 serving or recovered remotely");
            Recovered(world, village, cat, item, job, "water");
            Check(!cat.PersonalBrewUsed && World.Amount(pantry.Goods, "water") == 2.25 && World.Amount(pantry.Goods, "brew") == 0, "Brew/water completion retained its partial-serving flag or replayed consumption");
            Finish(world, village, cat, item, job);
        }

        static void Fish()
        {
            var world = Fixture("food", out var village, out var cat, out var pantry, out var item, out var job);
            World.Add(pantry.Goods, "fish", 1); World.Add(pantry.Goods, "preserves", 1);
            Avoid(world, village, pantry.Position);
            Wait(world, () => cat.Cargo.Count > 0, "Food preference failed to reach the open footprint");
            Check(World.Amount(cat.Cargo, "fish") == 1 && World.Amount(pantry.Goods, "fish") == 0 && World.Amount(pantry.Goods, "preserves") == 1 && World.Amount(pantry.Goods, "food") == 3 && cat.Position.Equals(Alternate(pantry)) && cat.Hunger < 35, "Footprint access changed food preferences, dose or physical pickup");
            Recovered(world, village, cat, item, job, "food");
            Check(World.Amount(pantry.Goods, "preserves") == 1 && World.Amount(pantry.Goods, "food") == 3, "Eating the preferred finite fish also consumed fallback food");
            Finish(world, village, cat, item, job);
        }

        static void Medicine()
        {
            var world = Fixture("medicine", out var village, out var cat, out var pantry, out var item, out var job);
            World.Add(pantry.Goods, "herbs", 1);
            Avoid(world, village, pantry.Position);
            Wait(world, () => cat.Cargo.Count > 0, "Medicine preference failed to reach the open footprint");
            Check(World.Amount(cat.Cargo, "medicine") == 1 && World.Amount(pantry.Goods, "medicine") == 2 && World.Amount(pantry.Goods, "herbs") == 1 && cat.Position.Equals(Alternate(pantry)) && cat.Health < 35, "Footprint access changed healing preferences, dose or physical pickup");
            Recovered(world, village, cat, item, job, "medicine");
            Check(World.Amount(pantry.Goods, "herbs") == 1, "Using preferred medicine also consumed fallback herbs");
            Finish(world, village, cat, item, job);
        }
    }
}
