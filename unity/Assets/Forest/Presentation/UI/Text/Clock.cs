using System;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Times and durations for player text. Absolute times use the game clock of <see cref="DayClock"/>
    /// ("Day 1 · 06:00", 24-hour); durations are real seconds ("40 s", "12 min", "6 h").
    /// </summary>
    public static class GameClock
    {
        /// <summary>Game minutes since midnight before Day 1 for a world time and day scale.</summary>
        public static double GameMinutes(double timeSeconds, double dayScale)
        {
            if (double.IsNaN(timeSeconds) || double.IsInfinity(timeSeconds) || timeSeconds < 0)
                timeSeconds = 0;
            if (double.IsNaN(dayScale) || double.IsInfinity(dayScale) || dayScale <= 0)
                dayScale = 1;
            return DayClock.StartMinutes + timeSeconds * dayScale / 60;
        }

        /// <summary>The day number, starting at 1.</summary>
        public static int Day(double timeSeconds, double dayScale) => (int)Math.Floor(GameMinutes(timeSeconds, dayScale) / DayClock.MinutesPerDay) + 1;

        /// <summary>"Day 1 · 06:00".</summary>
        public static string Clock(double timeSeconds, double dayScale) =>
            "Day " + Numbers.Whole(Day(timeSeconds, dayScale)) + " · " + TimeOfDay(timeSeconds, dayScale);

        /// <summary>"06:00": the time of day on a 24-hour clock, minutes rounded down.</summary>
        public static string TimeOfDay(double timeSeconds, double dayScale)
        {
            long minutes = (long)Math.Floor(GameMinutes(timeSeconds, dayScale) + 1e-9);
            long ofDay = minutes % (long)DayClock.MinutesPerDay;
            return (ofDay / 60).ToString("00", Numbers.Culture) + ":" + (ofDay % 60).ToString("00", Numbers.Culture);
        }

        /// <summary>
        /// A moment relative to now for the event log and cards: "06:21" on the same game day,
        /// "Day 2 · 06:21" otherwise.
        /// </summary>
        public static string Stamp(double timeSeconds, double nowSeconds, double dayScale) =>
            Day(timeSeconds, dayScale) == Day(nowSeconds, dayScale) ? TimeOfDay(timeSeconds, dayScale) : Clock(timeSeconds, dayScale);

        /// <summary>
        /// A real duration: "40 s" under a minute, "12 min" under an hour, "6 h" (or "1 h 30 min" under
        /// three hours), then "2 days". Never negative.
        /// </summary>
        public static string Duration(double seconds)
        {
            if (double.IsNaN(seconds) || double.IsInfinity(seconds) || seconds < 0)
                seconds = 0;
            string u = Numbers.UnitSpace;
            if (seconds < 59.5)
                return Numbers.Whole(Math.Max(1, Math.Round(seconds))) + u + "s";
            double minutes = Math.Round(seconds / 60);
            if (minutes < 60)
                return Numbers.Whole(minutes) + u + "min";
            double hours = Math.Floor(minutes / 60);
            double rest = minutes - hours * 60;
            if (hours < 3 && rest >= 1)
                return Numbers.Whole(hours) + u + "h " + Numbers.Whole(rest) + u + "min";
            if (seconds < 48 * 3600)
                return Numbers.Whole(Math.Round(seconds / 3600)) + u + "h";
            return Numbers.Plural(Math.Round(seconds / 86400), "day", "days");
        }

        /// <summary>An estimate: "about 4 min".</summary>
        public static string About(double seconds) => "about " + Duration(seconds);

        /// <summary>A past moment: "just now" under 10 seconds, otherwise "12 min ago".</summary>
        public static string Ago(double seconds)
        {
            if (double.IsNaN(seconds) || double.IsInfinity(seconds) || seconds < 10)
                return "just now";
            return Duration(seconds) + " ago";
        }
    }
}
