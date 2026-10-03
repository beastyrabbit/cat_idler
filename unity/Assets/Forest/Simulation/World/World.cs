using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>Authoritative, render-free aggregate. All time and entropy are explicit and saved.</summary>
    [Serializable]
    public partial class World
    {
        public const double SimulationStepSeconds = 0.05;
        // Requested elapsed time remains TimeSeconds; this cursor persists consumed actor time.
        public double SimulationTimeSeconds; public bool ContinuousClockInitialized;
        [NonSerialized] private Dictionary<Cat, double> actorTime;
        public int FormatVersion = 1; public uint Seed, RandomState; public long EpochUnixMs, NextId = 1; public double TimeSeconds, PendingSeconds;
        public List<Village> Villages = new List<Village>(); public List<Player> Players = new List<Player>();
        public List<Tile> Tiles = new List<Tile>(); public List<TradeOffer> TradeOffers = new List<TradeOffer>(); public List<Reservation> Reservations = new List<Reservation>();
        [NonSerialized] private Dictionary<Int2, Tile> tileIndex;
        [NonSerialized] private long pathSearchCount;
        public static World Create(int seed, long nowUnixMs = 0)
        {
            var w = new World { Seed = unchecked((uint)seed), RandomState = unchecked((uint)seed), EpochUnixMs = nowUnixMs, GenerationVersion = 1 };
            w.Villages.Add(w.Found("communal", "Grand Commons", "", new Int2(0, 0), true));
            w.EnsureVillageDungeon(w.Villages[0]);
            return w;
        }
        public string Id(string prefix) => prefix + "-" + (NextId++).ToString(System.Globalization.CultureInfo.InvariantCulture);
        public uint Random()
        {
            RandomState = unchecked(RandomState * 1664525u + 1013904223u);
            return RandomState;
        }
        public double Roll() => Random() / 4294967296.0;
        /// <summary>
        /// A uniform integer in [0, n) from one draw. Uses the high bits: the LCG's low bits have
        /// short periods, so <c>Random() % n</c> correlates consecutive draws.
        /// </summary>
        public int RandomInt(int n)
        {
            if (n <= 0)
                throw new ArgumentOutOfRangeException(nameof(n));
            return (int)(((ulong)Random() * (uint)n) >> 32);
        }
        public static uint Hash(string value)
        {
            uint h = 2166136261;
            foreach (char c in value)
                h = unchecked((h ^ c) * 16777619);
            return h;
        }
        public Village Village(string id) => Villages.Find(v => v.Id == id);
        public Cat Cat(string id) => Villages.SelectMany(v => v.Cats).FirstOrDefault(c => c.Id == id);
        public void Step(double seconds)
        {
            if (!Finite(seconds) || seconds < 0 || seconds > 86400 * 366)
                throw new ArgumentOutOfRangeException(nameof(seconds));
            if (seconds == 0)
                return;
            if (!ContinuousClockInitialized)
            {
                // Missing fields identify an old save. Never replay its already elapsed time.
                SimulationTimeSeconds = TimeSeconds;
                ContinuousClockInitialized = true;
            }
            double end = Math.Round(TimeSeconds + seconds, 9);
            double next = Math.Round((Math.Floor(SimulationTimeSeconds / SimulationStepSeconds + 1e-8) + 1) * SimulationStepSeconds, 9);
            if (actorTime == null)
                actorTime = new Dictionary<Cat, double>();
            while (next <= end + 1e-9)
            {
                double delta = next - SimulationTimeSeconds;
                TimeSeconds = SimulationTimeSeconds = next;
                bool planning = Math.Abs(next - Math.Round(next)) < 1e-8;
                actorTime.Clear();
                foreach (var v in Villages)
                    foreach (var c in v.Cats)
                        if (c.Alive)
                            actorTime[c] = delta;
                // Each phase is fault-isolated: one failing village, job or trade never stops the world.
                // No delegates here: this runs twenty times per simulated second.
                faultActor = null;
                try
                {
                    TickControls(delta);
                }
                catch (Exception e)
                {
                    RecoverFault("controls", null, e);
                }
                faultActor = null;
                try
                {
                    TickDungeons(delta, planning);
                }
                catch (Exception e)
                {
                    RecoverFault("dungeons", null, e);
                }
                if (planning)
                {
                    faultActor = null;
                    try
                    {
                        Ecology();
                    }
                    catch (Exception e)
                    {
                        RecoverFault("ecology", null, e);
                    }
                }
                foreach (var v in Villages.ToArray())
                {
                    faultActor = null;
                    try
                    {
                        TickVillage(v, delta, planning);
                    }
                    catch (Exception e)
                    {
                        RecoverFault("village", v, e);
                    }
                }
                faultActor = null;
                try
                {
                    TickTrades(delta, planning);
                }
                catch (Exception e)
                {
                    RecoverFault("trades", null, e);
                }
                faultActor = null;
                next = Math.Round(next + SimulationStepSeconds, 9);
            }
            TimeSeconds = end;
            PendingSeconds = TimeSeconds - Math.Floor(TimeSeconds);
        }
        private double ActorTime(Cat c)
        {
            // Whoever spends actor time is acting now; a fault in this phase is attributed to them.
            faultActor = c;
            return actorTime != null && actorTime.TryGetValue(c, out var remaining) ? remaining : 0;
        }
        private void UseActorTime(Cat c, double seconds)
        {
            faultActor = c;
            if (actorTime != null)
                actorTime[c] = Math.Max(0, ActorTime(c) - seconds);
        }
        private double SpendActorTime(Cat c)
        {
            double remaining = ActorTime(c);
            UseActorTime(c, remaining);
            return remaining;
        }
        public bool CanControl(PlayerContext context, Village village) => context != null && !string.IsNullOrEmpty(context.PlayerId) && village != null && (village.Communal || village.OwnerId == context.PlayerId);
        /// <summary>Every invariant breach: <see cref="ValidateFatal"/> followed by <see cref="ValidateRepairable"/>.</summary>
        public List<string> Validate()
        {
            var errors = ValidateFatal();
            errors.AddRange(ValidateRepairable());
            return errors;
        }
        /// <summary>
        /// Breaches <see cref="RepairInvariants"/> can fix without creating or destroying goods: over-claimed
        /// reservations, double work owners and overbooked beds.
        /// </summary>
        public List<string> ValidateRepairable()
        {
            var errors = new List<string>();
            foreach (var v in Villages)
            {
                foreach (var pile in v.Stockpiles)
                    foreach (var s in pile.Goods)
                        if (Reservations.Where(r => r.PileId == pile.Id && r.Resource == s.Resource).Sum(r => r.Amount) > s.Amount + 1e-6)
                            errors.Add("Overclaimed " + pile.Id);
                foreach (var bed in v.Buildings.Where(b => b.Kind == "den" && b.Completed))
                    if (v.Cats.Count(c => c.Alive && c.BedId == bed.Id) > DenCapacity(v))
                        errors.Add("Overbooked bed " + bed.Id);
                foreach (var c in v.Cats.Where(c => c.Alive))
                    if (v.Jobs.Count(j => !j.Completed && j.CatId == c.Id) > 1)
                        errors.Add("Double work owner " + c.Id);
            }
            return errors;
        }
        /// <summary>
        /// Repairs every <see cref="ValidateRepairable"/> breach and logs one <see cref="EventKind.Repaired"/>
        /// event per repair. Over-claims lose their newest reservations, extra jobs of a double owner are
        /// unassigned (kept for another worker), and cats beyond a den's capacity lose that bed. Returns
        /// what was repaired.
        /// </summary>
        public List<string> RepairInvariants()
        {
            var repaired = new List<string>();
            foreach (var v in Villages)
            {
                foreach (var pile in v.Stockpiles)
                    foreach (var s in pile.Goods)
                    {
                        double excess = Reservations.Where(r => r.PileId == pile.Id && r.Resource == s.Resource).Sum(r => r.Amount) - s.Amount;
                        if (excess <= 1e-6)
                            continue;
                        for (int i = Reservations.Count - 1; i >= 0 && excess > 1e-9; i--)
                        {
                            var claim = Reservations[i];
                            if (claim.PileId != pile.Id || claim.Resource != s.Resource)
                                continue;
                            double cut = Math.Min(claim.Amount, excess);
                            claim.Amount -= cut;
                            excess -= cut;
                            if (claim.Amount <= 1e-9)
                                Reservations.RemoveAt(i);
                        }
                        repaired.Add("Overclaimed " + pile.Id);
                        NoteEvent(v, EventKind.Repaired, "Released over-claimed " + s.Resource.Replace('_', ' ') + " at a store", pile.Id, null, "", RepairArgs("overclaimed", ("resource", s.Resource)));
                    }
                int capacity = DenCapacity(v);
                foreach (var bed in v.Buildings.Where(b => b.Kind == "den" && b.Completed))
                {
                    var sleepers = v.Cats.Where(c => c.Alive && c.BedId == bed.Id).ToList();
                    if (sleepers.Count <= capacity)
                        continue;
                    foreach (var extra in sleepers.Skip(capacity))
                        extra.BedId = "";
                    repaired.Add("Overbooked bed " + bed.Id);
                    NoteEvent(v, EventKind.Repaired, "A crowded den freed " + (sleepers.Count - capacity).ToString(System.Globalization.CultureInfo.InvariantCulture) + " beds", bed.Id, null, "", RepairArgs("overbooked_bed"));
                }
                foreach (var c in v.Cats.Where(c => c.Alive))
                {
                    var owned = v.Jobs.Where(j => !j.Completed && j.CatId == c.Id).ToList();
                    if (owned.Count <= 1)
                        continue;
                    var keep = owned.Find(j => j.Id == c.JobId) ?? owned[0];
                    foreach (var extra in owned.Where(j => !ReferenceEquals(j, keep)))
                    {
                        extra.CatId = "";
                        extra.BlockedReason = Blocked.WorkerRequired;
                    }
                    c.JobId = keep.Id;
                    repaired.Add("Double work owner " + c.Id);
                    NoteEvent(v, EventKind.Repaired, c.Name + " kept one task and released " + (owned.Count - 1).ToString(System.Globalization.CultureInfo.InvariantCulture) + " others", c.Id, null, "", RepairArgs("double_work_owner"));
                }
            }
            return repaired;
        }
        private static Dictionary<string, string> RepairArgs(string repair, params (string key, string value)[] extra)
        {
            var args = new Dictionary<string, string> { { "repair", repair } };
            foreach (var (key, value) in extra)
                args[key] = value ?? "";
            return args;
        }
        /// <summary>
        /// Breaches nothing can repair without inventing state: unsupported generation, a non-finite or
        /// inconsistent clock, invalid positions, duplicate identities and invalid goods.
        /// </summary>
        public List<string> ValidateFatal()
        {
            var errors = new List<string>();
            if (GenerationVersion < 0 || GenerationVersion > 2)
                errors.Add("Unsupported world generation version");
            if (!Finite(TimeSeconds) || TimeSeconds < 0)
                errors.Add("Invalid clock");
            if (ContinuousClockInitialized && (!Finite(SimulationTimeSeconds) || SimulationTimeSeconds < 0 || SimulationTimeSeconds > TimeSeconds + 1e-9 || TimeSeconds - SimulationTimeSeconds >= SimulationStepSeconds + 1e-8))
                errors.Add("Invalid simulation cursor");
            foreach (var trade in TradeOffers)
                if (trade.HasContinuousPosition && !ValidContinuousPosition(trade.Position, trade.X, trade.Z))
                    errors.Add("Invalid caravan position " + trade.Id);
            var ids = Villages.SelectMany(v => v.Cats.Select(c => c.Id).Concat(v.Buildings.Select(b => b.Id)).Concat(v.Stockpiles.Select(s => s.Id)).Concat(v.Items.Select(i => i.Id)).Concat(v.Jobs.Select(j => j.Id))).ToList();
            if (ids.Distinct().Count() != ids.Count)
                errors.Add("Duplicate entity identity");
            foreach (var v in Villages)
            {
                foreach (var vehicle in v.Vehicles)
                    if (vehicle.HasContinuousPosition && !ValidContinuousPosition(vehicle.Position, vehicle.X, vehicle.Z))
                        errors.Add("Invalid vehicle position " + vehicle.Id);
                foreach (var raid in v.Raids)
                    if (raid.HasContinuousPosition && !ValidContinuousPosition(raid.Position, raid.X, raid.Z))
                        errors.Add("Invalid raid position " + raid.Id);
                if (v.Trader.HasContinuousPosition && !ValidContinuousPosition(v.Trader.Position, v.Trader.X, v.Trader.Z))
                    errors.Add("Invalid merchant position " + v.Trader.Id);
                foreach (var pile in v.Stockpiles)
                    if (pile.Goods.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid stock " + pile.Id);
                foreach (var c in v.Cats)
                    if (c.Cargo.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid cargo " + c.Id);
            }
            return errors;
        }

        // ---- Fault isolation -------------------------------------------------------------------

        /// <summary>The cat acting when a step phase threw; set by actor time, movement and <see cref="BeginActor"/>.</summary>
        [NonSerialized] private Cat faultActor;
        [NonSerialized] private List<string> isolatedFaults;
        /// <summary>Most isolated fault descriptions kept in memory for diagnostics.</summary>
        public const int IsolatedFaultCap = 100;

        /// <summary>
        /// Faults caught by <see cref="Step"/> in this session (never saved), oldest first. Tests and hosts
        /// read this: a scenario that expects no fault should assert it is empty, and hosts should log it.
        /// </summary>
        public IReadOnlyList<string> IsolatedFaults() => isolatedFaults ?? (IReadOnlyList<string>)Array.Empty<string>();

        /// <summary>Marks <paramref name="c"/> as the acting cat, so a fault before it spends time is attributed to it.</summary>
        private void BeginActor(Cat c) => faultActor = c;
        /// <summary>Clears the acting cat after a per-cat loop, so later village systems are not blamed on it.</summary>
        private void EndActor() => faultActor = null;

        private void RecoverFault(string phase, Village scope, Exception error)
        {
            var actor = faultActor;
            faultActor = null;
            if (isolatedFaults == null)
                isolatedFaults = new List<string>();
            if (isolatedFaults.Count >= IsolatedFaultCap)
                isolatedFaults.RemoveAt(0);
            isolatedFaults.Add(TimeSeconds.ToString("0.###", System.Globalization.CultureInfo.InvariantCulture) + " " + phase + (scope == null ? "" : " " + scope.Id) + (actor == null ? "" : " " + actor.Id) + ": " + error.GetType().Name + ": " + error.Message);
            var home = actor != null ? Village(actor.VillageId) ?? scope : scope;
            bool cancelled = false;
            if (actor != null && home != null)
            {
                try
                {
                    CancelFaultedWork(home, actor);
                    cancelled = true;
                }
                catch (Exception inner)
                {
                    isolatedFaults.Add("recovery " + actor.Id + ": " + inner.GetType().Name + ": " + inner.Message);
                }
            }
            try
            {
                RepairInvariants();
            }
            catch (Exception inner)
            {
                isolatedFaults.Add("repair: " + inner.GetType().Name + ": " + inner.Message);
            }
            foreach (var v in home != null ? new[] { home } : Villages.ToArray())
                NoteFault(v, actor != null && ReferenceEquals(home, v) ? actor : null, phase, error, cancelled);
        }

        /// <summary>
        /// Cancels the faulting cat's work like an interruption (cargo spilled, reservations released) and
        /// completes a job that would otherwise wait for the next worker, so the fault cannot repeat. An
        /// explorer is brought back to its dungeon's surface entrance first.
        /// </summary>
        private void CancelFaultedWork(Village v, Cat c)
        {
            bool Corrupt(Stack s) => s == null || !Finite(s.Amount) || s.Amount < 0;
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            // Invalid stacks cannot be spilled and would fault the recovery too; they hold no real goods.
            c.Cargo.RemoveAll(Corrupt);
            if (job != null)
                job.Local.RemoveAll(Corrupt);
            if (c.Position.Level != 0)
            {
                var site = Dungeons.Find(d => d.Id == c.DungeonId);
                var surface = site?.Entrance ?? v.Center;
                surface = new Int2(surface.X, surface.Z, 0);
                c.Position = surface;
                c.X = surface.X;
                c.Z = surface.Z;
                c.Y = WalkHeight(surface);
                c.HasHeight = true;
            }
            CancelWork(v, c);
            if (job != null && !job.Completed)
            {
                Release(job.Id);
                Release(job.Id + ":resume");
                if (job.Local.Count > 0)
                    Spill(v, job.Position, job.Local);
                job.Completed = true;
                job.CatId = "";
            }
            c.BlockedReason = "";
        }

        private void NoteFault(Village v, Cat actor, string phase, Exception error, bool cancelled)
        {
            string entity = actor?.Id ?? "";
            for (int i = v.Events.Count - 1; i >= 0; i--)
            {
                var e = v.Events[i];
                if (TimeSeconds - e.Time >= ErrorRepeatSeconds)
                    break;
                if (e.Kind == EventKind.Error && e.EntityId == entity)
                    return;
            }
            string text = actor == null ? "Part of the colony stalled for a moment and carried on" : actor.Name + (cancelled ? " dropped a task that went wrong" : " ran into a problem");
            NoteEvent(v, EventKind.Error, text, entity, null, EventSeverity.Alert, new Dictionary<string, string> { { "phase", phase }, { "exception", error.GetType().Name }, { "cancelled", cancelled ? "true" : "false" } });
        }
        private static bool ValidContinuousPosition(Int2 anchor, double x, double z) => Finite(x) && Finite(z) && Math.Abs(x - anchor.X) + Math.Abs(z - anchor.Z) <= 1 + 1e-8 && (Math.Abs(x - anchor.X) < 1e-8 || Math.Abs(z - anchor.Z) < 1e-8);
    }
}
