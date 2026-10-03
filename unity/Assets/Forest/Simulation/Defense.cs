using System;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary><see cref="Cat.RalliedTo"/> of a cat waiting at the south gate for a warned raid.</summary>
        public const string RallyGate = "gate";

        private double RaidReferenceStrength => Pace.RaidReferenceStrength;
        private int RallySize => Pace.RallySize;
        private double RallyCooldownSeconds => Pace.RallyCooldownSeconds;
        private double DefenderFloorHealth => Pace.DefenderFloorHealth;
        private double RelicCombatBonus => Pace.RelicCombatBonus;

        public double EquipmentPower(Village v, Cat c, string kind)
        {
            var item = v.Items.FirstOrDefault(i => i.LocationId == c.Id && c.Equipment.Contains(i.Id) && i.Kind == kind && i.Condition > 0);
            if (item == null)
                return 1;
            double material = item.Material == "metal" ? 1.5 : item.Material == "stone" ? 1.2 : item.Material == "bone" ? 1.1 : 1;
            return 1 + (item.Quality + 1) * 0.2 * material * Math.Min(1, item.Condition / Math.Max(1, item.MaxCondition));
        }

        /// <summary>The colony's combat multiplier from a relic kept in its storage (1 without one; relics do not stack).</summary>
        public double ColonyRelicFactor(Village v) =>
            v != null && v.Items.Any(i => i.Kind == "relic" && i.Condition > 0 && v.Stockpiles.Any(p => p.Kind == "storage" && p.Id == i.LocationId)) ? RelicCombatBonus : 1;

        /// <summary>Share of a party still standing: raiders fall one by one as its shared health drops.</summary>
        private static double LivingShare(Raid raid)
        {
            if (raid.Members <= 0 || raid.MaxHealth <= 0)
                return 1;
            double each = raid.MaxHealth / raid.Members;
            return Math.Clamp(Math.Ceiling(Math.Max(0, raid.Health) / each - 1e-9), 0, raid.Members) / raid.Members;
        }

        /// <summary>The party's strength as a damage multiplier; a legacy raid of Strength 30 is exactly 1.</summary>
        private double RaiderStrength(Raid raid) => Math.Max(0, raid.Strength) / RaidReferenceStrength * LivingShare(raid);

        /// <summary>Just inside the south gate, where raiders come in and rallied defenders wait.</summary>
        private static Int2 SouthGate(Village v) => new Int2(v.Center.X, v.Center.Z + v.Radius - 1);

        private bool CanDefend(Cat c) => c.Alive && c.ControlledBy == "" && c.Position.Level == 0 && !IsDungeonExplorer(c) && !c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal) && c.Health > DefenderFloorHealth;

        private bool RaidThreat(Village v) => v.Raids.Count > 0 || v.ScriptedRaidStage == 1;

        /// <summary>
        /// Sends the nearest free adults to the raiders (or to the south gate while a raid is only
        /// warned): they drop their work and walk there physically. No direct damage; one rally per
        /// cooldown. The DefendRaid action and a Captain both use it.
        /// </summary>
        public ActionResult RallyDefenders(Village v)
        {
            if (v == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "No village to defend");
            if (!RaidThreat(v))
                return ActionResult.Fail(ResultCode.NoActiveRaid, "No raid is approaching");
            if (TimeSeconds + 1e-9 < v.RallyReadyAt)
                return ActionResult.Fail(ResultCode.RallyCooldown, "Defenders were rallied moments ago", ("seconds", Math.Ceiling(v.RallyReadyAt - TimeSeconds).ToString(CultureInfo.InvariantCulture)));
            var raid = v.Raids.FirstOrDefault();
            var point = raid?.Position ?? SouthGate(v);
            var chosen = v.Cats.Where(c => CanDefend(c) && c.RalliedTo == "" && c.AgeHours >= Pace.WorkerMinAgeHours && c.Migration != "arriving" && c.Migration != "departing" && !v.Routes.Any(r => r.CatId == c.Id))
                .OrderBy(c => Int2.Distance(c.Position, point)).ThenBy(c => c.Id, StringComparer.Ordinal).Take(RallySize).ToList();
            if (chosen.Count == 0)
                return ActionResult.Fail(ResultCode.NoDefenders, "No adult is free to defend the colony");
            foreach (var c in chosen)
            {
                if (c.JobId != "" || c.BuildingId != "")
                    CancelWork(v, c);
                c.Path.Clear();
                c.RalliedTo = raid?.Id ?? RallyGate;
                c.Goal = GoalKey.Defending;
            }
            v.RallyReadyAt = TimeSeconds + RallyCooldownSeconds;
            Note(v, EventKind.Defense, chosen.Count + " cats rallied to defend the south gate", raid?.Id ?? "");
            return ActionResult.Ok(ResultCode.Rallied, raid?.Id ?? "", ("count", chosen.Count.ToString(CultureInfo.InvariantCulture)));
        }

        private static void StandDown(Cat c)
        {
            c.RalliedTo = "";
            if (c.Goal == GoalKey.Defending)
            {
                c.Goal = GoalKey.Idle;
                c.Path.Clear();
            }
        }

        /// <summary>Points gate waiters at an arrived raid, moves rallies off finished raids, and releases them when no threat is left.</summary>
        private void ReconcileRallies(Village v)
        {
            foreach (var c in v.Cats.Where(c => c.RalliedTo != ""))
            {
                if (!c.Alive || c.ControlledBy != "" || IsDungeonExplorer(c))
                    StandDown(c);
                else if (v.Raids.Count > 0 && (c.RalliedTo == RallyGate || !v.Raids.Any(r => r.Id == c.RalliedTo)))
                    c.RalliedTo = v.Raids[0].Id;
                else if (v.Raids.Count == 0 && !(c.RalliedTo == RallyGate && v.ScriptedRaidStage == 1))
                    StandDown(c);
            }
        }

        /// <summary>After a raid ends, its rallied cats and any defender left without a raid go back to colony life.</summary>
        private void EndRaid(Village v, Raid raid)
        {
            v.Raids.Remove(raid);
            foreach (var c in v.Cats.Where(c => c.RalliedTo == raid.Id || v.Raids.Count == 0 && c.Goal == GoalKey.Defending))
                StandDown(c);
        }

        /// <summary>Raid injuries: lethal only where the profile allows raid deaths, and then recorded as a raid death.</summary>
        private void HurtByRaid(Village v, Cat c, double amount)
        {
            if (amount <= 0)
                return;
            if (!Pace.RaidCanKill)
            {
                c.Health = Math.Max(Math.Min(c.Health, 1), c.Health - amount);
                return;
            }
            c.Health -= amount;
            if (c.Health <= 0)
            {
                c.Health = 0;
                Die(v, c, "raid");
            }
        }

        private void TickRaids(Village v, double dt, bool planning)
        {
            if (planning)
            {
                ReconcileRallies(v);
                if (RaidThreat(v) && HasOfficer(v, "captain") && TimeSeconds + 1e-9 >= v.RallyReadyAt && !v.Cats.Any(c => c.Alive && c.RalliedTo != ""))
                    RallyDefenders(v);
            }
            foreach (var waiting in v.Cats.Where(c => c.RalliedTo == RallyGate && CanDefend(c)).ToArray())
            {
                if (waiting.JobId != "" || waiting.BuildingId != "")
                    CancelWork(v, waiting);
                waiting.Goal = GoalKey.Defending;
                Move(waiting, SouthGate(v), dt);
            }
            foreach (var raid in v.Raids.ToArray())
            {
                bool firstPlan = !raid.HasContinuousPosition;
                if (!raid.HasContinuousPosition)
                {
                    raid.X = raid.Position.X;
                    raid.Z = raid.Position.Z;
                    raid.Progress = 0;
                    raid.HasContinuousPosition = true;
                }
                if (raid.Health <= 0)
                {
                    Spill(v, raid.Position, raid.Loot);
                    EndRaid(v, raid);
                    v.RaidsRepelled++;
                    Note(v, EventKind.RaidRepelled, raid.Members > 0 ? "The raiders were driven off" : "Raid repelled", raid.Id);
                    continue;
                }
                // Every cat that can fight, and every cat rallied against this raid, engages it.
                bool engaged = false;
                var defenders = v.Cats.Where(c => CanDefend(c) && (Amount(c.Skills, "fight") > 0 || c.RalliedTo == raid.Id))
                    .OrderByDescending(c => Amount(c.Skills, "fight")).ThenBy(c => c.Id, StringComparer.Ordinal).ToArray();
                foreach (var defender in defenders)
                {
                    if (!defender.Alive || raid.Health <= 0)
                        break;
                    if (defender.JobId != "" || defender.BuildingId != "")
                        CancelWork(v, defender);
                    defender.Goal = GoalKey.Defending;
                    if (Move(defender, raid.Position, dt) && Math.Abs(defender.X - raid.X) + Math.Abs(defender.Z - raid.Z) <= 1)
                    {
                        double fighting = SpendActorTime(defender);
                        if (fighting > 0)
                        {
                            double injury = fighting * Pace.RaiderStrikePerSecond * RaiderStrength(raid) / Math.Max(1, EquipmentPower(v, defender, "armor") * Catalog.Effect(this, v, "defensePower", 1) * Catalog.Effect(this, v, "defenseMult", 1) * Service(v, "walls", "wallDefense", 1));
                            raid.Health -= fighting * Pace.DefenderStrikePerSecond * EquipmentPower(v, defender, "weapon") * WorkRate(v, defender, "fight", "barracks") * Catalog.Effect(this, v, "combatPower", 1) * Catalog.Effect(this, v, "combatPowerMult", 1) * Service(v, "barracks", "barracksReadiness", 1) * ColonyRelicFactor(v);
                            GainSkill(defender, "fight", fighting / Pace.DefenderSkillSecondsPerPoint);
                            foreach (var item in v.Items.Where(i => defender.Equipment.Contains(i.Id)))
                                item.Condition = Math.Max(0, item.Condition - fighting * Pace.DefenderWeaponWearPerSecond);
                            HurtByRaid(v, defender, injury);
                            engaged = true;
                        }
                    }
                }
                // Engaged raiders stand and fight instead of advancing.
                if (engaged)
                    continue;
                Int2 destination = raid.Phase == "departing" ? new Int2(v.Center.X, v.Center.Z + v.Radius + 3) : v.Center;
                if (!raid.Position.Equals(destination))
                {
                    if (raid.Path.Count == 0 || !Walkable(raid.Path[0]) || !Crossable(raid.Position, raid.Path[0]))
                    {
                        if (!AtTravelPoint(raid.X, raid.Z, raid.Position))
                        {
                            AdvanceTraveler(ref raid.X, ref raid.Z, raid.Position, Pace.RaidSpeed, dt);
                            raid.Progress = TravelProgress(raid.X, raid.Z, raid.Position);
                            continue;
                        }
                        if (planning || firstPlan)
                            raid.Path = Path(raid.Position, destination) ?? new System.Collections.Generic.List<Int2>();
                    }
                    if (raid.Path.Count > 0 && Walkable(raid.Path[0]) && Crossable(raid.Position, raid.Path[0]))
                    {
                        if (AdvanceTraveler(ref raid.X, ref raid.Z, raid.Path[0], Pace.RaidSpeed, dt))
                        {
                            raid.Position = raid.Path[0];
                            raid.Path.RemoveAt(0);
                        }
                        raid.Progress = TravelProgress(raid.X, raid.Z, raid.Position);
                    }
                    continue;
                }
                if (raid.Phase == "departing")
                {
                    EndRaid(v, raid);
                    Note(v, EventKind.RaidLoss, "Raiders escaped with stolen supplies", raid.Id);
                    continue;
                }
                raid.Phase = "departing";
                double amount = Math.Min(Pace.RaidLootMax, Available(v, "food") * Pace.RaidLootShare);
                if (amount > 0 && Spend(v, "food", amount))
                    Add(raid.Loot, "food", amount);
                // A lost demo raid costs food only; where raids can kill, the cat nearest the shrine is hurt.
                if (Pace.RaidCanKill)
                {
                    var victim = v.Cats.Where(c => c.Alive && c.ControlledBy == "").OrderBy(c => Int2.Distance(c.Position, v.Center)).FirstOrDefault();
                    if (victim != null)
                        HurtByRaid(v, victim, Math.Min(victim.Health, Pace.RaidBreachDamage / Service(v, "walls", "wallDefense", 1)));
                }
                Note(v, EventKind.RaidBreach, "Raiders reached the shrine and looted finite supplies", raid.Id);
            }
        }
    }
}
