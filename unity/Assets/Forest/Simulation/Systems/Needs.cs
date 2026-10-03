using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // PACING-HANDOFF NeedStagger: per-cat seeded needs, metabolism and threshold jitter. Classic false, demo true.
        private bool NeedStagger => Pace.Profile == Pacing.DemoProfile;
        // PACING-HANDOFF NeedSeedMin / NeedSeedMax: range of the first-tick seeded Hunger, Thirst and Rest (55 / 100).
        private const double NeedSeedMin = 55, NeedSeedMax = 100;
        // PACING-HANDOFF DeliveryLegFinishTiles: a critical need waits for a delivery leg shorter than this (15).
        private const int DeliveryLegFinishTiles = 15;
        // PACING-HANDOFF NeedBackoffMaxDoublings: the need retry delay doubles at most this often (4, so 3 s to 48 s).
        private const int NeedBackoffMaxDoublings = 4;
        // PACING-HANDOFF NeedSourceSearchLimit: reachable-route checks per need resource before the next one (4).
        private const int NeedSourceSearchLimit = 4;

        private static readonly string[] DrinkServings = { "water", "brew" };
        private static readonly string[] EatServings = { "food", "fish", "preserves" };
        private static readonly string[] HealServings = { "herbs", "medicine" };

        /// <summary>True while the cat is on a need trip (drinking, eating, healing or sleeping); such a cat takes no orders.</summary>
        public static bool BusyWithNeed(Cat c) => c != null && c.Goal != null && c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal);

        /// <summary>Rest recovered per second of sleep at the cat's own den, for AI and directed sleep alike.</summary>
        public double RestRate(Village v) => Pace.SleepRecoveryPerSecond * Service(v, "beds", "restRecovery", 1);

        /// <summary>
        /// The level below which this cat starts the given need trip: the profile threshold, the cat's own
        /// jitter in the demo profile and, for sleep, the night bias.
        /// </summary>
        public double NeedThreshold(Cat c, string need)
        {
            switch (need)
            {
                case GoalKey.NeedDrink:
                    return Pace.DrinkThreshold + NeedJitter(c, "drink");
                case GoalKey.NeedEat:
                    return Pace.EatThreshold + NeedJitter(c, "eat");
                case GoalKey.NeedHeal:
                    return Pace.HealThreshold;
                case GoalKey.NeedSleep:
                    return Pace.SleepThreshold + NeedJitter(c, "sleep") + DayClock.SleepThresholdBias(this);
                default:
                    return 0;
            }
        }

        private double NeedJitter(Cat c, string need) => NeedStagger ? (Unit(c.Id + ":jitter:" + need) * 2 - 1) * Pace.NeedThresholdJitter : 0;

        /// <summary>A well-mixed value in [0, 1) derived from a string; pure, so it never draws from the world RNG.</summary>
        private static double Unit(string key)
        {
            uint h = Hash(key);
            h ^= h >> 16;
            h = unchecked(h * 0x7feb352du);
            h ^= h >> 15;
            h = unchecked(h * 0x846ca68bu);
            h ^= h >> 16;
            return h / 4294967296.0;
        }

        private long TickIndex => (long)Math.Round(TimeSeconds / SimulationStepSeconds);

        /// <summary>Runs or starts a need trip. Returns true while the trip consumes this cat's step.</summary>
        private bool TickNeeds(Village v, Cat c, double dt)
        {
            // Stay aboard over water; the same needs can be met physically after docking.
            if (v.Routes.Any(r => r.CatId == c.Id && r.Mode == "shipping" && r.Phase != "boarding" && v.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.Position.Equals(c.Position) && (!Walkable(v, vehicle.Position) || vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9))))
            {
                if (BusyWithNeed(c))
                    Resume(c);
                return false;
            }
            SeedLife(c);
            if (BusyWithNeed(c))
                return c.Goal == GoalKey.NeedSleep ? TickSleep(v, c, dt) : TickServing(v, c, dt);
            if (TimeSeconds < c.NextNeedAttemptAt)
                return false;
            if (c.Cargo.Count > 0 && !PrepareCargoForNeed(v, c))
                return false;
            string need = NextNeed(c);
            if (need == "")
                return false;
            c.ResumeJobId = c.JobId;
            c.Goal = need;
            c.NeedSourceId = "";
            ClearNeedSpot(c);
            // Keep moving work recoverable until a real need destination is selected.
            return true;
        }

        /// <summary>
        /// Demo profile, first life tick: a cat that has never lived (every need still full) starts with
        /// its own Hunger, Thirst and Rest, and every cat gets its own metabolism, so the colony does not
        /// get thirsty in one tick.
        /// </summary>
        private void SeedLife(Cat c)
        {
            if (c.LifeSeeded || !NeedStagger)
                return;
            c.LifeSeeded = true;
            // Full needs (less this tick's decay, which runs first) mean a cat that has never lived.
            if (c.Hunger > 99 && c.Thirst > 99 && c.Rest > 99)
            {
                c.Hunger = NeedSeedMin + (NeedSeedMax - NeedSeedMin) * Unit(c.Id + ":seed:hunger");
                c.Thirst = NeedSeedMin + (NeedSeedMax - NeedSeedMin) * Unit(c.Id + ":seed:thirst");
                c.Rest = NeedSeedMin + (NeedSeedMax - NeedSeedMin) * Unit(c.Id + ":seed:rest");
            }
            c.Metabolism = Pace.MetabolismMin + (Pace.MetabolismMax - Pace.MetabolismMin) * Unit(c.Id + ":metabolism");
        }

        private string NextNeed(Cat c)
        {
            if (c.Thirst < NeedThreshold(c, GoalKey.NeedDrink))
                return GoalKey.NeedDrink;
            if (c.Hunger < NeedThreshold(c, GoalKey.NeedEat))
                return GoalKey.NeedEat;
            if (c.Health < NeedThreshold(c, GoalKey.NeedHeal))
                return GoalKey.NeedHeal;
            if (c.BedId != "" && c.Rest < NeedThreshold(c, GoalKey.NeedSleep))
                return GoalKey.NeedSleep;
            return "";
        }

        /// <summary>
        /// A cat carrying work cargo keeps working until a need turns critical. Then it finishes a short
        /// delivery leg first; otherwise it plans to put the cargo into the nearest storage that takes it
        /// (spilling only when none does). True when a need trip may start now.
        /// </summary>
        private bool PrepareCargoForNeed(Village v, Cat c)
        {
            if (c.Thirst >= Pace.CargoKeepThirst && c.Hunger >= Pace.CargoKeepHunger && c.Health >= Pace.CargoKeepHealth)
                return false;
            bool providesNeed = c.Thirst < Pace.CargoKeepThirst && Amount(c.Cargo, "water") > 0 || c.Thirst >= Pace.CargoKeepThirst && c.Hunger < Pace.CargoKeepHunger && c.Cargo.Any(s => EatServings.Contains(s.Resource));
            if (providesNeed)
                return true;
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job != null && c.BlockedReason == "" && job.BlockedReason == "" && DeliveryLegRemaining(v, c, job) < DeliveryLegFinishTiles)
                return false;
            var store = DepositStore(v, c);
            if (store != null)
                c.NeedDepositPileId = store.Id;
            else
                StashCargo(v, c, null);
            return true;
        }

        /// <summary>Tiles left on the current delivery leg, or int.MaxValue when the cat is not delivering.</summary>
        private int DeliveryLegRemaining(Village v, Cat c, Job job)
        {
            Int2 target;
            if (job.Phase == JobPhase.InputDelivery)
                target = job.Position;
            else if (job.Phase == JobPhase.OutputDelivery)
            {
                // Before the job picks its store, estimate the leg to the nearest store with room.
                var cargo = c.Cargo[0];
                var pile = v.Stockpiles.Find(s => s.Id == job.DeliveryPileId)
                    ?? v.Stockpiles.Where(s => s.Kind == "storage" && HasRoom(v, s, cargo.Resource, cargo.Amount)).OrderBy(s => Int2.Distance(s.Position, c.Position)).ThenBy(s => s.Id, StringComparer.Ordinal).FirstOrDefault();
                if (pile == null)
                    return int.MaxValue;
                target = pile.Position;
            }
            else
                return int.MaxValue;
            if (c.Path.Count > 0 && c.Path[c.Path.Count - 1].Equals(target))
                return c.Path.Count;
            return Int2.Distance(c.Position, target);
        }

        /// <summary>The nearest reachable storage with room for all of the cat's cargo.</summary>
        private Stockpile DepositStore(Village v, Cat c)
        {
            double total = c.Cargo.Sum(s => s.Amount);
            int tries = 0;
            foreach (var pile in v.Stockpiles.Where(s => s.Kind == "storage" && s.Position.Level == c.Position.Level && c.Cargo.All(stack => HasRoom(v, s, stack.Resource, total))).OrderBy(s => Int2.Distance(s.Position, c.Position)).ThenBy(s => s.Id, StringComparer.Ordinal))
            {
                if (tries++ >= NeedSourceSearchLimit)
                    break;
                if (RouteTo(v, c, PileSpot(v, pile, c)))
                    return pile;
            }
            return null;
        }

        /// <summary>Puts all work cargo into <paramref name="target"/>, or spills it here when null; the suspended job can reclaim it.</summary>
        private void StashCargo(Village v, Cat c, Stockpile target)
        {
            if (c.Cargo.Count == 0)
                return;
            var suspended = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            var quantities = c.Cargo.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
            Stockpile pile;
            if (target == null)
                pile = Spill(v, c.Position, c.Cargo);
            else
            {
                foreach (var s in c.Cargo)
                    Add(target.Goods, s.Resource, s.Amount);
                c.Cargo.Clear();
                pile = target;
            }
            if (suspended != null && pile != null)
            {
                suspended.SuspendedCargoPileId = pile.Id;
                foreach (var s in quantities)
                    Reservations.Add(new Reservation { OwnerId = suspended.Id + ":resume", VillageId = v.Id, PileId = pile.Id, Resource = s.Resource, Amount = s.Amount });
            }
        }

        /// <summary>Drink, eat or heal: walk to the source, linger there, then take and apply one serving.</summary>
        private bool TickServing(Village v, Cat c, double dt)
        {
            if (c.NeedDepositPileId != "" && !TickNeedDeposit(v, c, dt))
                return true;
            string[] servings = c.Goal == GoalKey.NeedDrink ? DrinkServings : c.Goal == GoalKey.NeedHeal ? HealServings : EatServings;
            // A serving already carried (work cargo that meets the need, or an old save) is taken where the cat stands.
            var carried = c.Cargo.Find(s => servings.Contains(s.Resource) && s.Amount > 1e-9);
            if (carried != null && !c.HasNeedSpot)
            {
                c.NeedSpot = c.Position;
                c.HasNeedSpot = true;
                c.NeedSourceId = "";
                c.NeedAtWater = false;
            }
            Stockpile pile = null;
            if (carried == null && !c.NeedAtWater)
            {
                pile = v.Stockpiles.Find(s => s.Id == c.NeedSourceId);
                string resource = pile == null ? null : NeedChoices(v, c).FirstOrDefault(choice => FreeServing(c, pile, choice));
                if (resource == null || !NeedPileKind(c, pile))
                {
                    // The source vanished or was emptied by someone else: search again without penalty.
                    pile = null;
                    c.NeedSourceId = "";
                    ClearNeedSpot(c);
                    if (!ChooseNeedSource(v, c, out pile))
                    {
                        c.BlockedReason = "need_supply_missing";
                        ScheduleNeedRetry(c);
                        Resume(c, preservePath: true);
                        return false;
                    }
                }
            }
            if (!WalkOpen(v, c, c.NeedSpot, dt))
                return !NeedRouteFailed(c);
            c.NeedBlockedSince = -1;
            if (c.NeedDwellUntil < 0)
            {
                c.NeedDwellUntil = TimeSeconds + NeedDwell(c);
                return true;
            }
            if (TimeSeconds + 1e-9 < c.NeedDwellUntil)
                return true;
            string taken;
            double dose;
            if (carried != null)
            {
                taken = carried.Resource;
                dose = Math.Min(c.Goal == GoalKey.NeedDrink ? NeedServingSize(c, taken) : Pace.NeedServing, carried.Amount);
                Add(c.Cargo, taken, -dose);
            }
            else if (c.NeedAtWater)
            {
                taken = "water";
                dose = NeedServingSize(c, taken);
            }
            else
            {
                taken = NeedChoices(v, c).FirstOrDefault(choice => FreeServing(c, pile, choice));
                if (taken == null)
                {
                    c.NeedSourceId = "";
                    ClearNeedSpot(c);
                    return true;
                }
                dose = NeedServingSize(c, taken);
                Add(pile.Goods, taken, -dose);
            }
            if (c.Goal == GoalKey.NeedDrink)
            {
                c.Thirst = Math.Min(100, c.Thirst + Pace.DrinkRestorePerServing * dose);
                if (taken == "brew")
                {
                    // Brew is followed by clean water at the nearest source.
                    c.PersonalBrewUsed = true;
                    c.NeedSourceId = "";
                    ClearNeedSpot(c);
                    return true;
                }
                c.PersonalBrewUsed = false;
            }
            else if (c.Goal == GoalKey.NeedHeal)
                c.Health = Math.Min(100, c.Health + Pace.HealRestorePerServing * dose * Service(v, "herb_garden", "herbMedicineEfficacy", 1));
            else
                c.Hunger = Math.Min(100, c.Hunger + Pace.EatRestorePerServing * dose);
            c.NeedBackoff = 0;
            Resume(c);
            return true;
        }

        /// <summary>Resources that meet the current need, best first.</summary>
        private string[] NeedChoices(Village v, Cat c) => c.Goal == GoalKey.NeedDrink ? (!c.PersonalBrewUsed && Available(v, "water") >= Pace.WaterServingAfterBrew ? new[] { "brew", "water" } : new[] { "water" }) : c.Goal == GoalKey.NeedHeal ? new[] { "medicine", "herbs" } : new[] { "fish", "preserves", "food" };

        /// <summary>Drink and medicine come from storage; food may also be eaten from spills and farm harvest piles.</summary>
        private static bool NeedPileKind(Cat c, Stockpile pile) => pile.Kind == "storage" || c.Goal == GoalKey.NeedEat && (pile.Kind == "spill" || pile.Kind == "farm_output");

        private bool FreeServing(Cat c, Stockpile pile, string resource) => Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount) >= NeedServingSize(c, resource) - 1e-9;

        /// <summary>Picks the nearest reachable source for the current need: a store, a spill or harvest pile for food, or open water to drink.</summary>
        private bool ChooseNeedSource(Village v, Cat c, out Stockpile chosen)
        {
            chosen = null;
            foreach (var choice in NeedChoices(v, c))
            {
                int tries = 0;
                foreach (var pile in v.Stockpiles.Where(s => s.Position.Level == c.Position.Level && NeedPileKind(c, s) && FreeServing(c, s, choice)).OrderBy(s => Int2.Distance(c.Position, s.Position)).ThenBy(s => s.Id, StringComparer.Ordinal))
                {
                    if (tries++ >= NeedSourceSearchLimit)
                        break;
                    var spot = PileSpot(v, pile, c);
                    if (!RouteTo(v, c, spot))
                        continue;
                    chosen = pile;
                    c.NeedSourceId = pile.Id;
                    c.NeedSpot = spot;
                    c.HasNeedSpot = true;
                    c.NeedAtWater = false;
                    return true;
                }
            }
            if (c.Goal != GoalKey.NeedDrink)
                return false;
            int edgeTries = 0;
            foreach (var edge in WaterEdges(v).OrderBy(p => Int2.Distance(c.Position, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
            {
                if (edgeTries++ >= NeedSourceSearchLimit)
                    break;
                if (!RouteTo(v, c, edge))
                    continue;
                c.NeedSourceId = "";
                c.NeedSpot = edge;
                c.HasNeedSpot = true;
                c.NeedAtWater = true;
                return true;
            }
            return false;
        }

        /// <summary>Dry, walkable, open tiles beside known open water, where a cat can drink.</summary>
        private List<Int2> WaterEdges(Village v)
        {
            var footprints = Footprints(v);
            var edges = new HashSet<Int2>();
            foreach (var p in v.Known)
            {
                if (p.Level != 0)
                    continue;
                var water = GetTile(p);
                if (water == null || !water.Water || water.Bridge)
                    continue;
                foreach (var n in Neighbors(p))
                {
                    var land = GetTile(n);
                    if (land != null && !land.Water && !footprints.Contains(n) && Walkable(v, n))
                        edges.Add(n);
                }
            }
            return edges.ToList();
        }

        /// <summary>Where a cat stands to use a pile: a building store's entrance, else a free tile beside the pile.</summary>
        private Int2 PileSpot(Village v, Stockpile pile, Cat c)
        {
            if (pile.ManagedBy != "")
            {
                var building = v.Buildings.Find(b => b.Id == pile.ManagedBy);
                if (building != null && building.HasEntrance)
                    return building.Entrance;
            }
            var footprints = Footprints(v);
            Int2? best = null;
            double bestScore = double.MaxValue;
            foreach (var p in Ring(pile.Position, pile.Width, pile.Depth))
            {
                if (footprints.Contains(p) || !Walkable(v, p) || TileAt(p).Water)
                    continue;
                int users = v.Cats.Count(o => o != c && o.Alive && BusyWithNeed(o) && o.HasNeedSpot && o.NeedSpot.Equals(p));
                double score = users * 100 + Int2.Distance(c.Position, p) + Unit(c.Id + ":spot:" + p) * 0.5;
                if (score < bestScore)
                {
                    bestScore = score;
                    best = p;
                }
            }
            return best ?? pile.Position;
        }

        /// <summary>Tiles orthogonally beside a rectangle, outside it.</summary>
        private static IEnumerable<Int2> Ring(Int2 at, int width, int depth)
        {
            for (int x = 0; x < width; x++)
            {
                yield return new Int2(at.X + x, at.Z - 1, at.Level);
                yield return new Int2(at.X + x, at.Z + depth, at.Level);
            }
            for (int z = 0; z < depth; z++)
            {
                yield return new Int2(at.X - 1, at.Z + z, at.Level);
                yield return new Int2(at.X + width, at.Z + z, at.Level);
            }
        }

        /// <summary>Walks to the deposit storage and puts the work cargo there; true once the cargo is stored (or spilled when the store is gone).</summary>
        private bool TickNeedDeposit(Village v, Cat c, double dt)
        {
            var store = v.Stockpiles.Find(s => s.Id == c.NeedDepositPileId && s.Kind == "storage");
            double total = c.Cargo.Sum(s => s.Amount);
            if (store == null || c.Cargo.Count == 0 || !c.Cargo.All(stack => HasRoom(v, store, stack.Resource, total)))
            {
                StashCargo(v, c, null);
                FinishDeposit(c);
                return true;
            }
            if (!c.HasNeedSpot)
            {
                c.NeedSpot = PileSpot(v, store, c);
                c.HasNeedSpot = true;
            }
            if (!WalkOpen(v, c, c.NeedSpot, dt))
            {
                if (c.BlockedReason != Blocked.BlockedRoute)
                {
                    c.NeedBlockedSince = -1;
                    return false;
                }
                if (c.NeedBlockedSince < 0)
                    c.NeedBlockedSince = TimeSeconds;
                if (TimeSeconds - c.NeedBlockedSince + 1e-9 < Pace.NeedTripBackoffSeconds)
                    return false;
                StashCargo(v, c, null);
                FinishDeposit(c);
                return true;
            }
            StashCargo(v, c, store);
            FinishDeposit(c);
            return true;
        }

        private static void FinishDeposit(Cat c)
        {
            c.NeedDepositPileId = "";
            ClearNeedSpot(c);
        }

        /// <summary>Sleep on the den's doorstep or a free tile beside the den until rested; wakes early for urgent thirst or hunger.</summary>
        private bool TickSleep(Village v, Cat c, double dt)
        {
            var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
            if (den == null || c.Thirst < Pace.CargoKeepThirst || c.Hunger < Pace.CargoKeepHunger)
            {
                Resume(c);
                return false;
            }
            if (!c.HasNeedSpot)
            {
                c.NeedSpot = SleepSpot(v, den, c);
                c.HasNeedSpot = true;
            }
            if (!WalkOpen(v, c, c.NeedSpot, dt))
                return !NeedRouteFailed(c);
            c.NeedBlockedSince = -1;
            c.Rest = Math.Min(100, c.Rest + SpendActorTime(c) * RestRate(v));
            if (c.Rest >= Pace.SleepWakeRest)
            {
                c.NeedBackoff = 0;
                Resume(c);
            }
            return true;
        }

        /// <summary>The den entrance, then free tiles around the den; one sleeper per tile while any is free, at most two.</summary>
        private Int2 SleepSpot(Village v, Building den, Cat c)
        {
            var footprints = Footprints(v);
            var anchor = den.HasEntrance ? den.Entrance : den.Position;
            var candidates = new List<Int2>();
            if (den.HasEntrance)
                candidates.Add(den.Entrance);
            candidates.AddRange(Ring(den.Position, den.Width, den.Depth).Where(p => !p.Equals(anchor)).OrderBy(p => Int2.Distance(p, anchor)).ThenBy(p => p.Z).ThenBy(p => p.X));
            var open = candidates.Where(p => !footprints.Contains(p) && Walkable(v, p) && !TileAt(p).Water).ToList();
            for (int limit = 1; limit <= 2; limit++)
                foreach (var p in open)
                    if (v.Cats.Count(o => o != c && o.Alive && o.Goal == GoalKey.NeedSleep && o.HasNeedSpot && o.NeedSpot.Equals(p)) < limit)
                        return p;
            return open.Count > 0 ? open[0] : anchor;
        }

        /// <summary>
        /// Handles a failed step toward the need spot. A route blocked for NeedTripBackoffSeconds ends the
        /// trip and schedules the next attempt with exponential backoff. True when the trip was aborted.
        /// </summary>
        private bool NeedRouteFailed(Cat c)
        {
            if (c.BlockedReason != Blocked.BlockedRoute)
            {
                c.NeedBlockedSince = -1;
                return false;
            }
            if (c.NeedBlockedSince < 0)
                c.NeedBlockedSince = TimeSeconds;
            if (TimeSeconds - c.NeedBlockedSince + 1e-9 < Pace.NeedTripBackoffSeconds)
                return false;
            ScheduleNeedRetry(c);
            Resume(c);
            return true;
        }

        /// <summary>Next attempt after a failed trip: NeedTripBackoffSeconds, doubling with each consecutive failure.</summary>
        private void ScheduleNeedRetry(Cat c)
        {
            int doublings = Math.Min(c.NeedBackoff, NeedBackoffMaxDoublings);
            c.NextNeedAttemptAt = Math.Floor(TimeSeconds) + Pace.NeedTripBackoffSeconds * (1 << doublings);
            if (c.NeedBackoff <= NeedBackoffMaxDoublings)
                c.NeedBackoff++;
        }

        private double NeedDwell(Cat c) => Pace.NeedDwellMin + (Pace.NeedDwellMax - Pace.NeedDwellMin) * Unit(c.Id + ":dwell:" + TickIndex.ToString(CultureInfo.InvariantCulture));

        /// <summary>One serving of a need resource: a small brew, a smaller water after a brew, or a whole unit.</summary>
        private double NeedServingSize(Cat c, string resource) => resource == "brew" ? Pace.BrewServing : resource == "water" && c.PersonalBrewUsed ? Pace.WaterServingAfterBrew : Pace.NeedServing;

        private static void ClearNeedSpot(Cat c)
        {
            c.HasNeedSpot = false;
            c.NeedAtWater = false;
            c.NeedDwellUntil = -1;
            c.NeedBlockedSince = -1;
        }

        /// <summary>Ends the need trip and returns to the suspended job, or keeps a job that was assigned meanwhile.</summary>
        private void Resume(Cat c, bool preservePath = false)
        {
            c.NeedSourceId = "";
            ClearNeedSpot(c);
            if (c.ResumeJobId != "" && (c.JobId == "" || c.JobId == c.ResumeJobId))
                c.JobId = c.ResumeJobId;
            c.ResumeJobId = "";
            if (!preservePath)
                c.Path.Clear();
            c.Goal = GoalKey.Idle;
        }

        // ---------------------------------------------------------------------------------------
        // Routes that keep cats out of building, stockpile and farm footprints.

        [NonSerialized] private Dictionary<string, (double At, HashSet<Int2> Tiles, HashSet<Int2> Entrances)> footprintCache;

        /// <summary>Tiles covered by buildings, stores and farms (the RoadSpace footprint); rebuilt once per tick.</summary>
        private HashSet<Int2> Footprints(Village v) => FootprintEntry(v).Tiles;

        private HashSet<Int2> Entrances(Village v) => FootprintEntry(v).Entrances;

        private (double At, HashSet<Int2> Tiles, HashSet<Int2> Entrances) FootprintEntry(Village v)
        {
            if (footprintCache == null)
                footprintCache = new Dictionary<string, (double, HashSet<Int2>, HashSet<Int2>)>();
            if (footprintCache.TryGetValue(v.Id, out var entry) && entry.At == TimeSeconds)
                return entry;
            var tiles = new HashSet<Int2>();
            void Cover(Int2 at, int width, int depth)
            {
                for (int x = 0; x < width; x++)
                    for (int z = 0; z < depth; z++)
                        tiles.Add(new Int2(at.X + x, at.Z + z, at.Level));
            }
            foreach (var b in v.Buildings)
                Cover(b.Position, b.Width, b.Depth);
            foreach (var s in v.Stockpiles.Where(s => s.Kind != "spill" && !s.Kind.StartsWith("zone_", StringComparison.Ordinal)))
                Cover(s.Position, s.Width, s.Depth);
            foreach (var f in v.Farms)
                Cover(f.Position, f.Width, f.Depth);
            var entrances = new HashSet<Int2>(v.Buildings.Where(b => b.HasEntrance).Select(b => b.Entrance));
            entry = (TimeSeconds, tiles, entrances);
            footprintCache[v.Id] = entry;
            return entry;
        }

        /// <summary>
        /// A route that never enters a footprint, except the footprint the cat starts in (leaving through
        /// the entrance) and the one it ends in. Null when no such route exists nearby.
        /// </summary>
        private List<Int2> OpenPath(Village v, Int2 start, Int2 end)
        {
            if (start.Level != 0 || end.Level != 0)
                return null;
            if (start.Equals(end))
                return new List<Int2>();
            var footprints = Footprints(v);
            var exempt = new List<(Int2 At, int W, int D)>();
            void Exempt(Int2 at, int w, int d)
            {
                if (Contains(at, w, d, start) || Contains(at, w, d, end))
                    exempt.Add((at, w, d));
            }
            foreach (var b in v.Buildings)
                Exempt(b.Position, b.Width, b.Depth);
            foreach (var s in v.Stockpiles)
                Exempt(s.Position, s.Width, s.Depth);
            foreach (var f in v.Farms)
                Exempt(f.Position, f.Width, f.Depth);
            bool Passable(Int2 p) => (!footprints.Contains(p) || exempt.Any(r => Contains(r.At, r.W, r.D, p))) && Walkable(v, p);
            if (!Passable(end))
                return null;
            var seen = new Dictionary<Int2, Int2> { { start, start } };
            var costs = new Dictionary<Int2, int> { { start, 0 } };
            int sequence = 0;
            var frontier = new SortedSet<(int F, int H, int Order, Int2 Position)>(Comparer<(int F, int H, int Order, Int2 Position)>.Create((a, b) => { int f = a.F.CompareTo(b.F); if (f != 0) return f; int h = a.H.CompareTo(b.H); return h != 0 ? h : a.Order.CompareTo(b.Order); }));
            frontier.Add((Int2.Distance(start, end), Int2.Distance(start, end), sequence++, start));
            int allowance = Int2.Distance(start, end) + 2 * v.Radius + 16;
            while (frontier.Count > 0 && seen.Count < 4096)
            {
                var current = frontier.Min;
                frontier.Remove(current);
                var at = current.Position;
                foreach (var n in Neighbors(at))
                {
                    int cost = costs[at] + 1;
                    if (costs.TryGetValue(n, out var previous) && previous <= cost || Int2.Distance(start, n) > allowance || !Passable(n) || !Crossable(at, n))
                        continue;
                    seen[n] = at;
                    costs[n] = cost;
                    if (n.Equals(end))
                    {
                        var result = new List<Int2>();
                        var step = n;
                        while (!step.Equals(start))
                        {
                            result.Add(step);
                            step = seen[step];
                        }
                        result.Reverse();
                        return result;
                    }
                    int h = Int2.Distance(n, end);
                    frontier.Add((cost + h, h, sequence++, n));
                }
            }
            return null;
        }

        /// <summary>
        /// True when the cat can reach <paramref name="spot"/>; an open route is kept as the cat's path
        /// when it stands on a tile centre, so the walk does not search again.
        /// </summary>
        private bool RouteTo(Village v, Cat c, Int2 spot)
        {
            if (c.Position.Equals(spot))
                return true;
            var open = OpenPath(v, c.Position, spot);
            if (open != null)
            {
                if (AtTileCentre(c))
                    c.Path = open;
                return true;
            }
            return Path(c.Position, spot, v) != null;
        }

        private static bool AtTileCentre(Cat c) => Math.Abs(c.X - c.Position.X) + Math.Abs(c.Z - c.Position.Z) < 1e-9;

        /// <summary>
        /// Moves like <c>Move</c>, but plans the route around footprints. A cat caught between tiles first
        /// steps back onto its tile, as <c>Move</c> does when the destination changes.
        /// </summary>
        private bool WalkOpen(Village v, Cat c, Int2 destination, double dt)
        {
            bool routed = c.Path.Count > 0 && c.Path[c.Path.Count - 1].Equals(destination);
            if (!routed && !c.Position.Equals(destination))
            {
                if (!AtTileCentre(c) && !Move(c, c.Position, dt))
                    return false;
                bool knownFailure = c.HasFailedPath && c.FailedPathStart.Equals(c.Position) && c.FailedPathDestination.Equals(destination) && TimeSeconds + 1e-9 < c.NextPathAttemptAt;
                if (!knownFailure)
                {
                    var open = OpenPath(v, c.Position, destination);
                    if (open != null && open.Count > 0)
                        c.Path = open;
                }
            }
            return Move(c, destination, dt);
        }
    }
}
