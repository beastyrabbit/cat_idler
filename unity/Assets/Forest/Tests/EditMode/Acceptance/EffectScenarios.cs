using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// The authored catalog by invariants (catalog.*) and every research effect mechanism once
    /// (service_effect.* per effect id, building_effect.* per building attribute, resource_effect.*
    /// per recipe-family stage). Each mechanism owns every study that carries it and checks that the
    /// measured change matches the summed catalog payload, not merely that something moved.
    /// </summary>
    public static class EffectScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("catalog.research_graph_reachable_with_exact_cost", ResearchGraph);
            yield return new Scenario("catalog.every_recipe_has_a_station_and_an_unlock", RecipesReachable);
            yield return new Scenario("catalog.every_payload_is_consumed", PayloadsConsumed);
            yield return new Scenario("catalog.all_buildings_public_construction", AllBuildings);
            yield return new Scenario("regression.resource_capacity_is_resource_specific", ResourceSpecificCapacity);
            foreach (var entry in new[] { ("clickPower", 2, 20), ("supplySpeed", 3, 10), ("huntMastery", 5, 10), ("buildMastery", 5, 10), ("ritualMastery", 6, 10), ("resilience", 7, 10) })
            {
                var e = entry;
                yield return new Scenario("legacy_upgrade." + e.Item1, () => LegacyUpgrade(e.Item1, e.Item2, e.Item3));
                yield return new Scenario("legacy_effect." + e.Item1, () => LegacyEffect(e.Item1));
            }
            foreach (var effect in ModifyEffects())
            {
                var id = effect;
                yield return new Scenario("service_effect." + id, () => ServiceEffect(id));
            }
            foreach (var attribute in BuildingAttributes())
            {
                var id = attribute;
                yield return new Scenario("building_effect." + id, () => BuildingEffect(id));
            }
            foreach (var stage in ResourceStages())
            {
                var id = stage;
                yield return new Scenario("resource_effect." + id, () => ResourceEffect(id));
            }
        }

        static IEnumerable<string> ModifyEffects() => Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify").Select(p => p.Id).Distinct().OrderBy(x => x, StringComparer.Ordinal);
        static IEnumerable<string> BuildingAttributes() => Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify_building").Select(p => p.Attribute).Distinct().OrderBy(x => x, StringComparer.Ordinal);
        static IEnumerable<string> ResourceStages() => Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "unlock_resource").Select(p => Stage(p.Id)).Distinct().OrderBy(x => x, StringComparer.Ordinal);
        static string Stage(string resourceId) => resourceId.Substring(resourceId.LastIndexOf('_') + 1);
        static string Family(string resourceId) => resourceId.Substring(0, resourceId.LastIndexOf('_'));

        // Mechanisms the simulation reads; catalog.every_payload_is_consumed fails for anything else.
        static readonly string[] Capabilities = { "mountain_travel", "rail_logistics", "water_travel" };
        static readonly string[] ResearchStations = { "research_hut", "school" };
        static readonly string[] ReserveFamilies = { "hunting", "foraging", "grain_milling", "baking" };

        // ---- Ratio checks -------------------------------------------------------------------

        /// <summary>The measured change must match the expected ratio to within a tenth of the effect itself.</summary>
        static void Ratio(string what, double before, double after, double expected)
        {
            Check(before > 0 && World.Finite(before) && World.Finite(after), what + " has no measurable baseline: before=" + before + " after=" + after);
            double ratio = after / before, tolerance = Math.Max(0.1 * Math.Abs(expected - 1), 1e-9);
            Check(Math.Abs(expected - 1) > 1e-9, what + " expects no change; the catalog payload is empty");
            Check(Math.Abs(ratio - expected) <= tolerance, what + " ratio=" + ratio + " expected=" + expected + " before=" + before + " after=" + after);
        }

        static double Sum(IEnumerable<ResearchPayload> payloads)
        {
            var list = payloads.ToList();
            Check(list.All(p => p.Operation == "add"), "Expected additive payloads: " + string.Join(",", list.Select(p => p.Operation)));
            return list.Sum(p => p.Value);
        }

        static double Product(IEnumerable<ResearchPayload> payloads)
        {
            var list = payloads.ToList();
            Check(list.All(p => p.Operation == "multiply"), "Expected multiplying payloads: " + string.Join(",", list.Select(p => p.Operation)));
            return list.Aggregate(1.0, (value, p) => value * p.Value);
        }

        /// <summary>Owns the carriers' prerequisites in both runs and the carriers only in the second.</summary>
        static World OwningFixture(IReadOnlyCollection<Study> carriers, bool own, out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            var ids = new HashSet<string>(carriers.Select(n => n.Id));
            var research = carriers.SelectMany(Ancestors).Where(id => !ids.Contains(id)).Distinct().ToList();
            if (own)
                research.AddRange(ids);
            v.Research = research.OrderBy(x => x, StringComparer.Ordinal).ToList();
            return w;
        }

        // ---- Service effects (modify payloads) ----------------------------------------------

        static void ServiceEffect(string effect)
        {
            var carriers = Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify" && p.Id == effect)).ToList();
            double total = Sum(carriers.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify" && p.Id == effect));
            if (effect == "elderProtection")
            {
                ElderProtection(carriers, 1 + total);
                return;
            }
            Check(Measures.ContainsKey(effect), "No mechanism measures " + effect);
            var measure = Measures[effect];
            double before = measure.Run(OwningFixture(carriers, false, out var v0, out var c0), v0, c0);
            double after = measure.Run(OwningFixture(carriers, true, out var v1, out var c1), v1, c1);
            Ratio(effect, before, after, measure.Expected(total));
        }

        sealed class Mechanism
        {
            public Func<World, Village, Cat, double> Run;
            public Func<double, double> Expected;
        }
        static Mechanism Direct(Func<World, Village, Cat, double> run) => new Mechanism { Run = run, Expected = total => 1 + total };
        static Mechanism Inverse(Func<World, Village, Cat, double> run) => new Mechanism { Run = run, Expected = total => 1 / (1 + total) };

        static readonly Dictionary<string, Mechanism> Measures = new Dictionary<string, Mechanism>
        {
            { "storageCapacity", Direct((w, v, c) => w.Capacity(v, v.Stockpiles[0])) },
            { "storagePerLevelMult", Direct((w, v, c) => w.Capacity(v, v.Stockpiles[0])) },
            // Every den sleeps (int)(5 + total) cats; fill them with arrivals and count beds taken.
            { "housingPerDen", new Mechanism { Run = Housing, Expected = total => Math.Floor(5 + total) / 5 } },
            { "movementSpeed", Direct(Walk) },
            { "moveSpeedMult", Direct(Walk) },
            { "tradeValue", Direct(Sale) },
            { "combatPower", Direct(RaidDamage) },
            { "combatPowerMult", Direct(RaidDamage) },
            { "barracksReadiness", Direct(RaidDamage) },
            { "defensePower", Inverse(DefenderDamage) },
            { "defenseMult", Inverse(DefenderDamage) },
            { "wallDefense", Inverse(DefenderDamage) },
            { "constructionSpeed", Direct(Construction) },
            { "productionRate", Direct(Production) },
            { "huntYieldMult", Direct((w, v, c) => Gather(w, v, c, "hunt", "food")) },
            { "gatherYieldMult", Direct((w, v, c) => Gather(w, v, c, "fibre", "fibre")) },
            { "materialYieldMult", Direct((w, v, c) => Gather(w, v, c, "logs", "logs")) },
            // haulCapacity multiplies the base load of 8 (W2-04, sim-work: "carry capacity scales as 8 * haulCapacity").
            { "haulCapacity", new Mechanism { Run = (w, v, c) => Gather(w, v, c, "logs", "logs"), Expected = total => 1 + total } },
            { "waterCarryCapacity", new Mechanism { Run = (w, v, c) => Gather(w, v, c, "water", "water"), Expected = total => (8 + total) / 8 } },
            { "denStewardship", Inverse(RestLoss) },
            { "waterStewardship", Inverse((w, v, c) => ThirstLoss(w, v, c, true)) },
            { "waterEfficiency", Inverse((w, v, c) => ThirstLoss(w, v, c, false)) },
            { "foodStorekeeping", Inverse((w, v, c) => Spoilage(w, v, true)) },
            { "spoilageResistance", Inverse((w, v, c) => Spoilage(w, v, false)) },
            { "healthRecovery", Direct(HealthRegen) },
            { "kittenGrowth", Direct(KittenGrowth) },
            { "restRecovery", Direct(Sleep) },
            { "herbMedicineEfficacy", Direct(Medicine) },
            { "researchRate", Direct(Research) },
            { "researchRateMult", Direct(Research) },
            { "accountingSpeed", Direct(Accounting) },
            { "fieldStewardship", Direct((w, v, c) => Farm(w, v, c, false)) },
            { "farmYield", Direct((w, v, c) => Farm(w, v, c, true)) },
            { "farmYieldMult", Direct((w, v, c) => Farm(w, v, c, true)) },
            { "shrineBlessingYield", Direct(Offering) },
            { "mouseFarmFood", Direct(MouseFarm) },
        };

        static double Housing(World w, Village v, Cat c)
        {
            for (int i = 0; i < 100; i++)
                v.Cats.Add(new Cat { Id = w.Id("arrival"), Name = "Bed candidate", VillageId = v.Id, Position = v.Center, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000, Migration = "probationary", ProbationUntil = 100000 });
            w.Step(60);
            return v.Cats.Count(cat => cat.Alive && cat.BedId != "");
        }

        static double Walk(World w, Village v, Cat c)
        {
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            double x = c.X;
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            w.Step(0.1);
            return c.X - x;
        }

        static double Sale(World w, Village v, Cat c)
        {
            v.Trader.Phase = "trading";
            v.Trader.Until = 10000;
            var item = new Item { Id = w.Id("ware"), Kind = "mug", Material = "wood", VillageId = v.Id, LocationId = v.Stockpiles[0].Id };
            v.Items.Add(item);
            Act(w, v, new GameAction { Kind = "SellGoods", TargetId = item.Id });
            return v.Coins;
        }

        static Raid Skirmish(World w, Village v, Cat c)
        {
            Station(w, v, "barracks");
            Station(w, v, "walls");
            c.Skills.Add(new Stack("fight", 10));
            c.Position = v.Center;
            c.X = 0;
            c.Z = 0;
            c.Health = 60;
            // Below the regeneration floor, so only the fight changes health; above every need threshold.
            c.Hunger = c.Thirst = 45;
            var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Health = 1000 };
            v.Raids.Add(raid);
            w.Step(30);
            return raid;
        }
        static double RaidDamage(World w, Village v, Cat c) => 1000 - Skirmish(w, v, c).Health;
        static double DefenderDamage(World w, Village v, Cat c)
        {
            Skirmish(w, v, c);
            return 60 - c.Health;
        }

        static double Construction(World w, Village v, Cat c)
        {
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
            w.Step(40);
            var site = v.Buildings.First(b => b.Id == planned.EntityId);
            Check(!site.Completed, "Construction probe finished before it could be measured");
            return site.Progress;
        }

        static double Production(World w, Village v, Cat c)
        {
            var b = Station(w, v, "wood_cutter");
            World.Add(v.Stockpiles[0].Goods, "logs", 100);
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = "logs_to_planks" });
            w.Step(100);
            return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
        }

        /// <summary>
        /// An empty store at the station that takes only these goods. Finished goods go to the nearest
        /// store, so they land here, away from the fixture's 1000 food whose spoilage would hide them.
        /// </summary>
        static Stockpile OutputBay(World w, Village v, Building station, IEnumerable<string> resources)
        {
            var bay = new Stockpile { Id = w.Id("probe-bay"), Position = station.Position, Capacity = 1000, Accepts = resources.ToList() };
            v.Stockpiles.Add(bay);
            return bay;
        }

        /// <summary>Runs one production cycle; returns what its job reserved and what reached the output bay.</summary>
        static (double Input, double Output) Produce(World w, Village v, Building station, Recipe recipe)
        {
            var bay = recipe.Outputs.Count > 0 ? OutputBay(w, v, station, recipe.Outputs.Select(s => s.Resource)) : null;
            double input = 0;
            bool Finished() => v.Jobs.Any(j => j.Kind == "production" && j.TargetId == station.Id && j.Completed);
            for (int i = 0; i < 1800 && !Finished(); i++)
            {
                w.Step(1);
                foreach (var j in v.Jobs.Where(j => j.Kind == "production" && j.TargetId == station.Id))
                    input = Math.Max(input, j.Reserved.Where(s => recipe.Inputs.Any(r => r.Resource == s.Resource)).Sum(s => s.Amount));
            }
            Check(Finished(), "Production probe never finished " + recipe.Id);
            return (input, bay == null ? 0 : recipe.Outputs.Sum(s => World.Amount(bay.Goods, s.Resource)));
        }

        static double Gather(World w, Village v, Cat c, string kind, string resource)
        {
            double initial = Goods(w, v, resource);
            c.BuildingId = "";
            Act(w, v, new GameAction { Kind = "RequestJob", Name = kind, CatId = c.Id });
            var job = v.Jobs.First(j => j.CatId == c.Id && !j.Completed);
            for (int t = 0; t < 300 && job.Phase != "output_delivery" && !job.Completed; t++)
                w.Step(1);
            Check(job.Phase == "output_delivery" || job.Completed, "Gather probe never reached its output: " + kind);
            return Goods(w, v, resource) - initial;
        }

        static double RestLoss(World w, Village v, Cat c)
        {
            c.Rest = 90;
            w.Step(60);
            return 90 - c.Rest;
        }

        static double ThirstLoss(World w, Village v, Cat c, bool bowl)
        {
            if (bowl)
                Station(w, v, "water_bowl");
            c.Thirst = 90;
            w.Step(60);
            return 90 - c.Thirst;
        }

        static double Spoilage(World w, Village v, bool store)
        {
            if (store)
                Station(w, v, "food_storage");
            double before = Goods(w, v, "food");
            w.Step(60);
            return before - Goods(w, v, "food");
        }

        static double HealthRegen(World w, Village v, Cat c)
        {
            c.Health = 60;
            w.Step(60);
            return c.Health - 60;
        }

        static double KittenGrowth(World w, Village v, Cat c)
        {
            Station(w, v, "nursery");
            c.AgeHours = 1;
            w.Step(60);
            return c.AgeHours - 1;
        }

        static double Sleep(World w, Village v, Cat c)
        {
            Station(w, v, "beds");
            c.Rest = 1;
            c.Goal = "need_sleep";
            var den = v.Buildings.First(b => b.Id == c.BedId);
            c.Position = den.Position;
            c.X = c.Position.X;
            c.Z = c.Position.Z;
            w.Step(20);
            return c.Rest - 1;
        }

        static double Medicine(World w, Village v, Cat c)
        {
            Station(w, v, "herb_garden");
            World.Add(v.Stockpiles[0].Goods, "medicine", 5);
            c.Health = 10;
            // Below the regeneration floor, so only medicine heals; above every other need threshold.
            c.Hunger = c.Thirst = 45;
            w.Step(30);
            double used = 5 - Goods(w, v, "medicine");
            Check(used > 0, "The wounded cat never took medicine");
            return (c.Health - 10) / used;
        }

        static double Research(World w, Village v, Cat c)
        {
            var b = Station(w, v, "research_hut");
            double before = v.ResearchPoints;
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
            w.Step(30);
            return v.ResearchPoints - before;
        }

        static double Accounting(World w, Village v, Cat c)
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
            // Two more seconds of counting, still short of a finished count.
            w.Step(1);
            w.Step(1);
            return b.Progress;
        }

        static double Farm(World w, Village v, Cat c, bool harvest)
        {
            Station(w, v, "field");
            var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = harvest ? 7199 : 0 };
            v.Farms.Add(f);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id });
            w.Step(1);
            return harvest ? HarvestTotal(f, c) : f.Growth;
        }

        // The farmer carries the first load away in the same step, so count the field and the cargo.
        static double HarvestTotal(Farm f, Cat c) => f.Harvest + World.Amount(c.Cargo, f.Crop);

        static double Offering(World w, Village v, Cat c)
        {
            World.Add(v.Stockpiles[0].Goods, "materials", 30);
            c.BuildingId = "";
            double before = v.Blessings;
            Act(w, v, new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id });
            w.Step(100);
            return v.Blessings - before;
        }

        static double MouseFarm(World w, Village v, Cat c)
        {
            var b = Station(w, v, "mouse_farm");
            World.Add(v.Stockpiles[0].Goods, "grain", 1);
            var bay = OutputBay(w, v, b, new[] { "food" });
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
            for (int i = 0; i < 1800 && !v.Jobs.Any(j => j.Kind == "mouse_farm" && j.Completed); i++)
                w.Step(1);
            Check(v.Jobs.Any(j => j.Kind == "mouse_farm" && j.Completed), "The mouse farm never finished a cycle");
            return World.Amount(bay.Goods, "food");
        }

        /// <summary>
        /// Old-age death rolls against chance / protection. Pick a world RNG state whose next roll lies
        /// between the protected and unprotected chance: the unprotected elder dies, the protected one lives,
        /// so the protection divides the chance by at least (1 + total) and by no more.
        /// </summary>
        static void ElderProtection(List<Study> carriers, double factor)
        {
            double chance = Pacing.Classic.OldAgeDeathChance;
            bool Dies(bool own, double roll)
            {
                var w = OwningFixture(carriers, own, out var v, out var c);
                Station(w, v, "elder_corner");
                c.AgeHours = 300;
                c.ControlledBy = "fixture-held";
                c.ControlLeaseUntil = 10000000;
                w.RandomState = StateBefore(roll);
                // The elder's old-age roll is the first draw of the upkeep tick inside this minute.
                w.Step(60);
                return !c.Alive;
            }
            // Just inside the protected chance: both die. Just above it: only the unprotected elder dies.
            double inside = chance / factor * 0.98, between = (chance / factor + chance) / 2, outside = chance * 1.02;
            Check(Dies(false, inside) && Dies(true, inside), "Elders do not die below the protected chance");
            Check(Dies(false, between) && !Dies(true, between), "Elder protection does not divide the death chance by " + factor);
            Check(!Dies(false, outside), "Elders die above the unprotected chance");
        }

        /// <summary>A RandomState whose next <see cref="World.Roll"/> lies just at or above <paramref name="roll"/>.</summary>
        static uint StateBefore(double roll)
        {
            var probe = new World();
            double best = double.MaxValue;
            uint found = 0;
            for (uint state = 1; state < 20000000; state++)
            {
                probe.RandomState = state;
                double value = probe.Roll();
                if (value >= roll && value - roll < best)
                {
                    best = value - roll;
                    found = state;
                    if (best < roll * 0.002)
                        break;
                }
            }
            Check(best < roll * 0.01, "No RNG state rolls near " + roll);
            return found;
        }

        // ---- Building effects (modify_building payloads) ------------------------------------

        static void BuildingEffect(string attribute)
        {
            var targets = Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify_building" && p.Attribute == attribute).Select(p => p.Id).Distinct().OrderBy(x => x, StringComparer.Ordinal).ToList();
            Check(targets.Count > 0, "No building carries " + attribute);
            foreach (var building in targets)
            {
                var carriers = Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "modify_building" && p.Id == building && p.Attribute == attribute)).ToList();
                var payloads = carriers.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify_building" && p.Id == building && p.Attribute == attribute).ToList();
                double factor = attribute == "cycle_time" ? Product(payloads) : 1 + Sum(payloads);
                bool itemInput = attribute == "output" && !ResearchStations.Contains(building) && building != "field" && !Catalog.Recipes.Any(r => r.Building == building && r.Outputs.Count > 0);
                double expected = attribute == "cycle_time" || itemInput ? 1 / factor : factor;
                double before = BuildingMeasure(building, attribute, carriers, false), after = BuildingMeasure(building, attribute, carriers, true);
                Ratio(building + " " + attribute, before, after, expected);
            }
        }

        static double BuildingMeasure(string building, string attribute, List<Study> carriers, bool own)
        {
            var w = OwningFixture(carriers, own, out var v, out var c);
            var b = Station(w, v, building);
            c.Position = b.Position;
            c.X = b.Position.X;
            c.Z = b.Position.Z;
            if (attribute == "capacity")
            {
                var pile = new Stockpile { Id = w.Id("station-store"), ManagedBy = b.Id, Position = b.Position, Capacity = 100 };
                v.Stockpiles.Add(pile);
                return w.Capacity(v, pile);
            }
            if (attribute == "worker_slots")
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
            if (building == "field")
            {
                var f = new Farm { Id = w.Id("farm"), Position = c.Position, Handoff = new Int2(c.Position.X + 1, c.Position.Z), Growth = attribute == "output" ? 7199 : 0 };
                v.Farms.Add(f);
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, TargetId = f.Id });
                w.Step(1);
                return attribute == "output" ? HarvestTotal(f, c) : f.Growth;
            }
            if (ResearchStations.Contains(building))
            {
                double before = v.ResearchPoints;
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
                w.Step(10);
                return v.ResearchPoints - before;
            }
            var recipe = Catalog.Recipes.Where(r => r.Building == building).OrderBy(r => attribute == "durability" ? (r.ItemKind != "" ? 0 : 1) : attribute == "output" ? (r.Outputs.Count > 0 ? 0 : 1) : 0).First();
            Check(attribute != "durability" || recipe.ItemKind != "", building + " durability has no item recipe");
            foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(e => e.Kind == "unlock_recipe" && e.Id == recipe.Id)))
                if (!v.Research.Contains(unlock.Id))
                    v.Research.Add(unlock.Id);
            foreach (var s in recipe.Inputs)
                World.Add(v.Stockpiles[0].Goods, s.Resource, 100);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = b.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = recipe.Id, Edit = "add" });
            if (attribute == "cycle_time")
            {
                w.Step(100);
                return v.Jobs.Where(j => j.Kind == "production").Sum(j => j.Progress);
            }
            var made = Produce(w, v, b, recipe);
            if (attribute == "durability")
                return v.Items.Sum(x => x.MaxCondition);
            // Exact goods stay whole: an output study on an item station saves ingredients instead.
            return recipe.Outputs.Count > 0 ? made.Output : made.Input;
        }

        // ---- Resource-family stages (unlock_resource payloads) ------------------------------

        static void ResourceEffect(string stage)
        {
            var pace = Pacing.Classic;
            var ids = Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "unlock_resource" && Stage(p.Id) == stage).Select(p => p.Id).Distinct().OrderBy(x => x, StringComparer.Ordinal).ToList();
            foreach (var id in ids)
            {
                var carriers = Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_resource" && p.Id == id)).ToList();
                string family = Family(id);
                if (stage == "reserves")
                {
                    string resource = family == "foraging" ? "fibre" : family == "grain_milling" ? "grain" : "food";
                    Near(FamilyCapacity(carriers, true, resource) - FamilyCapacity(carriers, false, resource), 50, id + " reserve does not add 50 " + resource);
                    continue;
                }
                var recipe = Catalog.Recipes.FirstOrDefault(r => World.RecipeFamily(r) == family);
                Check(recipe != null, "Resource family has no executable recipe " + family);
                if (stage == "preservation" && recipe.ItemKind == "")
                {
                    string resource = recipe.Outputs[0].Resource;
                    Near(FamilyCapacity(carriers, true, resource) - FamilyCapacity(carriers, false, resource), 50, id + " preservation does not add 50 " + resource + " capacity");
                    continue;
                }
                double before = FamilyMeasure(stage, recipe, carriers, false), after = FamilyMeasure(stage, recipe, carriers, true);
                double expected = stage == "bulk" ? pace.FamilyBulkDurationFactor
                    : stage == "preservation" ? pace.PreservationDurabilityMult
                    : stage == "sources" ? (recipe.ItemKind != "" ? pace.FamilySourcesInputFactor : pace.FamilySourcesOutputFactor)
                    : double.NaN;
                Check(!double.IsNaN(expected), "Unknown resource effect stage " + id);
                Ratio(id, before, after, expected);
            }
        }

        static double FamilyCapacity(List<Study> carriers, bool own, string resource)
        {
            var w = OwningFixture(carriers, own, out var v, out _);
            return w.ResourceCapacity(v, v.Stockpiles[0], resource);
        }

        static double FamilyMeasure(string stage, Recipe recipe, List<Study> carriers, bool own)
        {
            var w = OwningFixture(carriers, own, out var v, out var c);
            var station = Station(w, v, recipe.Building);
            foreach (var unlock in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == recipe.Id)))
                if (!v.Research.Contains(unlock.Id))
                    v.Research.Add(unlock.Id);
            foreach (var s in recipe.Inputs)
                World.Add(v.Stockpiles[0].Goods, s.Resource, 100);
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, RecipeId = recipe.Id, Edit = "add" });
            if (stage == "bulk")
            {
                for (int i = 0; i < 60 && !v.Jobs.Any(j => j.Kind == "production"); i++)
                    w.Step(1);
                var job = v.Jobs.FirstOrDefault(j => j.Kind == "production");
                Check(job != null, "Bulk probe never started production " + recipe.Id);
                return job.RequiredWork;
            }
            var made = Produce(w, v, station, recipe);
            if (stage == "preservation")
                return v.Items.Sum(i => i.MaxCondition);
            return recipe.ItemKind != "" ? made.Input : made.Output;
        }

        // ---- Catalog invariants ---------------------------------------------------------------

        static void RecipesReachable()
        {
            Check(Catalog.Recipes.Count > 0 && Catalog.Research.Count > 0, "The catalog is empty");
            Check(Catalog.Recipes.Select(r => r.Id).Distinct().Count() == Catalog.Recipes.Count, "Duplicate recipe ids");
            Check(Catalog.Research.Select(n => n.Id).Distinct().Count() == Catalog.Research.Count, "Duplicate study ids");
            foreach (var r in Catalog.Recipes)
            {
                Check(Catalog.Buildings.Contains(r.Building) && r.Building != "shrine", r.Id + " has no constructible station");
                Check(Catalog.Labors.Contains(r.Labor), r.Id + " uses unknown labour " + r.Labor);
                Check(r.Founding || Catalog.Research.Any(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == r.Id)), r.Id + " has no unlock");
                Check(r.Seconds > 0 && r.Inputs.Count > 0 && r.Inputs.All(s => s.Amount > 0) && r.Outputs.All(s => s.Amount > 0), r.Id + " has an empty or free conversion");
                Check(r.Outputs.Count > 0 ^ r.ItemKind != "", r.Id + " must make either goods or one exact item");
                Check(r.Inputs.Concat(r.Outputs).All(s => Catalog.Resources.Contains(s.Resource)), r.Id + " uses an unknown resource");
            }
        }

        static void PayloadsConsumed()
        {
            var stations = new HashSet<string>(Catalog.Recipes.Select(r => r.Building));
            var itemStations = new HashSet<string>(Catalog.Recipes.Where(r => r.ItemKind != "").Select(r => r.Building));
            var families = new HashSet<string>(Catalog.Recipes.Select(World.RecipeFamily));
            foreach (var n in Catalog.Research)
            {
                Check(n.Payloads.Count > 0, n.Id + " has no payload");
                foreach (var p in n.Payloads)
                {
                    string what = n.Id + " " + p.Kind + " " + p.Id + " " + p.Attribute;
                    switch (p.Kind)
                    {
                        case "modify":
                            Check(p.Id == "elderProtection" || Measures.ContainsKey(p.Id), what + ": no mechanism reads this effect");
                            Check(p.Operation == "add" && p.Value > 0, what + ": not a positive additive effect");
                            break;
                        case "modify_building":
                            Check(Catalog.Buildings.Contains(p.Id), what + ": unknown building");
                            bool works = p.Attribute == "capacity" ? stations.Contains(p.Id) || p.Id == "food_storage" || p.Id == "water_bowl"
                                : p.Attribute == "worker_slots" || p.Attribute == "output" || p.Attribute == "cycle_time" ? stations.Contains(p.Id) || ResearchStations.Contains(p.Id) || p.Id == "field"
                                : p.Attribute == "durability" ? itemStations.Contains(p.Id)
                                : false;
                            Check(works, what + ": the building has no mechanism for this attribute");
                            break;
                        case "unlock_recipe":
                            Check(Catalog.Recipe(p.Id) != null, what + ": unknown recipe");
                            break;
                        case "unlock_building":
                        case "building_available_at_founding":
                            Check(Catalog.Buildings.Contains(p.Id), what + ": unknown building");
                            break;
                        case "unlock_resource":
                            string stage = Stage(p.Id), family = Family(p.Id);
                            Check(stage == "reserves" ? ReserveFamilies.Contains(family) : (stage == "sources" || stage == "bulk" || stage == "preservation") && families.Contains(family), what + ": no mechanism reads this family stage");
                            break;
                        case "unlock_capability":
                            Check(Capabilities.Contains(p.Id), what + ": no mechanism reads this capability");
                            break;
                        default:
                            Check(false, what + ": no mechanism reads this payload kind");
                            break;
                    }
                }
            }
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
