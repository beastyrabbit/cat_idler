namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Every pacing number of the simulation, grouped by domain in partial files next to this one.
    /// Each domain file declares its knobs as public fields whose initialisers are the classic
    /// values, and implements its <c>Demo&lt;Domain&gt;()</c> hook to override them for the demo.
    /// Classic keeps the behaviour the mechanics scenarios and the server rely on.
    /// </summary>
    public sealed partial class Pacing
    {
        public const string ClassicProfile = "classic";
        public const string DemoProfile = "demo";

        public string Profile = ClassicProfile;
        public double DayScale = 1;

        // Classic is declared first so domain hooks may read it while Demo is built.
        public static readonly Pacing Classic = new Pacing();
        public static readonly Pacing Demo = MakeDemo();

        /// <summary>The profile named by a save; anything but "demo" is classic.</summary>
        public static Pacing For(string profile) => profile == DemoProfile ? Demo : Classic;

        private static Pacing MakeDemo()
        {
            var pacing = new Pacing { Profile = DemoProfile };
            pacing.DemoCore();
            pacing.DemoLife();
            pacing.DemoPopulation();
            pacing.DemoResearch();
            pacing.DemoJobs();
            pacing.DemoWorld();
            pacing.DemoThreats();
            pacing.DemoTrade();
            return pacing;
        }

        partial void DemoCore();
        partial void DemoLife();
        partial void DemoPopulation();
        partial void DemoResearch();
        partial void DemoJobs();
        partial void DemoWorld();
        partial void DemoThreats();
        partial void DemoTrade();
    }
}
