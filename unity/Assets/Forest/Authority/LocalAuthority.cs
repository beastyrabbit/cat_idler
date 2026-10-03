using System;
using System.Collections.Generic;
using System.IO;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    /// <summary>
    /// The embedded single-player authority. Local play has no expiring bearer: the local player id is stored in the
    /// world (its first player), so a long break, a clock change, <c>SESSION_HMAC_SECRET</c> or a missing
    /// <c>.identity</c> folder never locks a player out of their own world. An imported native credential at
    /// <c>&lt;save&gt;.identity/session.json</c> still names the owner, without checking its signature or age.
    /// </summary>
    public sealed class LocalAuthority : IDisposable
    {
        private readonly AuthorityRuntime runtime;
        private readonly ConnectionIdentity connection;
        private readonly List<string> loadWarnings = new List<string>();
        public World World => runtime.World;
        /// <summary>Carries only <see cref="SessionCredential.PlayerId"/>; local play has no session or signature.</summary>
        public SessionCredential Credential => connection.Credential;
        public string PlayerId => connection.Credential.PlayerId;
        public string SelectedVillageId => connection.SelectedVillageId;
        public string SavePath => runtime.SavePath;
        /// <summary>True when this session created the world.</summary>
        public bool Created => runtime.Created;
        /// <summary>What opening tolerated; the presentation logs these once.</summary>
        public IReadOnlyList<string> LoadWarnings => loadWarnings;

        /// <summary>Opens the save at <paramref name="path"/>; a missing save becomes a new demo colony founded on <paramref name="seed"/>.</summary>
        public LocalAuthority(string path, int seed) : this(path, seed, false, false) { }
        /// <summary>Like the default constructor, but a missing save becomes a classic world (<see cref="World.Create"/>).</summary>
        public static LocalAuthority OpenClassic(string path, int seed) => new LocalAuthority(path, seed, false, true);
        private LocalAuthority(string path, int seed, bool requireNew, bool classic)
        {
            runtime = AuthorityRuntime.OpenLocal(path, seed, requireNew, classic);
            try
            {
                loadWarnings.AddRange(runtime.LoadWarnings);
                connection = runtime.ConnectLocal(StoredPlayerId(Path.Combine(runtime.SavePath + ".identity", "session.json")));
                if (runtime.Created) runtime.WriteNew();
            }
            catch { runtime.Dispose(); throw; }
        }
        public static LocalAuthority CreateNew(string path, int seed) => new LocalAuthority(path, seed, true, false);
        public ActionResult Apply(GameAction action) => runtime.Apply(connection, action);
        public void Advance(double seconds) => runtime.Advance(seconds);
        /// <summary>Snapshots and writes on the calling thread.</summary>
        public void Save() => runtime.Save();
        /// <summary>Main-thread half of a background save: validates and encodes the world.</summary>
        public SaveSnapshot Snapshot() => runtime.Snapshot();
        /// <summary>Background half of a save: backup, invalid copy and the atomic write. Writes never overlap.</summary>
        public SaveResult Write(SaveSnapshot snapshot) => runtime.Write(snapshot);
        /// <summary>Replaces the live world with the last payload this session loaded or wrote. True on success.</summary>
        public bool RestoreLastGood() => runtime.RestoreLastGood();
        public void Dispose() => runtime.Dispose();

        private string StoredPlayerId(string credentialPath)
        {
            try { return CredentialStore.Load(credentialPath)?.PlayerId ?? ""; }
            catch (Exception error) when (error is IOException || error is InvalidDataException || error is UnauthorizedAccessException || error is Newtonsoft.Json.JsonException)
            {
                loadWarnings.Add("Ignored unreadable local identity file; the world's own owner was used.");
                return "";
            }
        }
    }
}
