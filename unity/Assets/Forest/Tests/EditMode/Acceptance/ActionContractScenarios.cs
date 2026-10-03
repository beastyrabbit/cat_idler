using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.RegularExpressions;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>The player action contract: stable result codes and typed args, availability, assignment, boosting and blessings.</summary>
    public static class ActionContractScenarios
    {
        // Kinds whose handlers live in files W2-06 does not own (construction W2-05; defense and dungeon W2-09;
        // trade and transport unowned in Wave 2). The router's "rejected" fallback is still allowed for them.
        static readonly string[] ForeignKinds =
        {
            "planbuilding", "designatestockpile", "designategatherspot", "designatefishingspot", "removestockpile", "removegatherspot",
            "designatefarm", "clearfarm", "createzone", "removezone", "trainwarrior", "defendraid", "exploredungeon", "recallexplorer",
            "attackcreature", "buildroad", "designaterail", "buildbridge", "builddock", "buildtransportvehicle", "createtransportroute",
            "canceltransportroute", "offervillagetrade", "acceptvillagetrade", "cancelvillagetrade", "buyresource", "sellgoods",
        };

        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("action_contract.every_kind_rejects_bad_input_with_a_code", EveryKindCoded);
            yield return new Scenario("action_contract.router_failures_are_coded", RouterFailures);
            yield return new Scenario("action_contract.successes_carry_code_and_entity", SuccessCodes);
            yield return new Scenario("action_contract.multiplayer_actions_require_players", RequiresPlayers);
            yield return new Scenario("action_contract.assign_rejects_non_workplace_in_demo", AssignNonWorkplace);
            yield return new Scenario("action_contract.assign_unknown_target_keeps_work", AssignUnknownTarget);
            yield return new Scenario("action_contract.assign_reports_interrupted_job", AssignInterrupted);
            yield return new Scenario("action_contract.assign_refuses_kittens", AssignKitten);
            yield return new Scenario("action_contract.release_worker_releases_workplace_only", ReleaseWorker);
            yield return new Scenario("action_contract.recipe_add_defaults_to_repeat_in_demo", RecipeRepeat);
            yield return new Scenario("action_contract.boost_resolves_cat_building_and_job", BoostTargets);
            yield return new Scenario("action_contract.boost_scales_with_the_job_in_demo", BoostScales);
            yield return new Scenario("action_contract.boost_research_hut_adds_progress", BoostResearch);
            yield return new Scenario("action_contract.prefer_for_work_is_a_visible_toggle", PreferForWork);
            yield return new Scenario("action_contract.offerings_share_one_rate", OfferingRate);
            yield return new Scenario("action_contract.offering_honours_amount_and_reserve", OfferingAmount);
            yield return new Scenario("action_contract.blessing_studies_cost_triple_in_demo", BlessingCost);
            yield return new Scenario("action_contract.hurry_study_buys_five_minutes", HurryStudy);
            yield return new Scenario("action_contract.failed_queue_edit_keeps_automation", QueueEditAtomic);
            yield return new Scenario("action_contract.explore_refusal_keeps_returning_transport", ExploreAtomic);
        }

        static readonly Regex SnakeCase = new Regex("^[a-z][a-z0-9_]*$");

        static void Coded(ActionResult r, string what)
        {
            Check(r != null && SnakeCase.IsMatch(r.Code ?? ""), what + " returned no stable snake_case code: '" + r?.Code + "' (" + r?.Error + ")");
            if (r.Args != null)
                foreach (var pair in r.Args)
                    Check(SnakeCase.IsMatch(pair.Key) && pair.Value != null && !pair.Value.Contains(","), what + " has an untyped or locale-dependent arg " + pair.Key + "=" + pair.Value);
        }

        static ActionResult Expect(World w, Village v, GameAction a, string code)
        {
            var r = w.Apply(Context(v), a);
            Coded(r, a.Kind);
            Check(r.Code == code, a.Kind + " expected " + code + " but got " + r.Code + " (" + r.Error + ")");
            return r;
        }

        static string Arg(ActionResult r, string key) => r.Args != null && r.Args.TryGetValue(key, out var value) ? value : null;
        static double NumberArg(ActionResult r, string key) => double.Parse(Arg(r, key) ?? "NaN", CultureInfo.InvariantCulture);

        static Job ManualJob(World w, Village v, Cat c, string kind = "hunt", double work = 1000)
        {
            c.BuildingId = "";
            var job = new Job { Id = w.Id("manual"), Kind = kind, Phase = "working", CatId = c.Id, Position = c.Position, RequiredWork = work, Manual = true };
            v.Jobs.Add(job);
            c.JobId = job.Id;
            c.Goal = kind;
            return job;
        }

        static Building Scholar(World w, Village v, Cat c)
        {
            var hut = Station(w, v, "research_hut");
            c.BuildingId = "";
            Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = hut.Id }, ResultCode.Assigned);
            for (int tick = 0; tick < 40 && c.Goal != GoalKey.Research; tick++)
                w.Step(World.SimulationStepSeconds);
            w.Step(2);
            Check(c.Goal == GoalKey.Research && c.Position.Equals(hut.Position), "Fixture scholar must be studying at the hut");
            return hut;
        }

        static void EveryKindCoded()
        {
            var w = Fixture(out var v, out var c);
            Check(World.ActionKinds.Count == World.ActionKinds.Distinct().Count(), "Action kinds must be listed once");
            foreach (string kind in World.ActionKinds)
            {
                var bad = new GameAction { Kind = kind, CatId = "missing-cat", TargetId = "missing-target", BuildingId = "missing-building", NodeId = "missing-node", Resource = "missing-resource", RecipeId = "missing-recipe", Role = "missing-role", Edit = "missing-edit", Labor = "missing-labor", Mode = "missing-mode", OtherVillageId = "missing-village", OtherResource = "missing-resource", Name = "" };
                var r = w.Apply(Context(v), bad);
                Coded(r, kind);
                Check(r.Code != ResultCode.UnknownAction, "Listed kind " + kind + " is not routed");
                if (!ForeignKinds.Contains(kind))
                    Check(r.Code != ResultCode.Rejected, kind + " fell back to the uncoded '" + ResultCode.Rejected + "' code: " + r.Error);
                if (!r.Success)
                    Check(r.Error != "", kind + " failure lost its message");
            }
            Expect(w, v, new GameAction { Kind = "NoSuchVerb" }, ResultCode.UnknownAction);
        }

        static void RouterFailures()
        {
            var w = Fixture(out var v, out var c);
            Check(w.Apply(Context(v), null).Code == ResultCode.InvalidAction, "Missing action must be coded");
            Expect(w, v, new GameAction { Kind = "MoveCat", Amount = double.NaN }, ResultCode.InvalidArguments);
            Expect(w, v, new GameAction { Kind = "PlanBuilding", Position = new Int2(1, 1, -1) }, ResultCode.SurfaceOnly);
            Check(w.Apply(new PlayerContext { PlayerId = "", VillageId = v.Id }, new GameAction { Kind = "BoostCat" }).Code == ResultCode.IdentityRequired, "Anonymous action must be coded");
            Check(w.Apply(new PlayerContext { PlayerId = "stranger", VillageId = "no-such-village" }, new GameAction { Kind = "BoostCat" }).Code == ResultCode.VillageDenied, "Ownership refusal must be coded");
            Expect(w, v, new GameAction { Kind = "AdvanceTime", Amount = 5 }, ResultCode.DeveloperOnly);
            Check(w.Apply(new PlayerContext { PlayerId = "dev", VillageId = v.Id, IsDeveloper = true }, new GameAction { Kind = "SetTestAcceleration" }).Code == ResultCode.NotImplemented, "Dead developer kind must say so");
        }

        static void SuccessCodes()
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var hut = Station(w, v, "research_hut");
            var assigned = Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = hut.Id }, ResultCode.Assigned);
            Check(assigned.EntityId == c.Id && Arg(assigned, ResultArg.Target) == hut.Id && Arg(assigned, ResultArg.Building) == "research_hut", "Assign must name the cat and the workplace");
            var released = Expect(w, v, new GameAction { Kind = "ReleaseWorker", CatId = c.Id }, ResultCode.Released);
            Check(released.EntityId == c.Id && Arg(released, ResultArg.Target) == hut.Id, "Release must name the cat and the old workplace");
            var entered = Expect(w, v, new GameAction { Kind = "EnterCatControl", CatId = c.Id }, ResultCode.Controlling);
            Check(entered.EntityId == c.Id, "Entering control must name the cat");
            Expect(w, v, new GameAction { Kind = "KeepCatControl", CatId = c.Id }, ResultCode.ControlKept);
            Expect(w, v, new GameAction { Kind = "LeaveCatControl", CatId = c.Id }, ResultCode.ControlLeft);
            v.Research.Clear();
            var study = Catalog.Research.First(n => n.Prerequisites.Count == 0);
            var studied = Expect(w, v, new GameAction { Kind = "ResearchNode", NodeId = study.Id }, ResultCode.Studied);
            Check(studied.EntityId == study.Id, "Research must name the study");
            var preference = Expect(w, v, new GameAction { Kind = "SetCatLaborPreference", CatId = c.Id, Labor = "hunt", Enabled = true }, ResultCode.PreferenceSet);
            Check(preference.EntityId == c.Id && Arg(preference, ResultArg.Enabled) == "true", "Labor preference must report its state");
        }

        static void RequiresPlayers()
        {
            var w = Fixture(out var v, out var c);
            var me = Context(v);
            w.Apply(me, new GameAction { Kind = "Presence" });
            foreach (var kind in new[] { "CastVote", "RequestVoteKick", "FoundVillage", "OfferVillageTrade", "AcceptVillageTrade", "CancelVillageTrade" })
            {
                var r = w.ActionAvailable(me, kind);
                Check(!r.Success && r.Code == ResultCode.RequiresPlayers, kind + " must require players in a single-player world, got " + r.Code);
            }
            foreach (var kind in new[] { "AssignWorker", "Boost", "MoveCat", "ResearchNode", "HurryStudy", "ReleaseWorker" })
                Check(w.ActionAvailable(me, kind).Success, kind + " must stay available in single player");
            Check(w.ActionAvailable(me, "NoSuchVerb").Code == ResultCode.UnknownAction, "Unknown kinds are not available");
            var other = new PlayerContext { PlayerId = "second-player" };
            Check(w.Apply(other, new GameAction { Kind = "FoundVillage", Name = "Second grove" }).Success, "A second player founds a village");
            Check(w.ActionAvailable(me, "CastVote").Success && w.ActionAvailable(me, "FoundVillage").Success && w.ActionAvailable(me, "OfferVillageTrade").Success, "Two players unlock votes, founding and trade");
            Check(w.ActionAvailable(me, "RequestVoteKick").Code == ResultCode.RequiresPlayers, "A petition needs five players");
            Check(!w.Villages.Any(x => x.OwnerId == "acceptance-player"), "Availability must not apply anything");
        }

        static void AssignNonWorkplace()
        {
            var w = Fixture(out var v, out var c);
            w.PacingProfile = Pacing.DemoProfile;
            c.BuildingId = "";
            var den = v.Buildings.First(b => b.Kind == "den" && b.Completed);
            int slots = den.Slots.Count;
            string worker = den.WorkerId;
            var r = Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = den.Id }, ResultCode.NotAWorkplace);
            Check(Arg(r, ResultArg.Target) == den.Id && c.BuildingId == "" && den.Slots.Count == slots && den.WorkerId == worker, "A den must not gain a phantom worker");
        }

        static void AssignUnknownTarget()
        {
            var w = Fixture(out var v, out var c);
            var job = ManualJob(w, v, c);
            var r = Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = "farm-that-was-cleared" }, ResultCode.UnknownTarget);
            Check(Arg(r, ResultArg.Target) == "farm-that-was-cleared", "Unknown target must be named");
            Check(c.JobId == job.Id && !job.Completed, "A stale assignment target must not cancel the cat's work");
        }

        static void AssignInterrupted()
        {
            var w = Fixture(out var v, out var c);
            var job = ManualJob(w, v, c, "hunt");
            var mill = Station(w, v, "research_hut");
            var r = Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = mill.Id }, ResultCode.Assigned);
            Check(Arg(r, ResultArg.Interrupted) == "hunt" && job.Completed && c.BuildingId == mill.Id, "Assignment must report the job it interrupted");
            var idle = Fixture(out var v2, out var c2);
            var hut = Station(idle, v2, "research_hut");
            c2.BuildingId = "";
            var quiet = Expect(idle, v2, new GameAction { Kind = "AssignWorker", CatId = c2.Id, BuildingId = hut.Id }, ResultCode.Assigned);
            Check(Arg(quiet, ResultArg.Interrupted) == null, "An idle cat interrupts nothing");
        }

        static void AssignKitten()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            c.AgeHours = 2;
            var hut = Station(w, v, "research_hut");
            Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = hut.Id }, ResultCode.Kitten);
            Check(c.BuildingId == "" && hut.Slots.Count == 0, "A kitten must not be staffed");
            c.AgeHours = 30;
            c.Migration = "arriving";
            Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = hut.Id }, ResultCode.Migrating);
        }

        static void ReleaseWorker()
        {
            var w = Fixture(out var v, out var c);
            var hut = Station(w, v, "research_hut");
            c.BuildingId = "";
            Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = hut.Id }, ResultCode.Assigned);
            var r = Expect(w, v, new GameAction { Kind = "ReleaseWorker", CatId = c.Id }, ResultCode.Released);
            Check(Arg(r, ResultArg.Building) == "research_hut" && c.BuildingId == "" && hut.Slots.All(s => s.CatId != c.Id) && hut.WorkerId == "", "Release must free the workplace");
            var job = ManualJob(w, v, c, "hunt");
            Expect(w, v, new GameAction { Kind = "ReleaseWorker", CatId = c.Id }, ResultCode.NotAssigned);
            Check(c.JobId == job.Id && !job.Completed, "Releasing a cat without a workplace must not cancel its hunt");
            var legacy = Expect(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id }, ResultCode.Released);
            Check(job.Completed && c.JobId == "" && Arg(legacy, ResultArg.Interrupted) == "hunt", "Legacy AssignWorker without a target still stands the cat down");
        }

        static void RecipeRepeat()
        {
            foreach (string profile in new[] { Pacing.ClassicProfile, Pacing.DemoProfile })
            {
                var w = Fixture(out var v, out var c);
                w.PacingProfile = profile;
                v.Research = Catalog.Research.Select(n => n.Id).ToList();
                var recipe = Catalog.Recipes.First(r => Catalog.RecipeAvailable(v, r));
                var station = Station(w, v, recipe.Building);
                var r = Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = recipe.Id }, ResultCode.QueueUpdated);
                Check(r.EntityId == station.Id, "Queue edit must name the station");
                Check(station.Queue.Single().Repeat == (profile == Pacing.DemoProfile), profile + ": an added recipe must " + (profile == Pacing.DemoProfile ? "" : "not ") + "repeat by default");
                Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add_once", RecipeId = recipe.Id }, ResultCode.QueueUpdated);
                Check(!station.Queue.Last().Repeat, profile + ": add_once adds a single batch");
            }
        }

        static void BoostTargets()
        {
            var w = Fixture(out var v, out var c);
            var job = ManualJob(w, v, c, "hunt", 1000);
            var byCat = Expect(w, v, new GameAction { Kind = "Boost", TargetId = c.Id }, ResultCode.Boosted);
            Check(byCat.EntityId == job.Id && job.BoostCount == 1, "Boosting a cat must boost its active job");
            Near(NumberArg(byCat, ResultArg.Seconds), 10, "Classic boost keeps the flat ten seconds");
            Near(job.RequiredWork, 990, "Boost seconds must come off the job");
            Check(Arg(byCat, ResultArg.Diminished) == "false", "The first boost is not diminished");
            var byJob = Expect(w, v, new GameAction { Kind = "Boost", TargetId = job.Id }, ResultCode.Boosted);
            Check(byJob.EntityId == job.Id && job.BoostCount == 2, "Job ids keep working");
            var scaffold = new Building { Id = w.Id("scaffold"), Kind = "wood_cutter", Position = new Int2(-5, 1), RequiredWork = 120 };
            v.Buildings.Add(scaffold);
            var builder = v.Cats[1];
            builder.ControlledBy = "";
            var build = ManualJob(w, v, builder, "build", 120);
            build.TargetId = scaffold.Id;
            var byBuilding = Expect(w, v, new GameAction { Kind = "Boost", TargetId = scaffold.Id }, ResultCode.Boosted);
            Check(byBuilding.EntityId == build.Id && scaffold.RequiredWork < 120, "Boosting a scaffold must speed its build");
            var idle = v.Cats[2];
            idle.ControlledBy = "";
            var none = Expect(w, v, new GameAction { Kind = "Boost", TargetId = idle.Id }, ResultCode.NoActiveJob);
            Check(Arg(none, ResultArg.Target) == idle.Id, "A cat without work names itself");
            Expect(w, v, new GameAction { Kind = "Boost", TargetId = "nothing-here" }, ResultCode.UnknownTarget);
        }

        static void BoostScales()
        {
            var w = Fixture(out var v, out var c);
            w.PacingProfile = Pacing.DemoProfile;
            var job = ManualJob(w, v, c, "hunt", 1000);
            var r = Expect(w, v, new GameAction { Kind = "Boost", TargetId = c.Id }, ResultCode.Boosted);
            Near(NumberArg(r, ResultArg.Seconds), 50, "Demo boost is five percent of the job");
            Near(job.RequiredWork, 950, "Demo boost seconds must come off the job");
            var other = v.Cats[1];
            other.ControlledBy = "";
            var small = ManualJob(w, v, other, "hunt", 10);
            var tiny = Expect(w, v, new GameAction { Kind = "Boost", TargetId = small.Id }, ResultCode.Boosted);
            Check(NumberArg(tiny, ResultArg.Seconds) >= 1 && small.RequiredWork >= small.Progress + 1 - 1e-9, "A tiny job still gains at least the minimum and is never completed by a click");
        }

        static void BoostResearch()
        {
            var w = Fixture(out var v, out var c);
            var hut = Scholar(w, v, c);
            double before = v.ResearchPoints;
            var r = Expect(w, v, new GameAction { Kind = "Boost", TargetId = hut.Id }, ResultCode.Boosted);
            double points = NumberArg(r, ResultArg.Points);
            Check(points > 0 && r.EntityId == hut.Id, "Boosting a research hut must add research progress");
            Near(v.ResearchPoints - before, points, "Reported points must match the added research", 0.001);
            var byScholar = Expect(w, v, new GameAction { Kind = "Boost", TargetId = c.Id }, ResultCode.Boosted);
            Check(NumberArg(byScholar, ResultArg.Points) > 0, "Boosting a studying scholar boosts the hut");
            var empty = Station(w, v, "research_hut", new Int2(-3, 3));
            Expect(w, v, new GameAction { Kind = "Boost", TargetId = empty.Id }, ResultCode.NoActiveJob);
        }

        static void PreferForWork()
        {
            var w = Fixture(out var v, out var c);
            var on = Expect(w, v, new GameAction { Kind = "BoostCat", CatId = c.Id, Enabled = true }, ResultCode.Preferred);
            Check(c.Boosted && on.EntityId == c.Id && Arg(on, ResultArg.Enabled) == "true", "Prefer for work turns on and says so");
            var off = Expect(w, v, new GameAction { Kind = "PreferForWork", CatId = c.Id, Enabled = false }, ResultCode.Preferred);
            Check(!c.Boosted && Arg(off, ResultArg.Enabled) == "false", "Prefer for work turns off and says so");
            Expect(w, v, new GameAction { Kind = "PreferForWork", CatId = "missing-cat" }, ResultCode.CatRequired);
        }

        static void OfferingRate()
        {
            var w = Fixture(out var v, out var c);
            double rate = w.OfferingBlessings(v, 10);
            Near(rate, 2, "Ten goods are worth two blessings at the base rate");
            var tithe = Expect(w, v, new GameAction { Kind = "OfferTithe" }, ResultCode.Offered);
            Near(v.Blessings, rate, "Tithe must use the shared offering rate");
            Near(NumberArg(tithe, ResultArg.Blessings), rate, "Tithe reports its blessings");
            v.Blessings = 0;
            Expect(w, v, new GameAction { Kind = "EnterCatControl", CatId = c.Id }, ResultCode.Controlling);
            c.Position = new Int2(v.Center.X + 1, v.Center.Z);
            c.X = c.Position.X;
            c.Z = c.Position.Z;
            c.Cargo.Add(new Stack("materials", 10));
            c.Cargo.Add(new Stack("logs", 3));
            var shrine = Expect(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id }, ResultCode.Offered);
            Near(v.Blessings, rate, "Shrine interaction must use the shared offering rate");
            Check(World.Amount(c.Cargo, "materials") == 0 && World.Amount(c.Cargo, "logs") == 3, "Only offerable cargo converts");
            Check(Arg(shrine, ResultArg.Blessings) != null, "Shrine offering reports its blessings");
            var nothing = Expect(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "offer" }, ResultCode.NothingToOffer);
            Check(nothing.Error != "", "Nothing to offer explains itself");
        }

        static void OfferingAmount()
        {
            double Offered(double stock, double amount, string expect)
            {
                var w = Fixture(out var v, out var c);
                c.BuildingId = "";
                World.Add(v.Stockpiles[0].Goods, "materials", stock);
                var r = Expect(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", Amount = amount, CatId = c.Id }, expect);
                if (!r.Success)
                    return -1;
                var job = v.Jobs.Single(j => j.Id == r.EntityId);
                Near(NumberArg(r, ResultArg.Amount), job.Amount, "Offering reports its amount");
                return job.Amount;
            }
            Near(Offered(40, 6, ResultCode.Queued), 6, "Offering must honour the requested amount");
            Near(Offered(40, 1, ResultCode.Queued), 5, "An offering carries at least one blessing's worth");
            Near(Offered(40, 100, ResultCode.Queued), 8, "An offering is limited to what one cat carries");
            Near(Offered(17, 100, ResultCode.Queued), 7, "An offering never dips into the reserve");
            Offered(12, 6, ResultCode.ReserveProtected);
        }

        static void BlessingCost()
        {
            foreach (string profile in new[] { Pacing.ClassicProfile, Pacing.DemoProfile })
            {
                var w = Fixture(out var v, out var c);
                w.PacingProfile = profile;
                v.Research.Clear();
                // The study as this world prices it: demo worlds read DemoContent's costs and prerequisites (W2-10).
                var study = Catalog.Research.Take(24).Select(n => Catalog.Study(w, n.Id)).First(n => n.Prerequisites.Count == 0 && n.Cost > 0);
                v.Blessings = study.Cost * 2;
                var r = w.Apply(Context(v), new GameAction { Kind = "UnlockNode", NodeId = study.Id });
                Coded(r, "UnlockNode");
                if (profile == Pacing.DemoProfile)
                {
                    Check(r.Code == ResultCode.CannotAfford && Arg(r, ResultArg.Resource) == "blessings", "Demo blessing studies must cost three times the research cost");
                    Near(NumberArg(r, ResultArg.Need), study.Cost * 3, "The refusal names the tripled cost");
                    Near(NumberArg(r, ResultArg.Have), study.Cost * 2, "The refusal names the blessings held");
                    v.Blessings = study.Cost * 3;
                    Expect(w, v, new GameAction { Kind = "UnlockNode", NodeId = study.Id }, ResultCode.Studied);
                    Near(v.Blessings, 0, "Demo blessing study spends three times its cost");
                }
                else
                {
                    Check(r.Code == ResultCode.Studied, "Classic blessing studies keep their cost");
                    Near(v.Blessings, study.Cost, "Classic blessing study spends its cost once");
                }
            }
            var poor = Fixture(out var pv, out var pc);
            pv.ResearchPoints = 0.4;
            pv.Research.Clear();
            var first = Catalog.Research.First(n => n.Prerequisites.Count == 0 && n.Cost > 1);
            var refusal = Expect(poor, pv, new GameAction { Kind = "ResearchNode", NodeId = first.Id }, ResultCode.CannotAfford);
            Check(Arg(refusal, ResultArg.Resource) == "research" && NumberArg(refusal, ResultArg.Have) == 0.4 && NumberArg(refusal, ResultArg.Need) == first.Cost, "Research refusal names need and have");
            var locked = Catalog.Research.First(n => n.Prerequisites.Count > 0);
            var missing = Expect(poor, pv, new GameAction { Kind = "ResearchNode", NodeId = locked.Id }, ResultCode.MissingPrerequisite);
            Check(locked.Prerequisites.Contains(Arg(missing, ResultArg.Study)), "Missing prerequisite names the study");
        }

        static void HurryStudy()
        {
            var w = Fixture(out var v, out var c);
            v.Blessings = 0;
            Expect(w, v, new GameAction { Kind = "HurryStudy" }, ResultCode.NoActiveResearch);
            Scholar(w, v, c);
            double before = v.ResearchPoints;
            w.Step(1);
            double perSecond = v.ResearchPoints - before;
            Check(perSecond > 0, "Fixture scholar must produce research");
            Expect(w, v, new GameAction { Kind = "HurryStudy" }, ResultCode.CannotAfford);
            v.Blessings = 2;
            before = v.ResearchPoints;
            var r = Expect(w, v, new GameAction { Kind = "HurryStudy" }, ResultCode.Hurried);
            Near(v.Blessings, 1, "Hurry spends one blessing");
            Near(v.ResearchPoints - before, perSecond * 300, "Hurry adds five minutes of current research", perSecond * 3);
            Near(NumberArg(r, ResultArg.Points), v.ResearchPoints - before, "Hurry reports its points", 0.001);
            Near(NumberArg(r, ResultArg.Seconds), 300, "Hurry reports five minutes");
        }

        static void QueueEditAtomic()
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var recipe = Catalog.Recipes.First(r => Catalog.RecipeAvailable(v, r));
            var station = Station(w, v, recipe.Building);
            var slot = new WorkSlot { CatId = c.Id, AutomatedBy = "steward", Queue = station.Queue };
            station.Slots.Add(slot);
            station.Queue.Add(new QueueEntry { RecipeId = recipe.Id, AutomatedBy = "steward" });
            Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "remove", Index = 5 }, ResultCode.QueueIndex);
            Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "juggle", Index = 0 }, ResultCode.UnknownEdit);
            Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "no-such-recipe" }, ResultCode.RecipeLocked);
            Check(slot.AutomatedBy == "steward" && station.Queue.All(e => e.AutomatedBy == "steward"), "A refused queue edit must not strip officer automation");
            Expect(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "setrepeat", Index = 0, Enabled = true }, ResultCode.QueueUpdated);
            Check(slot.AutomatedBy == "" && station.Queue.All(e => e.AutomatedBy == ""), "A player edit takes the queue over from the officer");
        }

        static void ExploreAtomic()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            var c = v.Cats[0];
            v.Cats.RemoveAll(x => x.Id != c.Id);
            c.Skills.Clear();
            c.Skills.Add(new Stack("fight", 12));
            c.Hunger = c.Thirst = c.Rest = c.Health = 100;
            var site = w.Dungeons[0];
            c.Position = site.Entrance;
            c.X = site.Entrance.X;
            c.Z = site.Entrance.Z;
            c.Y = w.WalkHeight(site.Entrance);
            c.HasHeight = true;
            c.Path.Clear();
            var vessel = new Vehicle { Id = w.Id("vessel"), Mode = "shipping", Position = new Int2(site.Entrance.X + 4, site.Entrance.Z) };
            v.Vehicles.Add(vessel);
            var route = new TransportRoute { Id = w.Id("route"), Mode = "shipping", CatId = c.Id, VehicleId = vessel.Id, Repeat = true, Path = new List<Int2> { site.Entrance, vessel.Position } };
            v.Routes.Add(route);
            c.BuildingId = route.Id;
            var r = Expect(w, v, new GameAction { Kind = "ExploreDungeon", CatId = c.Id, TargetId = site.Id }, ResultCode.TransportReturning);
            Check(!route.CancelRequested && route.Repeat && v.Routes.Contains(route) && c.BuildingId == route.Id && c.DungeonId == "", "A refused expedition must not cancel the driver's route first");
            Check(Arg(r, ResultArg.Target) == c.Id, "The refusal names the cat");
        }
    }
}
