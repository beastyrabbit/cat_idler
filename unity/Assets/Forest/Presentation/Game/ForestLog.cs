using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The game's diagnostic log. It writes a session header to the player log, logs every caught
    /// exception with its stack trace, counts every error the player log receives and keeps the first
    /// <see cref="MaxUniqueExceptions"/> distinct ones (de-duplicated by type and stack) in
    /// <c>logs/session-&lt;timestamp&gt;.log</c>. Player-facing text never comes from here.
    /// </summary>
    public static class ForestLog
    {
        public const int MaxUniqueExceptions = 20;
        private const int KeptSessionLogs = 10;

        private static readonly HashSet<string> signatures = new HashSet<string>();
        private static string directory = "";
        private static string header = "";
        private static bool listening;

        /// <summary>Errors, assertions and exceptions the player log received this session.</summary>
        public static int ErrorCount { get; private set; }

        /// <summary>Distinct errors (by type and stack) received this session.</summary>
        public static int UniqueErrorCount => signatures.Count;

        /// <summary>The session file, created when the first error arrives; "" until then.</summary>
        public static string SessionLogPath { get; private set; } = "";

        /// <summary>
        /// Starts a session: writes the header to the player log and listens for errors. The session file
        /// goes to <paramref name="logDirectory"/> and is created only when an error arrives.
        /// </summary>
        public static void Begin(string logDirectory, string saveRoot, int seed)
        {
            directory = logDirectory ?? "";
            SessionLogPath = "";
            signatures.Clear();
            ErrorCount = 0;
            header = Header(saveRoot, seed);
            Debug.Log(header);
            if (!listening) { Application.logMessageReceived += Received; listening = true; }
        }

        /// <summary>Stops listening. Counts and the session file path stay readable.</summary>
        public static void End()
        {
            if (listening) { Application.logMessageReceived -= Received; listening = false; }
        }

        /// <summary>Logs a caught exception with its stack trace, after one line of context.</summary>
        public static void Exception(Exception e, string context)
        {
            if (!string.IsNullOrEmpty(context)) Debug.Log("[forest] " + context);
            Debug.LogException(e);
        }

        /// <summary>Logs a non-exception problem as a warning in the player log.</summary>
        public static void Warning(string message) => Debug.LogWarning("[forest] " + message);

        private static string Header(string saveRoot, int seed)
        {
            var text = new StringBuilder();
            text.Append("[forest] session ").Append(DateTime.UtcNow.ToString("u", CultureInfo.InvariantCulture));
            text.Append(" | version ").Append(Application.version);
            text.Append(" | unity ").Append(Application.unityVersion);
            text.Append(Debug.isDebugBuild ? " development" : " release");
            text.Append(Application.isEditor ? " editor" : " player");
            text.Append(" | os ").Append(SystemInfo.operatingSystem);
            text.Append(" | cpu ").Append(SystemInfo.processorType).Append(" x").Append(SystemInfo.processorCount);
            text.Append(" | ram ").Append(SystemInfo.systemMemorySize).Append(" MB");
            text.Append(" | gpu ").Append(SystemInfo.graphicsDeviceName).Append(" (").Append(SystemInfo.graphicsDeviceType).Append(')');
            text.Append(" | screen ").Append(Screen.width).Append('x').Append(Screen.height);
            text.Append(" | save root ").Append(saveRoot);
            text.Append(" | seed ").Append(seed.ToString(CultureInfo.InvariantCulture));
            return text.ToString();
        }

        private static void Received(string condition, string stackTrace, LogType type)
        {
            if (type != LogType.Exception && type != LogType.Error && type != LogType.Assert) return;
            ErrorCount++;
            if (signatures.Count >= MaxUniqueExceptions) return;
            if (!signatures.Add(Signature(condition, stackTrace, type))) return;
            Write(condition, stackTrace, type);
        }

        /// <summary>Exceptions are keyed by type and stack, so the same failure with a different message is one entry.</summary>
        internal static string Signature(string condition, string stackTrace, LogType type)
        {
            condition = condition ?? "";
            string head = condition;
            if (type == LogType.Exception)
            {
                int colon = condition.IndexOf(':');
                if (colon > 0) head = condition.Substring(0, colon);
            }
            string key = type + "\n" + head + "\n" + (stackTrace ?? "");
            unchecked
            {
                ulong hash = 14695981039346656037UL;
                foreach (char c in key) { hash ^= c; hash *= 1099511628211UL; }
                return hash.ToString("x16", CultureInfo.InvariantCulture);
            }
        }

        // Never logs: a failure here must not feed back into the handler.
        private static void Write(string condition, string stackTrace, LogType type)
        {
            if (directory.Length == 0) return;
            try
            {
                bool first = SessionLogPath.Length == 0;
                if (first)
                {
                    Directory.CreateDirectory(directory);
                    Prune();
                    SessionLogPath = Path.Combine(directory, "session-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss", CultureInfo.InvariantCulture) + ".log");
                }
                var entry = new StringBuilder();
                if (first) entry.AppendLine(header).AppendLine();
                entry.Append('[').Append(DateTime.UtcNow.ToString("HH:mm:ss", CultureInfo.InvariantCulture)).Append("] ")
                    .Append(type).Append(" #").Append(signatures.Count.ToString(CultureInfo.InvariantCulture)).Append(": ")
                    .AppendLine(condition).AppendLine(stackTrace ?? "").AppendLine();
                if (signatures.Count == MaxUniqueExceptions)
                    entry.AppendLine("Further distinct errors this session are counted but not written.");
                File.AppendAllText(SessionLogPath, entry.ToString());
            }
            catch (Exception) { /* diagnostics must never take the game down */ }
        }

        private static void Prune()
        {
            var old = new DirectoryInfo(directory).GetFiles("session-*.log")
                .OrderByDescending(f => f.Name, StringComparer.Ordinal).Skip(KeptSessionLogs - 1);
            foreach (var file in old) file.Delete();
        }
    }
}
