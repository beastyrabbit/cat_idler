using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;
using Event = IdleCatForest.Simulation.Event;

namespace IdleCatForest.Presentation
{
    /// <summary>A new entry in the selected village's event log.</summary>
    public readonly struct FeedNote
    {
        public readonly Village Village; public readonly Event Event;
        /// <summary>Where it happened: the entity named by <c>Event.EntityId</c> when it is a cat, building or raid, else the village centre.</summary>
        public readonly Vector3 Position; public readonly bool HasEntityPosition;
        public FeedNote(Village village, Event e, Vector3 position, bool hasEntityPosition) { Village = village; Event = e; Position = position; HasEntityPosition = hasEntityPosition; }
        public string Kind => Event.Kind;
    }

    /// <summary>A building that appeared, finished or disappeared. Position is the footprint centre on the ground.</summary>
    public readonly struct FeedBuilding
    {
        public readonly Building Building; public readonly string Id, Kind; public readonly Vector3 Position;
        public FeedBuilding(Building building, string id, string kind, Vector3 position) { Building = building; Id = id; Kind = kind; Position = position; }
    }

    public enum CargoChange { Picked, Dropped, Swapped }

    /// <summary>
    /// What a cat carries changed. <see cref="Resource"/>/<see cref="Amount"/> are the new load (empty when dropped),
    /// <see cref="PreviousResource"/>/<see cref="PreviousAmount"/> the old one. For a drop, <see cref="TargetId"/> is the
    /// storage pile or building site within reach of the cat (empty when none).
    /// </summary>
    public readonly struct FeedCargo
    {
        public readonly Cat Cat; public readonly CargoChange Change;
        public readonly string Resource, PreviousResource, TargetId; public readonly double Amount, PreviousAmount;
        public readonly Vector3 Position, TargetPosition;
        public FeedCargo(Cat cat, CargoChange change, string resource, double amount, string previousResource, double previousAmount, Vector3 position, string targetId, Vector3 targetPosition)
        { Cat = cat; Change = change; Resource = resource; Amount = amount; PreviousResource = previousResource; PreviousAmount = previousAmount; Position = position; TargetId = targetId; TargetPosition = targetPosition; }
    }

    /// <summary>A resource tile's amount changed (harvest, depletion, regrowth or replanting).</summary>
    public readonly struct FeedTile
    {
        public readonly Tile Tile; public readonly double Before, After; public readonly Vector3 Position;
        public FeedTile(Tile tile, double before, double after, Vector3 position) { Tile = tile; Before = before; After = after; Position = position; }
        public bool Depleted => Before > 0 && After <= 0;
        public bool Regrown => Before <= 0 && After > 0;
    }

    /// <summary>Kinds of <see cref="FeedHealth"/> subjects.</summary>
    public static class FeedSubject
    {
        public const string Cat = "cat";
        public const string Raid = "raid";
        public const string Creature = "creature";
    }

    /// <summary>
    /// Health moved by at least <see cref="WorldEventFeed.HealthStep"/> since it was last published, or reached zero.
    /// <see cref="Kind"/> is a <see cref="FeedSubject"/> value.
    /// </summary>
    public readonly struct FeedHealth
    {
        public readonly string Kind, Id; public readonly double Before, After; public readonly Vector3 Position;
        public FeedHealth(string kind, string id, double before, double after, Vector3 position) { Kind = kind; Id = id; Before = before; After = after; Position = position; }
        public double Delta => After - Before;
        public bool Died => Before > 0 && After <= 0;
    }

    /// <summary>A village counter (research points, blessings) moved by at least one whole unit.</summary>
    public readonly struct FeedDelta
    {
        public readonly double Before, After;
        public FeedDelta(double before, double after) { Before = before; After = after; }
        public double Delta => After - Before;
    }

    /// <summary>
    /// A cat joined or left the rendered colony, or its boost was toggled. For <see cref="WorldEventFeed.CatGone"/>,
    /// <see cref="Cat"/> is null when the cat was removed from the village (it left), and <see cref="Died"/> is true
    /// when it is still listed but no longer alive.
    /// </summary>
    public readonly struct FeedCat
    {
        public readonly Cat Cat; public readonly Vector3 Position; public readonly bool Died;
        public FeedCat(Cat cat, Vector3 position, bool died) { Cat = cat; Position = position; Died = died; }
    }

