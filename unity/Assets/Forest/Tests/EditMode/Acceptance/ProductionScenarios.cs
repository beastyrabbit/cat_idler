using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Production: stations, recipe queues, research effects on work, officers and accounting.</summary>
    public static class ProductionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.farm_worker_replacement", FarmReplacement);
            yield return new Scenario("regression.production_expertise_affects_progress", Expertise);
            yield return new Scenario("regression.accountant_skips_unreachable_pile", Accounting);
            yield return new Scenario("regression.mountain_research_opens_route", Mountain);
            yield return new Scenario("regression.station_capacity_study_changes_headroom", StationCapacity);
            yield return new Scenario("regression.selected_quarry_resource", QuarryResource);
            yield return new Scenario("regression.repeat_queue_reaches_second_recipe", RepeatedQueue);
            yield return new Scenario("regression.pause_preserves_active_recipe", PauseQueue);
            yield return new Scenario("regression.officer_vacancy_stops_automatic_staffing", OfficerVacancy);
            yield return new Scenario("chain.field_to_handoff_to_mill_to_food", FarmFoodChain);
            foreach (var recipe in Catalog.Recipes)
            {
                var id = recipe.Id;
                yield return new Scenario("recipe." + id, () => RecipeChain(id));
            }
        }
        static void FarmReplacement()
        {
            var w = Fixture(out var v, out var a);
            var b = v.Cats[1];
            b.ControlledBy = "";
            var f = new Farm { Id = w.Id("farm"), Position = new Int2(8, 1), Handoff = new Int2(7, 1) };
            v.Farms.Add(f);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = a.Id, TargetId = f.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = b.Id, TargetId = f.Id });
            Check(a.BuildingId == "", "Replaced farm worker still assigned");
            Check(f.WorkerId == b.Id, "New worker absent");
        }
        static double ProductionProgress(double skill)
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var recipe = Catalog.Recipes.First(r => r.Labor == "mill");
            var b = Station(w, v, recipe.Building);
            c.Skills.Add(new Stack(recipe.Labor, skill));
            foreach (var s in recipe.Inputs)
                World.Add(v.Stockpiles[0].Goods, s.Resource, 100);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe.Id });
            w.Step(100);
            return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
        }
        static void Expertise() => Check(ProductionProgress(100) > ProductionProgress(0) * 1.05, "Earned milling skill does not improve milling work");
        static void Accounting()
        {
            var w = Fixture(out var v, out var c);
            var b = Station(w, v, "accounting_tent");
            v.Research.Add("basic_tools");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            var unreachable = new Stockpile { Id = "aaa-unreachable", Position = new Int2(-3, 2) };
            v.Stockpiles.Insert(0, unreachable);
            w.TileAt(unreachable.Position).Wall = true;
            w.Step(90);
            Check(v.Stockpiles.Any(p => p.Id != "aaa-unreachable" && p.CountedAt >= 0), "Unreachable pile starves all accounting rounds");
            Check(unreachable.CountedAt < 0, "Unreachable pile falsely counted");
        }
        static void Mountain()
        {
            var w = Fixture(out var v, out var c);
            foreach (var n in Catalog.Research)
                v.Research.Add(n.Id);
            var p = new Int2(1, 2);
            w.TileAt(p).Mountain = true;
            c.Position = new Int2(1, 1);
            c.X = 1;
            c.Z = 1;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(2);
            Check(c.Position.Equals(p), "Mountaineering did not open mountain route");
        }
        static void StationCapacity()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.Research = Catalog.Research.Where(n => n.Id != "clothier_stores").Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var made = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "clothier", Position = new Int2(-5, 1), CatId = c.Id });
            w.Step(500);
            var piles = v.Stockpiles.Where(p => p.ManagedBy == made.EntityId).ToArray();
            Check(piles.Length > 0, "Completed Clothier has no physical station reserve");
            double before = piles.Sum(p => w.Capacity(v, p));
            Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "clothier_stores" });
            Check(piles.Sum(p => w.Capacity(v, p)) > before, "Clothier stores research has no finite headroom effect");
        }
        static void QuarryResource()
        {
            var w = Fixture(out var v, out var c);
            Exterior(w, v);
            c.BuildingId = "";
            c.Position = new Int2(8, 8);
            c.X = 8;
            c.Z = 8;
            var at = new Int2(9, 8);
            w.TileAt(at).Resource = "gem";
            w.TileAt(at).Amount = 30;
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "quarry", Resource = "gem", CatId = c.Id });
            w.Step(240);
            Check(w.Total(v, "gem") > 0, "Selected Gem quarry did not deliver Gem");
            Near(w.TileAt(at).Amount + Goods(w, v, "gem"), 30, "Quarry source-to-storage Gem conservation");
        }
        static void RepeatedQueue()
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "fibre", 100);
            var b = Station(w, v, "clothier");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            foreach (var id in new[] { "fibre_to_thread", "fibre_to_cloth" })
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = id, Edit = "add", Repeat = true });
            w.Step(1500);
            Check(w.Total(v, "cloth") > 0, "Repeating first queue entry starves downstream recipe");
            Check(v.Jobs.Where(j => j.Kind == "production" && j.Completed).Select(j => j.RecipeId).Distinct().Count() == 2, "Both linked recipes were not completed");
            Valid(w);
        }
        static void PauseQueue()
        {
            var w = Fixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "logs", 50);
            var b = Station(w, v, "wood_cutter");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = "logs_to_planks", Edit = "add" });
            w.Step(100);
            var job = v.Jobs.Single(j => j.Kind == "production" && !j.Completed);
            double progress = job.Progress, goods = Goods(w, v, "logs");
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "pause", Enabled = true });
            w.Step(60);
            Near(job.Progress, progress, "Paused job kept working");
            Near(Goods(w, v, "logs"), goods, "Paused inputs lost");
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "pause", Enabled = false });
            w.Step(700);
            Check(job.Completed, "Paused recipe did not resume");
            Valid(w);
        }
        static void OfficerVacancy()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            foreach (var worker in v.Cats.Take(6))
                worker.ControlledBy = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "logs", 100);
            World.Add(v.Stockpiles[0].Goods, "stone", 100);
            // The Forester's own station. A wood cutter is a founding station that the Leader runs while
            // the office is vacant (colony_automation.leader_takes_over_vacant_founding_station).
            var station = Station(w, v, "sawmill");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = c.Id });
            w.Step(12);
            var slot = station.Slots.FirstOrDefault(s => s.CatId != "");
            Check(slot != null, "Forester did not start automatic station staffing while retaining an idle reserve");
            string workerId = slot.CatId;
            Act(w, v, new GameAction { Kind = "UnassignOfficer", Role = "forester" });
            w.Step(1200);
            Check(!station.Slots.Any(s => s.CatId == workerId) && v.Cats.Single(x => x.Id == workerId).BuildingId == "", "Vacant Forester retained automatic production worker");
            Check(!v.Jobs.Any(j => j.Kind == "production" && !j.Completed), "Vacant Forester kept creating production jobs");
            Valid(w);
        }
        static void FarmFoodChain()
        {
            var w = Fixture(out var v, out var c);
            Exterior(w, v);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var mill = Station(w, v, "mill");
            c.BuildingId = "";
            double foodBefore = Goods(w, v, "food");
            FixtureAccessRoad(w, v, new Int2(12, 11));
            Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(12, 12), CatId = c.Id });
            w.Step(600);
            var farm = Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(14, 12), End = new Int2(15, 13) });
            Act(w, v, new GameAction { Kind = "AssignWorker", TargetId = farm.EntityId, CatId = c.Id });
            w.Step(7800);
            var output = v.Stockpiles.SingleOrDefault(p => p.ManagedBy == farm.EntityId && p.Kind == "farm_output");
            Check(output != null && World.Amount(output.Goods, "grain") > 0, "Farm did not carry grain to its exterior handoff");
            Near(w.Total(v, "grain"), 0, "Harvest teleported into general storage without hauling");
            double grain = Goods(w, v, "grain");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = output.Id, CatId = c.Id });
            w.Step(200);
            Check(w.Total(v, "grain") > 0, "Manual handoff haul never reached storehouse");
            Check(Goods(w, v, "grain") <= grain && Goods(w, v, "grain") >= grain * 0.98, "Harvest/haul duplicated or discarded grain");
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = c.Id });
            foreach (string recipe in new[] { "grain_to_flour", "flour_to_food" })
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = mill.Id, RecipeId = recipe, Edit = "add" });
            bool completed = false;
            for (int i = 0; i < 1500 && !completed; i++)
            {
                w.Step(1);
                completed = v.Jobs.Any(j => j.RecipeId == "flour_to_food" && j.Completed);
            }
            Check(completed, "Farm grain did not complete flour-to-food production");
            Check(Goods(w, v, "food") > foodBefore * 0.90, "Food chain consumed unexplained survival supply");
            Valid(w);
        }
        static void RecipeChain(string id)
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var r = Catalog.Recipe(id);
            var station = Station(w, v, r.Building);
            var input = new Dictionary<string, double>();
            foreach (var s in r.Inputs)
            {
                double amount = w.RecipeInput(v, r, s.Amount);
                World.Add(v.Stockpiles[0].Goods, s.Resource, amount);
                input[s.Resource] = Goods(w, v, s.Resource);
            }
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = id });
            bool carried = false, worked = false, completed = false;
            for (int tick = 0; tick < 3600; tick++)
            {
                w.Step(1);
                carried |= c.Cargo.Count > 0;
                worked |= v.Jobs.Any(j => j.Kind == "production" && j.Progress > 0 && c.Position.Equals(station.Position));
                completed = v.Jobs.Any(j => j.Kind == "production" && j.Completed);
                if (completed)
                    break;
            }
            Check(completed, "Recipe never completed: " + id + " blocked=" + c.BlockedReason + " / " + station.BlockedReason);
            Check(carried && worked, "Recipe skipped physical input carry/work: " + id);
            Check(w.Reservations.Count == 0, "Recipe left orphan input claim: " + id);
            foreach (var s in r.Inputs.Where(s => !r.Outputs.Any(o => o.Resource == s.Resource)))
            {
                double remaining = input[s.Resource] - w.RecipeInput(v, r, s.Amount);
                double actual = Goods(w, v, s.Resource);
                Check(actual <= remaining + 0.0001 && actual >= remaining * 0.98 - 0.0001, "Wrong conserved input after recipe/spoilage " + id + " / " + s.Resource + " expected approximately=" + remaining + " actual=" + actual);
            }
            foreach (var s in r.Outputs)
            {
                double expected = w.RecipeOutput(v, r, s.Amount);
                Check(w.Total(v, s.Resource) >= expected * 0.98, "Output not physically stored: " + id + " / " + s.Resource);
            }
            if (r.ItemKind != "")
            {
                var items = v.Items.Where(i => i.Kind == r.ItemKind).ToArray();
                Check(items.Length == 1 && items[0].Material == r.Material && items[0].Quality == r.Quality, "Wrong exact item identity/material/quality " + id);
                Check(v.Stockpiles.Any(p => p.Id == items[0].LocationId), "Item never reached pile " + id);
            }
            Valid(w);
        }
    }
}
