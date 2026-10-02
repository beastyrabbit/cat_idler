using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Carries fetched inputs to the work position; true when the tick ends here, false when the job
        /// moves on to work in the same tick.
        /// </summary>
        private bool TickJobInputDelivery(Village v, Cat c, Job j, double dt)
        {
            if (!Move(c, j.Position, dt))
            {
                if (j.Kind == JobKind.Haul && c.BlockedReason == Blocked.BlockedRoute && j.Local.Count == 0 && c.Cargo.Count > 0 && TimeSeconds + 1e-9 >= j.NextStorageAttemptAt)
                {
                    var carried = c.Cargo[0];
                    var alternate = Storage(v, carried.Resource, carried.Amount, c.Position, j.SourceId);
                    j.NextStorageAttemptAt = alternate == null ? Math.Floor(TimeSeconds) + Pace.StorageRetrySeconds : 0;
                    if (alternate != null)
                    {
                        j.TargetId = alternate.Id;
                        j.Position = alternate.Position;
                        c.Path.Clear();
                        j.BlockedReason = c.BlockedReason = "";
                    }
                }
                return true;
            }
            foreach (var s in c.Cargo)
                Add(j.Local, s.Resource, s.Amount);
            c.Cargo.Clear();
            if (Reservations.Any(r => r.OwnerId == j.Id))
            {
                j.Phase = JobPhase.Fetch;
                return true;
            }
            if (j.Kind == JobKind.Haul)
            {
                j.Phase = JobPhase.OutputDelivery;
                foreach (var s in j.Local)
                    Add(c.Cargo, s.Resource, s.Amount);
                j.Local.Clear();
                return true;
            }
            j.Phase = JobPhase.Working;
            return false;
        }

        /// <summary>
        /// Carries the job's output to storage and finishes the job when nothing is left. One pickup takes a load
        /// of every output stack, so a hunt brings food, hide and bone home together; a load that no single pile
        /// can hold whole is split across the nearest piles with room.
        /// </summary>
        private void TickJobOutputDelivery(Village v, Cat c, Job j, double dt)
        {
            if (c.Cargo.Count == 0 && j.Local.Count > 0)
            {
                if (!Move(c, j.Position, dt))
                    return;
                foreach (var s in j.Local.ToArray())
                {
                    double take = Math.Min(CarryCapacity(v, s.Resource), s.Amount);
                    Add(c.Cargo, s.Resource, take);
                    Add(j.Local, s.Resource, -take);
                }
            }
            var cargo = c.Cargo.FirstOrDefault();
            var deliveredItem = cargo == null && j.ItemIds.Count > 0 ? v.Items.Find(i => i.Id == j.ItemIds[0]) : null;
            string resource = cargo?.Resource ?? (deliveredItem != null ? ItemResource(deliveredItem.Kind) : "");
            if (resource == "")
            {
                Finish(v, c, j);
                return;
            }
            string excluded = j.Kind == JobKind.Haul ? j.SourceId : null;
            var pile = v.Stockpiles.Find(s => s.Id == j.DeliveryPileId && s.Kind == "storage");
            if (pile != null && (cargo == null ? !HasRoom(v, pile, resource, 1) : RoomFor(v, pile, resource) <= PartialRoomMin))
                pile = null;
            if (pile == null && TimeSeconds + 1e-9 < j.NextStorageAttemptAt)
                return;
            if (pile == null)
            {
                pile = Storage(v, resource, cargo?.Amount ?? 1, c.Position, excluded) ?? (cargo != null ? PartialStorage(v, resource, c.Position, excluded) : null);
                j.NextStorageAttemptAt = Math.Floor(TimeSeconds) + Pace.StorageRetrySeconds;
            }
            if (pile == null)
            {
                j.BlockedReason = c.BlockedReason = Blocked.OutputStorageFullOrUnreachable;
                return;
            }
            j.DeliveryPileId = pile.Id;
            if (!Move(c, pile.Position, dt))
            {
                if (c.BlockedReason == Blocked.BlockedRoute)
                {
                    j.DeliveryPileId = "";
                    j.NextStorageAttemptAt = Math.Floor(TimeSeconds) + Pace.StorageRetrySeconds;
                }
                return;
            }
            // Everything carried that this pile accepts goes in at once; the rest waits for the next pile.
            foreach (var carried in c.Cargo.ToArray())
            {
                double put = Math.Min(carried.Amount, RoomFor(v, pile, carried.Resource));
                if (put <= PartialRoomMin)
                    continue;
                Add(pile.Goods, carried.Resource, put);
                Add(c.Cargo, carried.Resource, -put);
            }
            if (deliveredItem != null)
            {
                deliveredItem.LocationId = pile.Id;
                j.ItemIds.Remove(deliveredItem.Id);
            }
            GainSkill(c, "haul", Pace.HaulSkillPerDelivery);
            j.DeliveryPileId = "";
            if (j.Local.Count == 0 && c.Cargo.Count == 0 && j.ItemIds.Count == 0)
                Finish(v, c, j);
        }

        /// <summary>Smallest share of a load worth a trip; a pile with less room than this counts as full.</summary>
        private const double PartialRoomMin = 1e-6;

        /// <summary>How much more of <paramref name="resource"/> the pile accepts, by the same limits as <c>HasRoom</c>.</summary>
        private double RoomFor(Village v, Stockpile pile, string resource)
        {
            if (pile.Accepts.Count > 0 && !pile.Accepts.Contains(resource))
                return 0;
            var limit = pile.ResourceLimits.Find(l => l.Resource == resource);
            double room = limit != null ? limit.Amount + ResourceCapacityBonus(v, resource) - Amount(pile.Goods, resource) - PileItemCount(v, pile, resource) : ResourceCapacity(v, pile, resource) - PileLoad(v, pile);
            return Math.Max(0, room);
        }

        /// <summary>The nearest reachable storage with any room for the resource, for a load no single pile takes whole.</summary>
        private Stockpile PartialStorage(Village v, string resource, Int2 from, string excludedId) =>
            v.Stockpiles.Where(s => s.Kind == "storage" && (excludedId == null || s.Id != excludedId) && RoomFor(v, s, resource) > PartialRoomMin)
                .OrderBy(s => Int2.Distance(s.Position, from)).ThenBy(s => s.Id, StringComparer.Ordinal).FirstOrDefault(s => Path(from, s.Position, v) != null);
    }
}
