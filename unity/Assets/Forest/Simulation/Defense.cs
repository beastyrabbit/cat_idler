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
        private bool AvailableDefender(Village v, Cat c) => c != null && c.Alive && c.Health > 0 && c.AgeHours >= 12 && FreeWorker(v, c) && c.ResumeJobId == "" && c.Cargo.Count == 0 && !c.Goal.StartsWith("need_", StringComparison.Ordinal) && c.Migration != "arriving" && c.Migration != "departing" && !v.Jobs.Any(j => !j.Completed && j.CatId == c.Id);
        private Job StartRaidDefense(Village v, Cat c, Raid raid, bool manual)
        {
            var job = new Job { Kind = "defend_raid", TargetId = raid.Id, Position = raid.Position, Manual = manual, AutomatedBy = manual ? "" : "captain", Requester = manual ? "player" : "captain", RequiredWork = 0 };
            StartJob(v, c, job);
            return job;
        }
        private ActionResult OrderRaidDefense(Village v, Cat selected, GameAction action)
        {
            var raid = string.IsNullOrEmpty(action.TargetId) ? v.Raids.FirstOrDefault(r => r.Health > 0) : v.Raids.Find(r => r.Id == action.TargetId && r.Health > 0);
            if (raid == null)
                return ActionResult.Fail("No matching active raid");
            if (!string.IsNullOrEmpty(action.CatId) && selected == null)
                return ActionResult.Fail("Selected living cat does not belong to this village");
            var existing = v.Jobs.Find(j => !j.Completed && j.Kind == "defend_raid" && j.TargetId == raid.Id && (selected == null ? j.Manual : j.CatId == selected.Id) && v.Cats.Any(c => c.Id == j.CatId && c.Alive && (c.JobId == j.Id || c.ResumeJobId == j.Id)));
            if (existing != null)
                return ActionResult.Ok(existing.Id);
            var cat = selected ?? v.Cats.Where(c => AvailableDefender(v, c)).OrderByDescending(c => Amount(c.Skills, "fight")).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
            if (!AvailableDefender(v, cat))
                return ActionResult.Fail("Choose an available adult on the surface without work, cargo, control or urgent needs");
            return ActionResult.Ok(StartRaidDefense(v, cat, raid, true).Id);
        }
        private ActionResult CancelRaidDefense(Village v, Cat cat, string jobId)
        {
            var job = v.Jobs.Find(j => j.Id == jobId && !j.Completed && j.Kind == "defend_raid" && j.Manual && j.AutomatedBy == "");
            if (job == null || cat == null || job.CatId != cat.Id || cat.JobId != job.Id && (cat.JobId != "" || cat.ResumeJobId != job.Id))
                return ActionResult.Fail("The selected cat no longer owns that manual defense order");
            // Old saves can retain only ResumeJobId while attending a need.
            cat.JobId = job.Id;
            CancelWork(v, cat);
            return ActionResult.Ok(job.Id);
        }
        private void CompleteRaidDefense(Village v, Job job)
        {
            job.Completed = true;
            job.CompletionTime = TimeSeconds;
            job.BlockedReason = "";
            var cat = v.Cats.Find(c => c.Id == job.CatId);
            if (cat == null)
                return;
            bool attendingNeed = cat.Goal.StartsWith("need_", StringComparison.Ordinal);
            if (cat.JobId == job.Id)
            {
                cat.JobId = "";
                if (!attendingNeed)
                {
                    cat.Goal = "idle";
                    cat.Path.Clear();
                    cat.BlockedReason = "";
                }
            }
            if (cat.ResumeJobId == job.Id)
                cat.ResumeJobId = "";
        }
        private void CleanRaidDefense(Village v)
        {
            foreach (var job in v.Jobs.Where(j => !j.Completed && j.Kind == "defend_raid").ToArray())
            {
                var cat = v.Cats.Find(c => c.Id == job.CatId && c.Alive);
                if (cat == null || !v.Raids.Any(r => r.Id == job.TargetId && r.Health > 0) || cat.JobId != job.Id && cat.ResumeJobId != job.Id || job.AutomatedBy == "captain" && !HasOfficer(v, "captain"))
                    CompleteRaidDefense(v, job);
            }
        }
        private bool AdvanceRaidDefense(Village v, Raid raid, Cat warrior, Job job, double dt)
        {
            job.Position = raid.Position;
            job.Phase = "travel";
            warrior.Goal = "defending";
            if (!Move(warrior, raid.Position, dt))
            {
                job.BlockedReason = warrior.BlockedReason;
                return false;
            }
            job.BlockedReason = "";
            if (Math.Abs(warrior.X - raid.X) + Math.Abs(warrior.Z - raid.Z) > 1)
                return false;
            double fighting = SpendActorTime(warrior);
            if (fighting <= 0)
                return false;
            job.Phase = "fighting";
            job.Progress += fighting;
            raid.Health -= fighting * 0.1 * EquipmentPower(v, warrior, "weapon") * WorkRate(v, warrior, "fight", "barracks") * Catalog.Effect(v, "combatPower", 1) * Catalog.Effect(v, "combatPowerMult", 1) * Service(v, "barracks", "barracksReadiness", 1);
            warrior.Health -= fighting * 0.03 / Math.Max(1, EquipmentPower(v, warrior, "armor") * Catalog.Effect(v, "defensePower", 1) * Catalog.Effect(v, "defenseMult", 1) * Service(v, "walls", "wallDefense", 1));
            Add(warrior.Skills, "fight", fighting / 3600);
            foreach (var item in v.Items.Where(i => i.LocationId == warrior.Id && warrior.Equipment.Contains(i.Id)))
                item.Condition = Math.Max(0, item.Condition - fighting * 0.005);
            return true;
        }
        private bool TryDefendRaid(Village v, Raid raid, double dt)
        {
            bool engaged = false, pursuing = false;
            foreach (var job in v.Jobs.Where(j => !j.Completed && j.Kind == "defend_raid" && j.TargetId == raid.Id).OrderByDescending(j => j.Manual).ThenBy(j => j.Id, StringComparer.Ordinal).ToArray())
            {
                if (raid.Health <= 0)
                    break;
                var warrior = v.Cats.Find(c => c.Id == job.CatId && c.Alive && c.Health > 0 && c.ControlledBy == "" && c.Position.Level == 0 && !IsDungeonExplorer(c) && c.JobId == job.Id && !c.Goal.StartsWith("need_", StringComparison.Ordinal));
                if (warrior == null || ActorTime(warrior) <= 1e-12)
                    continue;
                double available = ActorTime(warrior);
                if (AdvanceRaidDefense(v, raid, warrior, job, dt))
                    engaged = true;
                if (ActorTime(warrior) < available - 1e-12)
                    pursuing = true;
            }
            // Every explicitly assigned defender gets its own quantum. An existing
            // pursuit or engagement needs no extra automatic draft this quantum.
            if (engaged || pursuing)
                return engaged;
            if (!HasOfficer(v, "captain"))
                return false;
            foreach (var warrior in v.Cats.Where(c => AvailableDefender(v, c) && Amount(c.Skills, "fight") > 0 && ActorTime(c) > 1e-12).OrderByDescending(c => Amount(c.Skills, "fight")).ThenBy(c => c.Id, StringComparer.Ordinal).ToArray())
            {
                var job = StartRaidDefense(v, warrior, raid, false);
                double available = ActorTime(warrior);
                if (AdvanceRaidDefense(v, raid, warrior, job, dt))
                    return true;
                // Pursuit spends this raid's defender quantum; obstruction may yield
                // to another genuinely free warrior without cancelling anyone's work.
                if (ActorTime(warrior) < available - 1e-12)
                    return false;
            }
            return false;
        }
        private void TickRaids(Village v, double dt, bool planning)
        {
            CleanRaidDefense(v);
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
                    v.Raids.Remove(raid);
                    Note(v, "defense", "Raid repelled", raid.Id);
                    continue;
                }
                if (TryDefendRaid(v, raid, dt))
                    continue;
                Int2 destination = raid.Phase == "departing" ? new Int2(v.Center.X, v.Center.Z + v.Radius + 3) : v.Center;
                if (!raid.Position.Equals(destination))
                {
                    if (raid.Path.Count == 0 || !Walkable(raid.Path[0]) || !Crossable(raid.Position, raid.Path[0]))
                    {
                        if (!AtTravelPoint(raid.X, raid.Z, raid.Position))
                        {
                            AdvanceTraveler(ref raid.X, ref raid.Z, raid.Position, 0.5, dt);
                            raid.Progress = TravelProgress(raid.X, raid.Z, raid.Position);
                            continue;
                        }
                        if (planning || firstPlan)
                            raid.Path = Path(raid.Position, destination) ?? new System.Collections.Generic.List<Int2>();
                    }
                    if (raid.Path.Count > 0 && Walkable(raid.Path[0]) && Crossable(raid.Position, raid.Path[0]))
                    {
                        if (AdvanceTraveler(ref raid.X, ref raid.Z, raid.Path[0], 0.5, dt))
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
            CleanRaidDefense(v);
        }
    }
}
