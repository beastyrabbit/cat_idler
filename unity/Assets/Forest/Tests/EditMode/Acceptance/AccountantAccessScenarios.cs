using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class AccountantAccessScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.accountant_access_blocked_anchor_counts_after_physical_dwell", BlockedAnchor);
            yield return new Scenario("regression.accountant_access_removed_current_storage_is_not_counted", () => Removed(false));
            yield return new Scenario("regression.accountant_access_removed_pending_storage_is_not_counted", () => Removed(true));
            yield return new Scenario("regression.accountant_access_reopened_anchor_keeps_walking_endpoint", () => AnchorReopens(false));
            yield return new Scenario("regression.accountant_access_reopened_anchor_keeps_counting_tile", () => AnchorReopens(true));
            yield return new Scenario("regression.accountant_access_closed_cached_endpoint_retraces", ClosedEndpoint);
            yield return new Scenario("regression.accountant_access_closed_cached_edge_retraces", ClosedEdge);
            yield return new Scenario("regression.accountant_access_whole_closure_retries_on_next_round", ClosedRound);
            yield return new Scenario("regression.accountant_access_needs_interrupt_and_resume_counting", Needs);
            yield return new Scenario("regression.accountant_access_removed_counting_storage_is_not_counted", RemovedCounting);
            yield return new Scenario("regression.accountant_access_scalar_and_all_exact_equipment_report", ExactEquipment);
            yield return new Scenario("regression.accountant_access_aligned_footprint_entry_keeps_cached_endpoint", AlignedEntry);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "accountant-access-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }
        static bool Contains(Int2 anchor, int width, int depth, Int2 at) => anchor.Level == at.Level && at.X >= anchor.X && at.X < anchor.X + width && at.Z >= anchor.Z && at.Z < anchor.Z + depth;
        static Int2 Alternate(Stockpile pile) => new Int2(pile.Position.X + 1, pile.Position.Z, pile.Position.Level);
        static string Avoid(World world, Village village, Int2 at) => Act(world, village, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = at, End = at }).EntityId;
        static void Wait(World world, Func<bool> ready, string message, int ticks = 1600)
        {
            for (int tick = 0; tick < ticks && !ready(); tick++) world.Step(.05);
            Check(ready(), message);
        }
        static void Valid(World world) => Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        static void Disjoint(World world, Village village, Int2 anchor, int width, int depth)
        {
            for (int z = 0; z < depth; z++)
                for (int x = 0; x < width; x++)
                {
                    var at = new Int2(anchor.X + x, anchor.Z + z, anchor.Level);
                    Check(!village.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, at)) && !village.Stockpiles.Any(p => Contains(p.Position, p.Width, p.Depth, at)), "Accounting fixture footprint overlaps a retained home, shrine, workshop store or pile at " + at);
                    World.ClearSurface(world.TileAt(at), 0);
                    world.TileAt(at).Wall = false;
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
        }
        static Stockpile Pile(World world, Village village, Int2 at)
        {
            Disjoint(world, village, at, 2, 1);
            var pile = new Stockpile { Id = world.Id("accountant-access-pile"), Position = at, Width = 2, Depth = 1, Capacity = 100, CountedAt = 0, Goods = new List<Stack> { new Stack("logs", 7), new Stack("stone", 3) }, Report = new List<Stack> { new Stack("logs", 2) } };
            village.Stockpiles.Add(pile);
            return pile;
        }
        static World Fixture(out Village village, out Cat cat, out Building tent, out Stockpile pile, out Item item)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.LastLeaderResearch = 0;
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            foreach (var existing in village.Stockpiles.ToArray())
            {
                existing.Goods.Clear();
                for (int z = 0; z < existing.Depth; z++)
                    for (int x = 0; x < existing.Width; x++)
                        Avoid(world, village, new Int2(existing.Position.X + x, existing.Position.Z + z));
            }
            // Keep the independent founding survival director funded without
            // providing another visitable store or a second needs source.
            village.Stockpiles.First(p => p.Kind == "storage").Goods.AddRange(new[] { new Stack("food", 1000), new Stack("water", 1000) });
            foreach (var other in village.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
                other.Hunger = other.Thirst = other.Rest = other.Health = 100;
            }
            var stationAt = new Int2(1, 5);
            Disjoint(world, village, stationAt, 1, 1);
            tent = new Building { Id = world.Id("accountant-access-tent"), Kind = "accounting_tent", Position = stationAt, Width = 1, Depth = 1, Completed = true };
            village.Buildings.Add(tent);
            pile = Pile(world, village, new Int2(3, 5));
            item = new Item { Id = world.Id("accountant-access-tool"), Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = pile.Id, Quality = 3, Condition = 17, MaxCondition = 42 };
            village.Items.Add(item);
            cat = village.Cats[1];
            cat.ControlledBy = "";
            cat.Position = stationAt; cat.X = stationAt.X; cat.Z = stationAt.Z;
            cat.Path.Clear(); cat.Skills.Clear();
            village.Research.Add("basic_tools");
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = cat.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = tent.Id, CatId = cat.Id });
            string workerId = cat.Id;
            Check(cat.BuildingId == tent.Id && tent.Slots.Any(s => s.CatId == workerId) && cat.OfficerRole == "accountant", "Accounting fixture did not publicly staff the unlocked office and tent");
            return world;
        }
        static void OldReport(Stockpile pile) => Check(pile.CountedAt == 0 && pile.Report.Count == 1 && World.Amount(pile.Report, "logs") == 2, "Accounting refreshed knowledge before physical arrival and five units of counting");
        static void Conserved(Village village, Stockpile pile, Item item)
        {
            Check(World.Amount(pile.Goods, "logs") == 7 && World.Amount(pile.Goods, "stone") == 3 && pile.Goods.Count == 2 && village.Items.Count(i => i.Id == item.Id) == 1 && item.LocationId == pile.Id && item.Kind == "tool" && item.Material == "wood" && item.Quality == 3 && item.Condition == 17 && item.MaxCondition == 42, "Accounting moved, duplicated or changed finite scalar goods or exact equipment");
        }
        static void Fresh(World world, Village village, Cat cat, Building tent, Stockpile pile, Item item, Int2 endpoint)
        {
            Check(cat.Position.Equals(endpoint) && cat.X == endpoint.X && cat.Z == endpoint.Z && pile.CountedAt == world.TimeSeconds && World.Amount(pile.Report, "logs") == 7 && World.Amount(pile.Report, "stone") == 3 && World.Amount(pile.Report, "tools") == 1 && pile.Report.Count == 3 && tent.Progress == 0, "Accounting did not produce one fresh exact report at the physical counting tile");
            Conserved(village, pile, item);
            Valid(world);
        }
        static void Arrive(World world, Village village, Cat cat, Stockpile pile)
        {
            var endpoint = Alternate(pile);
            Wait(world, () => village.Accounting != null && village.Accounting.TargetId == pile.Id && cat.Position.Equals(endpoint) && cat.X == endpoint.X && cat.Z == endpoint.Z, "Accountant never physically reached the alternate counting tile");
            OldReport(pile);
        }
        static void Finish(World world, Village village, Cat cat, Building tent, Stockpile pile, Item item)
        {
            Wait(world, () => pile.CountedAt > 0, "Accountant failed to complete the retained finite visit");
            Fresh(world, village, cat, tent, pile, item, Alternate(pile));
        }

        static void BlockedAnchor()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            Avoid(world, village, pile.Position);
            var endpoint = Alternate(pile);
            Check(world.Path(cat.Position, pile.Position, village) == null && world.Path(cat.Position, endpoint, village) != null, "Accounting fixture must block only the pile anchor");
            world.Step(.05);
            Check(village.Accounting != null && village.Accounting.WorkerId == cat.Id && village.Accounting.BuildingId == tent.Id && village.Accounting.TargetId == pile.Id && !village.Accounting.Unreachable.Contains(pile.Id), "Accountant skipped a stockpile with a reachable alternate footprint tile");
            while (!cat.Position.Equals(endpoint) || cat.X != endpoint.X || cat.Z != endpoint.Z)
            {
                OldReport(pile);
                Check(village.Accounting.DwellSeconds == 0, "Counting accumulated during physical travel");
                world.Step(.05);
                Check(world.TimeSeconds < 20, "Accountant failed to physically reach the open footprint tile");
            }
            double arrival = world.TimeSeconds;
            OldReport(pile);
            while (pile.CountedAt == 0)
            {
                Check(village.Accounting.DwellSeconds < 5 && tent.Progress == village.Accounting.DwellSeconds, "Counting state and tent progress disagree before report");
                world.Step(.05);
                if (pile.CountedAt == 0) OldReport(pile);
                Check(world.TimeSeconds - arrival < 6, "Accountant did not finish its five-unit dwell");
            }
            Check(world.TimeSeconds - arrival >= 4.9, "Accountant refreshed without a full physical counting dwell");
            Fresh(world, village, cat, tent, pile, item, endpoint);
            double counted = pile.CountedAt;
            world.Step(1);
            Check(pile.CountedAt == counted, "Accounting replayed the same report after the visit finished");
        }

        static void Removed(bool pending)
        {
            var world = Fixture(out var village, out var cat, out var tent, out var first, out var firstItem);
            var pile = pending ? Pile(world, village, new Int2(5, 5)) : first;
            var item = firstItem;
            if (pending)
            {
                item = new Item { Id = world.Id("accountant-access-pending-tool"), Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = pile.Id, Quality = 3, Condition = 17, MaxCondition = 42 };
                village.Items.Add(item);
            }
            world.Step(.05);
            Check(village.Accounting != null && village.Accounting.TargetId == first.Id && (!pending || village.Accounting.Pending.Contains(pile.Id)), "Removal fixture must snapshot the real current and pending storage before removal");
            Act(world, village, new GameAction { Kind = "RemoveStockpile", TargetId = pile.Id });
            Check(village.Stockpiles.Contains(pile) && pile.Kind == "spill" && item.LocationId == pile.Id, "Public removal did not retain the same finite nonempty pile as a spill");
            world.Step(20);
            OldReport(pile);
            Check(village.Accounting.TargetId != pile.Id && !village.Accounting.Pending.Contains(pile.Id), "Accountant retained a removed storage pile as a count target");
            if (pending) Check(first.CountedAt > 0, "Skipping removed pending storage starved the still-valid first visit");
            Conserved(village, pile, item);
            Valid(world);
        }

        static void AnchorReopens(bool counting)
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            string anchorZone = Avoid(world, village, pile.Position);
            if (counting)
            {
                Arrive(world, village, cat, pile);
                world.Step(.8);
                Check(village.Accounting.DwellSeconds > .7 && village.Accounting.DwellSeconds < 1, "Counting-reopen fixture did not begin a real dwell");
            }
            else
            {
                world.Step(.2);
                Check(village.Accounting.TargetId == pile.Id && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(Alternate(pile)) && cat.X > cat.Position.X, "Walking-reopen fixture did not begin fractional travel to the alternate");
            }
            double before = village.Accounting.DwellSeconds, beforeX = cat.X;
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = anchorZone });
            world.Step(.05);
            OldReport(pile);
            Check(village.Accounting.TargetId == pile.Id && world.Walkable(village, pile.Position), "Reopening the anchor replaced the active counting target");
            if (counting)
                Check(cat.Position.Equals(Alternate(pile)) && cat.X == Alternate(pile).X && cat.Z == Alternate(pile).Z && cat.Path.Count == 0 && village.Accounting.DwellSeconds > before, "Reopened anchor displaced a physically counting accountant or restarted its dwell");
            else
                Check(cat.X > beforeX && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(Alternate(pile)) && village.Accounting.DwellSeconds == 0, "Reopened anchor retargeted or reversed the accountant's cached alternate approach");
            Finish(world, village, cat, tent, pile, item);
        }

        static void ClosedEndpoint()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            Avoid(world, village, pile.Position);
            world.Step(.2);
            Int2 origin = cat.Position;
            double previous = cat.X;
            Check(previous > origin.X && village.Accounting.TargetId == pile.Id, "Endpoint-closure fixture did not begin fractional physical travel");
            string endZone = Avoid(world, village, Alternate(pile));
            world.Step(.05);
            OldReport(pile);
            Check(village.Accounting.TargetId == "" && village.Accounting.Unreachable.Contains(pile.Id) && cat.X == previous && cat.Position.Equals(origin), "Full footprint closure falsely counted, teleported or kept an unreachable target");
            world.Step(.05);
            Check(cat.Position.Equals(origin) && cat.X > origin.X && cat.X < previous, "Skipped closed endpoint failed to retrace its unfinished approach physically");
            Wait(world, () => village.Accounting.Phase == "waiting", "Closed endpoint did not return to the existing round wait");
            Check(cat.Position.Equals(tent.Position) && cat.X == tent.Position.X && cat.Z == tent.Position.Z, "Round wait began before the accountant physically returned to its tent");
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = endZone });
            Finish(world, village, cat, tent, pile, item);
        }

        static void ClosedEdge()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            Avoid(world, village, pile.Position);
            world.Step(.2);
            Int2 origin = cat.Position;
            double before = cat.X;
            Check(before > origin.X && village.Accounting.TargetId == pile.Id && cat.Path.Count > 0, "Cached-edge fixture did not begin fractional physical travel");
            var edges = new List<BoundaryEdge>();
            foreach (var delta in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                edges.Add(new BoundaryEdge { From = origin, To = new Int2(origin.X + delta.X, origin.Z + delta.Z) });
            village.BoundaryEdges.AddRange(edges);
            world.Step(.05);
            OldReport(pile);
            Check(cat.Position.Equals(origin) && cat.X > origin.X && cat.X < before && village.Accounting.TargetId == pile.Id && village.Accounting.DwellSeconds == 0, "Cached-edge closure bypassed movement or skipped the target before physical retracing finished");
            world.Step(.2);
            OldReport(pile);
            Check(cat.Position.Equals(origin) && cat.X == origin.X && cat.Z == origin.Z && cat.Path.Count == 0 && village.Accounting.TargetId == "" && village.Accounting.Unreachable.Contains(pile.Id), "Cached-edge closure failed to settle physically and skip the unreachable visit");
            foreach (var edge in edges) village.BoundaryEdges.Remove(edge);
            Finish(world, village, cat, tent, pile, item);
        }

        static void ClosedRound()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            Avoid(world, village, pile.Position);
            string endZone = Avoid(world, village, Alternate(pile));
            world.Step(.05);
            Check(village.Accounting.Phase == "waiting" && village.Accounting.Unreachable.Contains(pile.Id) && village.Accounting.TargetId == "", "Wholly closed stockpile did not leave an unreachable visit and finish its round");
            OldReport(pile);
            var round = village.Accounting;
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = endZone });
            world.Step(59);
            OldReport(pile);
            Check(village.Accounting == round && round.Phase == "waiting" && round.TargetId == "" && cat.Position.Equals(tent.Position) && cat.X == tent.Position.X && cat.Z == tent.Position.Z, "Reopening bypassed the existing bounded round wait or replaced its owner");
            Finish(world, village, cat, tent, pile, item);
            Check(pile.CountedAt >= 65, "Reopened storage was counted before the next physical round and five-unit dwell");
        }

        static void Needs()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            var water = Pile(world, village, new Int2(-3, 5));
            water.Goods = new List<Stack> { new Stack("water", 1) };
            Avoid(world, village, pile.Position);
            Arrive(world, village, cat, pile);
            world.Step(.8);
            var round = village.Accounting;
            double counted = round.DwellSeconds;
            Check(counted > .7 && counted < 1, "Needs fixture did not begin actual counting");
            cat.Thirst = 34;
            world.Step(.05);
            Check(cat.Goal == "need_drink" && round.TargetId == pile.Id && round.DwellSeconds == counted && cat.BuildingId == tent.Id, "Ordinary thirst failed to suspend the same accountant and partial dwell");
            Wait(world, () => cat.Cargo.Count > 0, "Accountant did not physically retrieve the finite water serving");
            OldReport(pile);
            Check(World.Amount(water.Goods, "water") == 0 && World.Amount(cat.Cargo, "water") == 1 && cat.Thirst < 35 && round.DwellSeconds == counted, "Needs counted remotely, recovered before shrine arrival or duplicated its water serving");
            Wait(world, () => cat.Thirst > 50 && !cat.Goal.StartsWith("need_", StringComparison.Ordinal), "Accountant failed to physically return and drink at the shrine");
            OldReport(pile);
            Check(cat.Position.Equals(village.Center) && cat.X == village.Center.X && cat.Z == village.Center.Z && cat.Cargo.Count == 0 && village.Accounting == round && round.TargetId == pile.Id && round.DwellSeconds == counted, "Shrine recovery lost the round, advanced counting away from the pile or moved the finite cargo twice");
            while (!cat.Position.Equals(Alternate(pile)) || cat.X != Alternate(pile).X || cat.Z != Alternate(pile).Z)
            {
                Check(round.DwellSeconds == counted, "Counting progressed during the needs return trip to the stockpile");
                OldReport(pile);
                world.Step(.05);
                Check(world.TimeSeconds < 45, "Accountant did not resume its physical counting visit after drinking");
            }
            Finish(world, village, cat, tent, pile, item);
            Check(World.Amount(water.Goods, "water") == 0 && cat.Cargo.Count == 0 && cat.BuildingId == tent.Id && cat.OfficerRole == "accountant", "Resumed accounting changed its office or replayed finite need consumption");
        }

        static void RemovedCounting()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            Avoid(world, village, pile.Position);
            Arrive(world, village, cat, pile);
            world.Step(1);
            Check(village.Accounting.DwellSeconds >= 1 && village.Accounting.DwellSeconds < 2, "Removal fixture did not begin counting");
            Act(world, village, new GameAction { Kind = "RemoveStockpile", TargetId = pile.Id });
            world.Step(10);
            OldReport(pile);
            Check(pile.Kind == "spill" && village.Accounting.TargetId != pile.Id, "A storage removal during dwell still produced a fresh spill report");
            Conserved(village, pile, item);
            Valid(world);
        }

        static void ExactEquipment()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            var exact = new List<Item> { item };
            foreach (string kind in new[] { "weapon", "armor" })
            {
                var added = new Item { Id = world.Id("accountant-access-" + kind), Kind = kind, Material = "stone", VillageId = village.Id, LocationId = pile.Id, Quality = 2, Condition = 23, MaxCondition = 51 };
                village.Items.Add(added);
                exact.Add(added);
            }
            Avoid(world, village, pile.Position);
            Wait(world, () => pile.CountedAt > 0, "Accountant failed to finish a physical exact-equipment count");
            Check(cat.Position.Equals(Alternate(pile)) && cat.X == Alternate(pile).X && cat.Z == Alternate(pile).Z && pile.CountedAt == world.TimeSeconds && World.Amount(pile.Report, "logs") == 7 && World.Amount(pile.Report, "stone") == 3 && World.Amount(pile.Report, "tools") == 1 && World.Amount(pile.Report, "weapons") == 1 && World.Amount(pile.Report, "armor") == 1 && pile.Report.Count == 5, "Accounting omitted scalar goods, an exact equipment kind or the actual observation time");
            Conserved(village, pile, item);
            foreach (var gear in exact.Skip(1))
                Check(village.Items.Count(i => i.Id == gear.Id) == 1 && gear.LocationId == pile.Id && gear.Material == "stone" && gear.Quality == 2 && gear.Condition == 23 && gear.MaxCondition == 51, "Exact accounting changed an equipment identity, material, quality or condition");
            Check(!pile.Goods.Any(s => s.Resource == "tools" || s.Resource == "weapons" || s.Resource == "armor"), "Accounting turned exact equipment reports into scalar equipment goods");
            Valid(world);
        }

        static void AlignedEntry()
        {
            var world = Fixture(out var village, out var cat, out var tent, out var pile, out var item);
            var stationAt = new Int2(6, 5);
            Disjoint(world, village, stationAt, 1, 1);
            tent.Position = stationAt;
            cat.Position = stationAt; cat.X = stationAt.X; cat.Z = stationAt.Z;
            // With a flat, plain corridor, 25 ordinary 50 ms ticks traverse two
            // units at 1.6 units/s and land on the footprint's nearer tile.
            // The preferred anchor is still one further tile along that path.
            for (int x = pile.Position.X; x <= stationAt.X; x++)
            {
                var at = new Int2(x, stationAt.Z);
                var tile = world.TileAt(at);
                World.ClearSurface(tile, 0);
                tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false;
                tile.Biome = "grass";
                if (!village.Known.Contains(at)) village.Known.Add(at);
            }
            world.Step(1.25);
            var entry = Alternate(pile);
            Check(village.Accounting.TargetId == pile.Id && cat.Position.Equals(entry) && Math.Abs(cat.X - entry.X) + Math.Abs(cat.Z - entry.Z) < 1e-9 && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(pile.Position) && village.Accounting.DwellSeconds == 0, "Aligned-entry fixture did not physically reach the near footprint tile with a valid farther cached endpoint");
            OldReport(pile);
            double beforeX = cat.X;
            world.Step(.05);
            Check(cat.X < beforeX && cat.Path.Count > 0 && cat.Path[cat.Path.Count - 1].Equals(pile.Position) && village.Accounting.DwellSeconds == 0, "Aligned footprint entry abandoned the cached farther endpoint and began counting before its chosen arrival");
            Wait(world, () => pile.CountedAt > 0, "Accountant did not finish the physical cached-endpoint visit");
            Fresh(world, village, cat, tent, pile, item, pile.Position);
        }
    }
}
