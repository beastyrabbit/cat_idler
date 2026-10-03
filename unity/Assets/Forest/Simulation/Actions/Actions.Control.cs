using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>True when a tile is inside a footprint or touches it (the ring of eight neighbours), on its level.</summary>
        public static bool InReach(Int2 at, Int2 origin, int width, int depth) =>
            at.Level == origin.Level && at.X >= origin.X - 1 && at.X <= origin.X + Math.Max(1, width) && at.Z >= origin.Z - 1 && at.Z <= origin.Z + Math.Max(1, depth);

        /// <summary>True when a cat on this tile can reach the pile: inside its footprint or on a tile touching it.</summary>
        public bool InReach(Int2 at, Stockpile s) => s != null && InReach(at, s.Position, s.Width, s.Depth);

        /// <summary>True when a cat on this tile can use the shrine: beside the village centre or touching the shrine.</summary>
        public bool AtShrine(Village v, Int2 at) => Int2.Distance(at, v.Center) <= 1 || v.Buildings.Any(b => b.Kind == "shrine" && InReach(at, b.Position, b.Width, b.Depth));

        private ActionResult HandleEnterCatControl(PlayerContext context, Village village, Cat cat)
        {
            if (cat == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living cat required");
            if (cat.ControlledBy != "" && cat.ControlledBy != context.PlayerId && cat.ControlLeaseUntil > TimeSeconds)
                return ActionResult.Fail(ResultCode.ControlledByOther, "Cat controlled by another player", (ResultArg.Target, cat.Id));
            foreach (var other in village.Cats.Where(c => c.ControlledBy == context.PlayerId))
            {
                other.ControlledBy = "";
                other.ControlDirection = default;
                other.Goal = GoalKey.Idle;
            }
            CancelWork(village, cat, preserveUnassignedCargo: true);
            cat.ControlledBy = context.PlayerId;
            cat.ControlLeaseUntil = TimeSeconds + 30;
            cat.ControlDirection = default;
            cat.Goal = GoalKey.PlayerControl;
            return ActionResult.Ok(ResultCode.Controlling, cat.Id);
        }
        private ActionResult HandleLeaveCatControl(PlayerContext context, Village village, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail(ResultCode.ControlRequired, "Control lease required");
            cat.ControlledBy = "";
            cat.ControlDirection = default;
            cat.Goal = GoalKey.Idle;
            // An explorer released underground resumes its expedition (W2-09).
            ContinueExpeditionAfterControl(village, cat);
            return ActionResult.Ok(ResultCode.ControlLeft, cat.Id);
        }
        private ActionResult HandleKeepCatControl(PlayerContext context, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId || cat.ControlLeaseUntil <= TimeSeconds)
                return ActionResult.Fail(ResultCode.ControlRequired, "Existing control lease required");
            cat.ControlLeaseUntil = TimeSeconds + 30;
            return ActionResult.Ok(ResultCode.ControlKept, cat.Id);
        }

        /// <summary>
        /// Holds a direction (one step of -1, 0 or 1 on X and Z). A cat standing still starts at once; a new
        /// direction mid-edge retraces the unfinished edge first. TickControls chains tiles while the client
        /// keeps re-sending MoveCat, until StopCat, a blocked tile or the lease ends.
        /// </summary>
        private ActionResult HandleMoveCat(PlayerContext context, Village village, GameAction action, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail(ResultCode.ControlRequired, "Control lease required");
            if (village.Routes.Any(r => r.CatId == cat.Id && r.CancelRequested && (r.PathIndex > 0 || village.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9))))
                return ActionResult.Fail(ResultCode.TransportReturning, "Vessel is physically returning this cat to its dock", (ResultArg.Target, cat.Id));
            var direction = new Int2(action.Position.X, action.Position.Z);
            if (Math.Abs(direction.X) > 1 || Math.Abs(direction.Z) > 1 || direction.Equals(default(Int2)))
                return ActionResult.Fail(ResultCode.InvalidDirection, "Movement is one step in one of eight directions", (ResultArg.Direction, direction.ToString()));
            // Holding the same direction mid-edge keeps the edge and checks the tile after it. Any other
            // direction starts from the tile the cat left; Move retraces the unfinished edge first.
            bool continuing = cat.Path.Count > 0 && Math.Abs(cat.X - cat.Position.X) + Math.Abs(cat.Z - cat.Position.Z) > 1e-9 && cat.ControlDirection.Equals(direction);
            var from = continuing ? cat.Path[cat.Path.Count - 1] : cat.Position;
            var next = ControlStep(village, from, direction);
            if (!next.HasValue)
            {
                cat.ControlDirection = default;
                return ActionResult.Fail(ResultCode.BlockedRoute, "Blocked route", (ResultArg.Target, cat.Id), (ResultArg.Direction, direction.ToString()));
            }
            if (!continuing)
                StepTo(village, cat, next.Value);
            // Stair endpoints the cat is about to use exist as tiles (W2-09); chained steps do the same in StepTo.
            MaterialiseStairEndpoints(village, cat);
            return HoldDirection(cat, direction);
        }
        private ActionResult HoldDirection(Cat cat, Int2 direction)
        {
            cat.ControlDirection = direction;
            cat.ControlHeldAt = TimeSeconds;
            cat.ControlLeaseUntil = TimeSeconds + 30;
            cat.Goal = GoalKey.PlayerControl;
            return ActionResult.Ok(ResultCode.Moving, cat.Id, (ResultArg.Direction, direction.ToString()));
        }
        /// <summary>Releases the held direction; the cat finishes the edge it is on and stands.</summary>
        private ActionResult HandleStopCat(PlayerContext context, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail(ResultCode.ControlRequired, "Control lease required");
            cat.ControlDirection = default;
            cat.ControlLeaseUntil = TimeSeconds + 30;
            return ActionResult.Ok(ResultCode.Stopped, cat.Id);
        }

        /// <summary>
        /// The tile one held step away, or null when blocked. Cardinal steps may take dungeon stairs; a
        /// diagonal needs both cardinal neighbours open so it never cuts a wall corner.
        /// </summary>
        private Int2? ControlStep(Village v, Int2 from, Int2 direction)
        {
            bool Open(Int2 a, Int2 b) => Walkable(v, b) && Crossable(a, b);
            if (direction.X == 0 || direction.Z == 0)
            {
                var stairs = DungeonNeighbors(from).Where(p => Math.Sign(p.X - from.X) == direction.X && Math.Sign(p.Z - from.Z) == direction.Z).Select(p => (Int2?)p).FirstOrDefault();
                if (stairs.HasValue)
                    return Open(from, stairs.Value) ? stairs : null;
            }
            var next = new Int2(from.X + direction.X, from.Z + direction.Z, from.Level);
            if (!Open(from, next))
                return null;
            if (direction.X != 0 && direction.Z != 0)
            {
                var alongX = new Int2(from.X + direction.X, from.Z, from.Level);
                var alongZ = new Int2(from.X, from.Z + direction.Z, from.Level);
                if (!Open(from, alongX) || !Open(alongX, next) || !Open(from, alongZ) || !Open(alongZ, next))
                    return null;
            }
            return next;
        }
        private void StepTo(Village v, Cat c, Int2 next)
        {
            if (next.Level != c.Position.Level && !v.Known.Contains(next))
                v.Known.Add(next);
            c.Path = new List<Int2> { next };
            c.ControlStepAt = TimeSeconds;
            MaterialiseStairEndpoints(v, c);
        }

        private ActionResult HandleInteractCat(PlayerContext context, Village village, GameAction action, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail(ResultCode.ControlRequired, "Control lease required");
            return Interact(village, cat, action);
        }

        /// <summary>
        /// The E key. An explicit Mode (eat, drink, rest, offer) or pile TargetId does exactly that; otherwise
        /// the first thing in reach: a dungeon chest, then at the shrine drink or eat when it helps and offer
        /// what can be offered, then rest at the cat's own den.
        /// </summary>
        private ActionResult Interact(Village v, Cat c, GameAction a)
        {
            string mode = (a.Mode ?? "").ToLowerInvariant();
            if (mode == "eat" || mode == "drink")
                return ControlledNeed(v, c, mode);
            if (mode == "rest")
                return ControlledRest(v, c);
            if (mode == "offer")
                return ShrineOffering(v, c);
            if (!string.IsNullOrEmpty(a.TargetId))
            {
                var pile = v.Stockpiles.Find(s => s.Id == a.TargetId);
                if (pile == null)
                    return ActionResult.Fail(ResultCode.UnknownTarget, "That pile no longer exists", (ResultArg.Target, a.TargetId));
                return PileInteraction(v, c, pile, a);
            }
            // Underground, E opens the chest in reach once its guardian is down (W2-09). With no chest in
            // reach, E answers like everywhere else.
            if (c.Position.Level < 0)
            {
                var chest = TakeDungeonChest(v, c);
                return chest.Code == ResultCode.NoChestInReach ? ActionResult.Fail(ResultCode.NothingInReach, "No interactable target within reach") : chest;
            }
            bool atShrine = AtShrine(v, c.Position);
            if (atShrine)
            {
                if (c.Thirst <= 100 - Pace.DrinkRestorePerServing * Pace.NeedServing && Serving(v, c, "drink") != null)
                    return ControlledNeed(v, c, "drink");
                if (c.Hunger <= 100 - Pace.EatRestorePerServing * Pace.NeedServing && Serving(v, c, "eat") != null)
                    return ControlledNeed(v, c, "eat");
                if (c.Cargo.Any(s => OfferableResources.Contains(s.Resource) && s.Amount > 0))
                    return ShrineOffering(v, c);
            }
            var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
            if (den != null && c.Rest < Pace.SleepWakeRest && InReach(c.Position, den.Position, den.Width, den.Depth))
                return ControlledRest(v, c);
            return atShrine && c.Cargo.Count > 0
                ? ActionResult.Fail(ResultCode.NothingToOffer, "Only food, herbs and materials can be offered")
                : ActionResult.Fail(ResultCode.NothingInReach, "No interactable target within reach");
        }

        private ActionResult PileInteraction(Village v, Cat c, Stockpile pile, GameAction a)
        {
            if (!InReach(c.Position, pile))
                return ActionResult.Fail(ResultCode.OutOfReach, "Walk to the pile first", (ResultArg.Target, pile.Id));
            if (c.Cargo.Count > 0)
            {
                if (c.Cargo.Any(s => !HasRoom(v, pile, s.Resource, s.Amount)) || c.Cargo.Where(s => !pile.ResourceLimits.Any(limit => limit.Resource == s.Resource)).Any(s => PileLoad(v, pile) + c.Cargo.Sum(g => g.Amount) > ResourceCapacity(v, pile, s.Resource)))
                    return ActionResult.Fail(ResultCode.StorageFull, "Storage full or incompatible", (ResultArg.Target, pile.Id), (ResultArg.Resource, c.Cargo[0].Resource));
                double carried = c.Cargo.Sum(s => s.Amount);
                string first = c.Cargo[0].Resource;
                foreach (var s in c.Cargo)
                    Add(pile.Goods, s.Resource, s.Amount);
                c.Cargo.Clear();
                return ActionResult.Ok(ResultCode.Deposited, pile.Id, (ResultArg.Resource, first), (ResultArg.Amount, ActionResult.Number(carried)));
            }
            double Free(string resource) => Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
            string wanted = a.Resource;
            if (string.IsNullOrEmpty(wanted))
            {
                // The contextual E: a full load of the most plentiful unclaimed good.
                var best = pile.Goods.Select(s => (s.Resource, Free: Free(s.Resource))).Where(s => s.Free >= 1).OrderByDescending(s => s.Free).ThenBy(s => s.Resource, StringComparer.Ordinal).FirstOrDefault();
                if (best.Resource == null)
                    return ActionResult.Fail(ResultCode.NothingToCollect, "Nothing unclaimed to take here", (ResultArg.Target, pile.Id));
                double load = Math.Min(Math.Floor(best.Free), Math.Min(CarryCapacity(v, best.Resource), Math.Max(1, a.Amount)));
                return TakeFromPile(c, pile, best.Resource, load);
            }
            double amount = Math.Min(CarryCapacity(v, wanted), Math.Max(1, a.Amount));
            double free = Free(wanted);
            if (free < amount)
                return ActionResult.Fail(ResultCode.MissingMaterial, "Unclaimed goods unavailable", (ResultArg.Resource, wanted), (ResultArg.Need, ActionResult.Number(amount)), (ResultArg.Have, ActionResult.Number(Math.Max(0, free))), (ResultArg.Target, pile.Id));
            return TakeFromPile(c, pile, wanted, amount);
        }
        private static ActionResult TakeFromPile(Cat c, Stockpile pile, string resource, double amount)
        {
            Add(pile.Goods, resource, -amount);
            Add(c.Cargo, resource, amount);
            return ActionResult.Ok(ResultCode.Taken, pile.Id, (ResultArg.Resource, resource), (ResultArg.Amount, ActionResult.Number(amount)));
        }

        /// <summary>Converts the cat's offerable cargo at the shrine at the shared offering rate.</summary>
        private ActionResult ShrineOffering(Village v, Cat c)
        {
            if (!AtShrine(v, c.Position))
                return ActionResult.Fail(ResultCode.GoToShrine, "Carry the offering to the shrine", (ResultArg.Target, v.Center.ToString()));
            var offered = c.Cargo.Where(s => OfferableResources.Contains(s.Resource) && s.Amount > 0).ToArray();
            if (offered.Length == 0)
                return ActionResult.Fail(ResultCode.NothingToOffer, "Only food, herbs and materials can be offered");
            double amount = offered.Sum(s => s.Amount);
            foreach (var s in offered)
                c.Cargo.Remove(s);
            double blessings = ConvertOffering(v, amount);
            return ActionResult.Ok(ResultCode.Offered, c.Id, (ResultArg.Amount, ActionResult.Number(amount)), (ResultArg.Blessings, ActionResult.Number(blessings)));
        }
    }
}
