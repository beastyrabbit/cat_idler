using System;
using System.IO;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    public sealed class ConnectionIdentity
    {
        public SessionCredential Credential;
        public string SelectedVillageId = "";
    }

    /// <summary>The same single-writer authority is used by the embedded game and the shared host.</summary>
    public sealed class AuthorityRuntime : IDisposable
    {
        public readonly object Sync = new object();
        public World World { get; private set; }
        private readonly string path;
        private readonly SessionAuthority sessions;
        private readonly FileStream writerLease;
        private long projectionSequence;

        public AuthorityRuntime(string path, int seed, string injectedSecret = null, bool requireNew = false)
        {
            this.path = Path.GetFullPath(path);
            Directory.CreateDirectory(Path.GetDirectoryName(this.path));
            writerLease = new FileStream(this.path + ".lock", FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
            try
            {
                if (requireNew && File.Exists(this.path)) throw new IOException("A world already exists at this path. Choose a new save path.");
                sessions = new SessionAuthority(CredentialStore.Key(this.path + ".identity", injectedSecret));
                World = File.Exists(this.path) ? SaveStore.Load<World>(this.path) : World.Create(seed, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
                ValidateWorld(World);
                if (!File.Exists(this.path)) SaveStore.Save(this.path, World, true);
            }
            catch { writerLease.Dispose(); throw; }
        }

        public ConnectionIdentity Connect(SessionCredential credential, long now)
        {
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
            return connection != null && connection.Credential != null && connection.Credential.SessionId == sessionId
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
        public void Save() { lock (Sync) { ValidateWorld(World); SaveStore.Save(path, World); } }
        public void Dispose() { writerLease.Dispose(); }

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
        public static void ValidateWorld(World world)
        {
            if (world == null || world.Villages == null || world.Villages.Count(village => village.Communal) != 1) throw new InvalidDataException("A saved world must contain exactly one communal village.");
            if (world.Villages.Any(village => village.Communal && !string.IsNullOrEmpty(village.OwnerId))) throw new InvalidDataException("The communal village cannot have a private owner.");
            if (world.Villages.Where(village => !village.Communal).GroupBy(village => village.OwnerId).Any(group => string.IsNullOrEmpty(group.Key) || group.Count() > 1)) throw new InvalidDataException("Personal village ownership is invalid.");
            if (world.Villages.GroupBy(village => village.Id).Any(group => group.Count() != 1)) throw new InvalidDataException("Village identities are not unique.");
            var failures = world.Validate();
            if (failures.Count > 0) throw new InvalidDataException("Saved world violates simulation invariants: " + string.Join("; ", failures));
        }
    }
}
