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

        /// <summary>Carries the job's output, one load at a time, to storage and finishes the job when empty.</summary>
        private void TickJobOutputDelivery(Village v, Cat c, Job j, double dt)
        {
            if (c.Cargo.Count == 0 && j.Local.Count > 0)
            {
                if (!Move(c, j.Position, dt))
                    return;
                var s = j.Local[0];
                double take = Math.Min(CarryCapacity(v, s.Resource), s.Amount);
                Add(c.Cargo, s.Resource, take);
                Add(j.Local, s.Resource, -take);
            }
            var cargo = c.Cargo.FirstOrDefault();
            var deliveredItem = cargo == null && j.ItemIds.Count > 0 ? v.Items.Find(i => i.Id == j.ItemIds[0]) : null;
            string resource = cargo?.Resource ?? (deliveredItem != null ? ItemResource(deliveredItem.Kind) : "");
            if (resource == "")
            {
                Finish(v, c, j);
                return;
            }
            var pile = v.Stockpiles.Find(s => s.Id == j.DeliveryPileId && s.Kind == "storage");
            if (pile != null && !HasRoom(v, pile, resource, cargo?.Amount ?? 1))
                pile = null;
            if (pile == null && TimeSeconds + 1e-9 < j.NextStorageAttemptAt)
                return;
            if (pile == null)
            {
                pile = Storage(v, resource, cargo?.Amount ?? 1, c.Position, j.Kind == JobKind.Haul ? j.SourceId : null);
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
            if (cargo != null)
            {
                Add(pile.Goods, cargo.Resource, cargo.Amount);
                c.Cargo.Remove(cargo);
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
    }
}
