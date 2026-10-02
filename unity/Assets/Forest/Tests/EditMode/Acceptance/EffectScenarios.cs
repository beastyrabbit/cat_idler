using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Catalog, research-node, legacy upgrade and legacy effect checks, mostly one case per catalog entry.</summary>
    public static class EffectScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("catalog.research_487_public_purchase", ResearchGraph);
            yield return new Scenario("catalog.all_buildings_public_construction", AllBuildings);
            yield return new Scenario("regression.resource_capacity_is_resource_specific", ResourceSpecificCapacity);
            foreach (var entry in new[] { ("clickPower", 2, 20), ("supplySpeed", 3, 10), ("huntMastery", 5, 10), ("buildMastery", 5, 10), ("ritualMastery", 6, 10), ("resilience", 7, 10) })
            {
                var e = entry;
                yield return new Scenario("legacy_upgrade." + e.Item1, () => LegacyUpgrade(e.Item1, e.Item2, e.Item3));
                yield return new Scenario("legacy_effect." + e.Item1, () => LegacyEffect(e.Item1));
            }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify_building")))
            {
                var id = node.Id;
                yield return new Scenario("building_effect." + id, () => BuildingStudy(id));
            }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify")))
            {
                var id = node.Id;
                yield return new Scenario("service_effect." + id, () => ServiceStudy(id));
            }
            foreach (var node in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_resource")))
            {
                var id = node.Id;
                yield return new Scenario("resource_effect." + id, () => ResourceStudy(id));
            }
        }
        static void BuildingStudy(string id)
        {
            var node = Catalog.Study(id);
            foreach (var payload in node.Payloads.Where(p => p.Kind == "modify_building"))
            {
                double before = BuildingMeasure(id, payload, false), after = BuildingMeasure(id, payload, true);
                Check(after > before + 1e-8, "No functioning " + payload.Id + " " + payload.Attribute + " improvement: " + id + " before=" + before + " after=" + after);
            }
        }
        static double BuildingMeasure(string id, ResearchPayload p, bool own)
        {
            var w = StudyFixture(id, own, out var v, out var c);
            var b = Station(w, v, p.Id);
            c.Position = b.Position;
            c.X = b.Position.X;
            c.Z = b.Position.Z;
            if (p.Attribute == "capacity")
            {
                var pile = new Stockpile { Id = w.Id("station-store"), ManagedBy = b.Id, Position = b.Position, Capacity = 100 };
                v.Stockpiles.Add(pile);
                return w.Capacity(v, pile);
            }
            if (p.Attribute == "worker_slots")
            {
                int assigned = 0;
                foreach (var worker in v.Cats.Take(8))
                {
                    worker.ControlledBy = "";
                    if (w.Apply(Context(v), new GameAction { Kind = "AssignWorker", CatId = worker.Id, BuildingId = b.Id }).Success)
                        assigned++;
                }
                return assigned;
            }
            if (p.Id == "field")
            {
                var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = p.Attribute == "output" ? 7199 : 0 };
                v.Farms.Add(f);
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id });
                w.Step(1);
                return p.Attribute == "output" ? f.Harvest : f.Growth;
            }
            if (p.Id == "school" || p.Id == "research_hut")
            {
                double before = v.ResearchPoints;
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
                w.Step(10);
                return v.ResearchPoints - before;
            }
            var recipe = Catalog.Recipes.FirstOrDefault(r => r.Building == p.Id && (p.Attribute == "durability" ? r.ItemKind != "" : p.Attribute == "output" ? r.Outputs.Count > 0 : true)) ?? Catalog.Recipes.First(r => r.Building == p.Id);
            foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(e => e.Kind == "unlock_recipe" && e.Id == recipe.Id)))
                if (!v.Research.Contains(unlock.Id))
                    v.Research.Add(unlock.Id);
            var tracked = recipe.Inputs.Concat(recipe.Outputs).Select(s => s.Resource).ToArray();
            foreach (var s in recipe.Inputs)
                World.Add(v.Stockpiles[0].Goods, s.Resource, 100);
            var initial = recipe.Outputs.Sum(s => Goods(w, v, s.Resource));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = recipe.Id, Edit = "add" });
            if (p.Attribute == "cycle_time")
            {
                w.Step(100);
                return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
            }
            for (int i = 0; i < 1800 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); i++)
                w.Step(1);
            Check(v.Jobs.Any(j => j.Kind == "production" && j.Completed), "Building effect probe recipe failed " + recipe.Id);
            // Exact goods stay whole; an output study may improve finite ingredient efficiency.
            return p.Attribute == "durability" ? v.Items.Sum(x => x.MaxCondition) : recipe.Outputs.Count > 0 ? recipe.Outputs.Sum(s => Goods(w, v, s.Resource)) - initial : recipe.Inputs.Sum(s => Goods(w, v, s.Resource));
        }
        static void ServiceStudy(string id)
        {
            var p = Catalog.Study(id).Payloads.First(p => p.Kind == "modify");
            double before = ServiceMeasure(id, p.Id, false), after = ServiceMeasure(id, p.Id, true);
            Check(after > before + 1e-9, "No physical service effect " + id + " / " + p.Id + " before=" + before + " after=" + after);
        }
        static double ServiceMeasure(string id, string effect, bool own)
        {
            var w = StudyFixture(id, own, out var v, out var c);
            if (effect == "storageCapacity" || effect == "storagePerLevelMult")
                return w.Capacity(v, v.Stockpiles[0]);
            if (effect == "housingPerDen")
            {
                for (int i = 0; i < 100; i++)
                    v.Cats.Add(new Cat { Id = w.Id("arrival"), Name = "Bed candidate", VillageId = v.Id, Position = v.Center, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000, Migration = "probationary", ProbationUntil = 100000 });
                w.Step(60);
                return v.Cats.Count(cat => cat.Alive && cat.BedId != "");
            }
            if (effect == "moveSpeedMult" || effect == "movementSpeed")
            {
                Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
                double x = c.X;
                Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
                w.Step(0.1);
                return c.X - x;
            }
            if (effect == "tradeValue")
            {
                v.Trader.Phase = "trading";
                v.Trader.Until = 10000;
                var item = new Item { Id = w.Id("ware"), Kind = "mug", Material = "wood", VillageId = v.Id, LocationId = v.Stockpiles[0].Id };
                v.Items.Add(item);
                Act(w, v, new GameAction { Kind = "SellGoods", TargetId = item.Id });
                return v.Coins;
            }
            if (new[] { "combatPower", "combatPowerMult", "barracksReadiness", "defensePower", "defenseMult", "wallDefense" }.Contains(effect))
            {
                Station(w, v, "barracks");
                Station(w, v, "walls");
                c.Skills.Add(new Stack("fight", 10));
                c.Position = v.Center;
                c.X = 0;
                c.Z = 0;
                c.Health = 60;
                var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Health = 1000 };
                v.Raids.Add(raid);
                w.Step(30);
                return effect == "defensePower" || effect == "defenseMult" || effect == "wallDefense" ? c.Health : 1000 - raid.Health;
            }
            if (effect == "constructionSpeed")
            {
                c.BuildingId = "";
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                w.Step(80);
                return v.Buildings.First(b => b.Id == planned.EntityId).Progress;
            }
            if (effect == "productionRate")
            {
                var b = Station(w, v, "wood_cutter");
                World.Add(v.Stockpiles[0].Goods, "logs", 100);
                Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = "logs_to_planks" });
                w.Step(100);
                return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
            }
            if (new[] { "huntYieldMult", "gatherYieldMult", "materialYieldMult", "haulCapacity", "waterCarryCapacity" }.Contains(effect))
            {
                string kind = effect == "huntYieldMult" ? "hunt" : effect == "gatherYieldMult" ? "fibre" : effect == "waterCarryCapacity" ? "water" : "logs";
                string resource = kind == "hunt" ? "food" : kind;
                double initial = Goods(w, v, resource);
                c.BuildingId = "";
                Act(w, v, new GameAction { Kind = "RequestJob", Name = kind, CatId = c.Id });
                var job = v.Jobs.First(j => j.CatId == c.Id && !j.Completed);
                for (int t = 0; t < 300 && job.Phase != "output_delivery" && !job.Completed; t++)
                    w.Step(1);
                Check(job.Phase == "output_delivery" || job.Completed, "Gather effect never reached finite output " + effect);
                return Goods(w, v, resource) - initial;
            }
            if (effect == "denStewardship")
            {
                w.Step(60);
                return c.Rest;
            }
            if (effect == "waterStewardship" || effect == "waterEfficiency")
            {
                Station(w, v, "water_bowl");
                w.Step(60);
                return c.Thirst;
            }
            if (effect == "foodStorekeeping" || effect == "spoilageResistance")
            {
                Station(w, v, "food_storage");
                w.Step(60);
                return Goods(w, v, "food");
            }
            if (effect == "healthRecovery")
            {
                c.Health = 60;
                w.Step(60);
                return c.Health;
            }
            if (effect == "kittenGrowth")
            {
                Station(w, v, "nursery");
                c.AgeHours = 1;
                w.Step(60);
                return c.AgeHours;
            }
            if (effect == "restRecovery")
            {
                Station(w, v, "beds");
                c.Rest = 1;
                c.Goal = "need_sleep";
                var den = v.Buildings.First(b => b.Id == c.BedId);
                c.Position = den.Position;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                w.Step(20);
                return c.Rest;
            }
            if (effect == "herbMedicineEfficacy")
            {
                Station(w, v, "herb_garden");
                World.Add(v.Stockpiles[0].Goods, "medicine", 5);
                c.Health = 10;
                w.Step(30);
                return c.Health;
            }
            if (effect == "researchRate" || effect == "researchRateMult")
            {
                var b = Station(w, v, "research_hut");
                double before = v.ResearchPoints;
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
                w.Step(30);
                return v.ResearchPoints - before;
            }
            if (effect == "accountingSpeed")
            {
                var b = Station(w, v, "accounting_tent");
                if (!v.Research.Contains("basic_tools"))
                    v.Research.Add("basic_tools");
                c.Position = v.Stockpiles[0].Position;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = c.Id });
                Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
                for (int i = 0; i < 60 && b.Progress == 0; i++)
                    w.Step(1);
                Check(b.Progress > 0, "Accountant never arrived and began counting");
                return b.Progress;
            }
            if (effect == "fieldStewardship" || effect == "farmYield" || effect == "farmYieldMult")
            {
                Station(w, v, "field");
                var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = effect == "fieldStewardship" ? 0 : 7199 };
                v.Farms.Add(f);
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id });
                w.Step(1);
                return effect == "fieldStewardship" ? f.Growth : f.Harvest;
            }
            if (effect == "shrineBlessingYield")
            {
                World.Add(v.Stockpiles[0].Goods, "materials", 30);
                c.BuildingId = "";
                Act(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id });
                w.Step(100);
                return v.Blessings;
            }
            if (effect == "mouseFarmFood")
            {
                var b = Station(w, v, "mouse_farm");
                World.Add(v.Stockpiles[0].Goods, "grain", 1);
                double before = Goods(w, v, "food");
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
                w.Step(660);
                return Goods(w, v, "food") - before;
            }
            if (effect == "elderProtection")
            {
                Station(w, v, "elder_corner");
                c.AgeHours = 300;
                c.ControlledBy = "fixture-held";
                c.ControlLeaseUntil = 10000000;
                var node = Catalog.Study(id);
                double baseline = 1;
                foreach (var ancestor in Ancestors(node))
                    foreach (var p in Catalog.Study(ancestor).Payloads.Where(p => p.Id == effect))
                        baseline = p.Operation == "add" ? baseline + p.Value : baseline * p.Value;
                var delta = node.Payloads.First(p => p.Id == effect);
                double upgraded = delta.Operation == "add" ? baseline + delta.Value : baseline * delta.Value;
                double low = 0.0002 / upgraded, high = 0.0002 / baseline;
                bool found = false;
                for (uint state = 1; state < 10000000; state++)
                {
                    double roll = unchecked(state * 1664525u + 1013904223u) / 4294967296.0;
                    if (roll >= low && roll < high)
                    {
                        w.RandomState = state;
                        found = true;
                        break;
                    }
                }
                Check(found, "No deterministic old-age boundary seed");
                w.Step(60);
                return c.Alive ? 1 : 0;
            }
            throw new InvalidOperationException("Missing concrete service measurement " + effect);
        }
        static void ResourceStudy(string id)
        {
            double Measure(bool own)
            {
                var w = StudyFixture(id, own, out var v, out var c);
                var split = id.LastIndexOf('_');
                string family = id.Substring(0, split), stage = id.Substring(split + 1);
                var recipe = Catalog.Recipes.FirstOrDefault(r => World.RecipeFamily(r) == family);
                if (stage == "reserves")
                {
                    string resource = family == "foraging" ? "fibre" : family == "grain_milling" ? "grain" : "food";
                    return w.ResourceCapacity(v, v.Stockpiles[0], resource);
                }
                Check(recipe != null, "Resource family has no executable recipe " + family);
                if (stage == "preservation" && recipe.ItemKind == "")
                    return w.ResourceCapacity(v, v.Stockpiles[0], recipe.Outputs[0].Resource);
                var station = Station(w, v, recipe.Building);
                foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == recipe.Id)))
                    if (!v.Research.Contains(unlock.Id))
                        v.Research.Add(unlock.Id);
                foreach (var s in recipe.Inputs)
                    World.Add(v.Stockpiles[0].Goods, s.Resource, 100);
                double initialInput = recipe.Inputs.Sum(s => Goods(w, v, s.Resource));
                double initialOutput = recipe.Outputs.Sum(s => Goods(w, v, s.Resource));
                Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = c.Id });
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, RecipeId = recipe.Id, Edit = "add" });
                int elapsed = 0;
                for (; elapsed < 1800 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); elapsed++)
                    w.Step(1);
                Check(elapsed < 1800, "Resource-effect production failed " + id);
                if (stage == "bulk")
                    return -elapsed;
                if (stage == "preservation")
                    return v.Items.Sum(i => i.MaxCondition);
                Check(stage == "sources", "Unknown resource effect stage " + id);
                return recipe.ItemKind != "" ? recipe.Inputs.Sum(s => Goods(w, v, s.Resource)) - initialInput : recipe.Outputs.Sum(s => Goods(w, v, s.Resource)) - initialOutput;
            }
            double before = Measure(false), after = Measure(true);
            Check(after > before + 1e-8, "No finite resource effect " + id + " before=" + before + " after=" + after);
        }
        static void AllBuildings()
        {
            foreach (string kind in Catalog.Buildings.Where(k => k != "shrine"))
            {
                var w = Fixture(out var v, out var c);
                v.Research = Catalog.Research.Select(n => n.Id).ToList();
                c.BuildingId = "";
                Exterior(w, v);
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                var at = kind == "field" ? new Int2(14, 14) : new Int2(-5, 1);
                if (kind == "field")
                    FixtureAccessRoad(w, v, new Int2(at.X, at.Z - 1));
                var made = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = kind, Position = at, CatId = c.Id });
                w.Step(600);
                var built = v.Buildings.Single(b => b.Id == made.EntityId);
                Check(built.Completed, "Public construction failed " + kind);
                Check(built.Inputs.Count == 0, "Completed construction retained spendable inputs " + kind);
                Valid(w);
            }
        }
        static void LegacyUpgrade(string name, int baseCost, int maximum)
        {
            var w = Fixture(out var v, out var c);
            v.Blessings = 1000000;
            double spent = 0, points = v.ResearchPoints;
            for (int level = 0; level < maximum; level++)
            {
                Act(w, v, new GameAction { Kind = "PurchaseUpgrade", Name = name });
                spent += baseCost * (level + 1);
                Near(v.Blessings, 1000000 - spent, "Wrong escalating legacy upgrade cost " + name);
            }
            Check(!w.Apply(Context(v), new GameAction { Kind = "PurchaseUpgrade", Name = name }).Success, "Legacy upgrade exceeds maximum " + name);
            Near(v.ResearchPoints, points, "Legacy blessing upgrade consumed research points");
            Near(v.Blessings, 1000000 - spent, "Rejected max upgrade charged again");
        }
        static void ResourceSpecificCapacity()
        {
            var w = StudyFixture("grain_milling_preservation", false, out var v, out var c);
            double logs = w.ResourceCapacity(v, v.Stockpiles[0], "logs"), flour = w.ResourceCapacity(v, v.Stockpiles[0], "flour");
            Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "grain_milling_preservation" });
            Near(w.ResourceCapacity(v, v.Stockpiles[0], "logs"), logs, "Grain preservation incorrectly increases Logs capacity");
            Check(w.ResourceCapacity(v, v.Stockpiles[0], "flour") > flour, "Grain preservation does not increase Flour capacity");
        }
        static void LegacyEffect(string name)
        {
            double Measure(bool owned)
            {
                var w = Fixture(out var v, out var c);
                c.BuildingId = "";
                v.Blessings = 10000;
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                World.Add(v.Stockpiles[0].Goods, "materials", 100);
                c.Hunger = 80;
                if (owned)
                    for (int i = 0; i < 5; i++)
                        Act(w, v, new GameAction { Kind = "PurchaseUpgrade", Name = name });
                if (name == "resilience")
                {
                    w.Step(600);
                    return c.Hunger;
                }
                if (name == "buildMastery")
                {
                    var build = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                    int ticks = 0;
                    for (; ticks < 1000 && !v.Buildings.Single(b => b.Id == build.EntityId).Completed; ticks++)
                        w.Step(1);
                    Check(ticks < 1000, "Legacy build effect never completed construction");
                    return -ticks;
                }
                if (name == "ritualMastery")
                    Act(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id });
                else
                    Act(w, v, new GameAction { Kind = "RequestJob", Name = name == "supplySpeed" ? "water" : "hunt", CatId = c.Id });
                var job = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
                if (name == "clickPower")
                {
                    double before = job.RequiredWork;
                    Act(w, v, new GameAction { Kind = "Boost", TargetId = job.Id });
                    return before - job.RequiredWork;
                }
                int elapsed = 0;
                for (; elapsed < 1000 && !job.Completed; elapsed++)
                    w.Step(1);
                Check(elapsed < 1000, "Legacy effect job never completed " + name);
                return -elapsed;
            }
            double baseline = Measure(false), upgraded = Measure(true);
            Check(upgraded > baseline, "Purchased legacy upgrade has no gameplay effect " + name + " baseline=" + baseline + " upgraded=" + upgraded);
        }
        static void ResearchGraph()
        {
            var w = Fixture(out var v, out var c);
            double initial = v.ResearchPoints, spent = 0;
            var pending = Catalog.Research.ToList();
            Check(pending.Count == 487, "Maintained research count changed");
            while (pending.Count > 0)
            {
                var ready = pending.Where(n => n.Prerequisites.All(v.Research.Contains)).ToArray();
                Check(ready.Length > 0, "Unreachable/cyclic research: " + string.Join(",", pending.Select(n => n.Id)));
                foreach (var n in ready)
                {
                    Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = n.Id });
                    spent += n.Cost;
                    pending.Remove(n);
                    Check(!w.Apply(Context(v), new GameAction { Kind = "ResearchNode", NodeId = n.Id }).Success, "Duplicate research charged again");
                }
            }
            Near(v.ResearchPoints, initial - spent, "Catalog exact research cost");
            Check(Catalog.Recipes.All(r => Catalog.RecipeAvailable(v, r)), "Recipe lacks reachable unlock");
            Check(Catalog.Buildings.All(b => Catalog.BuildingAvailable(v, b)), "Building lacks reachable unlock");
        }
    }
}
