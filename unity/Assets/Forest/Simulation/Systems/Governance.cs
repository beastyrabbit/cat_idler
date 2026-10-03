using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Elections, leader removal petitions and the raid schedule, once per planning second.</summary>
        private void Governance(Village v)
        {
            if (v.Election == null && TimeSeconds >= v.NextElection)
                v.Election = new Election { Id = Id("election"), EndAt = TimeSeconds + Pace.ElectionDurationSeconds };
            if (v.Election != null && TimeSeconds >= v.Election.EndAt)
            {
                var winner = v.Election.Votes.Where(b => v.Cats.Any(c => c.Id == b.CatId && c.Alive)).GroupBy(b => b.CatId).OrderByDescending(g => g.Sum(ballot => ballot.Weight)).ThenBy(g => g.Key, StringComparer.Ordinal).FirstOrDefault()?.Key ?? v.Cats.FirstOrDefault(c => c.Alive)?.Id;
                if (winner != null)
                    v.LeaderId = winner;
                v.Election = null;
                v.NextElection = TimeSeconds + Pace.ElectionIntervalSeconds;
                Note(v, EventKind.Election, "Leader term began", v.LeaderId);
            }
            if (v.KickPetition != null && TimeSeconds >= v.KickPetition.EndAt)
            {
                var petition = v.KickPetition;
                v.KickPetition = null;
                bool passed = petition.TargetId == v.LeaderId && petition.Votes.Where(b => b.CatId == petition.TargetId).Select(b => b.PlayerId).Distinct().Count() >= 5;
                if (passed)
                {
                    v.LeaderId = v.Cats.Where(c => c.Alive && c.Id != petition.TargetId && c.Migration != "arriving" && c.Migration != "departing").OrderByDescending(c => Amount(c.Stats, "leadership")).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault()?.Id ?? "";
                    if (v.Election == null)
                        v.Election = new Election { Id = Id("election"), Kind = "snap", EndAt = TimeSeconds + Pace.ElectionDurationSeconds };
                    Note(v, EventKind.Election, "Five players removed the leader; interim term began", v.LeaderId);
                }
                else
                    Note(v, EventKind.Election, "Leader removal petition expired without five signatures");
            }
            MaybeSpawnRaid(v);
        }
    }
}
