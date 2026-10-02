using System;
using System.IO;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Save path, autosave cadence and lifecycle saves of the local world.</summary>
    public sealed partial class ForestGame
    {
        public string SavePath { get; private set; } = "";
        public string InitialSavePath = "";
        private double sinceSave;

        private static string DefaultSavePath() => Path.Combine(Application.persistentDataPath, "unity", "world-v1.json");

        /// <summary>
        /// Called once per frame from <c>Update</c> with the simulated seconds stepped this frame
        /// (0 in remote play or while paused). Today it saves after every 5 simulated seconds.
        /// </summary>
        private void PersistenceTick(double steppedSeconds)
        {
            sinceSave += steppedSeconds;
            if (sinceSave >= 5) { Save(); sinceSave = 0; }
        }

        public void Save()
        {
            if (local == null) return;
            try { local.Save(); }
            catch (Exception e) { Status = "Save failed: " + e.Message; Debug.LogError(Status); }
        }

        private void OnApplicationPause(bool pause) { if (pause) Save(); }
        private void OnApplicationQuit() { Save(); }
    }
}
