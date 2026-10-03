using System;
using System.Collections.Generic;
using System.IO;
using System.Threading.Tasks;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>What a save indicator may show (W3-06 save chip, W3-04 recovery card).</summary>
    public enum SaveState
    {
        /// <summary>No local world is open: still opening, failed to open, or playing on a shared server.</summary>
        Unavailable,
        /// <summary>The world is open and this session has not written it yet.</summary>
        Idle,
        /// <summary>A save is being written in the background.</summary>
        Saving,
        /// <summary>The last save was written.</summary>
        Saved,
        /// <summary>The last save was written, but the world breaks simulation invariants (see <see cref="ForestGame.SaveWarnings"/>).</summary>
        SavedWithWarnings,
        /// <summary>The last save failed; the file on disk is the previous good one (see <see cref="ForestGame.LastSaveError"/>).</summary>
        Failed
    }

    /// <summary>
    /// Save path, autosave cadence and lifecycle saves of the local world. Autosave runs on wall-clock time, never on
    /// simulated time: every <see cref="AutosaveIntervalSeconds"/> while the world changed, <see cref="ActionSaveDelaySeconds"/>
    /// after a successful player action, on focus loss and on quit, and never while <see cref="Faulted"/>. The world is
    /// encoded on the main thread; hashing and writing run on a background task with at most one save in flight.
    /// </summary>
    public sealed partial class ForestGame
    {
        public const double AutosaveIntervalSeconds = 30;
        public const double ActionSaveDelaySeconds = 2;
        private const double SaveWaitSeconds = 10;
        private const string SaveFailedPrefix = "Save failed: ";

        public string SavePath { get; private set; } = "";
        public string InitialSavePath = "";
        public SaveState SaveState { get; private set; }
        /// <summary>Wall-clock time of the last successful save this session, or null.</summary>
        public DateTime? LastSaveUtc { get; private set; }
        /// <summary>Why the last save attempt failed; empty after a success.</summary>
        public string LastSaveError { get; private set; } = "";
        /// <summary>Simulation invariant failures of the last written world; empty when it was clean.</summary>
        public IReadOnlyList<string> SaveWarnings { get; private set; } = Array.Empty<string>();
        public int ConsecutiveSaveFailures { get; private set; }
        /// <summary>Duration of the last successful save: main-thread encode plus background write.</summary>
        public double SaveMilliseconds { get; private set; }
        /// <summary>The main-thread part of <see cref="SaveMilliseconds"/>, the only part that can cause a frame hitch.</summary>
        public double SaveMainThreadMilliseconds { get; private set; }

        private bool persistenceStarted, saveDirty;
        private int observedRevision;
        private double lastSaveAt, actionSaveAt = -1;
        private Task<SaveResult> saveInFlight;
        private SaveSnapshot inFlightSnapshot;
        private string statusBeforeSaveFailure = "";

        private static string DefaultSavePath() => Path.Combine(Application.persistentDataPath, Application.isEditor ? "unity-editor" : "unity", "world-v1.json");

        /// <summary>
        /// Called once per frame from <c>Update</c> with the simulated seconds stepped this frame (0 in remote play or
        /// while paused). Stepping marks the world dirty; any other revision change is a player action.
        /// </summary>
        private void PersistenceTick(double steppedSeconds)
        {
            CollectFinishedSave();
            if (local == null) { SaveState = SaveState.Unavailable; return; }
            var now = Time.realtimeSinceStartupAsDouble;
            if (!persistenceStarted) StartPersistence(now);
            ObserveChanges(steppedSeconds, now);
            if (!saveDirty || IsRemote || Faulted) return;
            if (actionSaveAt >= 0 && now >= actionSaveAt || now - lastSaveAt >= AutosaveIntervalSeconds) BeginSave(now);
        }

        /// <summary>Writes the local world now and waits for it. Does nothing while the simulation is faulted.</summary>
        public void Save()
        {
            if (local == null || Faulted) return;
            WaitForSave();
            var now = Time.realtimeSinceStartupAsDouble;
            SaveSnapshot snapshot = null;
            try
            {
                snapshot = local.Snapshot();
                var result = local.Write(snapshot);
                MarkSaved(now);
                RecordSaveSuccess(snapshot, result);
            }
            catch (Exception error) { saveDirty = true; lastSaveAt = now; RecordSaveFailure(error); }
        }

        private void StartPersistence(double now)
        {
            persistenceStarted = true;
            observedRevision = Revision;
            lastSaveAt = now;
            SaveState = SaveState.Idle;
            foreach (var warning in local.LoadWarnings) Debug.LogWarning("World opened with a tolerated problem: " + warning);
        }

        private void ObserveChanges(double steppedSeconds, double now)
        {
            var changes = Revision - observedRevision;
            observedRevision = Revision;
            if (IsRemote) return;
            var steps = (int)Math.Round(steppedSeconds / World.SimulationStepSeconds);
            if (steps > 0) saveDirty = true;
            if (changes <= steps) return;
            saveDirty = true;
            if (actionSaveAt < 0) actionSaveAt = now + ActionSaveDelaySeconds;
        }

        private void BeginSave(double now)
        {
            if (saveInFlight != null) { actionSaveAt = now; return; } // One save in flight; the next tick after it finishes saves again.
            SaveSnapshot snapshot;
            try { snapshot = local.Snapshot(); }
            catch (Exception error) { lastSaveAt = now; RecordSaveFailure(error); return; }
            MarkSaved(now);
            SaveState = SaveState.Saving;
            var authority = local;
            inFlightSnapshot = snapshot;
            saveInFlight = Task.Run(() => authority.Write(snapshot));
        }

        /// <summary>Everything up to the current revision is in the snapshot being written.</summary>
        private void MarkSaved(double now)
        {
            observedRevision = Revision;
            saveDirty = false;
            actionSaveAt = -1;
            lastSaveAt = now;
        }

        private void CollectFinishedSave()
        {
            if (saveInFlight == null || !saveInFlight.IsCompleted) return;
            var task = saveInFlight;
            saveInFlight = null;
            if (task.Status == TaskStatus.RanToCompletion) RecordSaveSuccess(inFlightSnapshot, task.Result);
            else { saveDirty = true; RecordSaveFailure(task.Exception?.GetBaseException()); }
            inFlightSnapshot = null;
        }

        private void WaitForSave()
        {
            if (saveInFlight == null) return;
            try { saveInFlight.Wait(TimeSpan.FromSeconds(SaveWaitSeconds)); }
            catch (AggregateException) { } // Recorded by CollectFinishedSave.
            CollectFinishedSave();
        }

        private void RecordSaveSuccess(SaveSnapshot snapshot, SaveResult result)
        {
            LastSaveUtc = DateTime.UtcNow;
            LastSaveError = "";
            ConsecutiveSaveFailures = 0;
            SaveMainThreadMilliseconds = snapshot.CaptureMilliseconds;
            SaveMilliseconds = snapshot.CaptureMilliseconds + result.WriteMilliseconds;
            SaveWarnings = snapshot.Failures;
            SaveState = snapshot.Failures.Count > 0 ? SaveState.SavedWithWarnings : SaveState.Saved;
            if (result.InvalidCopyPath.Length > 0)
                Debug.LogWarning("The world breaks simulation invariants and was saved anyway (" + string.Join("; ", snapshot.Failures) + "). A copy was kept at " + result.InvalidCopyPath);
            if (statusBeforeSaveFailure.Length > 0 && Status.StartsWith(SaveFailedPrefix, StringComparison.Ordinal)) Status = statusBeforeSaveFailure;
            statusBeforeSaveFailure = "";
        }

        private void RecordSaveFailure(Exception error)
        {
            ConsecutiveSaveFailures++;
            LastSaveError = error == null ? "Unknown save error." : error.Message;
            SaveState = SaveState.Failed;
            if (!Status.StartsWith(SaveFailedPrefix, StringComparison.Ordinal)) statusBeforeSaveFailure = Status;
            Status = SaveFailedPrefix + LastSaveError;
            if (ConsecutiveSaveFailures == 1 || ConsecutiveSaveFailures % 10 == 0) Debug.LogError(Status + " (" + ConsecutiveSaveFailures + " in a row)");
        }

        /// <summary>Starts a background save if anything changed since the last one.</summary>
        private void SaveIfChanged()
        {
            if (local == null || !persistenceStarted || IsRemote || Faulted) return;
            CollectFinishedSave();
            var now = Time.realtimeSinceStartupAsDouble;
            ObserveChanges(0, now);
            if (saveDirty) BeginSave(now);
        }

        private void OnApplicationFocus(bool hasFocus) { if (!hasFocus) SaveIfChanged(); }
        private void OnApplicationPause(bool pause) { if (pause) SaveIfChanged(); }

        private void OnApplicationQuit()
        {
            if (local == null || !persistenceStarted || IsRemote) return;
            ObserveChanges(0, Time.realtimeSinceStartupAsDouble);
            WaitForSave();
            if (saveDirty) Save();
        }

        // Runs before OnDestroy disposes the authority, so a background write never outlives its writer lease.
        private void OnDisable() => WaitForSave();
    }
}
