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
                return NoWorker(c, "No available carrier");
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
            var pile = SpillPile(v, p, Pace.SpillPileCapacity);
            foreach (var s in goods)
                Add(pile.Goods, s.Resource, s.Amount);
            goods.Clear();
            return pile;
        }

        /// <summary>The one spill pile on a tile, created on first use; goods and items dropped there share it.</summary>
        private Stockpile SpillPile(Village v, Int2 p, double capacity)
        {
            var pile = v.Stockpiles.Find(s => s.Kind == "spill" && s.Position.Equals(p));
            if (pile == null)
            {
                pile = new Stockpile { Id = Id("spill"), Position = p, Width = 1, Depth = 1, Kind = "spill", Capacity = capacity };
                v.Stockpiles.Add(pile);
            }
            return pile;
        }

        /// <summary>
        /// Removes a spill pile once nothing lies there and nothing refers to it: no goods, items, claims, need
        /// trips or other live job. <paramref name="emptiedBy"/> is the job that just emptied it.
        /// </summary>
        private void PruneSpill(Village v, Stockpile pile, Job emptiedBy = null)
        {
            if (pile == null || pile.Kind != "spill" || pile.Goods.Count > 0)
                return;
            foreach (var item in v.Items)
                if (item.LocationId == pile.Id)
                    return;
            foreach (var claim in Reservations)
                if (claim.PileId == pile.Id)
                    return;
            foreach (var job in v.Jobs)
                if (!job.Completed && job != emptiedBy && (job.SourceId == pile.Id || job.TargetId == pile.Id || job.DeliveryPileId == pile.Id || job.SuspendedCargoPileId == pile.Id))
                    return;
            foreach (var cat in v.Cats)
                if (cat.NeedSourceId == pile.Id)
                    return;
            v.Stockpiles.Remove(pile);
        }

        /// <summary>Removes every spill pile that has become empty, whatever emptied it.</summary>
        private void PruneEmptySpills(Village v)
        {
            foreach (var pile in v.Stockpiles.Where(s => s.Kind == "spill" && s.Goods.Count == 0).ToArray())
                PruneSpill(v, pile);
        }

        /// <summary>
        /// Claims <paramref name="amount"/> for <paramref name="ownerId"/> from the piles nearest to
        /// <paramref name="from"/> that a cat there can reach, skipping piles the job watchdog excluded. All or
        /// nothing, like <c>Reserve</c>.
        /// </summary>
        public bool ReserveNearest(Village v, string ownerId, string resource, double amount, Int2 from)
        {
            if (amount <= 0 || !Finite(amount))
                return false;
            var takes = new List<(Stockpile Pile, double Amount)>();
            double found = 0;
            foreach (var s in v.Stockpiles.OrderBy(s => Int2.Distance(from, s.Position)).ThenBy(s => s.Id, StringComparer.Ordinal))
            {
                double free = Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount);
                if (free <= 1e-9 || PileExcluded(v, s.Id) || Path(from, s.Position, v) == null)
                    continue;
                double take = Math.Min(free, amount - found);
                takes.Add((s, take));
                found += take;
                if (found + 1e-9 >= amount)
                    break;
            }
            if (found + 1e-9 < amount)
                return false;
            foreach (var (pile, take) in takes)
                Reservations.Add(new Reservation { OwnerId = ownerId, VillageId = v.Id, PileId = pile.Id, Resource = resource, Amount = take });
            return true;
        }

        /// <summary>Walks to the nearest of the owner's claims and picks up one load of it.</summary>
        private void PickupClaim(Village v, Cat c, string ownerId, Job j, double dt)
        {
            Reservation claim = null;
            Stockpile pile = null;
            int best = int.MaxValue;
            foreach (var r in Reservations)
            {
                if (r.OwnerId != ownerId)
                    continue;
                var at = v.Stockpiles.Find(s => s.Id == r.PileId);
                int distance = at == null ? -1 : Int2.Distance(c.Position, at.Position);
                if (distance < best)
                {
                    best = distance;
                    claim = r;
                    pile = at;
                }
            }
            if (claim == null)
                return;
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
            PruneSpill(v, pile, j);
        }
    }
}
