using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Job flow robustness: blocked routes release, blocked reasons clear, hunts deliver in one trip, spills
    /// merge and vanish, deliveries split across piles, claims prefer reachable piles, scouts skip dead
    /// waypoints and only cats that can work take orders.
    /// </summary>
    public static class JobRobustnessScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("job_robustness.blocked_route_releases_and_retries_elsewhere", BlockedRouteReleases);
            yield return new Scenario("job_robustness.blocked_reason_clears_when_block_ends", BlockedReasonClears);
            yield return new Scenario("job_robustness.hunt_delivers_hide_and_bone_in_one_trip", HuntOneTrip);
            yield return new Scenario("job_robustness.spills_share_one_pile_per_tile_and_vanish_when_empty", SpillsMergeAndVanish);
            yield return new Scenario("job_robustness.partial_delivery_fills_several_piles", PartialDelivery);
            yield return new Scenario("job_robustness.cancelled_production_returns_inputs", CancelledProductionReturnsInputs);
            yield return new Scenario("job_robustness.claims_prefer_nearest_reachable_pile", ClaimsPreferNearestReachable);
            yield return new Scenario("job_robustness.unreachable_claim_released_and_excluded", UnreachableClaimReleased);
            yield return new Scenario("job_robustness.scout_skips_unreachable_waypoints", ScoutSkipsUnreachableWaypoints);
            yield return new Scenario("job_robustness.carry_capacity_scales_with_haul_capacity", CarryCapacityScales);
            yield return new Scenario("job_robustness.manual_hunt_reports_all_grounds_busy", AllGroundsBusy);
            yield return new Scenario("job_robustness.water_jobs_use_distinct_edges", WaterEdgesDistinct);
            yield return new Scenario("job_robustness.water_jobs_respect_avoid_zones", WaterAvoidsZones);
            yield return new Scenario("job_robustness.only_working_cats_take_orders", OnlyWorkingCatsTakeOrders);
            yield return new Scenario("job_robustness.upgrade_levels_normalised_and_current", UpgradeLevelsCurrent);
            yield return new Scenario("job_robustness.demo_tools_auto_equip_and_scale_with_quality", DemoToolsAutoEquip);
            yield return new Scenario("job_robustness.hunt_keeps_regrowth_and_never_lowers_deposit", HuntKeepsRegrowth);
        }

        static void Free(Cat c)
        {
            c.ControlledBy = "";
            c.ControlLeaseUntil = 0;
            c.BuildingId = "";
        }

        static Tile Food(World w, Village v, Int2 at, double amount)
        {
            var t = w.TileAt(at);
            t.Wall = t.Water = t.Mountain = false;
            t.Resource = "food";
            t.Amount = amount;
            t.RegrowAt = -1;
            if (!v.Known.Contains(at))
                v.Known.Add(at);
            return t;
        }

        static BoundaryEdge[] Enclose(Village v, Int2 at)
        {
            var edges = new[] { new Int2(at.X, at.Z + 1), new Int2(at.X + 1, at.Z), new Int2(at.X, at.Z - 1), new Int2(at.X - 1, at.Z) }.Select(p => new BoundaryEdge { From = at, To = p }).ToArray();
            v.BoundaryEdges.AddRange(edges);
            return edges;
        }

        static void Open(Village v, BoundaryEdge[] edges)
        {
            foreach (var edge in edges)
                v.BoundaryEdges.Remove(edge);
        }

        static Job HeldJob(World w, Village v, Cat c, string kind, string phase)
        {
            var job = new Job { Id = w.Id("held-" + kind), Kind = kind, Phase = phase, CatId = c.Id, Position = c.Position, RequiredWork = 1000 };
            v.Jobs.Add(job);
            c.JobId = job.Id;
            return job;
        }

        static Job Started(World w, Village v, GameAction action)
        {
            string id = Act(w, v, action).EntityId;
            return v.Jobs.Single(j => j.Id == id);
        }

        static void StepUntil(World w, Func<bool> done, double seconds, double step = 0.05)
        {
            for (int tick = 0; tick < seconds / step && !done(); tick++)
                w.Step(step);
        }

        static void BlockedRouteReleases()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var near = new Int2(1, 5);
            var far = new Int2(5, -4);
            Food(w, v, near, 100);
            Food(w, v, far, 100);
            Check(w.Path(c.Position, near, v) != null && w.Path(c.Position, far, v) != null, "Fixture food tiles must start reachable");
            var first = Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            var job = v.Jobs.Single(j => j.Id == first.EntityId);
            Check(job.Position.Equals(near), "Fixture hunt did not pick the nearest food tile");
            var edges = Enclose(v, near);
            double releasedAt = -1;
            for (int tick = 0; tick < 30 / 0.05 && releasedAt < 0; tick++)
            {
                w.Step(0.05);
                if (job.Completed)
                    releasedAt = w.TimeSeconds;
            }
            Check(releasedAt > 0 && releasedAt <= 30 + 1e-9, "Hunt with a blocked route kept its cat for 30 s; reason=" + c.BlockedReason + " phase=" + job.Phase);
            Check(c.JobId != job.Id && w.Reservations.All(r => r.OwnerId != job.Id), "Released blocked job kept its cat or claims");
            Open(v, edges);
            var retry = Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            var next = v.Jobs.Single(j => j.Id == retry.EntityId);
            Check(!next.Position.Equals(near), "The next hunt went back to the source whose route had just been blocked");
            Valid(w);
        }

        static void BlockedReasonClears()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            var job = HeldJob(w, v, c, "haul", "output_delivery");
            c.Cargo.Add(new Stack("logs", 8));
            w.Step(2);
            Check(!job.Completed && job.BlockedReason == "output_storage_full_or_unreachable", "Fixture delivery did not block on missing storage: " + job.BlockedReason);
            var opened = new Stockpile { Id = w.Id("opened-store"), Position = new Int2(5, 2), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(opened);
            double x = c.X, z = c.Z;
            StepUntil(w, () => c.X != x || c.Z != z, 3);
            Check(c.X != x || c.Z != z, "Delivery did not resume once storage existed");
            Check(job.BlockedReason == "" && c.BlockedReason == "", "Blocked reason outlived its block: job=" + job.BlockedReason + " cat=" + c.BlockedReason);
            StepUntil(w, () => job.Completed, 20);
            Check(job.Completed, "Resumed delivery did not finish");
            Near(World.Amount(opened.Goods, "logs"), 8, "Resumed delivery lost cargo");
            Valid(w);
        }

        static void HuntOneTrip()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var store = v.Stockpiles[0];
            Food(w, v, new Int2(1, 5), 100);
            var result = Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            int pickups = 0;
            bool carrying = false;
            for (int tick = 0; tick < 600 / 0.05 && !job.Completed; tick++)
            {
                w.Step(0.05);
                bool now = !job.Completed && job.Phase == "output_delivery" && c.Cargo.Count > 0;
                if (now && !carrying)
                    pickups++;
                carrying = now;
            }
            Check(job.Completed, "Hunt never finished");
            Check(pickups == 1, "Hunt output took " + pickups + " trips instead of one");
            Check(World.Amount(store.Goods, "hide") > 0 && World.Amount(store.Goods, "bone") > 0, "Hunt did not deliver hide and bone");
            // The 1000 food in store spoils slightly while the hunt runs.
            Check(World.Amount(store.Goods, "food") > 1007, "Hunt did not deliver its food: " + World.Amount(store.Goods, "food"));
            Valid(w);
        }

        static void HuntKeepsRegrowth()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var tile = Food(w, v, new Int2(1, 5), 200);
            var result = Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            StepUntil(w, () => job.Completed, 600, 0.5);
            Check(job.Completed && tile.Amount < 200 && tile.Amount > 0, "Fixture hunt did not take part of the deposit");
            Check(tile.RegrowAt < 0, "A hunt on a deposit that still holds food scheduled a regrowth reset");
            double left = tile.Amount;
            tile.RegrowAt = w.TimeSeconds + 1;
            w.Step(121);
            Check(tile.Amount + 1e-9 >= left, "Regrowth lowered a deposit from " + left + " to " + tile.Amount);
        }

        static void SpillsMergeAndVanish()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var store = v.Stockpiles[0];
            var job = HeldJob(w, v, c, "haul", "output_delivery");
            var item = new Item { Id = w.Id("mug"), Kind = "mug", Material = "clay", Quality = 2, Condition = 40, MaxCondition = 100, VillageId = v.Id, LocationId = job.Id };
            v.Items.Add(item);
            job.ItemIds.Add(item.Id);
            c.Cargo.Add(new Stack("logs", 4));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            var spills = v.Stockpiles.Where(p => p.Kind == "spill" && p.Position.Equals(c.Position)).ToList();
            Check(spills.Count == 1, "Interrupted cargo and item made " + spills.Count + " spill piles on one tile");
            var spill = spills[0];
            Check(item.LocationId == spill.Id && World.Amount(spill.Goods, "logs") == 4, "Shared spill lost cargo or item");
            var scalar = Started(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = spill.Id });
            StepUntil(w, () => scalar.Completed, 60);
            Check(scalar.Completed && v.Stockpiles.Contains(spill) && item.LocationId == spill.Id, "Spill vanished while an item still lay there");
            var exact = Started(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = spill.Id });
            StepUntil(w, () => exact.Completed, 60);
            Check(exact.Completed && item.LocationId == store.Id, "Item haul from the spill did not finish");
            Check(!v.Stockpiles.Contains(spill), "Empty spill pile stayed in the world");
            Near(World.Amount(store.Goods, "logs"), 4, "Spill cargo did not reach storage");
            Valid(w);
        }

        static void PartialDelivery()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            var first = new Stockpile { Id = w.Id("small-store"), Position = new Int2(5, 2), Width = 1, Depth = 1, Capacity = 5, Accepts = new List<string> { "logs" } };
            var second = new Stockpile { Id = w.Id("small-store"), Position = new Int2(5, 4), Width = 1, Depth = 1, Capacity = 5, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(first);
            v.Stockpiles.Add(second);
            var job = HeldJob(w, v, c, "haul", "output_delivery");
            c.Cargo.Add(new Stack("logs", 8));
            StepUntil(w, () => job.Completed, 60);
            Check(job.Completed && c.Cargo.Count == 0, "A load larger than any one pile stayed undelivered: " + job.BlockedReason);
            Check(World.Amount(first.Goods, "logs") > 0 && World.Amount(second.Goods, "logs") > 0, "Partial delivery did not use both piles");
            Near(World.Amount(first.Goods, "logs") + World.Amount(second.Goods, "logs"), 8, "Partial delivery lost or duplicated cargo");
            Valid(w);
        }

        static void CancelledProductionReturnsInputs()
        {
            var w = Fixture(out var v, out var c);
            var station = Station(w, v, "wood_cutter");
            World.Add(v.Stockpiles[0].Goods, "logs", 100);
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" });
            Job job = null;
            StepUntil(w, () => (job = v.Jobs.FirstOrDefault(j => !j.Completed && j.Kind == "production" && j.CatId == c.Id)) != null && job.Phase == "working" && World.Amount(job.Local, "logs") >= 5, 200);
            Check(job != null && job.Phase == "working", "Fixture production never gathered its inputs");
            double logs = Goods(w, v, "logs");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Check(job.Completed && c.JobId == "", "Release did not cancel the production job");
            Near(World.Amount(station.Inputs, "logs"), 5, "Cancelled production did not return its inputs to the station");
            Check(!v.Stockpiles.Any(p => p.Kind == "spill" && World.Amount(p.Goods, "logs") > 0), "Cancelled production spilled its inputs");
            Near(Goods(w, v, "logs"), logs, "Cancelling production lost or duplicated inputs");
            Valid(w);
        }

        static void ClaimsPreferNearestReachable()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var near = v.Stockpiles[0];
            World.Add(near.Goods, "herbs", 10);
            var far = new Stockpile { Id = "aaa-far-herbs", Position = new Int2(-6, -6), Width = 1, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack("herbs", 10) } };
            v.Stockpiles.Add(far);
            var first = Act(w, v, new GameAction { Kind = "RequestJob", Name = "ritual", CatId = c.Id });
            var claims = w.Reservations.Where(r => r.OwnerId == first.EntityId).ToList();
            Check(claims.Count > 0 && claims.All(r => r.PileId == near.Id), "Claim skipped the nearest pile: " + string.Join(",", claims.Select(r => r.PileId)));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Check(w.Reservations.Count == 0, "Released ritual kept its claims");
            Enclose(v, near.Position);
            var second = Act(w, v, new GameAction { Kind = "RequestJob", Name = "ritual", CatId = c.Id });
            claims = w.Reservations.Where(r => r.OwnerId == second.EntityId).ToList();
            Check(claims.Count > 0 && claims.All(r => r.PileId == far.Id), "Claim went to an unreachable pile: " + string.Join(",", claims.Select(r => r.PileId)));
            Valid(w);
        }

        static void UnreachableClaimReleased()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var blocked = new Stockpile { Id = w.Id("herb-pile"), Position = new Int2(5, -3), Width = 1, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack("herbs", 10) } };
            var open = new Stockpile { Id = w.Id("herb-pile"), Position = new Int2(-3, -6), Width = 1, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack("herbs", 10) } };
            v.Stockpiles.Add(blocked);
            v.Stockpiles.Add(open);
            var first = Act(w, v, new GameAction { Kind = "RequestJob", Name = "ritual", CatId = c.Id });
            var job = v.Jobs.Single(j => j.Id == first.EntityId);
            Check(w.Reservations.Where(r => r.OwnerId == job.Id).All(r => r.PileId == blocked.Id), "Fixture did not claim the nearer herb pile");
            var edges = Enclose(v, blocked.Position);
            StepUntil(w, () => job.Completed || !w.Reservations.Any(r => r.PileId == blocked.Id), 30);
            Check(!w.Reservations.Any(r => r.PileId == blocked.Id), "A claim on an unreachable pile was held for 30 s");
            Check(job.Completed && c.JobId != job.Id, "The wedged ritual kept its cat");
            Open(v, edges);
            var second = Act(w, v, new GameAction { Kind = "RequestJob", Name = "ritual", CatId = c.Id });
            var claims = w.Reservations.Where(r => r.OwnerId == second.EntityId).ToList();
            Check(claims.Count > 0 && claims.All(r => r.PileId == open.Id), "New claim went straight back to the excluded pile");
            Valid(w);
        }

        static void ScoutSkipsUnreachableWaypoints()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var wall = new Int2(c.Position.X + 2, c.Position.Z + 2);
            w.TileAt(wall).Wall = true;
            var result = Act(w, v, new GameAction { Kind = "DispatchScout", CatId = c.Id, Resource = "moonstone" });
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            Check(job.Path.Count > 2 && job.Path[1].Equals(wall), "Fixture survey does not route through the walled waypoint");
            StepUntil(w, () => job.PathIndex > 1 || job.Completed, 30);
            Check(job.PathIndex > 1, "Scout stood at an unreachable waypoint; index=" + job.PathIndex + " reason=" + c.BlockedReason);
            Valid(w);
        }

        static void CarryCapacityScales()
        {
            var w = StudyFixture("logistics_basics", true, out var v, out var c);
            double factor = Catalog.Effect(v, "haulCapacity", 1);
            Check(factor > 1, "Fixture owns no haul capacity study");
            Near(w.CarryCapacity(v), 8 * factor, "Carry capacity does not scale as 8 x haulCapacity");
            Near(w.CarryCapacity(v, "logs"), 8 * factor, "Per-resource carry capacity does not scale as 8 x haulCapacity");
        }

        static void AllGroundsBusy()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            Food(w, v, new Int2(1, 5), 100);
            var holder = v.Cats[1];
            int grounds = 0;
            foreach (var at in v.Known.Where(p => w.TileAt(p).Resource == "food" && w.TileAt(p).Amount > 0 || World.Amount(w.TileAt(p).Deposits, "food") > 0).ToArray())
            {
                v.Jobs.Add(new Job { Id = w.Id("held-hunt"), Kind = "hunt", Phase = "working", CatId = holder.Id, Position = at, RequiredWork = 1000 });
                grounds++;
            }
            Check(grounds > 0, "Fixture has no known hunting ground");
            var result = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            Check(!result.Success && result.Code == ResultCode.AllGroundsBusy, "Busy hunting grounds reported code=" + result.Code + " error=" + result.Error);
            Check(c.JobId == "", "Refused hunt still assigned the cat");
        }

        static void WaterEdgesDistinct()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var other = v.Cats[1];
            Free(other);
            var first = Started(w, v, new GameAction { Kind = "RequestJob", Name = "water", CatId = c.Id });
            var second = Started(w, v, new GameAction { Kind = "RequestJob", Name = "water", CatId = other.Id });
            Check(!first.Position.Equals(second.Position), "Two water carriers were sent to one water edge");
            Valid(w);
        }

        static void WaterAvoidsZones()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            var first = Started(w, v, new GameAction { Kind = "RequestJob", Name = "water", CatId = c.Id });
            var edge = first.Position;
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = edge, End = edge });
            Check(!w.Walkable(v, edge), "Fixture avoid zone does not cover the first water edge");
            var second = Started(w, v, new GameAction { Kind = "RequestJob", Name = "water", CatId = c.Id });
            Check(!second.Position.Equals(edge), "Water carrier was sent into an avoid zone");
            Valid(w);
        }

        static void OnlyWorkingCatsTakeOrders()
        {
            var w = Fixture(out var v, out var c);
            Free(c);
            Food(w, v, new Int2(1, 5), 100);
            void Refused(string who)
            {
                var manual = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
                Check(!manual.Success && c.JobId == "", who + " took a manual order");
                var automatic = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "hunt" });
                Check(!automatic.Success && c.JobId == "", who + " was picked for an automatic order");
            }
            c.AgeHours = 2;
            Refused("A kitten");
            c.AgeHours = 24;
            c.Migration = "arriving";
            Refused("An arriving migrant");
            c.Migration = "departing";
            Refused("A departing migrant");
            c.Migration = "resident";
            c.Goal = GoalKey.NeedDrink;
            Refused("A cat on a need trip");
            c.Goal = GoalKey.Idle;
            c.Thirst = 10;
            c.Cargo.Add(new Stack("water", 1));
            Refused("A thirsty cat carrying its water");
            c.Cargo.Clear();
            c.Thirst = 100;
            var result = Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            Check(c.JobId == result.EntityId, "A resident adult with no need refused an order");
            Valid(w);
        }

        static void UpgradeLevelsCurrent()
        {
            var w = Fixture(out var v, out var c);
            v.UpgradeLevels.Add(new Stack("huntMastery", 2));
            Near(w.UpgradeLevel(v, "hunt_mastery"), 2, "Legacy camelCase upgrade key was not counted");
            Near(w.UpgradeLevel(v, "huntMastery"), 2, "Camel-case lookup missed the normalised level");
            v.Blessings = 1000;
            Act(w, v, new GameAction { Kind = "PurchaseUpgrade", Name = "hunt_mastery" });
            Near(w.UpgradeLevel(v, "hunt_mastery"), 3, "Purchased level not visible in the same step");
            Check(v.UpgradeLevels.Count == 1 && v.UpgradeLevels[0].Resource == "hunt_mastery", "Purchase did not store one normalised upgrade key");
            v.UpgradeLevels[0].Amount = 5;
            Near(w.UpgradeLevel(v, "hunt_mastery"), 5, "Changed level served from a stale cache");
            v.UpgradeLevels.Clear();
            Near(w.UpgradeLevel(v, "hunt_mastery"), 0, "Cleared levels served from a stale cache");
        }

        static void DemoToolsAutoEquip()
        {
            foreach (bool demo in new[] { true, false })
            {
                var w = Fixture(out var v, out var c);
                if (demo)
                    w.PacingProfile = Pacing.DemoProfile;
                Free(c);
                Food(w, v, new Int2(1, 5), 100);
                var store = v.Stockpiles[0];
                var wood = new Item { Id = w.Id("tool"), Kind = "tool", Material = "wood", Quality = 1, VillageId = v.Id, LocationId = store.Id };
                var metal = new Item { Id = w.Id("tool"), Kind = "tool", Material = "metal", Quality = 3, VillageId = v.Id, LocationId = store.Id };
                v.Items.Add(wood);
                v.Items.Add(metal);
                Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
                if (!demo)
                {
                    Check(c.Equipment.Count == 0 && metal.LocationId == store.Id, "Classic worker auto-equipped a tool");
                    Near(w.ToolWorkBonus(v, c), 1, "Unequipped classic worker got a tool bonus");
                    Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = metal.Id });
                    Near(w.ToolWorkBonus(v, c), w.CurrentPacing().WorkToolBonus, "Classic tool bonus is no longer flat");
                    continue;
                }
                Check(c.Equipment.Contains(metal.Id) && metal.LocationId == c.Id && wood.LocationId == store.Id, "Demo worker did not equip the best free tool");
                Check(w.ToolWorkBonus(v, c) > w.CurrentPacing().WorkToolBonus + 0.1, "A quality-3 metal tool works no better than a basic tool: " + w.ToolWorkBonus(v, c));
                Act(w, v, new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = metal.Id });
                Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = wood.Id });
                Near(w.ToolWorkBonus(v, c), w.CurrentPacing().WorkToolBonus, "A basic wood tool lost the classic bonus");
                Valid(w);
            }
        }
    }
}
