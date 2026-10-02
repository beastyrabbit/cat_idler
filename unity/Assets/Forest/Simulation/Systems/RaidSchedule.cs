using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // PACING-HANDOFF ScriptedBandMembers, ScriptedRaiderHealth, ScriptedRaiderStrength: the demo's fox band.
        private const int ScriptedBandMembers = 3;
        private const double ScriptedRaiderHealth = 6;
        private const double ScriptedRaiderStrength = 15;
        // PACING-HANDOFF NaturalRaiderHealth, NaturalRaiderStrength, CatsPerRaider, MaxRaiders: natural raiding parties.
        private const double NaturalRaiderHealth = 10;
        private const double NaturalRaiderStrength = 15;
        private const int CatsPerRaider = 8;
        private const int MaxRaiders = 6;

        /// <summary>
        /// Once per planning second: the demo's scripted raid (warning, then a fox band of three at the
        /// south gate) and the natural trigger. Natural raids need only game time and colony size; a
        /// colony's stores no longer decide whether raiders come.
        /// </summary>
        private void MaybeSpawnRaid(Village v)
        {
            ScriptedRaid(v);
            if ((long)TimeSeconds % Pace.RaidCheckSeconds == 0 && TimeSeconds - v.FoundedAt > Pace.RaidGraceSeconds && v.Raids.Count == 0)
            {
                int alive = v.Cats.Count(c => c.Alive);
                if (alive >= Pace.RaidMinCats && Roll() < Pace.RaidChance)
                {
                    var raid = Party(Math.Clamp(alive / CatsPerRaider, 2, MaxRaiders), NaturalRaiderHealth, NaturalRaiderStrength, new Int2(v.Center.X, v.Center.Z + v.Radius - 1));
                    v.Raids.Add(raid);
                    Note(v, EventKind.Raid, "Raiders approach the south gate", raid.Id);
                }
            }
        }

        private Raid Party(int members, double health, double strength, Int2 at) => new Raid
        {
            Id = Id("raid"),
            Kind = "fox",
            Members = members,
            Health = members * health,
            MaxHealth = members * health,
            Strength = members * strength,
            Position = at
        };

        /// <summary>
        /// The demo raid beat (D15): a raid_warning RaidWarningSeconds before DemoRaidAtSeconds of
        /// colony time, then one fox band of three outside the south gate. DemoRaidAtSeconds 0 (the
        /// classic profile) disables it.
        /// </summary>
        private void ScriptedRaid(Village v)
        {
            double at = Pace.DemoRaidAtSeconds;
            if (at <= 0 || v.ScriptedRaidStage >= 2)
                return;
            double elapsed = TimeSeconds - v.FoundedAt;
            if (v.ScriptedRaidStage == 0 && elapsed + 1e-9 >= at - Pace.RaidWarningSeconds)
            {
                v.ScriptedRaidStage = 1;
                Note(v, EventKind.RaidWarning, "Raiders spotted to the south", v.Id);
            }
            if (v.ScriptedRaidStage != 1 || elapsed + 1e-9 < at || v.Raids.Count > 0)
                return;
            v.ScriptedRaidStage = 2;
            var raid = Party(ScriptedBandMembers, ScriptedRaiderHealth, ScriptedRaiderStrength, SouthApproach(v));
            raid.Scripted = true;
            v.Raids.Add(raid);
            foreach (var c in v.Cats.Where(c => c.RalliedTo == RallyGate))
                c.RalliedTo = raid.Id;
            Note(v, EventKind.Raid, "A fox band attacks the south gate", raid.Id);
        }

        /// <summary>The first walkable tile on the trail outside the south gate, so the band is seen coming in.</summary>
        private Int2 SouthApproach(Village v)
        {
            for (int z = v.Radius + 3; z >= v.Radius - 1; z--)
            {
                var p = new Int2(v.Center.X, v.Center.Z + z);
                if (Walkable(p))
                    return p;
            }
            return new Int2(v.Center.X, v.Center.Z + v.Radius - 1);
        }
    }
}
