namespace IdleCatForest.Simulation
{
    public partial class Cat
    {
        /// <summary>
        /// The held direct-control intent: a step of -1, 0 or 1 on X and Z (both 0 when nothing is held).
        /// MoveCat sets it; StopCat, leaving control, lease expiry and an unrefreshed hold clear it.
        /// TickControls chains tiles in this direction until it is cleared or the next tile is blocked.
        /// </summary>
        public Int2 ControlDirection;

        /// <summary>World time of the last MoveCat. Clients re-send MoveCat while the key is down.</summary>
        public double ControlHeldAt;

        /// <summary>
        /// World time the current controlled step began. At the next tile the cat only walks on when a
        /// MoveCat arrived after this, so a single MoveCat is one step and a missed key release stops the
        /// cat within one tile.
        /// </summary>
        public double ControlStepAt;
    }
}
