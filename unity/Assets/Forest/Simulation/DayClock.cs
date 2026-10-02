using System;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// The in-game clock: <c>TimeSeconds × DayScale</c> of game time, starting on Day 1 at 06:00.
    /// Derived from saved time only, so it needs no saved state of its own.
    /// </summary>
    public static class DayClock
    {
        public const double MinutesPerDay = 24 * 60;
        /// <summary>Game minutes past midnight at TimeSeconds 0 (06:00).</summary>
        public const double StartMinutes = 6 * 60;
        /// <summary>Night runs from this hour to <see cref="NightEndHour"/> the next morning.</summary>
        public const double NightStartHour = 21;
        public const double NightEndHour = 5;

        /// <summary>Game minutes since midnight before Day 1.</summary>
        public static double GameMinutes(World world) => StartMinutes + world.TimeSeconds * world.Pace.DayScale / 60;

        /// <summary>The day number, starting at 1.</summary>
        public static int Day(World world) => (int)Math.Floor(GameMinutes(world) / MinutesPerDay) + 1;

        /// <summary>Time of day from 0 (midnight) up to 1; 0.25 is 06:00 and 0.5 is noon.</summary>
        public static double DayFraction(World world)
        {
            double minutes = GameMinutes(world) % MinutesPerDay;
            return (minutes < 0 ? minutes + MinutesPerDay : minutes) / MinutesPerDay;
        }

        public static bool IsNight(World world)
        {
            double hour = DayFraction(world) * 24;
            return hour >= NightStartHour || hour < NightEndHour;
        }

        /// <summary>
        /// How far a cat's sleep threshold rises now: <c>NightSleepBias</c> at night, 0 by day, so tired
        /// cats bed down together at dusk.
        /// </summary>
        public static double SleepThresholdBias(World world) => IsNight(world) ? world.Pace.NightSleepBias : 0;
    }
}
