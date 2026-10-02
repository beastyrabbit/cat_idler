using System;

namespace IdleCatForest.Presentation
{
    public enum QualityLevel { Low, Medium, High }

    /// <summary>
    /// Player settings that live outside the world save. Wave 1 stub: defaults only, nothing is
    /// persisted or applied yet (W3-05 persists them; W2-13 reads UiScale).
    /// </summary>
    public static class GameSettings
    {
        private static float uiScale = 1;
        private static bool largerText, reducedMotion, muteInBackground;
        private static QualityLevel quality = QualityLevel.High;

        public static event Action Changed;

        /// <summary>Interface size multiplier on top of the display backing factor (1 = 100 %).</summary>
        public static float UiScale
        {
            get => uiScale;
            set
            {
                if (uiScale.Equals(value)) return;
                uiScale = value;
                Changed?.Invoke();
            }
        }

        public static bool LargerText
        {
            get => largerText;
            set => Set(ref largerText, value);
        }

        public static bool ReducedMotion
        {
            get => reducedMotion;
            set => Set(ref reducedMotion, value);
        }

        public static bool MuteInBackground
        {
            get => muteInBackground;
            set => Set(ref muteInBackground, value);
        }

        public static QualityLevel Quality
        {
            get => quality;
            set
            {
                if (quality == value) return;
                quality = value;
                Changed?.Invoke();
            }
        }

        /// <summary>The 0-1 volume of an audio bus (master, music, ambience, sfx, ui).</summary>
        public static float Volume(string bus) => 1;

        private static void Set(ref bool field, bool value)
        {
            if (field == value) return;
            field = value;
            Changed?.Invoke();
        }
    }
}
