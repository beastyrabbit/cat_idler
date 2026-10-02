using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult StartHaul(Village v, Cat c, Stockpile source)
        {
            c = c ?? AvailableCat(v, "haul");
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available carrier");
            var resource = source.Goods.FirstOrDefault(s => s.Amount > 0);
            if (resource == null)
            {
                var item = v.Items.Where(i => i.LocationId == source.Id).OrderBy(i => i.Id, StringComparer.Ordinal).FirstOrDefault();
                if (item == null)
                    return ActionResult.Fail("Empty source");
                var itemStorage = Storage(v, ItemResource(item.Kind), 1, source.Position, source.Id);
                if (itemStorage == null)
                    return ActionResult.Fail("No accepting destination");
                if (Path(c.Position, source.Position, v) == null)
                    return ActionResult.Fail("Source unreachable");
                var haul = new Job { Id = Id("job"), Kind = JobKind.Haul, Phase = JobPhase.ItemFetch, SourceId = source.Id, TargetId = itemStorage.Id, Position = source.Position, Resource = ItemResource(item.Kind), Amount = 1, ItemIds = new List<string> { item.Id } };
                StartJob(v, c, haul);
                item.LocationId = haul.Id;
                return ActionResult.Ok(haul.Id);
            }
            double free = resource.Amount - Reservations.Where(r => r.PileId == source.Id && r.Resource == resource.Resource).Sum(r => r.Amount);
            double amount = Math.Min(Pace.HaulBatchMax, free);
            if (amount <= 0)
                return ActionResult.Fail("Cargo already claimed");
            var destination = Storage(v, resource.Resource, amount, source.Position, source.Id);
            if (destination == null)
                return ActionResult.Fail("No accepting destination");
            string id = Id("job");
            Reservations.Add(new Reservation { OwnerId = id, VillageId = v.Id, PileId = source.Id, Resource = resource.Resource, Amount = amount });
            StartJob(v, c, new Job { Id = id, Kind = JobKind.Haul, Phase = JobPhase.Fetch, SourceId = source.Id, TargetId = destination.Id, Position = destination.Position, Resource = resource.Resource, Amount = amount });
            return ActionResult.Ok(id);
        }
        private Stockpile Spill(Village v, Int2 p, List<Stack> goods)
        {
            if (goods.Count == 0)
                return null;
            var pile = v.Stockpiles.Find(s => s.Kind == "spill" && s.Position.Equals(p));
            if (pile == null)
            {
                pile = new Stockpile { Id = Id("spill"), Position = p, Width = 1, Depth = 1, Kind = "spill", Capacity = Pace.SpillPileCapacity };
                v.Stockpiles.Add(pile);
            }
            foreach (var s in goods)
                Add(pile.Goods, s.Resource, s.Amount);
            goods.Clear();
            return pile;
        }
        private void PickupClaim(Village v, Cat c, string ownerId, Job j, double dt)
        {
            var claim = Reservations.FirstOrDefault(r => r.OwnerId == ownerId);
            if (claim == null)
                return;
            var pile = v.Stockpiles.Find(s => s.Id == claim.PileId);
            if (pile == null || Amount(pile.Goods, claim.Resource) < claim.Amount)
            {
                Release(ownerId);
                j.BlockedReason = Blocked.SourceChanged;
                return;
            }
            if (!Move(c, pile.Position, dt))
                return;
            double quantity = Math.Min(CarryCapacity(v, claim.Resource), claim.Amount);
            Add(pile.Goods, claim.Resource, -quantity);
            Add(c.Cargo, claim.Resource, quantity);
            claim.Amount -= quantity;
            if (claim.Amount <= 1e-9)
                Reservations.Remove(claim);
        }
    }
}
