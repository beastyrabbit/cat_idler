using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Living cats: a varied idle life outside every footprint, and a typed activity for every goal.</summary>
    public static class LivelinessScenarios
    {
        static readonly string[] Profiles = { Pacing.ClassicProfile, Pacing.DemoProfile };
        static readonly string[] IdleGoals = { GoalKey.IdleNap, GoalKey.IdleGroom, GoalKey.IdleSocial, GoalKey.IdlePatrol, GoalKey.IdleShrine, GoalKey.IdlePlay };

        public static IEnumerable<Scenario> Cases()
        {
            foreach (var profile in Profiles)
            {
                string p = profile;
                yield return new Scenario("liveliness.colony_ten_minutes." + p, () => ColonyTenMinutes(p));
                yield return new Scenario("liveliness.idle_spot_built_over." + p, () => IdleSpotBuiltOver(p));
                yield return new Scenario("liveliness.kitten_plays." + p, () => KittenPlays(p));
            }
            yield return new Scenario("liveliness.activity_for_every_goal", ActivityForEveryGoal);
            yield return new Scenario("liveliness.activity_for_every_job_kind_and_phase", ActivityForEveryJob);
        }

        internal static bool Inside(Int2 at, int width, int depth, Int2 p) => p.Level == at.Level && p.X >= at.X && p.X < at.X + width && p.Z >= at.Z && p.Z < at.Z + depth;

        /// <summary>A building, store (not a spill) or farm covers this tile: the RoadSpace footprint.</summary>
        internal static bool InFootprint(Village v, Int2 p) =>
            v.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, p))
            || v.Stockpiles.Any(s => s.Kind != "spill" && !s.Kind.StartsWith("zone_", StringComparison.Ordinal) && Inside(s.Position, s.Width, s.Depth, p))
            || v.Farms.Any(f => Inside(f.Position, f.Width, f.Depth, p));

        internal static bool InBuilding(Village v, Int2 p) => v.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, p));

        internal static bool AtCentre(Cat c) => Math.Abs(c.X - c.Position.X) + Math.Abs(c.Z - c.Position.Z) < 1e-9;

        internal static World ProfileWorld(string profile, out Village v)
        {
            var w = World.Create(41);
            w.PacingProfile = profile;
            v = w.Villages[0];
            return w;
        }

        /// <summary>
        /// Ten minutes of a fresh 30-cat colony: every idle activity appears, no idle cat ever stands in a
        /// footprint, at most two share a tile, nobody stays without an activity, and no need trip is
        /// served inside a building.
        /// </summary>
        static void ColonyTenMinutes(string profile)
        {
            var w = ProfileWorld(profile, out var v);
            var seen = new HashSet<string>();
            var idleSince = new Dictionary<string, int>();
            int violations = 0, crowded = 0, picks = 0, needStarts = 0, longestIdle = 0;
            var previous = v.Cats.ToDictionary(c => c.Id, c => c.Goal);
            for (int second = 1; second <= 600; second++)
            {
                w.Step(1);
                var standing = new Dictionary<Int2, int>();
                foreach (var c in v.Cats.Where(c => c.Alive && c.Position.Level == 0))
                {
                    if (previous.TryGetValue(c.Id, out var before) && before != c.Goal)
                    {
                        if (World.IsIdleActivity(c.Goal))
                            picks++;
                        if (World.BusyWithNeed(c) && !before.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                            needStarts++;
                    }
                    previous[c.Id] = c.Goal;
                    bool still = c.Path.Count == 0 && AtCentre(c);
                    if (World.IsIdleActivity(c.Goal))
                    {
                        seen.Add(c.Goal);
                        if (still)
                        {
                            if (InFootprint(v, c.Position))
                                violations++;
                            standing[c.Position] = standing.TryGetValue(c.Position, out int n) ? n + 1 : 1;
                        }
                    }
                    if (World.BusyWithNeed(c) && still && c.HasNeedSpot && c.Position.Equals(c.NeedSpot) && InBuilding(v, c.Position))
                        violations++;
                    bool unoccupied = c.Goal == GoalKey.Idle && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.Migration != "arriving" && c.Migration != "departing";
                    idleSince[c.Id] = unoccupied ? (idleSince.TryGetValue(c.Id, out int s) ? s + 1 : 1) : 0;
                    longestIdle = Math.Max(longestIdle, idleSince[c.Id]);
                }
                crowded += standing.Count(e => e.Value > 2);
            }
            Console.WriteLine("liveliness.curve profile=" + profile + " distinct=" + seen.Count + " activities=[" + string.Join(",", seen.OrderBy(x => x, StringComparer.Ordinal)) + "] picks=" + picks + " need_starts=" + needStarts + " footprint_violations=" + violations + " crowded_tiles=" + crowded + " longest_unoccupied_s=" + longestIdle);
            Check(violations == 0, "Idle or need-serving cats stood inside footprints " + violations + " times");
            Check(crowded == 0, "More than two idle cats shared a tile " + crowded + " times");
            Check(seen.Count >= 6, "Only " + seen.Count + " distinct idle activities in ten minutes: " + string.Join(",", seen));
            Check(longestIdle <= 3, "A cat stood without any activity for " + longestIdle + " s");
            Valid(w);
        }

        /// <summary>An idle cat whose spot gets built over leaves it within the next planning second.</summary>
        static void IdleSpotBuiltOver(string profile)
        {
            var w = ProfileWorld(profile, out var v);
            Cat c = null;
            for (int second = 0; second < 120 && c == null; second++)
            {
                w.Step(1);
                c = v.Cats.Where(o => o.Alive && World.IsIdleActivity(o.Goal) && o.Path.Count == 0 && AtCentre(o) && o.HasIdleSpot && o.Position.Equals(o.IdleSpot)).OrderBy(o => o.Id, StringComparer.Ordinal).FirstOrDefault();
            }
            Check(c != null, "No idle cat settled on a spot within two minutes");
            var spot = c.Position;
            v.Buildings.Add(new Building { Id = w.Id("built-over"), Kind = "den", Position = spot, Width = 1, Depth = 1 });
            w.Step(2);
            Check(!c.HasIdleSpot || !c.IdleSpot.Equals(spot) || !World.IsIdleActivity(c.Goal), "Idle cat kept a spot inside a new footprint");
            w.Step(10);
            Check(!(World.IsIdleActivity(c.Goal) && c.Path.Count == 0 && InFootprint(v, c.Position)), "Idle cat still stands inside the new footprint");
        }

        /// <summary>An idle kitten plays.</summary>
        static void KittenPlays(string profile)
        {
            var w = Fixture(out var v, out var c);
            w.PacingProfile = profile;
            c.BuildingId = "";
            c.AgeHours = 2;
            var goals = new HashSet<string>();
            for (int second = 0; second < 240; second++)
            {
                w.Step(1);
                goals.Add(c.Goal);
                Check(!(World.IsIdleActivity(c.Goal) && c.Path.Count == 0 && AtCentre(c) && InFootprint(v, c.Position)), "Kitten stood inside a footprint");
            }
            Check(goals.Contains(GoalKey.IdlePlay), "An idle kitten never played: " + string.Join(",", goals));
            Check(!goals.Contains(GoalKey.IdlePatrol), "A kitten patrolled the fence");
        }

        static Cat ActivityCat(out World w, out Village v)
        {
            w = Fixture(out v, out var c);
            c.BuildingId = "";
            c.Path.Clear();
            c.Cargo.Clear();
            return c;
        }

        /// <summary>Every goal key the simulation sets maps to its own activity, and unknown goals fall back.</summary>
        static void ActivityForEveryGoal()
        {
            var expected = new Dictionary<string, CatActivity>
            {
                { GoalKey.Idle, CatActivity.Idle },
                { GoalKey.RestingAtHome, CatActivity.Idle },
                { GoalKey.Leaving, CatActivity.Leave },
                { GoalKey.Research, CatActivity.Research },
                { GoalKey.Defending, CatActivity.Fight },
                { GoalKey.PlayerControl, CatActivity.Controlled },
                { GoalKey.PlayerRest, CatActivity.Sleep },
                { GoalKey.ProductionPaused, CatActivity.Idle },
                { GoalKey.NeedDrink, CatActivity.Drink },
                { GoalKey.NeedEat, CatActivity.Eat },
                { GoalKey.NeedHeal, CatActivity.Eat },
                { GoalKey.NeedSleep, CatActivity.Sleep },
                { GoalKey.AccountingReturning, CatActivity.Count },
                { GoalKey.ExploringDungeon, CatActivity.Explore },
                { GoalKey.ReturningFromDungeon, CatActivity.Explore },
                { GoalKey.DescendingDungeonStairs, CatActivity.Explore },
                { GoalKey.FightingDungeonGuardian, CatActivity.Fight },
                { GoalKey.CollectingDungeonSupplies, CatActivity.Explore },
                { GoalKey.ReturningRailVehicle, CatActivity.Carry },
                { GoalKey.ReturningVesselToDock, CatActivity.Carry },
                { GoalKey.PlayerControlReturningToDock, CatActivity.Carry },
                { GoalKey.TransportReturning, CatActivity.Carry },
                { GoalKey.IdleNap, CatActivity.Nap },
                { GoalKey.IdleGroom, CatActivity.Groom },
                { GoalKey.IdleSocial, CatActivity.Social },
                { GoalKey.IdlePatrol, CatActivity.Patrol },
                { GoalKey.IdleShrine, CatActivity.Shrine },
                { GoalKey.IdlePlay, CatActivity.Play },
                // Goals built from a prefix and a detail.
                { GoalKey.FarmPrefix + "soil", CatActivity.Farm },
                { GoalKey.AccountingPrefix + "counting", CatActivity.Count },
                { GoalKey.ReturningToPrefix + "rail", CatActivity.Carry },
                { JobKind.Rail + GoalKey.Separator + "outbound", CatActivity.Carry },
                { "shipping" + GoalKey.Separator + "outbound", CatActivity.Carry },
                { JobKind.Hunt + GoalKey.Separator + JobPhase.Working, CatActivity.Hunt },
                { JobKind.Haul + GoalKey.Separator + JobPhase.Travel, CatActivity.Walk },
            };
            var prefixes = new[] { nameof(GoalKey.NeedPrefix), nameof(GoalKey.Separator), nameof(GoalKey.FarmPrefix), nameof(GoalKey.AccountingPrefix), nameof(GoalKey.ReturningToPrefix) };
            foreach (var field in typeof(GoalKey).GetFields(BindingFlags.Public | BindingFlags.Static).Where(f => f.IsLiteral && !prefixes.Contains(f.Name)))
            {
                string goal = (string)field.GetValue(null);
                Check(expected.ContainsKey(goal), "GoalKey." + field.Name + " has no expected activity");
            }
            foreach (var entry in expected)
            {
                var c = ActivityCat(out var w, out var v);
                c.Goal = entry.Key;
                Check(w.ActivityOf(v, c) == entry.Value, "Goal '" + entry.Key + "' maps to " + w.ActivityOf(v, c) + ", expected " + entry.Value);
            }
            {
                var c = ActivityCat(out var w, out var v);
                c.Goal = "something_unknown";
                Check(w.ActivityOf(v, c) == CatActivity.Idle, "Unknown goal without a path is not Idle");
                c.Path.Add(new Int2(c.Position.X + 1, c.Position.Z));
                Check(w.ActivityOf(v, c) == CatActivity.Walk, "Unknown goal with a path is not Walk");
                c.Goal = GoalKey.IdleNap;
                Check(w.ActivityOf(v, c) == CatActivity.Nap, "Walking to a nap is not shown as napping");
            }
        }

        /// <summary>Every job kind has a working activity of its own; travel, fetch and delivery phases read as walking or carrying.</summary>
        static void ActivityForEveryJob()
        {
            var expected = new Dictionary<string, CatActivity>
            {
                { JobKind.Hunt, CatActivity.Hunt }, { JobKind.HuntExpedition, CatActivity.Hunt }, { JobKind.LeaderPlanHunt, CatActivity.Hunt }, { JobKind.SupplyFood, CatActivity.Hunt },
                { JobKind.Fish, CatActivity.Fish },
                { JobKind.Water, CatActivity.Gather }, { JobKind.SupplyWater, CatActivity.Gather }, { JobKind.FetchWater, CatActivity.Gather },
                { JobKind.Logs, CatActivity.Gather }, { JobKind.GatherLogs, CatActivity.Gather }, { JobKind.Quarry, CatActivity.Gather },
                { JobKind.Fibre, CatActivity.Gather }, { JobKind.ForageFibre, CatActivity.Gather },
                { JobKind.Build, CatActivity.Build }, { JobKind.BuildHouse, CatActivity.Build }, { JobKind.LeaderPlanHouse, CatActivity.Build },
                { JobKind.Expand, CatActivity.Build }, { JobKind.ExpandVillage, CatActivity.Build }, { JobKind.Road, CatActivity.Build },
                { JobKind.Rail, CatActivity.Build }, { JobKind.Bridge, CatActivity.Build }, { JobKind.Dock, CatActivity.Build },
                { JobKind.Wagon, CatActivity.Build }, { JobKind.Vessel, CatActivity.Build },
                { JobKind.Production, CatActivity.Craft }, { JobKind.MouseFarm, CatActivity.Craft }, { JobKind.Offering, CatActivity.Craft },
                { JobKind.Ritual, CatActivity.Craft }, { JobKind.PerformOffering, CatActivity.Craft },
                { JobKind.Replant, CatActivity.Farm }, { JobKind.ReplantTree, CatActivity.Farm },
                { JobKind.Train, CatActivity.Train }, { JobKind.TrainWarrior, CatActivity.Train },
                { JobKind.Scout, CatActivity.Explore }, { JobKind.Explore, CatActivity.Explore },
                { JobKind.Haul, CatActivity.Carry }, { JobKind.CarryOffering, CatActivity.Carry },
            };
            foreach (var field in typeof(JobKind).GetFields(BindingFlags.Public | BindingFlags.Static).Where(f => f.IsLiteral))
            {
                string kind = (string)field.GetValue(null);
                Check(expected.ContainsKey(kind), "JobKind." + field.Name + " has no expected activity");
                var c = ActivityCat(out var w, out var v);
                var job = new Job { Id = w.Id("activity-job"), Kind = kind, Phase = JobPhase.Working, CatId = c.Id, Position = c.Position };
                v.Jobs.Add(job);
                c.JobId = job.Id;
                c.Goal = kind + GoalKey.Separator + job.Phase;
                Check(w.ActivityOf(v, c) == expected[kind], "Job kind '" + kind + "' works as " + w.ActivityOf(v, c) + ", expected " + expected[kind]);
                c.Goal = kind;
                Check(w.ActivityOf(v, c) == expected[kind], "Job kind '" + kind + "' just assigned works as " + w.ActivityOf(v, c));
            }
            var phases = new Dictionary<string, (CatActivity Empty, CatActivity Loaded)>
            {
                { JobPhase.Travel, (CatActivity.Walk, CatActivity.Walk) },
                { JobPhase.Fetch, (CatActivity.Walk, CatActivity.Carry) },
                { JobPhase.ItemFetch, (CatActivity.Walk, CatActivity.Carry) },
                { JobPhase.Return, (CatActivity.Walk, CatActivity.Carry) },
                { JobPhase.InputDelivery, (CatActivity.Carry, CatActivity.Carry) },
                { JobPhase.OutputDelivery, (CatActivity.Carry, CatActivity.Carry) },
                { JobPhase.Working, (CatActivity.Gather, CatActivity.Gather) },
            };
            foreach (var field in typeof(JobPhase).GetFields(BindingFlags.Public | BindingFlags.Static).Where(f => f.IsLiteral))
            {
                string phase = (string)field.GetValue(null);
                Check(phases.ContainsKey(phase), "JobPhase." + field.Name + " has no expected activity");
                foreach (bool loaded in new[] { false, true })
                {
                    var c = ActivityCat(out var w, out var v);
                    var job = new Job { Id = w.Id("activity-phase"), Kind = JobKind.Logs, Phase = phase, CatId = c.Id, Position = c.Position };
                    v.Jobs.Add(job);
                    c.JobId = job.Id;
                    if (loaded)
                        c.Cargo.Add(new Stack("logs", 4));
                    var want = loaded ? phases[phase].Loaded : phases[phase].Empty;
                    Check(w.ActivityOf(v, c) == want, "Phase '" + phase + "' (loaded=" + loaded.ToString(CultureInfo.InvariantCulture) + ") is " + w.ActivityOf(v, c) + ", expected " + want);
                }
            }
        }
    }
}
