using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>One simulation step of a village: staffing, regrowth, every cat, then the village systems in a fixed order.</summary>
        private void TickVillage(Village v, double dt, bool planning)
        {
            if (planning)
                EnsureScaffoldJobs(v);
            if (planning)
                foreach (var pending in v.Jobs.Where(j => !j.Completed && j.CatId == "" && (j.AutomatedBy == "" || j.AutomatedBy == "leader" || HasOfficer(v, j.AutomatedBy))).ToArray())
                {
                    var worker = AvailableCat(v, JobLabor(pending.Kind));
                    if (worker == null)
                        break;
                    if (worker.Cargo.Count > 0)
                        Spill(v, worker.Position, worker.Cargo);
                    pending.CatId = worker.Id;
                    worker.JobId = pending.Id;
                    worker.Goal = pending.Kind;
                }
            if (planning && (long)TimeSeconds % Pace.RegrowthCheckSeconds == 0)
                foreach (var tile in Tiles.Where(t => t.RegrowAt >= 0 && t.RegrowAt <= TimeSeconds).ToArray())
                {
                    tile.Amount = Math.Max(tile.Amount, tile.Resource == "logs" ? Pace.RegrownLogs : Pace.RegrownForage);
                    tile.RegrowAt = -1;
                }
            foreach (var c in v.Cats.Where(c => c.Alive).ToArray())
            {
                BeginActor(c);
                if (c.Health <= 0)
                {
                    Die(v, c, "needs");
                    continue;
                }
                double resilience = Math.Max(Pace.ResilienceFloor, 1 - UpgradeLevel(v, "resilience") * Pace.ResiliencePerLevel);
                c.AgeHours += dt * (c.AgeHours < Pace.KittenUntilHours ? Service(v, "nursery", "kittenGrowth", 1) : 1) / 3600;
                c.Hunger = Math.Max(0, c.Hunger - dt * Pace.HungerDecayPerHour / 3600 * resilience * c.Metabolism);
                c.Thirst = Math.Max(0, c.Thirst - dt * Pace.ThirstDecayPerHour / 3600 * resilience * c.Metabolism / Catalog.Effect(this, v, "waterEfficiency", 1) / Service(v, "water_bowl", "waterStewardship", 1));
                c.Rest = Math.Max(0, c.Rest - dt * Pace.RestDecayPerHour / 3600 * c.Metabolism / Service(v, "den", "denStewardship", 1));
                if (c.Health < 100 && c.Hunger > Pace.HealthRegenMinNeed && c.Thirst > Pace.HealthRegenMinNeed)
                    c.Health = Math.Min(100, c.Health + dt * Pace.HealthRegenPerSecond * Catalog.Effect(this, v, "healthRecovery", 1));
                double beforeNeeds = c.Health;
                if (c.Hunger <= 0)
                    c.Health -= dt * Pace.StarvationDamagePerMinute / 60;
                if (c.Thirst <= 0)
                    c.Health -= dt * Pace.DehydrationDamagePerMinute / 60;
                // Demo worlds stop need damage at a critical floor instead of killing (D16).
                if (!Pace.NeedsCanKill && c.Health < beforeNeeds)
                    c.Health = Math.Max(c.Health, Math.Min(beforeNeeds, Pace.NeedsHealthFloor));
                if (c.Health <= 0)
                {
                    Die(v, c, "needs");
                    continue;
                }
                if (c.Migration == "arriving" || c.Migration == "departing")
                {
                    Int2 destination = c.Migration == "arriving" ? v.Center : c.MigrationExterior ?? new Int2(v.Center.X, v.Center.Z + v.Radius + 3);
                    if (Move(c, destination, dt))
                    {
                        if (c.Migration == "arriving")
                        {
                            c.Migration = "probationary";
                            c.ProbationUntil = TimeSeconds + Pace.ProbationSeconds;
                        }
                        else
                        {
                            c.Alive = false;
                            v.MigrationDepartures++;
                            Note(v, EventKind.Migration, c.Name + " left the colony", c.Id);
                        }
                    }
                    continue;
                }
                if (c.ControlledBy != "")
                {
                    if (c.ControlLeaseUntil <= TimeSeconds)
                    {
                        c.ControlledBy = "";
                        c.Path.Clear();
                        c.Goal = GoalKey.Idle;
                    }
                    else
                        continue;
                }
                if (IsDungeonExplorer(c) || c.Position.Level != 0)
                    continue;
                if (TickNeeds(v, c, dt))
                {
                    SpendActorTime(c);
                    continue;
                }
                var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
                if (job != null)
                    TickJob(v, c, job, dt);
                else if (c.JobId != "")
                    c.JobId = "";
                else if (c.BuildingId == "" && !DefendsThisStep(v, c))
                    TickIdle(v, c, dt, planning);
            }
            EndActor();
            TickStations(v, dt, planning);
            TickFarms(v, dt);
            TickAccounting(v, dt);
            TickTransport(v, dt, planning);
            TickTrader(v, dt, planning);
            if (planning && (long)TimeSeconds % Pace.LeaderDirectSeconds == 0)
                Direct(v);
            if (planning)
                Governance(v);
            if (planning && (long)TimeSeconds % Pace.UpkeepTickSeconds == 0)
            {
                Population(v);
                Spoilage(v);
                v.Jobs.RemoveAll(j => j.Completed && TimeSeconds - j.StartedAt > Pace.CompletedJobRetentionSeconds);
            }
            TickRaids(v, dt, planning);
        }

        /// <summary>
        /// A cat that TickRaids moves and fights this step: rallied, or able to fight while a raid is on. Idle life
        /// must not spend its movement time first (W2-03 idle walks against W2-09 defenders).
        /// </summary>
        private bool DefendsThisStep(Village v, Cat c) => c.RalliedTo != "" || v.Raids.Count > 0 && CanDefend(c) && Amount(c.Skills, "fight") > 0;
    }
}
