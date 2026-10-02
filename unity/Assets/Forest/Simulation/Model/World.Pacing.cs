namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // Saved with the world; saves written before this field existed load as classic.
        public string PacingProfile = Pacing.ClassicProfile;

        // Not public, so it never appears in save JSON; always derived from PacingProfile.
        internal Pacing Pace => Pacing.For(PacingProfile);

        public Pacing CurrentPacing() => Pace;
    }
}
