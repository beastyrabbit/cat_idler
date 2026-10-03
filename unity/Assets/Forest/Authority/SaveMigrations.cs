using System;
using System.Collections.Generic;
using Newtonsoft.Json.Linq;

namespace IdleCatForest.Authority
{
    /// <summary>
    /// Upgrades a save payload, as JSON, from the version it was written at to <see cref="SaveStore.Version"/>
    /// before the typed decode. Rules for later changes to the serialized model:
    /// adding a field needs nothing (old saves decode it as its initial value);
    /// removing a field needs nothing to load (the value is skipped and logged) but loses that data;
    /// renaming a field, or changing its meaning or type, bumps <see cref="SaveStore.Version"/> and adds the step
    /// <c>Steps[old version]</c> that rewrites the old JSON, plus a test that loads the checked-in fixture
    /// <c>server/Forest.Tests/Fixtures/world-v1.main-5ec7278.json.gz</c> through it.
    /// </summary>
    public static class SaveMigrations
    {
        public const int OldestSupported = 1;

        /// <summary>Step <c>n</c> rewrites a version <c>n</c> payload into version <c>n + 1</c> in place.</summary>
        private static readonly Dictionary<int, Action<JObject>> Steps = new Dictionary<int, Action<JObject>>();

        public static JObject Migrate(JObject payload, int fromVersion)
        {
            if (payload == null) throw new ArgumentNullException(nameof(payload));
            if (fromVersion > SaveStore.Version)
                throw new SaveException(SaveFailure.MadeByNewerVersion, "This world was saved by a newer version of the game. Update the game to open it. The original was left untouched.");
            if (fromVersion < OldestSupported)
                throw new SaveException(SaveFailure.UnsupportedVersion, "Unsupported save version. The original was left untouched.");
            for (var version = fromVersion; version < SaveStore.Version; version++)
            {
                if (!Steps.TryGetValue(version, out var step))
                    throw new InvalidOperationException("No save migration from version " + version + " to " + (version + 1) + ".");
                step(payload);
            }
            return payload;
        }
    }
}
