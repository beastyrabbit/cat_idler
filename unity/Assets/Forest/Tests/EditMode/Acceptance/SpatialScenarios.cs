using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class SpatialScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.world_levels_distinct", () =>
            {
                var surface = new Int2(4, 8);
                var lower = new Int2(4, 8, -1);
                if (surface.Equals(lower) || new HashSet<Int2> { surface, lower }.Count != 2 || Int2.Distance(surface, lower) <= 0)
                    throw new Exception("Overlapping dungeon and surface cells must have different spatial identities");
            });
        }
    }
}
