using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        public static double Amount(List<Stack> goods, string resource) => goods.Where(s => s.Resource == resource).Sum(s => s.Amount);
        public static void Add(List<Stack> goods, string resource, double amount)
        {
            if (double.IsNaN(amount) || double.IsInfinity(amount))
                throw new ArgumentException("Finite goods required");
            var s = goods.Find(x => x.Resource == resource);
            if (s == null)
            {
                s = new Stack(resource, 0);
                goods.Add(s);
            }
            s.Amount += amount;
            if (s.Amount < -0.000001)
                throw new InvalidOperationException("Negative " + resource);
            if (Math.Abs(s.Amount) < 0.000001)
                goods.Remove(s);
        }
        public double Total(Village v, string resource) => resource == "blessings" ? v.Blessings : EquipmentKind(resource) != "" ? v.Items.Count(i => i.Kind == EquipmentKind(resource) && v.Stockpiles.Any(s => s.Kind == "storage" && s.Id == i.LocationId)) : v.Stockpiles.Where(s => s.Kind == "storage").Sum(s => Amount(s.Goods, resource));
        public double ReportedTotal(Village v, string resource) => resource == "blessings" ? v.Blessings : v.Stockpiles.Where(s => s.Kind == "storage").Sum(s => Amount(s.Report, resource));
        public double Available(Village v, string resource) => v.Stockpiles.Sum(s => Math.Max(0, Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount)));
        public bool Reserve(Village v, string ownerId, string resource, double amount)
        {
            if (amount <= 0 || !Finite(amount) || Available(v, resource) + 1e-9 < amount)
                return false;
            foreach (var s in v.Stockpiles.OrderBy(s => s.Id, StringComparer.Ordinal))
            {
                double free = Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount);
                double take = Math.Min(amount, Math.Max(0, free));
                if (take > 0)
                    Reservations.Add(new Reservation { OwnerId = ownerId, VillageId = v.Id, PileId = s.Id, Resource = resource, Amount = take });
                amount -= take;
                if (amount < 1e-9)
                    return true;
            }
            return false;
        }
        public void Release(string ownerId) => Reservations.RemoveAll(r => r.OwnerId == ownerId);
        public static bool Finite(double value) => !double.IsNaN(value) && !double.IsInfinity(value);
        private bool Spend(Village v, string resource, double amount)
        {
            if (!Finite(amount) || amount < 0 || Available(v, resource) + 1e-9 < amount)
                return false;
            foreach (var s in v.Stockpiles.OrderBy(s => s.Id, StringComparer.Ordinal))
            {
                double free = Math.Max(0, Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount));
                double take = Math.Min(amount, free);
                if (take > 0)
                    Add(s.Goods, resource, -take);
                amount -= take;
                if (amount < 1e-9)
                    return true;
            }
            return true;
        }
        private Stockpile Storage(Village v, string resource, double amount, Int2 from, string excludedId = null) => v.Stockpiles.Where(s => s.Kind == "storage" && (excludedId == null || s.Id != excludedId) && HasRoom(v, s, resource, amount)).OrderBy(s => Int2.Distance(s.Position, from)).ThenBy(s => s.Id, StringComparer.Ordinal).FirstOrDefault(s => Path(from, s.Position, v) != null);
        public bool HasRoom(Village v, Stockpile s, string resource, double amount)
        {
            if (s.Accepts.Count > 0 && !s.Accepts.Contains(resource))
                return false;
            var limit = s.ResourceLimits.Find(l => l.Resource == resource);
            return limit != null ? Amount(s.Goods, resource) + PileItemCount(v, s, resource) + amount <= limit.Amount + ResourceCapacityBonus(v, resource) : PileLoad(v, s) + amount <= ResourceCapacity(v, s, resource);
        }
        private static int PileItemCount(Village v, Stockpile s, string resource = null)
        {
            bool Matches(Item item) => resource == null || ItemResource(item.Kind) == resource;
            int count = v.Items.Count(i => i.LocationId == s.Id && Matches(i));
            foreach (var job in v.Jobs.Where(j => !j.Completed && j.Phase == "item_fetch" && j.SourceId == s.Id))
                count += v.Items.Count(i => i.LocationId == job.Id && job.ItemIds.Contains(i.Id) && Matches(i));
            return count;
        }
        private static double PileLoad(Village v, Stockpile s) => s.Goods.Sum(g => g.Amount) + PileItemCount(v, s);
        public double Capacity(Village v, Stockpile s)
        {
            var building = v.Buildings.Find(b => b.Id == s.ManagedBy);
            return s.Capacity * Catalog.Effect(v, "storageCapacity", 1) * Catalog.Effect(v, "storagePerLevelMult", 1) * (building == null ? 1 : Catalog.BuildingEffect(v, building.Kind, "capacity", 1));
        }
        public double ResourceCapacity(Village v, Stockpile s, string resource) => Capacity(v, s) + ResourceCapacityBonus(v, resource);
        public double StationCapacity(Village v, string building) => 16 * Catalog.BuildingEffect(v, building, "capacity", 1);
    }
}