    /// <summary>
    /// Presentation-side diff of the selected village into typed C# events, so effects, audio and UI can react
    /// to what just happened without the simulation knowing about them. Observed once per view frame.
    /// It never publishes history: on the first observation, when the world or village changes, when time
    /// runs backwards (a load or restore) or jumps by more than <see cref="SilentJumpSeconds"/> (catch-up), it
    /// re-reads the state silently and raises only <see cref="Reseeded"/>. New log entries are found by
    /// <c>Event.Seq</c>, never by list index, because the log is capped and remote snapshots replace the world.
    /// </summary>
    public sealed class WorldEventFeed
    {
        /// <summary>Health changes smaller than this accumulate until they cross it.</summary>
        public const double HealthStep = 1;
        /// <summary>A step of more simulated seconds than this between two observations is treated as a catch-up.</summary>
        public const double SilentJumpSeconds = 30;
        /// <summary>Tiles are compared at this real-time interval (they are many and change rarely).</summary>
        public const float TileInterval = .2f;

        public event Action<FeedNote> Noted;
        public event Action<FeedBuilding> BuildingPlaced;
        public event Action<FeedBuilding> BuildingCompleted;
        public event Action<FeedBuilding> BuildingRemoved;
        public event Action<FeedCargo> CargoChanged;
        public event Action<FeedTile> TileAmountChanged;
        public event Action<FeedHealth> HealthChanged;
        public event Action<FeedDelta> ResearchChanged;
        public event Action<FeedDelta> BlessingsChanged;
        public event Action<FeedCat> CatAppeared;
        public event Action<FeedCat> CatGone;
        public event Action<FeedCat> CatBoostChanged;
        /// <summary>The feed re-read the world without publishing; the argument names why (load, world, village, rewind, jump).</summary>
        public event Action<string> Reseeded;

        private sealed class CatState
        {
            public string Resource = ""; public double Amount, Health, PublishedHealth; public bool Boosted; public int Seen;
        }

        private sealed class BuildingState
        {
            public bool Completed; public string Kind = ""; public Vector3 Position; public int Seen;
        }

        private sealed class HealthState { public double Published; public Vector3 Position; public int Seen; }

        private readonly Dictionary<string, CatState> cats = new Dictionary<string, CatState>();
        private readonly Dictionary<string, BuildingState> buildings = new Dictionary<string, BuildingState>();
        private readonly Dictionary<string, HealthState> raids = new Dictionary<string, HealthState>();
        private readonly Dictionary<string, HealthState> creatures = new Dictionary<string, HealthState>();
        private readonly List<string> removals = new List<string>();
        private readonly List<Event> fresh = new List<Event>();
        private double[] tileAmounts = new double[0];
        private Int2[] tilePositions = new Int2[0];
        private int tileCount;
        private bool seeded;
        private string villageId = "";
        private uint worldSeed; private long worldEpoch;
        private double lastTime, research, blessings;
        private long lastSeq;
        private int stamp;
        private float nextTiles;

        /// <summary>Publications since the feed was created, by event name (for telemetry and tests).</summary>
        public readonly Dictionary<string, int> Counts = new Dictionary<string, int>();

        /// <summary>Forgets everything; the next observation seeds silently.</summary>
        public void Reset() { seeded = false; }

        public void Observe(World world, Village village)
        {
            if (world == null || village == null) return;
            string reason = !seeded ? "load"
                : world.Seed != worldSeed || world.EpochUnixMs != worldEpoch ? "world"
                : village.Id != villageId ? "village"
                : world.TimeSeconds < lastTime - 1e-6 ? "rewind"
                : world.TimeSeconds - lastTime > SilentJumpSeconds ? "jump"
                : null;
            if (reason != null) { Seed(world, village); Reseeded?.Invoke(reason); Count("Reseeded"); return; }
            lastTime = world.TimeSeconds;
            stamp++;
            ObserveEvents(world, village);
            ObserveBuildings(world, village);
            ObserveCats(world, village);
            ObserveThreats(world, village);
            ObserveCounters(village);
            if (Time.unscaledTime >= nextTiles) { nextTiles = Time.unscaledTime + TileInterval; ObserveTiles(world, true); }
        }

