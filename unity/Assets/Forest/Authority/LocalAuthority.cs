using System;
using System.IO;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    public sealed class LocalAuthority : IDisposable
    {
        private readonly AuthorityRuntime runtime;
        private readonly ConnectionIdentity connection;
        public World World => runtime.World;
        public SessionCredential Credential => connection.Credential;
        public string SelectedVillageId => connection.SelectedVillageId;
        public LocalAuthority(string path, int seed) : this(path, seed, false) { }
        private LocalAuthority(string path, int seed, bool requireNew)
        {
            runtime = new AuthorityRuntime(path, seed, Environment.GetEnvironmentVariable("SESSION_HMAC_SECRET"), requireNew);
            try
            {
                var credentialPath = Path.GetFullPath(path) + ".identity/session.json";
                connection = runtime.Connect(CredentialStore.Load(credentialPath), DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
                CredentialStore.Save(credentialPath, connection.Credential, connection.SelectedVillageId);
                runtime.Save();
            }
            catch { runtime.Dispose(); throw; }
        }
        public static LocalAuthority CreateNew(string path, int seed) => new LocalAuthority(path, seed, true);
        public ActionResult Apply(GameAction action) => runtime.Apply(connection, action);
        public void Advance(double seconds) => runtime.Advance(seconds);
        public void Save() => runtime.Save();
        public void Dispose() => runtime.Dispose();
    }
}
