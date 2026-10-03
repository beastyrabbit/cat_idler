using System;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// The game speed the presentation is stepping this world at (1, 2, 4 ...; 0 while paused). Never
        /// saved. Controlled-cat movement is divided by it, so a steered cat walks in real time whatever
        /// the game speed; at 0 it does not move. Authorities that step in real time leave it at 1.
        /// </summary>
        [NonSerialized] public double ControlTimeScale = 1;
    }
}