        private void Seed(World world, Village village)
        {
            seeded = true;
            worldSeed = world.Seed; worldEpoch = world.EpochUnixMs; villageId = village.Id; lastTime = world.TimeSeconds;
            cats.Clear(); buildings.Clear(); raids.Clear(); creatures.Clear();
            lastSeq = 0;
            foreach (var e in village.Events) if (e.Seq > lastSeq) lastSeq = e.Seq;
            stamp++;
            foreach (var b in village.Buildings) buildings[b.Id] = new BuildingState { Completed = b.Completed, Kind = b.Kind, Position = Footprint(world, b), Seen = stamp };
            foreach (var c in village.Cats)
            {
                if (!c.Alive) continue;
                var load = Load(c);
                cats[c.Id] = new CatState { Resource = load.Resource, Amount = load.Amount, Health = c.Health, PublishedHealth = c.Health, Boosted = c.Boosted, Seen = stamp };
            }
            foreach (var r in village.Raids) raids[r.Id] = new HealthState { Published = r.Health, Position = RaidPosition(world, r), Seen = stamp };
            foreach (var m in world.Creatures) creatures[m.Id] = new HealthState { Published = m.Health, Position = CreaturePosition(world, m), Seen = stamp };
            research = village.ResearchPoints; blessings = village.Blessings;
            tileCount = 0;
            ObserveTiles(world, false);
            nextTiles = Time.unscaledTime + TileInterval;
        }

        private void ObserveEvents(World world, Village village)
        {
            var log = village.Events;
            if (log.Count == 0 || log[log.Count - 1].Seq <= lastSeq) return;
            fresh.Clear();
            for (int i = log.Count - 1; i >= 0 && log[i].Seq > lastSeq; i--) fresh.Add(log[i]);
            for (int i = fresh.Count - 1; i >= 0; i--)
            {
                var e = fresh[i];
                lastSeq = Math.Max(lastSeq, e.Seq);
                bool located = TryLocate(world, village, e.EntityId, out var position);
                if (!located) position = At(world, village.Center);
                Noted?.Invoke(new FeedNote(village, e, position, located));
                Count("Noted");
            }
            fresh.Clear();
        }

        private void ObserveBuildings(World world, Village village)
        {
            foreach (var b in village.Buildings)
            {
                if (!buildings.TryGetValue(b.Id, out var state))
                {
                    state = new BuildingState { Completed = b.Completed, Kind = b.Kind, Position = Footprint(world, b), Seen = stamp };
                    buildings[b.Id] = state;
                    BuildingPlaced?.Invoke(new FeedBuilding(b, b.Id, b.Kind, state.Position)); Count("BuildingPlaced");
                    if (b.Completed) { BuildingCompleted?.Invoke(new FeedBuilding(b, b.Id, b.Kind, state.Position)); Count("BuildingCompleted"); }
                    continue;
                }
                state.Seen = stamp;
                state.Kind = b.Kind;
                if (b.Completed && !state.Completed)
                {
                    state.Position = Footprint(world, b);
                    BuildingCompleted?.Invoke(new FeedBuilding(b, b.Id, b.Kind, state.Position)); Count("BuildingCompleted");
                }
                state.Completed = b.Completed;
            }
            removals.Clear();
            foreach (var pair in buildings) if (pair.Value.Seen != stamp) removals.Add(pair.Key);
            foreach (var id in removals)
            {
                var state = buildings[id]; buildings.Remove(id);
                BuildingRemoved?.Invoke(new FeedBuilding(null, id, state.Kind, state.Position)); Count("BuildingRemoved");
            }
        }

