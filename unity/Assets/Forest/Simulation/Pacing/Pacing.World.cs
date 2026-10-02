namespace IdleCatForest.Simulation
{
    /// <summary>Cat movement speeds and terrain factors (World/Pathfinding.cs).</summary>
    public sealed partial class Pacing
    {
        /// <summary>Tiles per second on open ground before research and terrain multipliers.</summary>
        public double CatBaseSpeed = 1.6;
        public double RoadSpeedFactor = 1.75;
        public double DirtSpeedFactor = 1.05;
        public double MountainSpeedFactor = 0.4;
        public double ForestSpeedFactor = 0.85;
        public double OpenGroundSpeedFactor = 1;
        /// <summary>Applied on top of the terrain factor while wading through unbridged water.</summary>
        public double WadingSpeedFactor = .55;
        /// <summary>Footsteps that wear an untouched tile into a dirt path.</summary>
        public double PathWearToDirt = 50;

        /// <summary>Demo equals classic: the demo shortens timers, not walking, so cats keep a readable pace.</summary>
        partial void DemoWorld()
        {
        }
    }
}
