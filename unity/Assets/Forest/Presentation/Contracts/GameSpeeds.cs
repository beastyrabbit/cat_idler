using System;
using System.Collections.Generic;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The simulation speeds a local world may run at (D18). Players get pause, 1x, 2x and 4x
    /// (Space pauses or resumes; 1, 2 and 3 select 1x, 2x and 4x). The faster dev speeds exist
    /// only in debug builds or when the player is started with --forest-dev-speeds.
    /// </summary>
    public static class GameSpeeds
    {
        public const float Paused = 0;
        public static readonly IReadOnlyList<float> Player = Array.AsReadOnly(new float[] { 1, 2, 4 });
        public static readonly IReadOnlyList<float> Dev = Array.AsReadOnly(new float[] { 8, 16 });

        private static bool? devFlag;

        public static bool DevSpeedsEnabled => Debug.isDebugBuild || DevFlag;

        private static bool DevFlag
        {
            get
            {
                if (devFlag == null) devFlag = Array.IndexOf(Environment.GetCommandLineArgs(), "--forest-dev-speeds") >= 0;
                return devFlag.Value;
            }
        }

        /// <summary>Snaps a requested speed to the nearest allowed one for this build.</summary>
        public static float Clamp(float requested) => Clamp(requested, DevSpeedsEnabled);

        /// <summary>
        /// Zero, negative and NaN requests pause. Any positive request keeps running at the
        /// nearest allowed speed (ties go to the slower one), so it never pauses by rounding.
        /// </summary>
        public static float Clamp(float requested, bool devSpeeds)
        {
            if (float.IsNaN(requested) || requested <= Paused) return Paused;
            float fastest = devSpeeds ? Dev[Dev.Count - 1] : Player[Player.Count - 1];
            if (requested >= fastest) return fastest;
            float best = Player[0];
            void Consider(IReadOnlyList<float> speeds)
            {
                foreach (float speed in speeds)
                    if (Math.Abs(speed - requested) < Math.Abs(best - requested)) best = speed;
            }
            Consider(Player);
            if (devSpeeds) Consider(Dev);
            return best;
        }
    }
}
