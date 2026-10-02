using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Storage piles and the goods stacked on them.</summary>
    public sealed partial class ForestView
    {
        private void ReconcilePiles(Village v, int level)
        {
            foreach (var pile in v.Stockpiles.Where(p => p.Position.Level == level))
            {
                if (pile.Kind != "storage") continue;
                Entity("pile:" + pile.Id, "stockpile", At(pile.Position), 1.25f);
                int i = 0;
                foreach (var stack in pile.Report.Where(s => s.Amount > 0).Take(5))
                {
                    Entity("pilegoods:" + pile.Id + ":" + stack.Resource, "cargo_" + stack.Resource.ToLowerInvariant(), At(pile.Position) + new Vector3((i % 2) * .35f - .2f, .35f + i / 2 * .18f, 0), .32f); i++;
                }
            }
        }
    }
}
