using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Kills a cat and releases everything it held: work, cargo, items, bed and offices.
        /// <paramref name="cause"/> names the death in the village log ("needs", "old age", "dungeon").
        /// </summary>
        private void Die(Village v, Cat c, string cause)
        {
            CancelWork(v, c);
            Spill(v, c.Position, c.Cargo);
            c.Alive = false;
            c.DeathUnixMilliseconds = EpochUnixMs + (long)(TimeSeconds * 1000);
            c.ControlledBy = "";
            c.PregnantUntil = -1;
            c.PregnancyMateId = "";
            c.BedId = "";
            foreach (var item in v.Items.Where(i => i.LocationId == c.Id))
            {
                var pile = new Stockpile { Id = Id("spill"), Kind = "spill", Position = c.Position, Width = 1, Depth = 1, Capacity = 10000 };
                v.Stockpiles.Add(pile);
                item.LocationId = pile.Id;
            }
            c.Equipment.Clear();
            foreach (var officer in v.Officers.Where(o => o.CatId == c.Id).ToArray())
            {
                var successor = v.Cats.Where(x => x.Alive && x.Id != c.Id && x.OfficerRole == "").OrderBy(x => x.Id, StringComparer.Ordinal).FirstOrDefault();
                if (successor == null)
                    v.Officers.Remove(officer);
                else
                {
                    officer.CatId = successor.Id;
                    successor.OfficerRole = officer.Role;
                }
            }
            if (v.LeaderId == c.Id)
                v.LeaderId = v.Cats.Where(x => x.Alive).OrderBy(x => x.Id, StringComparer.Ordinal).FirstOrDefault()?.Id ?? "";
            Note(v, "death", c.Name + " died from " + cause, c.Id);
        }
        /// <summary>Refounding after extinction, housing, births, old age, breeding and migration, once per upkeep tick.</summary>
        private void Population(Village v)
        {
            var alive = v.Cats.Where(c => c.Alive).ToArray();
            if (alive.Length == 0)
            {
                if (!CanFoundAt(v.Id, v.Center, v.Communal))
                {
                    if (!v.Events.Any(e => e.Kind == "recovery_blocked"))
                        Note(v, "recovery_blocked", "Refounding blocked by foreign territory", v.Id);
                    return;
                }
                foreach (var r in Reservations.Where(r => r.VillageId == v.Id).ToArray())
                    Reservations.Remove(r);
                foreach (var trade in TradeOffers.Where(t => t.FromVillageId == v.Id || t.ToVillageId == v.Id).ToArray())
                    CancelTrade(v, trade.Id);
                var replacement = Found(v.Id, v.Name, v.OwnerId, v.Center, v.Communal);
                replacement.Run = v.Run + 1;
                replacement.Blessings = v.Blessings;
                replacement.Research = new List<string>(v.Research);
                Villages[Villages.IndexOf(v)] = replacement;
                Note(replacement, "recovery", "Colony refounded after extinction");
                return;
            }
            int beds = v.Buildings.Count(b => b.Completed && b.Kind == "den") * DenCapacity(v);
            foreach (var c in alive)
            {
                if (c.BedId == "" && c.Migration != "arriving" && c.Migration != "departing")
                {
                    var den = v.Buildings.Where(b => b.Completed && b.Kind == "den").FirstOrDefault(b => v.Cats.Count(x => x.Alive && x.BedId == b.Id) < DenCapacity(v));
                    int reserved = alive.Count(x => x.PregnantUntil > 0);
                    if (den != null && alive.Count(x => x.BedId != "") + reserved < beds)
                    {
                        c.BedId = den.Id;
                        c.Migration = "resident";
                        c.ProbationUntil = -1;
                    }
                    else if (c.ProbationUntil >= 0 && TimeSeconds >= c.ProbationUntil)
                    {
                        CancelWork(v, c);
                        c.Migration = "departing";
                        c.Goal = GoalKey.Leaving;
                    }
                }
                if (c.PregnantUntil > 0 && TimeSeconds >= c.PregnantUntil)
                {
                    var kitten = NewCat(v, "Kitten " + NextId, c.Position);
                    kitten.AgeHours = 0;
                    kitten.BirthUnixMilliseconds = EpochUnixMs + (long)(TimeSeconds * 1000);
                    kitten.ParentIds.Add(c.Id);
                    if (c.PregnancyMateId != "")
                        kitten.ParentIds.Add(c.PregnancyMateId);
                    var mate = v.Cats.Find(x => x.Id == c.PregnancyMateId);
                    foreach (var stat in kitten.Stats)
                    {
                        double maternal = Amount(c.Stats, stat.Resource), paternal = mate == null ? maternal : Amount(mate.Stats, stat.Resource);
                        stat.Amount = Math.Clamp((maternal + paternal) * 0.5 + (Roll() - 0.5) * Pace.KittenStatSpread, 0, 100);
                    }
                    var den = v.Buildings.Where(b => b.Completed && b.Kind == "den").FirstOrDefault(b => v.Cats.Count(x => x.Alive && x.BedId == b.Id) < DenCapacity(v));
                    if (den != null)
                    {
                        kitten.BedId = den.Id;
                        v.Cats.Add(kitten);
                        c.PregnantUntil = -1;
                        c.PregnancyMateId = "";
                        Note(v, "birth", kitten.Name + " was born", kitten.Id);
                    }
                }
                if (c.AgeHours >= (c.Id == v.LeaderId || c.OfficerRole == "loremaster" ? Pace.HonouredElderAgeHours : Pace.ElderAgeHours) && Roll() < Pace.OldAgeDeathChance / Service(v, "elder_corner", "elderProtection", 1))
                    Die(v, c, "old age");
            }
            if ((long)TimeSeconds % Pace.BreedingCheckSeconds == 0 && TimeSeconds - v.FoundedAt >= Pace.BreedingAfterFoundingSeconds && Total(v, "food") >= alive.Length * Pace.BreedingFoodPerCat && Total(v, "water") >= alive.Length * Pace.BreedingWaterPerCat)
            {
                int reservations = alive.Count(c => c.PregnantUntil > 0);
                if (alive.Length + reservations < beds)
                {
                    var mother = alive.FirstOrDefault(c => c.AgeHours >= Pace.ParentMinAgeHours && c.AgeHours < Pace.ParentMaxAgeHours && c.PregnantUntil < 0);
                    var mate = alive.FirstOrDefault(c => c != mother && c.AgeHours >= Pace.ParentMinAgeHours && c.AgeHours < Pace.ParentMaxAgeHours);
                    if (mother != null && mate != null && Roll() < Pace.BreedChanceBase + Math.Min(Pace.BreedBlessingCap, v.Blessings * Pace.BreedBlessingStep) * Pace.BreedBlessingWeight)
                    {
                        mother.PregnantUntil = TimeSeconds + Pace.GestationSeconds;
                        mother.PregnancyMateId = mate.Id;
                        Note(v, "pregnancy", mother.Name + " reserved a kitten bed", mother.Id);
                    }
                }
            }
            if (TimeSeconds - v.FoundedAt >= Pace.MigrationFirstAtSeconds && TimeSeconds - v.LastMigration >= Pace.MigrationIntervalSeconds)
            {
                v.LastMigration = TimeSeconds;
                if (Total(v, "food") >= alive.Length * Pace.MigrationFoodPerCat && Total(v, "water") >= alive.Length * Pace.MigrationWaterPerCat && alive.Length < Pace.MigrationPopulationCap)
                {
                    var migrant = NewCat(v, "Wanderer " + NextId, new Int2(v.Center.X, v.Center.Z + v.Radius + 3));
                    migrant.Migration = "arriving";
                    v.Cats.Add(migrant);
                    Note(v, "migration", migrant.Name + " approaches the gate", migrant.Id);
                }
            }
        }
    }
}
