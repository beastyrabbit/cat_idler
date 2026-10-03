using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;

namespace IdleCatForest.Authority
{
    /// <summary>
    /// Recovery files kept next to a save. For <c>world-v1.json</c>:
    /// <c>world-v1.backup.json</c> is the save as it was when the current session opened it;
    /// <c>world-v1.json.previous</c> is the file each write replaced;
    /// <c>world-v1.invalid-&lt;utc&gt;.json</c> is a world that broke simulation invariants when it was saved.
    /// None of them is ever deleted or restored automatically.
    /// </summary>
    public static class Backups
    {
        public static string SessionBackupPath(string savePath) => Sibling(savePath, ".backup");

        public static string PreviousPath(string savePath) => Path.GetFullPath(savePath) + ".previous";

        public static string InvalidCopyPath(string savePath, DateTime utc)
            => Sibling(savePath, ".invalid-" + utc.ToUniversalTime().ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture));

        /// <summary>Existing copies of this world. While any exists, a missing main save is never replaced by a new world.</summary>
        public static IReadOnlyList<string> Recoverable(string savePath)
            => new[] { SessionBackupPath(savePath), PreviousPath(savePath) }.Where(File.Exists).ToList();

        /// <summary>Copies the save as this session found it to the session backup, replacing the previous session's backup.</summary>
        public static string TakeSessionSnapshot(string savePath)
        {
            var backup = SessionBackupPath(savePath);
            AtomicFile.Write(backup, File.ReadAllBytes(savePath), false, false);
            return backup;
        }

        /// <summary>
        /// Deletes temporary files that a crash or kill left behind while writing this save, its backup or an invalid copy.
        /// Call only while holding the save's writer lease. Returns how many were removed.
        /// </summary>
        public static int RemoveStaleTemporaries(string savePath)
        {
            var full = Path.GetFullPath(savePath);
            var directory = Path.GetDirectoryName(full);
            if (!Directory.Exists(directory)) return 0;
            var save = Path.GetFileName(full);
            var backup = Path.GetFileName(SessionBackupPath(full));
            var invalid = Path.GetFileNameWithoutExtension(full) + ".invalid-";
            var removed = 0;
            foreach (var file in Directory.GetFiles(directory, "*" + AtomicFile.TemporaryMarker + "*"))
            {
                var name = Path.GetFileName(file);
                var marker = name.LastIndexOf(AtomicFile.TemporaryMarker, StringComparison.Ordinal);
                if (marker < 0 || !IsTemporarySuffix(name.Substring(marker + AtomicFile.TemporaryMarker.Length))) continue;
                var target = name.Substring(0, marker);
                var ours = target == save || target == backup
                    || target.StartsWith(invalid, StringComparison.Ordinal) && target.EndsWith(Path.GetExtension(full), StringComparison.Ordinal);
                if (!ours) continue;
                try { File.Delete(file); removed++; }
                catch (IOException) { }
                catch (UnauthorizedAccessException) { }
            }
            return removed;
        }

        private static bool IsTemporarySuffix(string suffix)
        {
            if (suffix.Length != 24) return false;
            foreach (var character in suffix) if (!(character >= '0' && character <= '9' || character >= 'a' && character <= 'f')) return false;
            return true;
        }

        private static string Sibling(string savePath, string marker)
        {
            var full = Path.GetFullPath(savePath);
            return Path.Combine(Path.GetDirectoryName(full), Path.GetFileNameWithoutExtension(full) + marker + Path.GetExtension(full));
        }
    }
}
