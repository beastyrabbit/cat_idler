using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleEnterCatControl(PlayerContext context, Village village, Cat cat)
        {
            if (cat == null)
                return ActionResult.Fail("Living cat required");
            if (cat.ControlledBy != "" && cat.ControlledBy != context.PlayerId && cat.ControlLeaseUntil > TimeSeconds)
                return ActionResult.Fail("Cat controlled by another player");
            foreach (var other in village.Cats.Where(c => c.ControlledBy == context.PlayerId))
            {
                other.ControlledBy = "";
                other.Goal = "idle";
            }
            CancelWork(village, cat, preserveUnassignedCargo: true);
            cat.ControlledBy = context.PlayerId;
            cat.ControlLeaseUntil = TimeSeconds + 30;
            cat.Goal = "player_control";
            return ActionResult.Ok(cat.Id);
        }
        private ActionResult HandleLeaveCatControl(PlayerContext context, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail("Control lease required");
            cat.ControlledBy = "";
            cat.Goal = "idle";
            return ActionResult.Ok(cat.Id);
        }
        private ActionResult HandleKeepCatControl(PlayerContext context, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId || cat.ControlLeaseUntil <= TimeSeconds)
                return ActionResult.Fail("Existing control lease required");
            cat.ControlLeaseUntil = TimeSeconds + 30;
            return ActionResult.Ok();
        }
        private ActionResult HandleMoveCat(PlayerContext context, Village village, GameAction action, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail("Control lease required");
            if (village.Routes.Any(r => r.CatId == cat.Id && r.CancelRequested && (r.PathIndex > 0 || village.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9))))
                return ActionResult.Fail("Vessel is physically returning this cat to its dock");
            if (Math.Abs(action.Position.X) + Math.Abs(action.Position.Z) != 1)
                return ActionResult.Fail("Movement must be one cardinal direction");
            var stairs = DungeonNeighbors(cat.Position).Where(p => Math.Sign(p.X - cat.Position.X) == action.Position.X && Math.Sign(p.Z - cat.Position.Z) == action.Position.Z).Select(p => (Int2?)p).FirstOrDefault();
            var next = stairs ?? new Int2(cat.Position.X + action.Position.X, cat.Position.Z + action.Position.Z, cat.Position.Level);
            if (!Walkable(village, next) || !Crossable(cat.Position, next))
                return ActionResult.Fail("Blocked route");
            if (stairs.HasValue && !village.Known.Contains(next))
                village.Known.Add(next);
            cat.Path = new List<Int2> { next };
            cat.ControlLeaseUntil = TimeSeconds + 30;
            return ActionResult.Ok();
        }
        private ActionResult HandleInteractCat(PlayerContext context, Village village, GameAction action, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail("Control lease required");
            return Interact(village, cat, action);
        }
        private ActionResult Interact(Village v, Cat c, GameAction a)
        {
            if (a.Mode == "eat" || a.Mode == "drink" || a.Mode == "rest")
                return ControlledNeed(v, c, a.Mode);
            if (c.Position.Level < 0 && a.TargetId == "")
            {
                var chest = Dungeons.SelectMany(d => d.Floors).FirstOrDefault(f => f.LootPosition.Equals(c.Position) && Math.Abs(c.X - f.LootPosition.X) + Math.Abs(c.Z - f.LootPosition.Z) < 1e-8 && f.Loot.Any(s => s.Amount > 0));
                if (chest == null)
                    return ActionResult.Fail("Walk to a dungeon chest or select a nearby pile");
                foreach (var stack in chest.Loot.ToArray())
                {
                    double take = Math.Min(stack.Amount, Math.Max(0, CarryCapacity(v, stack.Resource) - Amount(c.Cargo, stack.Resource)));
                    Add(c.Cargo, stack.Resource, take);
                    Add(chest.Loot, stack.Resource, -take);
                }
                return ActionResult.Ok();
            }
            var pile = v.Stockpiles.Find(s => s.Id == a.TargetId);
            if (pile != null)
            {
                if (Int2.Distance(c.Position, pile.Position) > 1)
                    return ActionResult.Fail("Walk to the pile first");
                if (c.Cargo.Count > 0)
                {
                    if (c.Cargo.Any(s => !HasRoom(v, pile, s.Resource, s.Amount)) || c.Cargo.Where(s => !pile.ResourceLimits.Any(limit => limit.Resource == s.Resource)).Any(s => PileLoad(v, pile) + c.Cargo.Sum(g => g.Amount) > ResourceCapacity(v, pile, s.Resource)))
                        return ActionResult.Fail("Storage full or incompatible");
                    foreach (var s in c.Cargo)
                        Add(pile.Goods, s.Resource, s.Amount);
                    c.Cargo.Clear();
                    return ActionResult.Ok();
                }
                string resource = a.Resource;
                double amount = Math.Min(8, Math.Max(1, a.Amount));
                double free = Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
                if (free < amount)
                    return ActionResult.Fail("Unclaimed goods unavailable");
                Add(pile.Goods, resource, -amount);
                Add(c.Cargo, resource, amount);
                return ActionResult.Ok();
            }
            if (Int2.Distance(c.Position, v.Center) <= 1 && c.Cargo.Count > 0)
            {
                foreach (var s in c.Cargo.ToArray())
                    if (new[] { "food", "herbs", "materials" }.Contains(s.Resource))
                    {
                        v.Blessings += s.Amount / 5;
                        c.Cargo.Remove(s);
                    }
                return ActionResult.Ok();
            }
            return ActionResult.Fail("No interactable target within reach");
        }
    }
}
