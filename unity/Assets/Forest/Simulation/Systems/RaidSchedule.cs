using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Once per raid check, a large and wealthy colony with no raid in progress may draw raiders to its south gate.</summary>
        private void MaybeSpawnRaid(Village v)
        {
            if ((long)TimeSeconds % Pace.RaidCheckSeconds == 0 && TimeSeconds - v.FoundedAt > Pace.RaidGraceSeconds && v.Cats.Count(c => c.Alive) >= Pace.RaidMinCats && Total(v, "materials") + Total(v, "refined") > Pace.RaidMinWealth && v.Raids.Count == 0 && Roll() < Pace.RaidChance)
            {
                v.Raids.Add(new Raid { Id = Id("raid"), Position = new Int2(v.Center.X, v.Center.Z + v.Radius - 1) });
                Note(v, "raid", "Raiders approach the south gate");
            }
        }
    }
}
