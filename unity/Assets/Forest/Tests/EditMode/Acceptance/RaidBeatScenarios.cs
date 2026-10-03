using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// The demo's scripted raid beat (D15, D16): a telegraphed fox band of three that rallied adults
    /// beat, that costs only food when lost, and that never kills a cat in the demo profile.
    /// </summary>
    public static class RaidBeatScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("raid_beat.warning_precedes_a_fox_band_of_three_at_the_south_gate", WarningThenBand);
            yield return new Scenario("raid_beat.rallied_adults_repel_the_band", RalliedAdultsRepel);
            yield return new Scenario("raid_beat.lost_raid_costs_food_only", LostRaidCostsFoodOnly);
            yield return new Scenario("raid_beat.demo_raid_never_kills", DemoRaidNeverKills);
            yield return new Scenario("raid_beat.classic_world_has_no_scripted_raid", ClassicHasNoScript);
        }

        /// <summary>A demo-profile founding colony whose clock starts <paramref name="lead"/> seconds before the raid warning.</summary>
        private static World DemoWorld(out Village v, double lead)
        {
            var w = World.Create(41);
            w.PacingProfile = Pacing.DemoProfile;
            v = w.Villages[0];
            var pace = w.CurrentPacing();
            Check(pace.DemoRaidAtSeconds > pace.RaidWarningSeconds && !pace.RaidCanKill, "Demo threat knobs are not set");
            // Skip the first thirteen minutes of colony life: only the raid beat is under test.
            v.FoundedAt = w.TimeSeconds - (pace.DemoRaidAtSeconds - pace.RaidWarningSeconds - lead);
            return w;
        }

        private static void Untrained(Village v)
        {
            foreach (var cat in v.Cats)
                cat.Skills.RemoveAll(s => s.Resource == "fight");
            v.Officers.RemoveAll(o => o.Role == "captain");
        }

        private static void StepUntil(World w, Func<bool> done, double limit, string what)
        {
            for (double t = 0; t < limit && !done(); t += 1)
                w.Step(1);
            Check(done(), what);
        }

        private static void WarningThenBand()
        {
            var w = DemoWorld(out var v, 3);
            var pace = w.CurrentPacing();
            double warnAt = v.FoundedAt + pace.DemoRaidAtSeconds - pace.RaidWarningSeconds;
            double raidAt = v.FoundedAt + pace.DemoRaidAtSeconds;
            StepUntil(w, () => v.Events.Any(e => e.Kind == EventKind.RaidWarning), 10, "No raid_warning was raised");
            var warning = v.Events.Single(e => e.Kind == EventKind.RaidWarning);
            Check(Math.Abs(warning.Time - warnAt) <= 1, "The warning must come RaidWarningSeconds before the raid: at " + warning.Time + ", expected " + warnAt);
            Check(v.Raids.Count == 0, "The warning must come before any raider");
            StepUntil(w, () => v.Raids.Count > 0, pace.RaidWarningSeconds + 5, "The fox band never arrived");
            Check(Math.Abs(w.TimeSeconds - raidAt) <= 1.01, "The band must attack at DemoRaidAtSeconds: at " + w.TimeSeconds + ", expected " + raidAt);
            var raid = v.Raids.Single();
            Check(raid.Members == 3 && raid.Kind == "fox" && raid.Scripted, "The scripted raid must be a fox band of three: " + raid.Members + " " + raid.Kind);
            Check(raid.Position.X == v.Center.X && raid.Position.Z >= v.Center.Z + v.Radius - 1, "The band must come at the south gate: " + raid.Position);
            Check(raid.Health > 0 && raid.Health == raid.MaxHealth, "The band starts at full strength");
            Check(v.Events.Count(e => e.Kind == EventKind.RaidWarning) == 1 && v.ScriptedRaidStage == 2, "The scripted raid happens once");
        }

        private static void RalliedAdultsRepel()
        {
            var w = DemoWorld(out var v, 2);
            Untrained(v);
            int alive = v.Cats.Count(c => c.Alive);
            StepUntil(w, () => v.Events.Any(e => e.Kind == EventKind.RaidWarning), 10, "No raid_warning was raised");
            var rally = Act(w, v, new GameAction { Kind = "DefendRaid" });
            Check(rally.Code == "rallied" && v.Cats.Count(c => c.RalliedTo != "") >= 3, "The warning must let the player rally defenders to the gate");
            StepUntil(w, () => v.Raids.Count > 0, w.CurrentPacing().RaidWarningSeconds + 5, "The fox band never arrived");
            var raid = v.Raids.Single();
            Check(v.Cats.Where(c => c.RalliedTo != "").All(c => c.RalliedTo == raid.Id), "Cats waiting at the gate must turn on the arriving band");
            StepUntil(w, () => !v.Raids.Contains(raid), 240, "Rallied adults did not drive off the band (health " + raid.Health + ")");
            Check(v.Events.Any(e => e.Kind == EventKind.RaidRepelled && e.EntityId == raid.Id) && v.RaidsRepelled == 1, "Driving off the band must fire raid_repelled");
            Check(!v.Events.Any(e => e.Kind == EventKind.RaidBreach), "Rallied defenders must stop the band before it loots the shrine");
            Check(v.Cats.Count(c => c.Alive) == alive && !v.Events.Any(e => e.Kind == EventKind.Death), "No cat may die in the demo raid");
            Check(v.Cats.All(c => c.RalliedTo == ""), "Defenders must be released after the victory");
            Valid(w);
        }

        private static void LostRaidCostsFoodOnly()
        {
            var w = DemoWorld(out var v, 2);
            Untrained(v);
            StepUntil(w, () => v.Raids.Count > 0, w.CurrentPacing().RaidWarningSeconds + 10, "The fox band never arrived");
            var raid = v.Raids.Single();
            var health = v.Cats.Where(c => c.Alive).ToDictionary(c => c.Id, c => c.Health);
            double stolen = 0;
            for (int t = 0; t < 600 && v.Raids.Contains(raid); t++)
            {
                w.Step(1);
                stolen = Math.Max(stolen, World.Amount(raid.Loot, "food"));
                foreach (var c in v.Cats.Where(c => health.ContainsKey(c.Id)))
                    Check(c.Alive && c.Health >= health[c.Id] - 1e-6, c.Name + " was hurt by an undefended demo raid");
            }
            Check(!v.Raids.Contains(raid) && v.Events.Any(e => e.Kind == EventKind.RaidBreach && e.EntityId == raid.Id), "The undefended band must reach the shrine and leave");
            Check(stolen > 0 && v.Events.Any(e => e.Kind == EventKind.RaidLoss && e.EntityId == raid.Id), "Losing the raid must cost stored food");
            Check(!v.Events.Any(e => e.Kind == EventKind.Death), "Losing the demo raid must never cost a cat");
        }

        private static void DemoRaidNeverKills()
        {
            var w = DemoWorld(out var v, 1000);
            var c = v.Cats.First(x => x.Alive && x.ControlledBy == "");
            c.Skills.RemoveAll(s => s.Resource == "fight");
            c.Skills.Add(new Stack("fight", 10));
            c.Position = v.Center;
            c.X = v.Center.X;
            c.Z = v.Center.Z;
            c.Path.Clear();
            c.Health = 16;
            var raid = new Raid { Id = w.Id("brutal-raid"), Position = v.Center, Health = 100000, MaxHealth = 100000, Members = 3, Kind = "fox", Strength = 1000000 };
            v.Raids.Add(raid);
            int alive = v.Cats.Count(x => x.Alive);
            w.Step(5);
            Check(v.Cats.Count(x => x.Alive) == alive && !v.Events.Any(e => e.Kind == EventKind.Death), "Demo raids must never kill: " + string.Join(";", v.Events.Where(e => e.Kind == EventKind.Death).Select(e => e.Text)));
            Check(c.Health >= 1, "A demo defender stays standing");
            Check(v.Events.Any(e => e.Kind == EventKind.RaidBreach && e.EntityId == raid.Id), "A beaten defence still lets the band loot food");
        }

        private static void ClassicHasNoScript()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            v.FoundedAt = -2000;
            w.Step(5);
            Check(v.ScriptedRaidStage == 0 && v.Raids.Count == 0 && !v.Events.Any(e => e.Kind == EventKind.RaidWarning), "Classic worlds must not run the demo raid script");
        }
    }
}
