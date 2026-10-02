using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// A cat with no job, station or need. It finishes its current walk; on its own planning
        /// second it picks a spot near the village centre.
        /// </summary>
        private void TickIdle(Village v, Cat c, double dt, bool planning)
        {
            if (c.Path.Count > 0)
                Move(c, c.Path[c.Path.Count - 1], dt);
            else if (planning && (long)TimeSeconds % Pace.IdleWanderIntervalSeconds == Hash(c.Id) % Pace.IdleWanderIntervalSeconds)
            {
                int radius = Pace.IdleWanderRadius, span = 2 * radius + 1;
                int x = (int)(Hash(c.Id + ":" + (long)TimeSeconds) % span) - radius;
                int z = (int)(Hash(c.Id + ":z:" + (long)TimeSeconds) % span) - radius;
                var target = new Int2(v.Center.X + x, v.Center.Z + z);
                c.Path = Path(c.Position, target, v) ?? new List<Int2>();
                c.Goal = GoalKey.RestingAtHome;
            }
        }
    }
}
