using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class EquipmentRetrievalScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.hourly_equipment_order_reserves_exact_source_without_remote_benefit", PhysicalOrder);
            yield return new Scenario("regression.hourly_equipment_pickup_reaches_exact_source_without_wear", Pickup);
            yield return new Scenario("regression.hourly_equipment_claim_prevents_double_equip_haul_and_removal", ExclusiveClaim);
            yield return new Scenario("regression.hourly_equipment_blocked_preflight_preserves_idle_state", BlockedPreflight);
            yield return new Scenario("regression.hourly_equipment_later_blockage_preserves_source_claim", BlockedAfterOrder);
            yield return new Scenario("regression.hourly_equipment_fractional_boundary_cannot_pick_up_remotely", FractionalBoundary);
            yield return new Scenario("regression.hourly_equipment_needs_preserve_exact_claim_and_resume", NeedsResume);
            yield return new Scenario("regression.hourly_equipment_cancel_during_needs_preserves_need_cargo", CancelDuringNeeds);
            yield return new Scenario("regression.hourly_equipment_cancel_checks_exact_job_and_cat", ExactCancellation);
            yield return new Scenario("regression.hourly_equipment_source_disappearance_releases_at_source", MissingSource);
            yield return new Scenario("regression.hourly_equipment_control_releases_source_claim", DirectControl);
            yield return new Scenario("regression.hourly_equipment_resume_only_control_releases_source_claim", ResumeOnlyControl);
            yield return new Scenario("regression.hourly_equipment_death_preserves_source_and_equipped_identity", Death);
            foreach (var policy in new[] { "job", "building", "cargo", "need", "resume", "control", "kitten", "arriving", "departing", "underground", "accounting" })
            {
                string state = policy;
                yield return new Scenario("regression.hourly_equipment_refuses_" + state + "_without_stealing_ownership", () => Refusal(state));
            }
            yield return new Scenario("regression.hourly_equipment_return_keeps_bearer_until_storage_arrival", PhysicalReturn);
            yield return new Scenario("regression.hourly_equipment_return_cancel_keeps_exact_equipped_item", CancelReturn);
            yield return new Scenario("regression.hourly_equipment_return_full_storage_holds_bearer", FullReturn);
            yield return new Scenario("regression.hourly_equipment_return_removed_storage_holds_bearer", MissingReturnStorage);
            yield return new Scenario("regression.hourly_equipment_validate_rejects_duplicate_and_orphan_claims", ValidateClaims);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, string kind, Cat cat, string target) => world.Apply(new PlayerContext { PlayerId = "equipment-player", VillageId = village.Id }, new GameAction { Kind = kind, CatId = cat.Id, TargetId = target });
        static World Fixture(out Village village, out Cat cat, out Stockpile source, out Item item)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.LastLeaderResearch = 0;
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            foreach (var pile in village.Stockpiles.Where(pile => pile.Kind == "storage"))
                pile.Accepts = new List<string> { "food", "water" };
            foreach (var other in village.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
            }
            cat = village.Cats[1];
            cat.ControlledBy = "";
            cat.Skills.Clear();
            cat.Position = new Int2(-5, 1); cat.X = -5; cat.Z = 1;
            source = new Stockpile { Id = world.Id("equipment-source"), Position = new Int2(5, 1), Width = 1, Depth = 1, Capacity = 2, Accepts = new List<string> { "weapons", "tools", "armor" } };
            village.Stockpiles.Add(source);
            item = new Item { Id = world.Id("exact-weapon"), Kind = "weapon", Material = "metal", Quality = 3, Condition = 37.5, MaxCondition = 73, VillageId = village.Id, LocationId = source.Id };
            village.Items.Add(item);
            Check(world.Path(cat.Position, source.Position, village) != null, "Equipment fixture source is not reachable");
            return world;
        }
        static Job Order(World world, Village village, Cat cat, Item item, string kind = "EquipItem")
        {
            var result = Act(world, village, kind, cat, item.Id);
            Check(result.Success, kind + " refused: " + result.Error);
            var job = village.Jobs.Find(j => !j.Completed && j.Id == result.EntityId);
            Check(job != null, kind + " must create an authoritative physical order");
            return job;
        }
        static void UntilComplete(World world, Job job)
        {
            for (int second = 0; second < 120 && !job.Completed; second++) world.Step(1);
            Check(job.Completed, "Equipment order did not complete: " + job.BlockedReason);
        }
        static void EquipFixture(Cat cat, Item item) { item.LocationId = cat.Id; cat.Equipment.Add(item.Id); }
        static void Enclose(Village village, Int2 point)
        {
            foreach (var d in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = point, To = new Int2(point.X + d.X, point.Z + d.Z) });
        }
        static void PhysicalOrder()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var result = Act(world, village, "EquipItem", cat, item.Id);
            Check(result.Success && !cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") == 1, "EquipItem granted equipment remotely before physical pickup");
            var job = village.Jobs.Single(j => !j.Completed && j.CatId == cat.Id);
            Check(result.EntityId == job.Id && job.Kind == "equip" && job.Phase == "item_fetch" && job.SourceId == source.Id && job.TargetId == item.Id && job.Position.Equals(source.Position) && job.Manual && job.AutomatedBy == "" && job.ItemIds.SequenceEqual(new[] { item.Id }) && item.LocationId == job.Id, "Equipment source reservation lost exact item/order identity");
            var repeat = Act(world, village, "EquipItem", cat, item.Id);
            Check(repeat.Success && repeat.EntityId == job.Id && village.Jobs.Count(j => !j.Completed && j.Kind == "equip") == 1, "Repeated exact order duplicated work or refused its existing identity");
            world.Step(World.SimulationStepSeconds);
            Check(cat.X > -5 && cat.X < 5 && item.LocationId == job.Id && cat.Equipment.Count == 0, "Equipment retrieval did not begin physical travel");
        }
        static void Pickup()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var tool = new Item { Id = world.Id("existing-tool"), Kind = "tool", VillageId = village.Id, LocationId = cat.Id, Condition = 42 };
            village.Items.Add(tool); cat.Equipment.Add(tool.Id);
            var job = Order(world, village, cat, item);
            UntilComplete(world, job);
            Check(cat.Position.Equals(source.Position) && cat.X == source.Position.X && cat.Z == source.Position.Z && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id), "Pickup happened away from its physical exact source");
            Check(item.Condition == 37.5 && item.MaxCondition == 73 && tool.Condition == 42 && cat.Skills.Count == 0 && world.EquipmentPower(village, cat, "weapon") > 1 && cat.JobId == "" && job.ItemIds.Count == 0, "Equipment completion changed condition, XP or retained ownership claims");
            Check(world.Validate().Count == 0, string.Join(";", world.Validate()));
        }
        static void ExclusiveClaim()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            var other = village.Cats[2]; other.ControlledBy = "";
            Check(!Act(world, village, "EquipItem", other, item.Id).Success && !Act(world, village, "HaulGatherSpot", other, source.Id).Success && !Act(world, village, "RemoveStockpile", cat, source.Id).Success && item.LocationId == job.Id && other.JobId == "", "A pending exact equipment claim was stolen, hauled or removed");
            source.Capacity = 1;
            Check(!world.HasRoom(village, source, "weapons", 1), "Reserved equipment no longer counts at its physical source");
        }
        static void BlockedPreflight()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            Enclose(village, source.Position);
            Check(!Act(world, village, "EquipItem", cat, item.Id).Success && item.LocationId == source.Id && cat.JobId == "" && cat.Equipment.Count == 0, "Unreachable equipment dispatch changed physical ownership");
        }
        static void BlockedAfterOrder()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            Enclose(village, source.Position);
            world.Step(1);
            Check(!job.Completed && item.LocationId == job.Id && cat.Equipment.Count == 0 && job.BlockedReason == "blocked_route", "A later blocked route remotely equipped or lost the exact claim");
            village.BoundaryEdges.Clear(); UntilComplete(world, job);
            Check(item.LocationId == cat.Id, "Reopened route failed to resume the exact order");
        }
        static void FractionalBoundary()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            source.Position = new Int2(-4, 1);
            cat.X = -4.1;
            var job = Order(world, village, cat, item);
            village.BoundaryEdges.Add(new BoundaryEdge { From = cat.Position, To = source.Position });
            Enclose(village, source.Position);
            world.Step(World.SimulationStepSeconds);
            Check(item.LocationId == job.Id && !cat.Equipment.Contains(item.Id) && !job.Completed, "Fractional proximity bypassed a closed equipment interaction edge");
        }
        static void NeedsResume()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            cat.Thirst = 34;
            world.Step(1);
            Check(cat.Goal == "need_drink" && cat.ResumeJobId == job.Id && item.LocationId == job.Id && !job.Completed, "Needs lost the exact pending equipment order");
            UntilComplete(world, job);
            Check(cat.Thirst > 34 && item.LocationId == cat.Id && cat.ResumeJobId == "", "Need completion failed to return to physical equipment retrieval");
        }
        static void CancelDuringNeeds()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            cat.Goal = "need_drink"; cat.ResumeJobId = job.Id; cat.Cargo.Add(new Stack("water", 1));
            cat.Path.Add(village.Center);
            Check(Act(world, village, "CancelEquipment", cat, job.Id).Success && job.Completed && item.LocationId == source.Id && cat.JobId == "" && cat.ResumeJobId == "" && cat.Goal == "need_drink" && World.Amount(cat.Cargo, "water") == 1 && cat.Path.Count == 1, "Cancelling equipment stole need cargo, path or resumption ownership");
        }
        static void ExactCancellation()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            Check(!Act(world, village, "CancelEquipment", cat, "stale-job").Success && !Act(world, village, "CancelEquipment", village.Cats[2], job.Id).Success && !job.Completed, "Stale or foreign equipment cancellation changed work");
            Check(Act(world, village, "CancelEquipment", cat, job.Id).Success && job.Completed && item.LocationId == source.Id && cat.JobId == "", "Exact cancellation did not restore the uncollected source item");
            var replacement = new Job { Id = world.Id("replacement"), Kind = "hunt", CatId = cat.Id, Position = cat.Position }; village.Jobs.Add(replacement); cat.JobId = replacement.Id;
            Check(!Act(world, village, "CancelEquipment", cat, job.Id).Success && cat.JobId == replacement.Id && !replacement.Completed, "Repeated equipment cancellation stole replacement work");
        }
        static void MissingSource()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            village.Stockpiles.Remove(source); cat.Goal = "need_drink"; cat.ResumeJobId = job.Id;
            world.Step(World.SimulationStepSeconds);
            var spill = village.Stockpiles.SingleOrDefault(p => p.Id == item.LocationId);
            Check(job.Completed && spill != null && spill.Kind == "spill" && spill.Position.Equals(source.Position) && cat.Equipment.Count == 0 && cat.ResumeJobId == "", "Missing source left an orphan claim or teleported the uncollected item to its cat");
        }
        static void DirectControl()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            Check(Act(world, village, "EnterCatControl", cat, "").Success && job.Completed && item.LocationId == source.Id && cat.Equipment.Count == 0 && cat.JobId == "", "Direct control stole or orphaned the uncollected equipment");
        }
        static void ResumeOnlyControl()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            cat.JobId = ""; cat.ResumeJobId = job.Id; cat.Goal = "need_drink"; cat.Cargo.Add(new Stack("water", 1));
            Check(world.Validate().Count == 0, "Legal needs-only equipment ownership fixture is invalid");
            Check(Act(world, village, "EnterCatControl", cat, "").Success && job.Completed && item.LocationId == source.Id && cat.ResumeJobId == "" && World.Amount(cat.Cargo, "water") == 1 && world.Validate().Count == 0, "Needs-only control handoff left an active orphan equipment claim before the next tick");
        }
        static void Death()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var equipped = new Item { Id = world.Id("equipped-armor"), Kind = "armor", VillageId = village.Id, LocationId = cat.Id }; village.Items.Add(equipped); cat.Equipment.Add(equipped.Id);
            var job = Order(world, village, cat, item); cat.Health = 0;
            world.Step(World.SimulationStepSeconds);
            Check(!cat.Alive && job.Completed && item.LocationId == source.Id && village.Stockpiles.Any(p => p.Id == equipped.LocationId && p.Position.Equals(cat.Position)) && cat.Equipment.Count == 0 && village.Items.Count == 2, "Death lost, duplicated or teleported equipment ownership");
        }
        static void Refusal(string state)
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            switch (state)
            {
                case "job": var job = new Job { Id = world.Id("busy"), Kind = "hunt", CatId = cat.Id }; village.Jobs.Add(job); cat.JobId = job.Id; break;
                case "building": cat.BuildingId = "busy-station"; break;
                case "cargo": cat.Cargo.Add(new Stack("logs", 2)); break;
                case "need": cat.Goal = "need_drink"; cat.Cargo.Add(new Stack("water", 1)); break;
                case "resume": cat.ResumeJobId = "suspended"; break;
                case "control": cat.ControlledBy = "equipment-player"; break;
                case "kitten": cat.AgeHours = 4; break;
                case "arriving": case "departing": cat.Migration = state; break;
                case "underground": cat.Position.Level = -1; break;
                case "accounting": village.Accounting = new AccountingRound { WorkerId = cat.Id }; break;
            }
            string oldJob = cat.JobId, oldResume = cat.ResumeJobId, oldBuilding = cat.BuildingId, oldGoal = cat.Goal; int oldJobs = village.Jobs.Count; double cargo = cat.Cargo.Sum(s => s.Amount);
            Check(!Act(world, village, "EquipItem", cat, item.Id).Success && item.LocationId == source.Id && cat.Equipment.Count == 0 && cat.JobId == oldJob && cat.ResumeJobId == oldResume && cat.BuildingId == oldBuilding && cat.Goal == oldGoal && village.Jobs.Count == oldJobs && cat.Cargo.Sum(s => s.Amount) == cargo, "Equipment dispatch stole " + state + " ownership or bypassed bearer policy");
        }
        static void PhysicalReturn()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item); EquipFixture(cat, item);
            var job = Order(world, village, cat, item, "UnequipItem");
            Check(job.Kind == "unequip" && job.Phase == "item_return" && job.TargetId == source.Id && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && world.EquipmentPower(village, cat, "weapon") > 1, "UnequipItem removed equipment remotely before physical storage arrival");
            UntilComplete(world, job);
            Check(cat.Position.Equals(source.Position) && item.LocationId == source.Id && !cat.Equipment.Contains(item.Id) && item.Condition == 37.5 && item.MaxCondition == 73 && cat.Skills.Count == 0 && job.ItemIds.Count == 0 && world.Validate().Count == 0, "Physical return lost identity, condition, ownership or exact accepting storage");
        }
        static void CancelReturn()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item); EquipFixture(cat, item);
            var job = Order(world, village, cat, item, "UnequipItem");
            Check(Act(world, village, "CancelEquipment", cat, job.Id).Success && job.Completed && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && cat.JobId == "", "Return cancellation dropped or teleported the still-equipped item");
        }
        static void FullReturn()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item); EquipFixture(cat, item);
            var job = Order(world, village, cat, item, "UnequipItem");
            source.Capacity = 0; world.Step(1);
            Check(!job.Completed && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && job.BlockedReason == "storage_full", "Return overflowed storage or unequipped into no physical destination");
            source.Capacity = 2; UntilComplete(world, job); Check(item.LocationId == source.Id, "Return did not recover after storage capacity reopened");
        }
        static void MissingReturnStorage()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item); EquipFixture(cat, item);
            var job = Order(world, village, cat, item, "UnequipItem"); village.Stockpiles.Remove(source); world.Step(1);
            Check(!job.Completed && item.LocationId == cat.Id && cat.Equipment.Contains(item.Id) && job.BlockedReason == "storage_missing", "Removed return destination lost the still-equipped item");
        }
        static void ValidateClaims()
        {
            var world = Fixture(out var village, out var cat, out var source, out var item);
            var job = Order(world, village, cat, item);
            Check(world.Validate().Count == 0, "Valid equipment claim was rejected");
            var duplicate = new Job { Id = world.Id("duplicate"), Kind = "equip", Phase = "item_fetch", SourceId = source.Id, TargetId = item.Id, Position = source.Position, CatId = village.Cats[2].Id, Manual = true, ItemIds = new List<string> { item.Id } };
            village.Cats[2].JobId = duplicate.Id; village.Jobs.Add(duplicate);
            Check(world.Validate().Any(e => e.Contains("equipment", StringComparison.OrdinalIgnoreCase)), "Authority validation accepted duplicate exact equipment claims");
            village.Jobs.Remove(duplicate); village.Cats[2].JobId = ""; cat.JobId = "";
            Check(world.Validate().Any(e => e.Contains("equipment", StringComparison.OrdinalIgnoreCase)), "Authority validation accepted an ownerless active equipment order");
        }
    }
}
