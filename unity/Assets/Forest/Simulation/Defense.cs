using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        public double EquipmentPower(Village v, Cat c, string kind)
        {
            var item = v.Items.FirstOrDefault(i => i.LocationId == c.Id && c.Equipment.Contains(i.Id) && i.Kind == kind && i.Condition > 0);
            if (item == null)
                return 1;
            double material = item.Material == "metal" ? 1.5 : item.Material == "stone" ? 1.2 : item.Material == "bone" ? 1.1 : 1;
            return 1 + (item.Quality + 1) * 0.2 * material * Math.Min(1, item.Condition / Math.Max(1, item.MaxCondition));
        }
        private void TickRaids(Village v)
        {
            foreach (var raid in v.Raids.ToArray())
            {
                if (raid.Health <= 0)
                {
                    Spill(v, raid.Position, raid.Loot);
                    v.Raids.Remove(raid);
                    Note(v, "defense", "Raid repelled", raid.Id);
                    continue;
                }
                var warrior = v.Cats.Where(c => c.Alive && c.ControlledBy == "" && !c.Goal.StartsWith("need_", StringComparison.Ordinal) && Amount(c.Skills, "fight") > 0).OrderByDescending(c => Amount(c.Skills, "fight")).FirstOrDefault();
                if (warrior != null)
                {
                    if (warrior.JobId != "" || warrior.BuildingId != "")
                        CancelWork(v, warrior);
                    warrior.Goal = "defending";
                    if (Move(warrior, raid.Position, 1))
                    {
                        raid.Health -= 0.1 * EquipmentPower(v, warrior, "weapon") * WorkRate(v, warrior, "fight", "barracks") * Catalog.Effect(v, "combatPower", 1) * Catalog.Effect(v, "combatPowerMult", 1) * Service(v, "barracks", "barracksReadiness", 1);
                        warrior.Health -= 0.03 / Math.Max(1, EquipmentPower(v, warrior, "armor") * Catalog.Effect(v, "defensePower", 1) * Catalog.Effect(v, "defenseMult", 1) * Service(v, "walls", "wallDefense", 1));
                        Add(warrior.Skills, "fight", 1.0 / 3600);
                        foreach (var item in v.Items.Where(i => warrior.Equipment.Contains(i.Id)))
                            item.Condition = Math.Max(0, item.Condition - 0.005);
                        continue;
                    }
                }
                Int2 destination = raid.Phase == "departing" ? new Int2(v.Center.X, v.Center.Z + v.Radius + 3) : v.Center;
                if (!raid.Position.Equals(destination))
                {
                    if (raid.Path.Count == 0 || !Walkable(raid.Path[0]) || !Crossable(raid.Position, raid.Path[0]))
                        raid.Path = Path(raid.Position, destination) ?? new System.Collections.Generic.List<Int2>();
                    raid.Progress += 0.5;
                    if (raid.Progress >= 1 && raid.Path.Count > 0)
                    {
                        raid.Progress -= 1;
                        raid.Position = raid.Path[0];
                        raid.Path.RemoveAt(0);
                    }
                    continue;
                }
                if (raid.Phase == "departing")
                {
                    v.Raids.Remove(raid);
                    Note(v, "raid_loss", "Raiders escaped with stolen supplies", raid.Id);
                    continue;
                }
                raid.Phase = "departing";
                double amount = Math.Min(20, Available(v, "food") * 0.3);
                if (amount > 0 && Spend(v, "food", amount))
                    Add(raid.Loot, "food", amount);
                var victim = v.Cats.Where(c => c.Alive && c.ControlledBy == "").OrderBy(c => Int2.Distance(c.Position, v.Center)).FirstOrDefault();
                if (victim != null)
                    victim.Health = Math.Max(0, victim.Health - 20 / Service(v, "walls", "wallDefense", 1));
                Note(v, "raid_breach", "Raiders reached the shrine and looted finite supplies", raid.Id);
            }
        }
    }
}
