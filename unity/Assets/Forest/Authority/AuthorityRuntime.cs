using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    public sealed class ConnectionIdentity
    {
        public SessionCredential Credential;
        public string SelectedVillageId = "";
    }

    /// <summary>
    /// The same single-writer authority is used by the embedded game and the shared host. The shared host signs
    /// expiring bearer sessions; the embedded game (<see cref="OpenLocal"/>) has one stable local player and no bearer.
    /// </summary>
    public sealed class AuthorityRuntime : IDisposable
    {
        public readonly object Sync = new object();
        public World World { get; private set; }
        public string SavePath => path;
        /// <summary>True when this session created the world rather than loading it.</summary>
        public bool Created { get; }
        /// <summary>What opening tolerated: skipped unknown save members and simulation invariant failures.</summary>
        public IReadOnlyList<string> LoadWarnings => loadWarnings;
        private readonly string path;
        private readonly SessionAuthority sessions;
        private readonly FileStream writerLease;
        private readonly List<string> loadWarnings = new List<string>();
        private readonly object writeGate = new object();
        private bool sessionBackupTaken, invalidCopyAttempted;
        private volatile WrittenPayload lastGood;
        private long projectionSequence;
        private int released;
        /// <summary>Lock files whose lease this process holds.</summary>
        private static readonly HashSet<string> HeldLeases = new HashSet<string>(StringComparer.Ordinal);

        public AuthorityRuntime(string path, int seed, string injectedSecret = null, bool requireNew = false) : this(path, seed, requireNew, false, injectedSecret)
        {
            try { if (Created) WriteNew(); }
            catch { ReleaseLease(); throw; }
        }

        /// <summary>Opens the embedded game's world. It never signs, verifies or expires bearer sessions and ignores <c>SESSION_HMAC_SECRET</c>.</summary>
        internal static AuthorityRuntime OpenLocal(string path, int seed, bool requireNew) => new AuthorityRuntime(path, seed, requireNew, true, null);

        private AuthorityRuntime(string path, int seed, bool requireNew, bool local, string injectedSecret)
        {
            this.path = Path.GetFullPath(path);
            Directory.CreateDirectory(Path.GetDirectoryName(this.path));
            writerLease = AcquireLease(this.path + ".lock");
            try
            {
                Backups.RemoveStaleTemporaries(this.path);
                var exists = File.Exists(this.path);
                if (requireNew && exists) throw new SaveException(SaveFailure.AlreadyExists, "A world already exists at this path. Choose a new save path.");
                if (!exists && Backups.Recoverable(this.path).Count > 0)
                    throw new SaveException(SaveFailure.MissingSaveWithBackup, "The save is missing but a backup of it exists. Nothing new was created; restore the backup or choose a new save path.");
                if (!local) sessions = new SessionAuthority(CredentialStore.Key(this.path + ".identity", injectedSecret));
                if (exists)
                {
                    World = SaveStore.Load<World>(this.path, out var report);
                    foreach (var member in report.DroppedMembers) loadWarnings.Add("Skipped unknown save member " + member);
                    lastGood = new WrittenPayload(report.Payload, report.FromVersion);
                }
                else
                {
                    World = World.Create(seed, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
                    Created = true;
                }
                try { ValidateStructure(World); }
                catch (InvalidDataException error) { throw new SaveException(SaveFailure.InvalidOwnership, error.Message + " The original was left untouched.", error); }
                foreach (var failure in World.Validate()) loadWarnings.Add("Simulation invariant: " + failure);
            }
            catch { ReleaseLease(); throw; }
        }

        /// <summary>
        /// Opens the save's lock file exclusively and, where the runtime supports it, also takes a byte-range lock,
        /// which the IL2CPP player enforces across processes. No P/Invoke and no <c>flock</c>. Leases this process
        /// already holds are refused before the file is touched: IL2CPP opens and then closes a descriptor when its
        /// in-process sharing check fails, and closing any descriptor of the file drops the process's byte-range lock.
        /// </summary>
        private static FileStream AcquireLease(string lockPath)
        {
            lock (HeldLeases) if (!HeldLeases.Add(lockPath)) throw AlreadyRunning(null);
            try
            {
                FileStream lease;
                try { lease = new FileStream(lockPath, FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None); }
                catch (IOException error) when (error.GetType() == typeof(IOException)) { throw AlreadyRunning(error); }
                try { LockFirstByte(lease); }
                catch (PlatformNotSupportedException) { } // .NET on macOS: the FileShare.None open above already holds a whole-file lock.
                catch (IOException error) { lease.Dispose(); throw AlreadyRunning(error); }
                return lease;
            }
            catch { lock (HeldLeases) HeldLeases.Remove(lockPath); throw; }
        }
        private void ReleaseLease()
        {
            if (Interlocked.Exchange(ref released, 1) != 0) return;
            writerLease.Dispose();
            lock (HeldLeases) HeldLeases.Remove(path + ".lock");
        }
#pragma warning disable CA1416 // Guarded by the PlatformNotSupportedException handler in AcquireLease.
        private static void LockFirstByte(FileStream lease) => lease.Lock(0, 1);
#pragma warning restore CA1416
        private static SaveException AlreadyRunning(Exception inner)
            => new SaveException(SaveFailure.AlreadyRunning, "This world is already open in another window of Idle Cat Forest.", inner);

        /// <summary>The embedded game's identity: the stored local player, else the world's first player, else a new one.</summary>
        internal ConnectionIdentity ConnectLocal(string storedPlayerId)
        {
            lock (Sync)
            {
                var id = !string.IsNullOrEmpty(storedPlayerId) ? storedPlayerId
                    : World.Players.Count > 0 ? World.Players[0].Id : "player_" + SessionAuthority.RandomHex(8);
                var player = World.Players.FirstOrDefault(candidate => candidate.Id == id) ?? new Player { Id = id, SelectedVillageId = CommunalId() };
                // Players[0] is the local owner of an embedded world, so ownership survives a missing identity folder.
                World.Players.Remove(player);
                World.Players.Insert(0, player);
                var selected = World.Villages.FirstOrDefault(village => village.Id == player.SelectedVillageId && CanControl(village, id));
                return new ConnectionIdentity { Credential = new SessionCredential { PlayerId = id }, SelectedVillageId = selected == null ? CommunalId() : selected.Id };
            }
        }

        public ConnectionIdentity Connect(SessionCredential credential, long now)
        {
            if (sessions == null) throw new InvalidOperationException("The embedded authority has no bearer sessions.");
            lock (Sync)
            {
                SessionCredential signed;
                if (credential == null) signed = sessions.Issue(now);
                else
                {
                    var restored = sessions.Restore(credential.SessionId, credential.Sig);
                    signed = sessions.Verify(restored, now) ? restored : sessions.Renew(restored, now);
                    if (signed == null) throw new UnauthorizedAccessException("Stored identity could not be authenticated. Its file was preserved.");
                }
                var player = World.Players.FirstOrDefault(candidate => candidate.Id == signed.PlayerId);
                if (player == null)
                {
                    player = new Player { Id = signed.PlayerId, SelectedVillageId = CommunalId() };
                    World.Players.Add(player);
                }
                var selected = World.Villages.FirstOrDefault(village => village.Id == player.SelectedVillageId && CanControl(village, signed.PlayerId));
                return new ConnectionIdentity { Credential = signed, SelectedVillageId = selected == null ? CommunalId() : selected.Id };
            }
        }

        public bool Authenticate(ConnectionIdentity connection, string sessionId, string signature, long now)
        {
            return sessions != null && connection != null && connection.Credential != null && connection.Credential.SessionId == sessionId
                && sessions.Verify(sessions.Restore(sessionId, signature), now);
        }

        public ActionResult Apply(ConnectionIdentity connection, GameAction action)
        {
            lock (Sync)
            {
                if (connection == null || connection.Credential == null) return ActionResult.Fail("Authenticate before sending actions.");
                if (action == null || !ActionValid(action)) return ActionResult.Fail("Action is malformed.");
                var kind = action.Kind.Replace("_", "").ToLowerInvariant();
                if (kind == "advancetime" || kind == "settestacceleration" || kind == "settestrngseed" || kind == "ensure")
                    return ActionResult.Fail("Developer controls are unavailable on this authority.");
                if (kind == "foundvillage" && World.Villages.Count >= 256) return ActionResult.Fail("The shared world has reached its village limit.");
                var context = new PlayerContext { PlayerId = connection.Credential.PlayerId, VillageId = connection.SelectedVillageId, IsDeveloper = false };
                var result = World.Apply(context, action);
                if (result.Success)
                {
                    var requested = !string.IsNullOrEmpty(result.VillageId) ? result.VillageId
                        : kind == "joinvillage" ? action.TargetId
                        : kind == "foundvillage" ? result.EntityId : connection.SelectedVillageId;
                    if (World.Villages.Any(village => village.Id == requested && CanControl(village, context.PlayerId)))
                    {
                        connection.SelectedVillageId = requested;
                        var player = World.Players.First(candidate => candidate.Id == context.PlayerId);
                        player.SelectedVillageId = requested;
                    }
                }
                return result;
            }
        }

        public void Advance(double seconds)
        {
            if (seconds < 0 || double.IsNaN(seconds) || double.IsInfinity(seconds)) throw new ArgumentOutOfRangeException(nameof(seconds));
            lock (Sync) World.Step(seconds);
        }
        /// <summary>Snapshots and writes the world on the calling thread.</summary>
        public SaveResult Save() => Write(Snapshot());

        /// <summary>
        /// Encodes the world for a save and lists the simulation invariants it breaks. Call on the thread that steps the
        /// world; the encode is the only part of a save that must see a consistent world.
        /// </summary>
        public SaveSnapshot Snapshot()
        {
            lock (Sync)
            {
                var clock = System.Diagnostics.Stopwatch.StartNew();
                var failures = World.Validate();
                var structure = "";
                try { ValidateStructure(World); }
                catch (InvalidDataException error) { structure = error.Message; failures.Insert(0, structure); }
                return new SaveSnapshot(SaveStore.Encode(World), failures, World.TimeSeconds, clock.Elapsed.TotalMilliseconds, structure);
            }
        }

        /// <summary>
        /// Writes a snapshot; safe on a background thread, and writes never overlap. Before the session's first write
        /// over a loaded world it copies that world to the session backup. A snapshot that breaks simulation invariants
        /// is still written, and the first such snapshot of a session is also kept as an invalid copy for diagnosis.
        /// A snapshot with broken ownership structure could not be reopened, so it only gets the invalid copy and the
        /// save keeps its last good contents (<see cref="SaveFailure.InvalidOwnership"/>).
        /// </summary>
        public SaveResult Write(SaveSnapshot snapshot) => Write(snapshot, false);

        private SaveResult Write(SaveSnapshot snapshot, bool requireNew)
        {
            if (snapshot == null) throw new ArgumentNullException(nameof(snapshot));
            lock (writeGate)
            {
                var clock = System.Diagnostics.Stopwatch.StartNew();
                var result = new SaveResult();
                if (!Created && !sessionBackupTaken && File.Exists(path))
                {
                    result.BackupPath = Backups.TakeSessionSnapshot(path);
                    sessionBackupTaken = true;
                }
                if (snapshot.Failures.Count > 0 && !invalidCopyAttempted)
                {
                    invalidCopyAttempted = true;
                    var copy = Backups.InvalidCopyPath(path, DateTime.UtcNow);
                    try { SaveStore.Write(copy, snapshot.Payload, true); result.InvalidCopyPath = copy; }
                    catch (IOException) { } // The diagnostic copy never blocks the save itself.
                }
                if (snapshot.StructureError.Length > 0)
                    throw new SaveException(SaveFailure.InvalidOwnership, snapshot.StructureError + " The last good save was kept.");
                result.Bytes = SaveStore.Write(path, snapshot.Payload, requireNew);
                lastGood = new WrittenPayload(snapshot.Payload, SaveStore.Version);
                result.WriteMilliseconds = clock.Elapsed.TotalMilliseconds;
                return result;
            }
        }

        /// <summary>Writes a world this session created; fails rather than replace an existing file.</summary>
        internal SaveResult WriteNew() => Write(Snapshot(), true);

        /// <summary>Replaces the live world with the last payload this session loaded or wrote. False if there is none or it no longer decodes.</summary>
        internal bool RestoreLastGood()
        {
            var good = lastGood;
            if (good == null) return false;
            World restored;
            try
            {
                restored = SaveStore.DecodePayload<World>(good.Payload, good.Version, out _);
                ValidateStructure(restored);
            }
            catch (IOException) { return false; }
            catch (InvalidDataException) { return false; }
            catch (InvalidOperationException) { return false; }
            lock (Sync) World = restored;
            return true;
        }

        public void Dispose() => ReleaseLease();

        public WorldFrame Project(ConnectionIdentity connection)
        {
            lock (Sync)
            {
                var playerId = connection == null || connection.Credential == null ? "" : connection.Credential.PlayerId;
                var selected = connection == null ? CommunalId() : connection.SelectedVillageId;
                var allowed = World.Villages.Where(village => village.Communal || village.OwnerId == playerId && playerId.Length > 0).Select(village => village.Id).ToHashSet();
                var projected = WireJson.Clone(World);
                projected.Villages.RemoveAll(village => !allowed.Contains(village.Id));
                projected.Players.RemoveAll(player => player.Id != playerId);
                projected.Reservations.Clear();
                projected.RandomState = 0;
                projected.NextId = 0;
                var frame = new WorldFrame { Sequence = checked(++projectionSequence), World = projected, SelectedVillageId = selected };
                foreach (var village in projected.Villages)
                {
                    if (playerId.Length > 0 && CanControl(village, playerId)) frame.ControlledVillageIds.Add(village.Id);
                    var exactEquipment = village.Id == selected && playerId.Length > 0 && CanControl(village, playerId) && BooksCurrent(village);
                    village.OwnerId = "";
                    foreach (var pile in village.Stockpiles) pile.Goods = WireJson.Clone(pile.Report);
                    if (!exactEquipment)
                    {
                        village.Items.RemoveAll(item => Functional(item.Kind));
                        foreach (var cat in village.Cats) cat.Equipment.Clear();
                        foreach (var job in village.Jobs) job.ItemIds.Clear();
                    }
                    else
                    {
                        var unreported = village.Stockpiles.Where(pile => pile.Kind != "storage").Select(pile => pile.Id).ToHashSet();
                        var hiddenItems = village.Items.Where(item => Functional(item.Kind) && unreported.Contains(item.LocationId)).Select(item => item.Id).ToHashSet();
                        village.Items.RemoveAll(item => hiddenItems.Contains(item.Id));
                        foreach (var cat in village.Cats) cat.Equipment.RemoveAll(id => hiddenItems.Contains(id));
                        foreach (var job in village.Jobs) job.ItemIds.RemoveAll(id => hiddenItems.Contains(id));
                    }
                    foreach (var job in village.Jobs) job.Reserved.Clear();
                    foreach (var cat in village.Cats) if (cat.ControlledBy.Length > 0 && cat.ControlledBy != playerId) cat.ControlledBy = "another-player";
                    if (village.Election != null) foreach (var ballot in village.Election.Votes) if (ballot.PlayerId != playerId) ballot.PlayerId = "other-voter";
                    if (village.KickPetition != null) foreach (var ballot in village.KickPetition.Votes) if (ballot.PlayerId != playerId) ballot.PlayerId = "other-voter";
                    foreach (var election in village.ElectionHistory) foreach (var ballot in election.Votes) if (ballot.PlayerId != playerId) ballot.PlayerId = "other-voter";
                }
                var knowledge = projected.Villages.SelectMany(village => village.Known).ToHashSet();
                projected.Tiles.RemoveAll(tile => !knowledge.Contains(tile.Position));
                projected.SurfacePlateaus.RemoveAll(plateau => !allowed.Contains(plateau.VillageId));
                projected.Dungeons.RemoveAll(site => !site.RumoredBy.Any(allowed.Contains) && !knowledge.Contains(site.Entrance));
                foreach (var site in projected.Dungeons)
                {
                    site.RumoredBy.RemoveAll(id => !allowed.Contains(id));
                    site.Stairs.RemoveAll(stair => !knowledge.Contains(stair.From) || !knowledge.Contains(stair.To));
                    site.Floors.RemoveAll(floor => !floor.Cells.Any(knowledge.Contains));
                    foreach (var floor in site.Floors)
                    {
                        floor.Cells.RemoveAll(p => !knowledge.Contains(p));
                        if (!knowledge.Contains(floor.Spawn)) floor.Spawn = default;
                        if (!knowledge.Contains(floor.LootPosition)) { floor.Loot.Clear(); floor.LootPosition = default; }
                    }
                }
                projected.Creatures.RemoveAll(creature => !knowledge.Contains(creature.Position));
                foreach (var creature in projected.Creatures) { creature.Path.Clear(); creature.Home = creature.Position; creature.TargetCatId = ""; }
                foreach (var cat in projected.Villages.SelectMany(village => village.Cats)) cat.Path.RemoveAll(p => p.Level < 0 && !knowledge.Contains(p));
                projected.TradeOffers.RemoveAll(offer => playerId.Length == 0 || !allowed.Contains(offer.FromVillageId) && !allowed.Contains(offer.ToVillageId));
                foreach (var offer in projected.TradeOffers) { offer.OfferedSources.Clear(); offer.RequestedSources.Clear(); }
                var selectedVillage = World.Villages.FirstOrDefault(village => village.Id == selected);
                if (selectedVillage != null)
                    foreach (var village in World.Villages.Where(candidate => selectedVillage.Contacts.Contains(candidate.Id)))
                        frame.KnownVillages.Add(new VillageSummary { Id = village.Id, Name = village.Name, Center = village.Center, Communal = village.Communal, CanControl = playerId.Length > 0 && CanControl(village, playerId) });
                return frame;
            }
        }
        private string CommunalId() => World.Villages.First(village => village.Communal).Id;
        private static bool CanControl(Village village, string playerId) => playerId.Length > 0 && (village.Communal || village.OwnerId == playerId);
        private static bool Functional(string kind) => kind == "tool" || kind == "weapon" || kind == "armor";
        private static bool BooksCurrent(Village village)
        {
            var storage = village.Stockpiles.Where(pile => pile.Kind == "storage").ToArray();
            return storage.Length > 0 && storage.All(pile =>
            {
                if (pile.CountedAt < 0) return false;
                var actual = pile.Goods.Select(good => new Stack(good.Resource, good.Amount)).ToList();
                foreach (var item in village.Items.Where(item => item.LocationId == pile.Id && Functional(item.Kind)))
                    World.Add(actual, item.Kind == "armor" ? "armor" : item.Kind + "s", 1);
                return actual.Select(good => good.Resource).Concat(pile.Report.Select(report => report.Resource)).Distinct()
                    .All(resource => Math.Abs(World.Amount(actual, resource) - World.Amount(pile.Report, resource)) < 0.000001);
            });
        }
        private static bool ActionValid(GameAction action)
        {
            if (action.Kind == null || action.Kind.Length > 64 || action.Path == null || action.Path.Count > 4096 || action.Accepts == null || action.Accepts.Count > 32) return false;
            if (double.IsNaN(action.Amount) || double.IsInfinity(action.Amount) || double.IsNaN(action.OtherAmount) || double.IsInfinity(action.OtherAmount)) return false;
            if (Math.Abs((long)action.Position.X) > 1000000 || Math.Abs((long)action.Position.Z) > 1000000 || Math.Abs((long)action.End.X) > 1000000 || Math.Abs((long)action.End.Z) > 1000000) return false;
            return new[] { action.TargetId, action.CatId, action.BuildingId, action.Resource, action.RecipeId, action.NodeId, action.Role, action.Name, action.OtherResource, action.OtherVillageId, action.Edit, action.Labor, action.Mode }.All(value => value != null && value.Length <= 256);
        }
        /// <summary>Structural ownership checks and every simulation invariant; throws on the first problem.</summary>
        public static void ValidateWorld(World world)
        {
            ValidateStructure(world);
            var failures = world.Validate();
            if (failures.Count > 0) throw new InvalidDataException("Saved world violates simulation invariants: " + string.Join("; ", failures));
        }

        /// <summary>The ownership structure an authority cannot run without. Simulation invariants are checked separately and never block a save or an open.</summary>
        public static void ValidateStructure(World world)
        {
            if (world == null || world.Villages == null || world.Villages.Count(village => village.Communal) != 1) throw new InvalidDataException("A saved world must contain exactly one communal village.");
            if (world.Villages.Any(village => village.Communal && !string.IsNullOrEmpty(village.OwnerId))) throw new InvalidDataException("The communal village cannot have a private owner.");
            if (world.Villages.Where(village => !village.Communal).GroupBy(village => village.OwnerId).Any(group => string.IsNullOrEmpty(group.Key) || group.Count() > 1)) throw new InvalidDataException("Personal village ownership is invalid.");
            if (world.Villages.GroupBy(village => village.Id).Any(group => group.Count() != 1)) throw new InvalidDataException("Village identities are not unique.");
        }

        private sealed class WrittenPayload
        {
            public readonly string Payload;
            public readonly int Version;
            public WrittenPayload(string payload, int version) { Payload = payload; Version = version; }
        }
    }
}