        private void ObserveCats(World world, Village village)
        {
            foreach (var c in village.Cats)
            {
                if (!c.Alive) continue;
                var position = CatPosition(world, c);
                var load = Load(c);
                if (!cats.TryGetValue(c.Id, out var state))
                {
                    cats[c.Id] = new CatState { Resource = load.Resource, Amount = load.Amount, Health = c.Health, PublishedHealth = c.Health, Boosted = c.Boosted, Seen = stamp };
                    CatAppeared?.Invoke(new FeedCat(c, position, false)); Count("CatAppeared");
                    continue;
                }
                state.Seen = stamp;
                if (load.Resource != state.Resource || Math.Abs(load.Amount - state.Amount) > 1e-6)
                {
                    bool had = state.Amount > 0, has = load.Amount > 0;
                    var change = had && !has ? CargoChange.Dropped : !had && has ? CargoChange.Picked : CargoChange.Swapped;
                    string target = ""; var targetPosition = position;
                    if (change == CargoChange.Dropped) FindDropTarget(world, village, c, ref target, ref targetPosition);
                    if (had || has)
                    {
                        CargoChanged?.Invoke(new FeedCargo(c, change, load.Resource, load.Amount, state.Resource, state.Amount, position, target, targetPosition));
                        Count("CargoChanged");
                    }
                    state.Resource = load.Resource; state.Amount = load.Amount;
                }
                if (c.Boosted != state.Boosted)
                {
                    state.Boosted = c.Boosted;
                    CatBoostChanged?.Invoke(new FeedCat(c, position, false)); Count("CatBoostChanged");
                }
                state.Health = c.Health;
                if (Math.Abs(c.Health - state.PublishedHealth) >= HealthStep || c.Health <= 0 && state.PublishedHealth > 0)
                {
                    HealthChanged?.Invoke(new FeedHealth(FeedSubject.Cat, c.Id, state.PublishedHealth, c.Health, position)); Count("HealthChanged");
                    state.PublishedHealth = c.Health;
                }
            }
            removals.Clear();
            foreach (var pair in cats) if (pair.Value.Seen != stamp) removals.Add(pair.Key);
            foreach (var id in removals)
            {
                cats.Remove(id);
                var cat = village.Cats.Find(c => c.Id == id);
                CatGone?.Invoke(new FeedCat(cat, cat != null ? CatPosition(world, cat) : At(world, village.Center), cat != null && !cat.Alive)); Count("CatGone");
            }
        }

        private void ObserveThreats(World world, Village village)
        {
            foreach (var r in village.Raids)
            {
                var position = RaidPosition(world, r);
                if (!raids.TryGetValue(r.Id, out var state)) { raids[r.Id] = new HealthState { Published = r.Health, Position = position, Seen = stamp }; continue; }
                state.Seen = stamp; state.Position = position;
                PublishHealth(FeedSubject.Raid, r.Id, state, r.Health);
            }
            Sweep(raids, FeedSubject.Raid);
            foreach (var m in world.Creatures)
            {
                var position = CreaturePosition(world, m);
                if (!creatures.TryGetValue(m.Id, out var state)) { creatures[m.Id] = new HealthState { Published = m.Health, Position = position, Seen = stamp }; continue; }
                state.Seen = stamp; state.Position = position;
                PublishHealth(FeedSubject.Creature, m.Id, state, m.Health);
            }
            Sweep(creatures, FeedSubject.Creature);
        }

        private void PublishHealth(string kind, string id, HealthState state, double health)
        {
            if (Math.Abs(health - state.Published) < HealthStep && !(health <= 0 && state.Published > 0)) return;
            HealthChanged?.Invoke(new FeedHealth(kind, id, state.Published, health, state.Position)); Count("HealthChanged");
            state.Published = health;
        }

        // A raid or creature that vanished with health left (killed between observations or removed) reports its end once.
        private void Sweep(Dictionary<string, HealthState> states, string kind)
        {
            removals.Clear();
            foreach (var pair in states) if (pair.Value.Seen != stamp) removals.Add(pair.Key);
            foreach (var id in removals)
            {
                var state = states[id]; states.Remove(id);
                if (state.Published > 0) { HealthChanged?.Invoke(new FeedHealth(kind, id, state.Published, 0, state.Position)); Count("HealthChanged"); }
            }
        }

