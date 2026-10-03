using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Colony automation with no player input: the Leader's fallback staffing and supply, officer
    /// recipe choice and staffing, Emergency limits, automatic appointment, station queue defaults
    /// and the stock estimates shown before an Accountant counts.
    /// </summary>
    public static class ColonyAutomationScenarios
    {
        static readonly string[] Profiles = { Pacing.ClassicProfile, Pacing.DemoProfile };
        static readonly string[] FoundingStations = { "wood_cutter", "stone_prep", "woodworking" };

        public static IEnumerable<Scenario> Cases()
        {
            foreach (var profile in Profiles)
            {
                string p = profile;
                yield return new Scenario("colony_automation.fresh_colony_works_" + p, () => FreshColonyWorks(p));
                yield return new Scenario("colony_automation.research_hut_scholar_within_30s_" + p, () => ScholarWithin30Seconds(p));
                yield return new Scenario("colony_automation.founding_stations_repeat_" + p, () => FoundingStationsRepeat(p));
                yield return new Scenario("colony_automation.spill_rehauled_without_steward_" + p, () => SpillRehauled(p));
                yield return new Scenario("colony_automation.estimate_from_second_zero_" + p, () => EstimateFromSecondZero(p));
            }
            yield return new Scenario("demo_timing.colony_idle_ratio_10min", DemoIdleRatio);
            yield return new Scenario("colony_automation.leader_keeps_den_materials", LeaderKeepsDenMaterials);
            yield return new Scenario("colony_automation.leader_takes_over_vacant_founding_station", LeaderTakesOverVacantStation);
            yield return new Scenario("colony_automation.accountant_count_ends_estimate", AccountantCountEndsEstimate);
            yield return new Scenario("colony_automation.officer_prefers_efficient_conversion", OfficerPrefersEfficientConversion);
            yield return new Scenario("colony_automation.officer_skips_decorative_and_refined", OfficerSkipsDecorativeAndRefined);
            yield return new Scenario("colony_automation.officer_gates_preserves_on_food_surplus", OfficerGatesPreserves);
            yield return new Scenario("colony_automation.officer_staffs_worker_slots_by_labor", OfficerStaffsWorkerSlots);
            yield return new Scenario("colony_automation.assigned_station_queues_repeat_default", AssignedStationQueuesDefault);
            yield return new Scenario("colony_automation.restored_busy_worker_gets_no_default", RestoredBusyWorkerGetsNoDefault);
            yield return new Scenario("colony_automation.emergency_spares_need_trips_and_late_work", EmergencySparesNeedTripsAndLateWork);
            yield return new Scenario("colony_automation.emergency_dry_run_keeps_work", EmergencyDryRunKeepsWork);
            yield return new Scenario("colony_automation.emergency_rate_limited", EmergencyRateLimited);
            yield return new Scenario("colony_automation.office_auto_appointed_once", OfficeAutoAppointedOnce);
            yield return new Scenario("colony_automation.housing_siting_backs_off", HousingSitingBacksOff);
        }

        static World Fresh(string profile, out Village v)
        {
            var w = World.Create(41);
            w.PacingProfile = profile;
            v = w.Villages[0];
            return w;
        }

        static bool NeedTrip(Cat c) => c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal);

        /// <summary>Doing colony work right now: an active job, a scholar's or accountant's round, or farm work.</summary>
        static bool Working(Village v, Cat c) => c.Alive && c.ControlledBy == "" && !NeedTrip(c)
            && (v.Jobs.Any(j => !j.Completed && j.Id == c.JobId && j.CatId == c.Id) || c.Goal == GoalKey.Research
                || c.Goal.StartsWith(GoalKey.AccountingPrefix, StringComparison.Ordinal) || c.Goal.StartsWith(GoalKey.FarmPrefix, StringComparison.Ordinal));

        static bool Idle(Village v, Cat c) => c.Alive && c.ControlledBy == "" && !NeedTrip(c) && !Working(v, c) && c.Migration != "arriving" && c.Migration != "departing";

        static double IdleRatio(Village v) => v.Cats.Count(c => Idle(v, c)) / (double)Math.Max(1, v.Cats.Count(c => c.Alive));

        static string F(double value) => value.ToString("F2", CultureInfo.InvariantCulture);

        static void Emergency(World w, Village v, string kind) =>
            typeof(World).GetMethod("Emergency", BindingFlags.Instance | BindingFlags.NonPublic).Invoke(w, new object[] { v, kind });

        static void FreeCats(Village v, params Cat[] cats)
        {
            foreach (var c in cats)
            {
                c.ControlledBy = "";
                c.BuildingId = "";
                c.Position = new Int2(1, 2);
                c.X = 1;
                c.Z = 2;
            }
        }

        static void FreshColonyWorks(string profile)
        {
            var w = Fresh(profile, out var v);
            var ratios = MeasureIdle(w, v, profile);
            Check(ratios.working10 >= 8, "Fewer than 8 cats work after 10 minutes without player input: " + ratios.working10);
            Valid(w);
        }

        /// <summary>
        /// The demo acceptance line for colony automation. Its value depends on the demo pacing and recipe
        /// overrides (W2-01, W2-10), so it runs only with --include-demo-timing.
        /// </summary>
        static void DemoIdleRatio()
        {
            // The demo colony every new single-player world starts from (W2-07).
            var w = World.CreateColony(World.DemoSeed, null, Pacing.DemoProfile, 0);
            var v = w.Villages[0];
            var ratios = MeasureIdle(w, v, Pacing.DemoProfile);
            Check(ratios.idle10 < 0.6, "Demo idle ratio after 10 minutes is not under 60%: " + F(ratios.idle10));
            Valid(w);
        }

        static (double idle5, double idle10, int working10) MeasureIdle(World w, Village v, string profile)
        {
            w.Step(300);
            double idle5 = IdleRatio(v);
            int working5 = v.Cats.Count(c => Working(v, c));
            w.Step(300);
            double idle10 = IdleRatio(v);
            int working10 = v.Cats.Count(c => Working(v, c));
            Console.WriteLine("colony_automation.idle profile=" + profile + " idle_5min=" + F(idle5) + " working_5min=" + working5 + " idle_10min=" + F(idle10) + " working_10min=" + working10 + " alive=" + v.Cats.Count(c => c.Alive));
            return (idle5, idle10, working10);
        }

        static void ScholarWithin30Seconds(string profile)
        {
            var w = Fresh(profile, out var v);
            var hut = v.Buildings.Single(b => b.Kind == "research_hut");
            w.Step(30);
            Check(hut.Slots.Any(s => s.CatId != "" && v.Cats.Any(c => c.Id == s.CatId && c.Alive)), "The founding Research Hut has no scholar after 30 seconds");
            w.Step(60);
            Check(v.ResearchPoints > 0, "The automatic scholar produced no research");
            Valid(w);
        }

        static void FoundingStationsRepeat(string profile)
        {
            var w = Fresh(profile, out var v);
            var staffed = new HashSet<string>();
            var progressed = new HashSet<string>();
            for (int t = 10; t <= 600; t += 10)
            {
                w.Step(10);
                foreach (var b in v.Buildings.Where(b => b.Completed && FoundingStations.Contains(b.Kind)))
                {
                    if (b.Slots.Any(s => s.CatId != "" && s.Queue.Any(q => q.Repeat)))
                        staffed.Add(b.Kind);
                    if (v.Jobs.Any(j => j.Kind == "production" && j.TargetId == b.Id && (j.Progress > 0 || j.Completed)))
                        progressed.Add(b.Kind);
                }
            }
            foreach (var kind in FoundingStations)
            {
                Check(staffed.Contains(kind), "Founding station never staffed with a repeating recipe: " + kind);
                Check(progressed.Contains(kind), "Founding station never made production progress: " + kind);
            }
            Valid(w);
        }

        static void SpillRehauled(string profile)
        {
            var w = Fresh(profile, out var v);
            var at = new Int2(v.Center.X + 1, v.Center.Z + 4);
            var spill = new Stockpile { Id = w.Id("spill"), Kind = "spill", Position = at, Width = 1, Depth = 1, Capacity = 10000 };
            spill.Goods.Add(new Stack("planks", 12));
            v.Stockpiles.Add(spill);
            double before = Goods(w, v, "planks");
            w.Step(180);
            Check(World.Amount(spill.Goods, "planks") <= 0, "Spilled goods were not re-hauled without a Steward: " + World.Amount(spill.Goods, "planks"));
            Check(Goods(w, v, "planks") <= before + 1e-6, "Spill recovery created goods");
            Valid(w);
        }

        static void EstimateFromSecondZero(string profile)
        {
            var w = Fresh(profile, out var v);
            double age = w.CurrentPacing().LeaderDirectSeconds;
            for (int t = 0; t <= 300; t += 10)
            {
                if (t > 0)
                    w.Step(10);
                foreach (var resource in new[] { "food", "water", "planks" })
                {
                    double total = w.Total(v, resource), estimate = w.EstimatedTotal(v, resource, out bool isEstimate);
                    Check(isEstimate, "Uncounted stock is not flagged as an estimate: " + resource + " t=" + t);
                    Check(Math.Abs(estimate - total) <= total * 0.1 + 1e-9, "Estimate is not within 10% of the stock: " + resource + " t=" + t + " estimate=" + estimate + " total=" + total);
                }
                Check(w.EstimateAgeSeconds(v) <= age + 1e-9, "Leader estimate is older than one direction period: " + w.EstimateAgeSeconds(v));
            }
            Valid(w);
        }

        static void LeaderKeepsDenMaterials()
        {
            var w = Fresh(Pacing.ClassicProfile, out var v);
            var pace = w.CurrentPacing();
            int dens = v.Buildings.Count(b => b.Kind == "den");
            double timber = pace.ScaffoldTimberBase + dens * pace.ScaffoldTimberPerExisting, blocks = pace.ScaffoldBlocksBase + dens * pace.ScaffoldBlocksPerExisting;
            for (int t = 10; t <= 900; t += 10)
            {
                w.Step(10);
                Check(w.Total(v, "planks") + 1e-9 >= timber && w.Total(v, "blocks") + 1e-9 >= blocks, "Leader automation burned the next Den's materials t=" + t + " planks=" + w.Total(v, "planks") + " blocks=" + w.Total(v, "blocks"));
            }
            Check(v.Items.Any(i => i.Kind == "tool"), "Woodworking made nothing from the surplus above the Den reserve");
            Valid(w);
        }

        static void LeaderTakesOverVacantStation()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            foreach (var worker in v.Cats.Take(8))
                worker.ControlledBy = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "logs", 100);
            World.Add(v.Stockpiles[0].Goods, "stone", 100);
            var cutter = Station(w, v, "wood_cutter");
            Station(w, v, "sawmill");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = c.Id });
            w.Step(12);
            Check(cutter.Slots.Any(s => s.CatId != "" && s.AutomatedBy == "forester"), "Forester did not staff the wood cutter");
            Act(w, v, new GameAction { Kind = "UnassignOfficer", Role = "forester" });
            w.Step(30);
            Check(!v.Officers.Any(o => o.Role == "forester"), "A vacated office was refilled automatically");
            Check(!cutter.Slots.Any(s => s.AutomatedBy == "forester"), "Vacant Forester kept a station slot");
            Check(cutter.Slots.Any(s => s.CatId != "" && s.AutomatedBy == "leader" && s.Queue.Any(q => q.Repeat && q.AutomatedBy == "leader")), "The Leader did not take over the vacant founding station");
            Valid(w);
        }

        static void AccountantCountEndsEstimate()
        {
            var w = Fixture(out var v, out var c);
            var tent = Station(w, v, "accounting_tent");
            v.Research.Add("basic_tools");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = tent.Id, CatId = c.Id });
            w.EstimatedTotal(v, "food", out bool before);
            Check(before, "Stock counted before the Accountant's first round is not an estimate");
            w.Step(90);
            Check(v.Stockpiles.Where(p => p.Kind == "storage").All(p => p.CountedAt >= 0), "Accountant did not count the store");
            double counted = w.EstimatedTotal(v, "food", out bool after);
            Check(!after, "Counted stock is still flagged as an estimate");
            Near(counted, w.ReportedTotal(v, "food"), "Counted stock does not show the Accountant's report");
            Check(w.EstimateAgeSeconds(v) <= 90, "Counted stock age is wrong: " + w.EstimateAgeSeconds(v));
        }

        static World OfficerFixture(out Village v, out Cat c, int free)
        {
            var w = Fixture(out v, out c);
            c.BuildingId = "";
            foreach (var worker in v.Cats.Take(free))
                worker.ControlledBy = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            return w;
        }

        static void OfficerPrefersEfficientConversion()
        {
            var w = OfficerFixture(out var v, out var c, 8);
            World.Add(v.Stockpiles[0].Goods, "stone", 100);
            var yard = Station(w, v, "stone_prep");
            Station(w, v, "sawmill", new Int2(-3, 3));
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = c.Id });
            w.Step(12);
            var entries = yard.Slots.SelectMany(s => s.Queue).ToArray();
            Check(entries.Length > 0, "Forester queued nothing at the stone yard");
            Check(entries.All(q => q.RecipeId == "stonecraft_masterwork"), "Forester chose a worse block conversion: " + string.Join(",", entries.Select(q => q.RecipeId)));
        }

        static void OfficerSkipsDecorativeAndRefined()
        {
            var w = OfficerFixture(out var v, out var c, 10);
            World.Add(v.Stockpiles[0].Goods, "materials", 100);
            World.Add(v.Stockpiles[0].Goods, "clay", 100);
            var workshop = Station(w, v, "workshop");
            var yard = Station(w, v, "stone_prep", new Int2(-3, 3));
            Station(w, v, "sawmill", new Int2(-3, -3));
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = v.Cats[1].Id });
            w.Step(12);
            var queued = v.Buildings.SelectMany(b => b.Slots).SelectMany(s => s.Queue).Select(q => Catalog.Recipe(q.RecipeId)).ToArray();
            Check(queued.Length > 0, "Steward queued nothing useful at the workshop");
            Check(!queued.Any(r => r.Outputs.Any(s => s.Resource == "refined")), "Officer automation burns materials into refined goods");
            Check(queued.All(r => r.ItemKind == "" || r.ItemKind == "tool" || r.ItemKind == "weapon" || r.ItemKind == "armor"), "Officer automation makes decorative items: " + string.Join(",", queued.Select(r => r.Id)));
            Check(!yard.Slots.Any(s => s.CatId != ""), "Officer staffed a station that has nothing useful to make");
            Check(workshop.Slots.Any(s => s.CatId != ""), "Steward did not staff the workshop");
        }

        static void OfficerGatesPreserves()
        {
            var w = OfficerFixture(out var v, out var c, 12);
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "food");
            World.Add(v.Stockpiles[0].Goods, "food", 100);
            var mill = Station(w, v, "mill");
            Station(w, v, "field", new Int2(-3, 3));
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = c.Id });
            bool Preserving() => v.Buildings.SelectMany(b => b.Slots).SelectMany(s => s.Queue).Any(q => Catalog.Recipe(q.RecipeId).Outputs.Any(s => s.Resource == "preserves"))
                || v.Jobs.Any(j => !j.Completed && j.Kind == "production" && Catalog.Recipe(j.RecipeId)?.Outputs.Any(s => s.Resource == "preserves") == true);
            w.Step(12);
            Check(!Preserving(), "Farmer turns scarce food into preserves");
            World.Add(v.Stockpiles[0].Goods, "food", 2000);
            w.Step(10);
            Check(Preserving(), "Farmer does not preserve a food surplus");
            Check(mill.Slots.Any(s => s.CatId != ""), "Farmer did not staff the mill for preserves");
        }

        static void OfficerStaffsWorkerSlots()
        {
            var w = OfficerFixture(out var v, out var c, 12);
            World.Add(v.Stockpiles[0].Goods, "fibre", 200);
            var clothier = Station(w, v, "clothier");
            var weaver = v.Cats[7];
            weaver.Preferences.Clear();
            weaver.Preferences.Add("textile");
            foreach (var other in v.Cats.Take(12).Where(x => x != weaver))
                other.Preferences.Remove("textile");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = c.Id });
            w.Step(12);
            int slots = (int)Catalog.BuildingEffect(v, "clothier", "worker_slots", 1);
            Check(slots > 1, "Fixture needs a multi-slot clothier");
            Check(clothier.Slots.Count(s => s.CatId != "") == slots, "Cloth leader staffed " + clothier.Slots.Count(s => s.CatId != "") + " of " + slots + " work positions");
            Check(clothier.Slots.Any(s => s.CatId == weaver.Id), "Cloth leader ignored the textile preference");
        }

        static void AssignedStationQueuesDefault()
        {
            var w = Fixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "logs", 50);
            var cutter = Station(w, v, "wood_cutter");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = cutter.Id, CatId = c.Id });
            w.Step(3);
            var slot = cutter.Slots.Single(s => s.CatId == c.Id);
            Check(slot.Queue.Count == 1 && slot.Queue[0].RecipeId == "logs_to_planks" && slot.Queue[0].Repeat, "Assigned worker's empty queue got no repeating default recipe");
            Check(v.Jobs.Any(j => !j.Completed && j.Kind == "production" && j.CatId == c.Id), "Default recipe did not start");
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = cutter.Id, Edit = "remove", Index = 0 });
            w.Step(3);
            Check(slot.Queue.Count == 0, "A queue the player emptied was refilled");
            Valid(w);
        }

        static void RestoredBusyWorkerGetsNoDefault()
        {
            var w = Fixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "logs", 50);
            var cutter = Station(w, v, "wood_cutter");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = cutter.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = cutter.Id, Edit = "add", RecipeId = "logs_to_planks" });
            w.Step(30);
            var job = v.Jobs.Single(j => j.Kind == "production" && !j.Completed);
            // A save written before queue defaults existed: a worker mid-batch whose queue is already empty.
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = cutter.Id, Edit = "remove", Index = 0 });
            v.QueueDefaultsApplied.Clear();
            w.Step(900);
            Check(job.Completed, "Restored batch did not finish");
            Check(cutter.Slots.All(s => s.Queue.Count == 0) && !v.Jobs.Any(j => j.Kind == "production" && !j.Completed), "A worker first seen mid-batch got a default recipe after its queue ran out");
            Valid(w);
        }

        static void EmergencySparesNeedTripsAndLateWork()
        {
            var w = Fixture(out var v, out var c);
            Cat d = v.Cats[1], e = v.Cats[2];
            FreeCats(v, c, d, e);
            var build = new Job { Id = w.Id("late-build"), Kind = "build", Phase = "working", CatId = c.Id, Position = c.Position, Progress = 70, RequiredWork = 100 };
            v.Jobs.Add(build);
            c.JobId = build.Id;
            d.Goal = GoalKey.NeedEat;
            d.Hunger = 20;
            d.Cargo.Add(new Stack("food", 1));
            var scout = new Job { Id = w.Id("scout"), Kind = "scout", Phase = "travel", CatId = e.Id, Position = new Int2(4, 2), RequiredWork = 15 };
            v.Jobs.Add(scout);
            e.JobId = scout.Id;
            e.Health = 40;
            Emergency(w, v, "water");
            Check(!build.Completed && c.JobId == build.Id, "Emergency pre-empted a builder past half progress");
            Check(World.Amount(d.Cargo, "food") == 1 && d.Goal == GoalKey.NeedEat, "Emergency took a cat carrying its meal");
            Check(scout.Completed && v.Jobs.Any(j => !j.Completed && j.Kind == "water" && j.CatId == e.Id), "Emergency did not prefer the scouting cat");
            Valid(w);
        }

        static void EmergencyDryRunKeepsWork()
        {
            var w = Fixture(out var v, out var c);
            FreeCats(v, c);
            foreach (var p in v.Known)
                w.TileAt(p).Water = false;
            var scout = new Job { Id = w.Id("scout"), Kind = "scout", Phase = "travel", CatId = c.Id, Position = new Int2(4, 2), RequiredWork = 15 };
            v.Jobs.Add(scout);
            c.JobId = scout.Id;
            Emergency(w, v, "water");
            Check(!scout.Completed && c.JobId == scout.Id, "Emergency cancelled work although no water source exists");
        }

        static void EmergencyRateLimited()
        {
            var w = Fixture(out var v, out var c);
            var d = v.Cats[1];
            FreeCats(v, c, d);
            Job Scout(Cat x)
            {
                var job = new Job { Id = w.Id("scout"), Kind = "scout", Phase = "travel", CatId = x.Id, Position = new Int2(4, 2), RequiredWork = 15 };
                v.Jobs.Add(job);
                x.JobId = job.Id;
                return job;
            }
            Scout(c);
            Scout(d);
            Emergency(w, v, "water");
            var first = v.Jobs.Single(j => !j.Completed && j.Kind == "water");
            first.Completed = true;
            var taken = v.Cats.Single(x => x.Id == first.CatId);
            taken.JobId = "";
            // Both cats are busy again: a second pre-emption inside the cooldown would cancel one of these.
            var again = Scout(taken);
            var other = v.Jobs.Single(j => !j.Completed && j.Kind == "scout" && j.CatId != taken.Id);
            Emergency(w, v, "water");
            Check(!again.Completed && !other.Completed, "Emergency pre-empted twice within its cooldown");
        }

        static void OfficeAutoAppointedOnce()
        {
            var w = Fresh(Pacing.ClassicProfile, out var v);
            var kitten = v.Cats[5];
            kitten.AgeHours = 1;
            var migrant = v.Cats[6];
            migrant.Migration = "probationary";
            migrant.ProbationUntil = 1e9;
            foreach (var x in new[] { kitten, migrant })
            {
                x.Preferences.Add("research");
                World.Add(x.Skills, "research", 500);
            }
            v.Research.Add("research_hut");
            w.Step(10);
            var office = v.Officers.SingleOrDefault(o => o.Role == "loremaster");
            Check(office != null, "Loremaster was not appointed when its study and building completed");
            var holder = v.Cats.Single(x => x.Id == office.CatId);
            Check(holder != kitten && holder != migrant && holder.Migration == "resident" && holder.AgeHours >= w.CurrentPacing().KittenUntilHours, "Automatic appointment chose a kitten or a migrant on probation");
            Check(holder.OfficerRole == "loremaster", "Appointed cat does not hold the office");
            Check(v.Events.Any(e => e.EntityId == holder.Id && e.Text.Contains("Loremaster")), "Automatic appointment logged no event");
            Act(w, v, new GameAction { Kind = "UnassignOfficer", Role = "loremaster" });
            w.Step(30);
            Check(!v.Officers.Any(o => o.Role == "loremaster"), "Automatic appointment overrode the player's vacate");
            Valid(w);
        }

        static long HousingScans(World w) => (long)typeof(World).GetField("housingSitingScans", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(w);

        static void HousingSitingBacksOff()
        {
            var w = Fixture(out var v, out var c);
            v.FoundedAt = -w.CurrentPacing().HousingPlanAfterSeconds - 1;
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            v.Known.Clear();
            w.Step(10);
            long first = HousingScans(w);
            Check(first == 1, "Housing siting did not scan once: " + first);
            w.Step(60);
            Check(HousingScans(w) == first, "Housing siting rescanned an unchanged layout: " + HousingScans(w));
            v.Known.Add(new Int2(4, 4));
            w.Step(10);
            Check(HousingScans(w) == first + 1, "Housing siting ignored a layout change: " + HousingScans(w));
        }
    }
}
