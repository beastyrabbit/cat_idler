using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static bool IsEquipmentJob(Job job) => job.Kind == "equip" || job.Kind == "unequip";

        private bool AvailableEquipmentBearer(Village village, Cat cat) => cat != null && cat.Alive && cat.Health > 0 && cat.AgeHours >= 12
            && FreeWorker(village, cat) && cat.ResumeJobId == "" && cat.Cargo.Count == 0 && !cat.Goal.StartsWith("need_", StringComparison.Ordinal)
            && cat.Migration != "arriving" && cat.Migration != "departing" && village.Accounting?.WorkerId != cat.Id
            && !village.Jobs.Any(job => !job.Completed && job.CatId == cat.Id);

        private ActionResult OrderEquipment(Village village, Cat cat, Item item, bool equip)
        {
            string kind = equip ? "equip" : "unequip";
            var existing = village.Jobs.Find(job => !job.Completed && job.Kind == kind && job.CatId == cat?.Id && job.ItemIds.Contains(item.Id)
                && (cat.JobId == job.Id || cat.ResumeJobId == job.Id));
            if (existing != null)
                return ActionResult.Ok(existing.Id);
            if (!AvailableEquipmentBearer(village, cat))
                return ActionResult.Fail("Choose an available adult on the surface without work, cargo, control or urgent needs");
            if (!new[] { "tool", "weapon", "armor" }.Contains(item.Kind) || item.VillageId != village.Id)
                return ActionResult.Fail("Unavailable equipment item");
            if (equip)
            {
                var source = village.Stockpiles.Find(pile => pile.Id == item.LocationId);
                if (item.Condition <= 0 || source == null || village.Items.Any(other => other.LocationId == cat.Id && other.Kind == item.Kind)
                    || village.Jobs.Any(job => !job.Completed && job.ItemIds.Contains(item.Id)))
                    return ActionResult.Fail("Unavailable or occupied equipment slot");
                if (!StockpileAccess(village, source, cat.Position).HasValue)
                    return ActionResult.Fail("Equipment source is unreachable");
                var job = new Job { Kind = kind, Phase = "item_fetch", SourceId = source.Id, TargetId = item.Id, Position = source.Position, Resource = ItemResource(item.Kind), Amount = 1, RequiredWork = 0, Manual = true, ItemIds = new List<string> { item.Id } };
                StartJob(village, cat, job);
                // The existing item_fetch convention reserves its identity while it remains physically at SourceId.
                item.LocationId = job.Id;
                return ActionResult.Ok(job.Id);
            }
            if (item.LocationId != cat.Id || !cat.Equipment.Contains(item.Id) || village.Jobs.Any(job => !job.Completed && job.ItemIds.Contains(item.Id)))
                return ActionResult.Fail("Item not equipped by this cat");
            var storage = DeliveryStorage(village, ItemResource(item.Kind), 1, cat.Position, out _);
            if (storage == null)
                return ActionResult.Fail("No reachable accepting equipment storage");
            var returning = new Job { Kind = kind, Phase = "item_return", SourceId = cat.Id, TargetId = storage.Id, Position = storage.Position, Resource = ItemResource(item.Kind), Amount = 1, RequiredWork = 0, Manual = true, ItemIds = new List<string> { item.Id } };
            StartJob(village, cat, returning);
            return ActionResult.Ok(returning.Id);
        }

        private void ReleaseEquipmentClaim(Village village, Job job, Item item)
        {
            // Returns keep the exact item equipped; only uncollected source claims need releasing.
            if (job.Kind != "equip" || item.LocationId != job.Id)
                return;
            var source = village.Stockpiles.Find(pile => pile.Id == job.SourceId);
            if (source == null)
            {
                source = village.Stockpiles.Find(pile => pile.Kind == "spill" && pile.Position.Equals(job.Position));
                if (source == null)
                {
                    source = new Stockpile { Id = Id("spill"), Kind = "spill", Position = job.Position, Width = 1, Depth = 1, Capacity = 1000000 };
                    village.Stockpiles.Add(source);
                }
            }
            item.LocationId = source.Id;
        }

        private void CompleteEquipmentOrder(Village village, Job job, bool releaseClaim)
        {
            if (releaseClaim)
                foreach (var item in village.Items.Where(item => job.ItemIds.Contains(item.Id)))
                    ReleaseEquipmentClaim(village, job, item);
            job.ItemIds.Clear();
            job.Completed = true;
            job.CompletionTime = TimeSeconds;
            job.BlockedReason = "";
            Release(job.Id);
            var cat = village.Cats.Find(cat => cat.Id == job.CatId);
            if (cat == null)
                return;
            bool attendingNeed = cat.Goal.StartsWith("need_", StringComparison.Ordinal);
            bool ownsCurrent = cat.JobId == job.Id;
            if (ownsCurrent)
                cat.JobId = "";
            if (cat.ResumeJobId == job.Id)
                cat.ResumeJobId = "";
            if (ownsCurrent && !attendingNeed)
            {
                cat.Path.Clear();
                cat.Goal = "idle";
                cat.BlockedReason = "";
            }
        }

        private ActionResult CancelEquipment(Village village, Cat cat, string jobId)
        {
            var job = village.Jobs.Find(job => job.Id == jobId && !job.Completed && IsEquipmentJob(job) && job.Manual && job.AutomatedBy == "");
            if (job == null || cat == null || job.CatId != cat.Id || cat.JobId != job.Id && (cat.JobId != "" || cat.ResumeJobId != job.Id))
                return ActionResult.Fail("The selected cat no longer owns that manual equipment order");
            CompleteEquipmentOrder(village, job, true);
            return ActionResult.Ok(job.Id);
        }

        private void CleanEquipmentOrders(Village village)
        {
            foreach (var job in village.Jobs.Where(job => !job.Completed && IsEquipmentJob(job)).ToArray())
            {
                var cat = village.Cats.Find(cat => cat.Id == job.CatId && cat.Alive && cat.Health > 0);
                var item = job.ItemIds.Count == 1 ? village.Items.Find(item => item.Id == job.ItemIds[0]) : null;
                bool ownerLost = cat == null || cat.JobId != job.Id && cat.ResumeJobId != job.Id;
                bool itemLost = item == null || (job.Kind == "equip" ? item.LocationId != job.Id || !village.Stockpiles.Any(pile => pile.Id == job.SourceId) : item.LocationId != cat?.Id || cat?.Equipment.Contains(item.Id) != true);
                if (ownerLost || itemLost)
                    CompleteEquipmentOrder(village, job, true);
            }
        }

        private void TickEquipment(Village village, Cat cat, Job job, double dt)
        {
            var item = job.ItemIds.Count == 1 ? village.Items.Find(item => item.Id == job.ItemIds[0]) : null;
            if (item == null)
            {
                CompleteEquipmentOrder(village, job, true);
                return;
            }
            cat.Goal = job.Kind + " · " + job.Phase;
            Stockpile destination;
            if (job.Kind == "equip")
            {
                destination = village.Stockpiles.Find(pile => pile.Id == job.SourceId);
                if (destination == null || item.LocationId != job.Id || item.Condition <= 0 || village.Items.Any(other => other.Id != item.Id && other.LocationId == cat.Id && other.Kind == item.Kind))
                {
                    CompleteEquipmentOrder(village, job, true);
                    return;
                }
            }
            else
            {
                destination = village.Stockpiles.Find(pile => pile.Id == job.TargetId && pile.Kind == "storage");
                if (destination == null || !HasRoom(village, destination, ItemResource(item.Kind), 1))
                {
                    job.BlockedReason = cat.BlockedReason = destination == null ? "storage_missing" : "storage_full";
                    return;
                }
            }
            job.Position = destination.Position;
            if (TimeSeconds + 1e-9 < job.NextStorageAttemptAt)
                return;
            var access = DeliveryAccess(village, cat, destination);
            if (!access.HasValue)
            {
                cat.Path.Clear();
                job.NextStorageAttemptAt = Math.Floor(TimeSeconds) + 1;
                job.BlockedReason = cat.BlockedReason = "blocked_route";
                return;
            }
            job.NextStorageAttemptAt = 0;
            if (!Move(cat, access.Value, dt))
            {
                job.BlockedReason = cat.BlockedReason;
                return;
            }
            // Reach a physical footprint tile, including any unfinished fractional edge.
            if (job.Kind == "equip")
            {
                item.LocationId = cat.Id;
                cat.Equipment.Add(item.Id);
            }
            else
            {
                item.LocationId = destination.Id;
                cat.Equipment.Remove(item.Id);
            }
            CompleteEquipmentOrder(village, job, false);
            Note(village, "completed", cat.Name + (job.Kind == "equip" ? " equipped " : " returned ") + item.Kind, job.Id);
        }

        private void ValidateEquipment(Village village, List<string> errors)
        {
            foreach (var job in village.Jobs.Where(job => !job.Completed && IsEquipmentJob(job)))
            {
                var cat = village.Cats.Find(cat => cat.Id == job.CatId && cat.Alive);
                var item = job.ItemIds.Count == 1 ? village.Items.Find(item => item.Id == job.ItemIds[0]) : null;
                bool invalid = cat == null || cat.JobId != job.Id && cat.ResumeJobId != job.Id || !job.Manual || job.AutomatedBy != "" || item == null;
                if (item != null)
                {
                    invalid |= item.VillageId != village.Id || village.Jobs.Count(other => !other.Completed && other.ItemIds.Contains(item.Id)) != 1;
                    invalid |= job.Kind == "equip"
                        ? job.Phase != "item_fetch" || job.TargetId != item.Id || item.LocationId != job.Id || !village.Stockpiles.Any(pile => pile.Id == job.SourceId && pile.Position.Equals(job.Position)) || cat?.Equipment.Contains(item.Id) == true
                        : job.Phase != "item_return" || job.SourceId != cat?.Id || item.LocationId != cat?.Id || cat?.Equipment.Contains(item.Id) != true;
                }
                if (invalid)
                    errors.Add("Invalid equipment order " + job.Id);
            }
        }
    }
}
