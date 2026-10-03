using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;

/// <summary>
/// Embedded-game persistence: local identity, tolerant loading, recovery files and the single writer.
/// Every case works in its own synthetic temporary directory; no player save or identity is read.
/// </summary>
static class PersistenceTests
{
    const string SaveName = "world-v1.json";

    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("persistence: local world opens with a credential issued 90 days ago", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var credential = WriteSignedCredential(path, now - 90L * 24 * 60 * 60 * 1000);
            using var reopened = new LocalAuthority(path, 99);
            check(reopened.Credential.PlayerId == credential.PlayerId, "aged local credential changed the local owner");
        }));

        test("persistence: local world opens after the clock moved back one day", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var credential = WriteSignedCredential(path, now + 24L * 60 * 60 * 1000);
            using var reopened = new LocalAuthority(path, 99);
            check(reopened.Credential.PlayerId == credential.PlayerId, "future-dated local credential changed the local owner");
        }));

        test("persistence: local world keeps its owner when the identity folder is missing", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            string owner, personal;
            using (var authority = LocalAuthority.CreateNew(path, 4242))
            {
                owner = authority.Credential.PlayerId;
                var founded = authority.Apply(new GameAction { Kind = "FoundVillage", Name = "Fixture Hollow" });
                check(founded.Success, founded.Error);
                personal = founded.VillageId;
                authority.Save();
            }
            if (Directory.Exists(path + ".identity")) Directory.Delete(path + ".identity", true);
            using var reopened = new LocalAuthority(path, 99);
            check(reopened.Credential.PlayerId == owner, "missing identity folder minted a new owner");
            check(reopened.World.Village(personal).OwnerId == reopened.Credential.PlayerId, "personal village became foreign");
            check(reopened.World.Players.Count(p => p.Id == owner) == 1, "local owner record duplicated");
        }));

        test("persistence: local world ignores SESSION_HMAC_SECRET", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            var previous = Environment.GetEnvironmentVariable("SESSION_HMAC_SECRET");
            try
            {
                string owner;
                Environment.SetEnvironmentVariable("SESSION_HMAC_SECRET", "synthetic-fixture-secret-not-a-credential");
                using (var authority = LocalAuthority.CreateNew(path, 4242)) { owner = authority.Credential.PlayerId; authority.Save(); }
                Environment.SetEnvironmentVariable("SESSION_HMAC_SECRET", null);
                using (var authority = new LocalAuthority(path, 99)) check(authority.Credential.PlayerId == owner, "removing the server secret changed the local owner");
                Environment.SetEnvironmentVariable("SESSION_HMAC_SECRET", "another-synthetic-secret-not-a-credential");
                using (var authority = new LocalAuthority(path, 99)) check(authority.Credential.PlayerId == owner, "a different server secret changed the local owner");
            }
            finally { Environment.SetEnvironmentVariable("SESSION_HMAC_SECRET", previous); }
        }));

        test("persistence: save with an unknown member loads", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (var authority = LocalAuthority.CreateNew(path, 4242)) { }
            EditPayload(path, payload =>
            {
                payload["RetiredWorldField"] = 7;
                ((JObject)payload["Villages"][0]["Cats"][0])["RetiredCatField"] = "gone";
            });
            var loaded = SaveStore.Load<World>(path, out var report);
            check(loaded.Villages[0].Cats.Count > 0, "tolerant load lost the roster");
            check(report.DroppedMembers.OrderBy(m => m, StringComparer.Ordinal).SequenceEqual(new[] { "Cat.RetiredCatField", "World.RetiredWorldField" }), "dropped members not reported: " + string.Join(",", report.DroppedMembers));
            using var reopened = new LocalAuthority(path, 99);
            check(reopened.World.Villages[0].Cats.Count == loaded.Villages[0].Cats.Count, "embedded authority refused a save with an unknown member");
            check(reopened.LoadWarnings.Count(w => w.Contains("RetiredWorldField") || w.Contains("RetiredCatField")) == 2, "embedded authority did not surface skipped members for the log");
            try { WireJson.Decode<World>(JObject.Parse(File.ReadAllText(path)).Value<string>("Payload")); throw new Exception("wire input accepted an unknown member"); }
            catch (Newtonsoft.Json.JsonException) { }
        }));

        test("persistence: main build v1 save fixture loads and keeps playing", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            File.WriteAllBytes(path, Fixture("world-v1.main-5ec7278.json.gz"));
            string owner;
            using (var authority = new LocalAuthority(path, 99))
            {
                owner = authority.Credential.PlayerId;
                check(authority.World.Players.Count == 1 && owner == authority.World.Players[0].Id, "fixture owner not kept without its identity folder");
                check(authority.World.Villages.Single().Cats.Count(c => c.Alive) == 30 && Near(authority.World.TimeSeconds, 120), "fixture world changed on load");
                check(authority.World.Validate().Count == 0, "fixture world violates invariants after load");
                for (var i = 0; i < 100; i++) authority.Advance(World.SimulationStepSeconds);
                authority.Save();
            }
            using (var authority = new LocalAuthority(path, 99))
                check(Near(authority.World.TimeSeconds, 125) && authority.Credential.PlayerId == owner, "fixture did not resume after its first save");
        }));

        test("persistence: invariant failure still writes the save and an invalid copy", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (var authority = LocalAuthority.CreateNew(path, 4242))
            {
                authority.World.Villages[0].Cats[0].Cargo.Add(new IdleCatForest.Simulation.Stack("logs", -1));
                authority.Advance(1);
                authority.Save();
            }
            var saved = SaveStore.Load<World>(path);
            check(Near(saved.TimeSeconds, 1) && saved.Villages[0].Cats[0].Cargo.Any(s => s.Amount < 0), "invalid world was not saved");
            var copies = Directory.GetFiles(directory, "world-v1.invalid-*.json");
            check(copies.Length == 1, "invalid copy count " + copies.Length);
            check(Near(SaveStore.Load<World>(copies[0]).TimeSeconds, 1), "invalid copy does not hold the failing world");
            using var reopened = new LocalAuthority(path, 99);
            check(Near(reopened.World.TimeSeconds, 1), "invalid world could not be reopened");
            check(reopened.LoadWarnings.Any(w => w.Contains("Invalid cargo")), "reopened invariant failure was not reported");
            reopened.Advance(1); reopened.Save(); reopened.Advance(1); reopened.Save();
            check(Directory.GetFiles(directory, "world-v1.invalid-*.json").Length <= 2, "invalid copies multiplied within one session");
        }));

        test("persistence: missing main save with an existing backup refuses to create a new world", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (var authority = LocalAuthority.CreateNew(path, 4242)) { authority.Advance(1); authority.Save(); authority.Advance(1); authority.Save(); }
            var previous = File.ReadAllBytes(path + ".previous");
            File.Delete(path);
            try { using var recreated = new LocalAuthority(path, 99); throw new Exception("a fresh world replaced the missing save"); }
            catch (SaveException error) when (error.Code == SaveFailure.MissingSaveWithBackup) { }
            check(!File.Exists(path), "refusal created a main save");
            check(File.ReadAllBytes(path + ".previous").SequenceEqual(previous), "refusal changed the surviving backup");
            File.Move(path + ".previous", Backups.SessionBackupPath(path));
            try { using var created = LocalAuthority.CreateNew(path, 99); throw new Exception("a new world was created beside an orphaned session backup"); }
            catch (SaveException error) when (error.Code == SaveFailure.MissingSaveWithBackup) { }
            check(!File.Exists(path), "refused creation wrote a main save");
        }));

        test("persistence: second writer on the same save is refused", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            var first = LocalAuthority.CreateNew(path, 4242);
            try { using var second = new LocalAuthority(path, 99); throw new Exception("second writer opened the live save"); }
            catch (SaveException error) when (error.Code == SaveFailure.AlreadyRunning) { }
            try { using var shared = new AuthorityRuntime(path, 99); throw new Exception("shared host opened the live embedded save"); }
            catch (SaveException error) when (error.Code == SaveFailure.AlreadyRunning) { }
            first.Dispose(); first.Dispose();
            using var reopened = new LocalAuthority(path, 99);
            try { using var third = new LocalAuthority(path, 99); throw new Exception("a released lease admitted two writers"); }
            catch (SaveException error) when (error.Code == SaveFailure.AlreadyRunning) { }
        }));

        test("persistence: stale temporary save files are removed on open", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            var stale = path + ".tmp-0123456789abcdef01234567";
            File.WriteAllText(stale, "{\"partial\":");
            var unrelated = Path.Combine(directory, "notes.tmp-keep");
            File.WriteAllText(unrelated, "keep");
            using (new LocalAuthority(path, 99)) { }
            check(!File.Exists(stale), "stale temporary save survived open");
            check(File.Exists(unrelated), "open removed an unrelated file");
        }));

        test("persistence: opening a world writes nothing until the first save", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            check(!File.Exists(path + ".previous"), "creating a world wrote it twice");
            var bytes = File.ReadAllBytes(path); var written = File.GetLastWriteTimeUtc(path);
            using (new LocalAuthority(path, 99)) { }
            check(File.ReadAllBytes(path).SequenceEqual(bytes) && File.GetLastWriteTimeUtc(path) == written && !File.Exists(path + ".previous"), "open rewrote the save");
        }));

        test("persistence: session start backup is taken before the first save and kept", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            var backup = Path.Combine(directory, "world-v1.backup.json");
            using (var authority = LocalAuthority.CreateNew(path, 4242)) { authority.Advance(1); authority.Save(); }
            check(!File.Exists(backup), "a fresh world wrote a session backup");
            var sessionStart = File.ReadAllBytes(path);
            using (var authority = new LocalAuthority(path, 99)) { authority.Advance(1); authority.Save(); authority.Advance(1); authority.Save(); }
            check(File.Exists(backup) && File.ReadAllBytes(backup).SequenceEqual(sessionStart), "backup is not the session start save");
            check(Near(SaveStore.Load<World>(backup).TimeSeconds, 1) && Near(SaveStore.Load<World>(path).TimeSeconds, 3), "backup or save holds the wrong time");
        }));

        test("persistence: restore last good reloads the last written world", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using var authority = LocalAuthority.CreateNew(path, 4242);
            authority.Advance(1); authority.Save();
            var expected = WireJson.Encode(authority.World);
            authority.Advance(2); authority.World.Villages[0].Cats[0].Cargo.Add(new IdleCatForest.Simulation.Stack("logs", 4));
            check(authority.RestoreLastGood(), "restore reported failure");
            check(WireJson.Encode(authority.World) == expected, "restore did not return the last written world");
        }));
    }

    public static void RunContracts(Action<string, Action> test, Action<bool, string> check)
    {
        test("persistence: newer damaged and foreign saves fail with distinct codes and stay untouched", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            var original = File.ReadAllText(path);
            void Expect(string contents, string code)
            {
                File.WriteAllText(path, contents);
                try { using var opened = new LocalAuthority(path, 99); throw new Exception("opened a save that should fail with " + code); }
                catch (SaveException error) when (error.Code == code) { }
                check(File.ReadAllText(path) == contents, code + " failure changed the file");
                check(!File.Exists(Backups.SessionBackupPath(path)) && !File.Exists(path + ".previous"), code + " failure wrote a recovery file");
            }
            var newer = JObject.Parse(original); newer["Version"] = SaveStore.Version + 1; newer["AddedByNewerBuild"] = true;
            Expect(newer.ToString(Newtonsoft.Json.Formatting.None), SaveFailure.MadeByNewerVersion);
            var tampered = JObject.Parse(original); tampered["Payload"] = tampered.Value<string>("Payload") + " ";
            Expect(tampered.ToString(Newtonsoft.Json.Formatting.None), SaveFailure.Damaged);
            var foreign = JObject.Parse(original); foreign["Format"] = "another-game";
            Expect(foreign.ToString(Newtonsoft.Json.Formatting.None), SaveFailure.Damaged);
            Expect("not json", SaveFailure.Damaged);
            File.WriteAllText(path, original);
            EditPayload(path, root => ((JArray)root["Villages"]).Add(root["Villages"][0].DeepClone()));
            Expect(File.ReadAllText(path), SaveFailure.InvalidOwnership);
            File.WriteAllText(path, original);
            try { using var created = LocalAuthority.CreateNew(path, 1); throw new Exception("created over an existing world"); }
            catch (SaveException error) when (error.Code == SaveFailure.AlreadyExists) { }
            check(File.ReadAllText(path) == original, "refused creation changed the world");
        }));

        test("persistence: broken ownership keeps the last good save and an invalid copy", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using var authority = LocalAuthority.CreateNew(path, 4242);
            var good = File.ReadAllBytes(path);
            authority.World.Villages.Single(v => v.Communal).OwnerId = "player_intruder";
            try { authority.Save(); throw new Exception("an unopenable world replaced the save"); }
            catch (SaveException error) when (error.Code == SaveFailure.InvalidOwnership) { }
            check(File.ReadAllBytes(path).SequenceEqual(good), "broken ownership changed the save");
            check(Directory.GetFiles(directory, "world-v1.invalid-*.json").Length == 1, "broken ownership kept no diagnostic copy");
            authority.World.Villages.Single(v => v.Communal).OwnerId = "";
            authority.Advance(1);
            authority.Save();
            check(Near(SaveStore.Load<World>(path).TimeSeconds, 1), "repaired world was not saved");
        }));

        test("persistence: migrations run before decode and refuse versions they cannot read", () =>
        {
            var payload = JObject.Parse(WireJson.Encode(World.Create(4242)));
            var expected = payload.ToString(Newtonsoft.Json.Formatting.None);
            check(SaveMigrations.Migrate(payload, SaveStore.Version).ToString(Newtonsoft.Json.Formatting.None) == expected, "current version payload changed by migration");
            try { SaveMigrations.Migrate(payload, SaveStore.Version + 1); throw new Exception("migrated a newer payload"); }
            catch (SaveException error) when (error.Code == SaveFailure.MadeByNewerVersion) { }
            try { SaveMigrations.Migrate(payload, SaveMigrations.OldestSupported - 1); throw new Exception("migrated an unsupported payload"); }
            catch (SaveException error) when (error.Code == SaveFailure.UnsupportedVersion) { }
        });

        test("persistence: background write stores the snapshot not the live world", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            using (LocalAuthority.CreateNew(path, 4242)) { }
            using var authority = new LocalAuthority(path, 99);
            authority.Advance(1);
            var snapshot = authority.Snapshot();
            var expected = WireJson.Encode(authority.World);
            var writes = new[] { Task.Run(() => authority.Write(snapshot)), Task.Run(() => authority.Write(snapshot)) };
            authority.Advance(2);
            Task.WaitAll(writes);
            check(Near(snapshot.TimeSeconds, 1) && snapshot.Failures.Count == 0, "snapshot recorded the wrong time or false failures");
            check(writes.Count(w => w.Result.BackupPath.Length > 0) == 1 && writes.All(w => w.Result.Bytes > 0), "session backup taken more than once or a write reported nothing");
            check(WireJson.Encode(SaveStore.Load<World>(path)) == expected, "background write did not store the snapshot");
            check(Near(SaveStore.Load<World>(Backups.SessionBackupPath(path)).TimeSeconds, 0), "session backup is not the opened world");
            check(Directory.GetFiles(directory, "*.tmp-*").Length == 0, "overlapping writes left temporary files");
        }));

        test("persistence: local play writes no bearer or key and keeps an imported owner", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, SaveName);
            string owner;
            using (var authority = LocalAuthority.CreateNew(path, 4242)) { owner = authority.PlayerId; authority.Save(); }
            check(!Directory.Exists(path + ".identity"), "local play created identity files");
            var imported = WriteSignedCredential(path, 1000);
            using (var authority = new LocalAuthority(path, 99))
            {
                check(authority.PlayerId == imported.PlayerId && authority.World.Players[0].Id == imported.PlayerId, "imported native owner was not used");
                check(authority.World.Players.Any(p => p.Id == owner), "previous local player record was dropped");
                authority.Save();
            }
            File.WriteAllText(Path.Combine(path + ".identity", "session.json"), "{}");
            using (var authority = new LocalAuthority(path, 99))
            {
                check(authority.PlayerId == imported.PlayerId, "unreadable identity file changed the stored owner");
                check(authority.LoadWarnings.Any(w => w.Contains("identity")), "unreadable identity file was not reported");
            }
        }));
    }

    /// <summary>Writes a validly signed native credential for a new player into the save's identity folder.</summary>
    static SessionCredential WriteSignedCredential(string savePath, long issuedAt)
    {
        var identity = savePath + ".identity";
        Directory.CreateDirectory(identity);
        var keyPath = Path.Combine(identity, "authority.key");
        if (!File.Exists(keyPath)) File.WriteAllText(keyPath, Convert.ToBase64String(RandomNumberGenerator.GetBytes(32)));
        var key = Convert.FromBase64String(File.ReadAllText(keyPath));
        var id = "session_v2_" + issuedAt + "_" + Convert.ToHexString(RandomNumberGenerator.GetBytes(8)).ToLowerInvariant() + "_" + Convert.ToHexString(RandomNumberGenerator.GetBytes(16)).ToLowerInvariant();
        var credential = new SessionCredential { SessionId = id, Sig = Convert.ToHexString(HMACSHA256.HashData(key, Encoding.UTF8.GetBytes(id))).ToLowerInvariant(), PlayerId = SessionAuthority.PlayerId(id) };
        var credentialPath = Path.Combine(identity, "session.json");
        if (File.Exists(credentialPath)) File.Delete(credentialPath);
        CredentialStore.Save(credentialPath, credential);
        return credential;
    }

    /// <summary>Rewrites a save's payload and its checksum, as a different build would have written it.</summary>
    static void EditPayload(string path, Action<JObject> edit)
    {
        var envelope = JObject.Parse(File.ReadAllText(path));
        var payload = JObject.Parse((string)envelope["Payload"]);
        edit(payload);
        var text = payload.ToString(Newtonsoft.Json.Formatting.None);
        envelope["Payload"] = text;
        envelope["Sha256"] = Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(text))).ToLowerInvariant();
        File.WriteAllText(path, envelope.ToString(Newtonsoft.Json.Formatting.None));
    }

    static byte[] Fixture(string name, [System.Runtime.CompilerServices.CallerFilePath] string source = "")
    {
        using var input = File.OpenRead(Path.Combine(Path.GetDirectoryName(source), name));
        using var gzip = new GZipStream(input, CompressionMode.Decompress);
        using var output = new MemoryStream();
        gzip.CopyTo(output);
        return output.ToArray();
    }

    static bool Near(double actual, double expected) => Math.Abs(actual - expected) < 1e-6;

    static void WithDirectory(Action<string> action)
    {
        var directory = Path.Combine(Path.GetTempPath(), "forest-persistence-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try { action(directory); }
        finally { Directory.Delete(directory, true); }
    }
}
