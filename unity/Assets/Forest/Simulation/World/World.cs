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
        private void Note(Village v, string kind, string text, string id = "")
        {
            v.Events.Add(new Event { Time = TimeSeconds, Kind = kind, Text = text, EntityId = id, Seq = TakeEventSeq() });
            if (v.Events.Count > 100)
                v.Events.RemoveAt(0);
        }
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
                TickControls(delta);
                TickDungeons(delta, planning);
                if (planning)
                    Ecology();
                foreach (var v in Villages.ToArray())
                    TickVillage(v, delta, planning);
                TickTrades(delta, planning);
                next = Math.Round(next + SimulationStepSeconds, 9);
            }
            TimeSeconds = end;
            PendingSeconds = TimeSeconds - Math.Floor(TimeSeconds);
        }
        private double ActorTime(Cat c) => actorTime != null && actorTime.TryGetValue(c, out var remaining) ? remaining : 0;
        private void UseActorTime(Cat c, double seconds)
        {
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
        public List<string> Validate()
        {
            var errors = new List<string>();
            if (GenerationVersion < 0 || GenerationVersion > 1)
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
                {
                    if (pile.Goods.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid stock " + pile.Id);
                    foreach (var s in pile.Goods)
                        if (Reservations.Where(r => r.PileId == pile.Id && r.Resource == s.Resource).Sum(r => r.Amount) > s.Amount + 1e-6)
                            errors.Add("Overclaimed " + pile.Id);
                }
                foreach (var c in v.Cats)
                    if (c.Cargo.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid cargo " + c.Id);
                foreach (var bed in v.Buildings.Where(b => b.Kind == "den" && b.Completed))
                    if (v.Cats.Count(c => c.Alive && c.BedId == bed.Id) > DenCapacity(v))
                        errors.Add("Overbooked bed " + bed.Id);
                foreach (var c in v.Cats.Where(c => c.Alive))
                    if (v.Jobs.Count(j => !j.Completed && j.CatId == c.Id) > 1)
                        errors.Add("Double work owner " + c.Id);
            }
            return errors;
        }
        private static bool ValidContinuousPosition(Int2 anchor, double x, double z) => Finite(x) && Finite(z) && Math.Abs(x - anchor.X) + Math.Abs(z - anchor.Z) <= 1 + 1e-8 && (Math.Abs(x - anchor.X) < 1e-8 || Math.Abs(z - anchor.Z) < 1e-8);
    }
}