        private void ObserveCounters(Village village)
        {
            if (Math.Abs(village.ResearchPoints - research) >= 1)
            { ResearchChanged?.Invoke(new FeedDelta(research, village.ResearchPoints)); Count("ResearchChanged"); research = village.ResearchPoints; }
            if (Math.Abs(village.Blessings - blessings) >= 1)
            { BlessingsChanged?.Invoke(new FeedDelta(blessings, village.Blessings)); Count("BlessingsChanged"); blessings = village.Blessings; }
        }

        private void ObserveTiles(World world, bool publish)
        {
            var tiles = world.Tiles;
            int count = tiles.Count;
            if (tileAmounts.Length < count)
            {
                int size = Math.Max(count, tileAmounts.Length * 2);
                Array.Resize(ref tileAmounts, size); Array.Resize(ref tilePositions, size);
            }
            for (int i = 0; i < count; i++)
            {
                var tile = tiles[i];
                double amount = tile.Amount;
                if (i >= tileCount || !tilePositions[i].Equals(tile.Position))
                {
                    // A tile generated since the last pass, or a list rebuilt by a remote snapshot: learn it silently.
                    tilePositions[i] = tile.Position; tileAmounts[i] = amount; continue;
                }
                double before = tileAmounts[i];
                if (Math.Abs(amount - before) <= 1e-9) continue;
                tileAmounts[i] = amount;
                if (!publish) continue;
                TileAmountChanged?.Invoke(new FeedTile(tile, before, amount, At(world, tile.Position))); Count("TileAmountChanged");
            }
            tileCount = count;
        }

        private void FindDropTarget(World world, Village village, Cat c, ref string target, ref Vector3 targetPosition)
        {
            double best = 9;
            foreach (var pile in village.Stockpiles)
            {
                if (pile.Position.Level != c.Position.Level) continue;
                double cx = pile.Position.X + (pile.Width - 1) * .5, cz = pile.Position.Z + (pile.Depth - 1) * .5;
                double d = (cx - c.X) * (cx - c.X) + (cz - c.Z) * (cz - c.Z);
                if (d < best) { best = d; target = pile.Id; targetPosition = new Vector3((float)cx, (float)world.WalkHeight(pile.Position), (float)cz); }
            }
            if (target != "") return;
            foreach (var b in village.Buildings)
            {
                if (b.Position.Level != c.Position.Level) continue;
                var centre = Footprint(world, b);
                double reach = Math.Max(b.Width, b.Depth) * .5 + 1.5;
                double d = (centre.x - c.X) * (centre.x - c.X) + (centre.z - c.Z) * (centre.z - c.Z);
                if (d < reach * reach && d < best) { best = d; target = b.Id; targetPosition = centre; }
            }
        }

        private static bool TryLocate(World world, Village village, string id, out Vector3 position)
        {
            position = default;
            if (string.IsNullOrEmpty(id)) return false;
            foreach (var c in village.Cats) if (c.Id == id) { position = CatPosition(world, c); return true; }
            foreach (var b in village.Buildings) if (b.Id == id) { position = Footprint(world, b); return true; }
            foreach (var r in village.Raids) if (r.Id == id) { position = RaidPosition(world, r); return true; }
            return false;
        }

        private static (string Resource, double Amount) Load(Cat c)
        {
            if (c.Cargo.Count == 0) return ("", 0);
            var stack = c.Cargo[0];
            return stack.Amount > 0 ? (stack.Resource ?? "", stack.Amount) : ("", 0);
        }

        private void Count(string name) { Counts.TryGetValue(name, out int n); Counts[name] = n + 1; }

        internal static Vector3 At(World world, Int2 p) => new Vector3(p.X, (float)world.WalkHeight(p), p.Z);
        internal static Vector3 CatPosition(World world, Cat c) => new Vector3((float)c.X, (float)(c.HasHeight ? c.Y : world.WalkHeight(c.Position)), (float)c.Z);
        internal static Vector3 Footprint(World world, Building b) => At(world, b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f);
        private static Vector3 RaidPosition(World world, Raid r) => r.HasContinuousPosition ? new Vector3((float)r.X, (float)world.WalkHeight(r.Position), (float)r.Z) : At(world, r.Position);
        private static Vector3 CreaturePosition(World world, DungeonCreature m) => new Vector3((float)m.X, (float)world.DungeonHeight(m.Position), (float)m.Z);
    }
}
