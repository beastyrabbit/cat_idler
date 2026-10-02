using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Demo content: the curated study slice, demo recipe overrides, merged stewardship, founding
    /// service effects and the guarantee that classic worlds see none of it.
    /// </summary>
    public static class DemoContentScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("demo_content.classic_recipe_of_is_catalog", ClassicRecipeOf);
            yield return new Scenario("demo_content.classic_queries_are_unchanged", ClassicQueries);
            yield return new Scenario("demo_content.recipe_seconds_in_demo_band", RecipeSeconds);
            yield return new Scenario("demo_content.founding_conversions_three_to_one", FoundingConversions);
            yield return new Scenario("demo_content.metal_tiers_cost_more_for_better_items", MetalTiers);
            yield return new Scenario("demo_content.logs_to_materials_runs_at_wood_cutter", LogsToMaterials);
            yield return new Scenario("demo_content.demo_recipe_speed_runs_in_the_simulation", DemoRecipeSpeed);
            yield return new Scenario("demo_content.slice_size", SliceSize);
            yield return new Scenario("demo_content.slice_prerequisites_stay_in_the_slice", SlicePrerequisites);
            yield return new Scenario("demo_content.slice_unlock_pacing", SlicePacing);
            yield return new Scenario("demo_content.slice_has_no_empty_study", SliceHasNoEmptyStudy);
            yield return new Scenario("demo_content.research_hut_owned_at_founding", ResearchHutOwned);
            yield return new Scenario("demo_content.research_hut_seats_two", ResearchHutSlots);
            yield return new Scenario("demo_content.building_study_unlocks_base_recipe", BuildingStudyUnlocksBaseRecipe);
            yield return new Scenario("demo_content.every_demo_station_has_base_recipe", EveryStationHasBaseRecipe);
            yield return new Scenario("demo_content.no_dominated_recipe_in_slice", NoDominatedRecipe);
            yield return new Scenario("demo_content.functionless_recipes_hidden", FunctionlessHidden);
            yield return new Scenario("demo_content.demo_buildings_visible", BuildingsVisible);
            yield return new Scenario("demo_content.stewardship_chains_merged", StewardshipMerged);
            yield return new Scenario("demo_content.service_buildings_have_founding_effects", ServiceFoundingEffects);
            yield return new Scenario("demo_content.demo_effects_follow_the_world", DemoEffects);
            yield return new Scenario("demo_content.player_voice_text", PlayerVoiceText);
        }

        static World DemoWorld()
        {
            var w = World.Create(41);
            w.PacingProfile = Pacing.DemoProfile;
            return w;
        }

        static string Csv(IEnumerable<string> ids) => string.Join(",", ids);

        // ---- Classic isolation -------------------------------------------------------------------

        static void ClassicRecipeOf()
        {
            var w = World.Create(41);
            foreach (var recipe in Catalog.Recipes)
                Check(ReferenceEquals(w.RecipeOf(recipe.Id), recipe), "Classic RecipeOf differs from the catalog for " + recipe.Id);
            Check(w.RecipeOf(DemoContent.LogsToMaterials) == null, "Classic worlds gained the demo-only logs_to_materials recipe");
            Check(w.RecipeOf("no_such_recipe") == null && DemoWorld().RecipeOf("no_such_recipe") == null, "RecipeOf invented an unknown recipe");
            Check(ReferenceEquals(DemoContent.Recipes(w), Catalog.Recipes) && ReferenceEquals(DemoContent.Studies(w), Catalog.Research), "Classic worlds do not list the catalog");
            foreach (var n in Catalog.Research)
                Check(ReferenceEquals(DemoContent.Study(w, n.Id), n) && DemoContent.PriorityOf(w, n) == n.LeaderPriority, "Classic study lookup differs for " + n.Id);
            Check(DemoContent.SlotsFor(w, "research_hut") == 1 && !DemoContent.OwnedAtFounding(w, "research_hut"), "Classic founding changed");
        }

        static void ClassicQueries()
        {
            // A classic village that owns everything reads exactly the classic tables through the world overloads.
            var w = World.Create(41);
            var v = w.Villages[0];
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            foreach (var id in Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify").Select(p => p.Id).Distinct())
                Check(Catalog.Effect(w, v, id) == Catalog.Effect(v, id), "Classic effect differs for " + id);
            foreach (var p in Catalog.Research.SelectMany(n => n.Payloads).Where(p => p.Kind == "modify_building"))
                Check(Catalog.BuildingEffect(w, v, p.Id, p.Attribute) == Catalog.BuildingEffect(v, p.Id, p.Attribute), "Classic building effect differs for " + p.Id + " " + p.Attribute);
            foreach (var b in Catalog.Buildings)
                Check(Catalog.BuildingAvailable(w, v, b) == Catalog.BuildingAvailable(v, b), "Classic availability differs for " + b);
            foreach (var r in Catalog.Recipes)
                Check(Catalog.RecipeAvailable(w, v, r) == Catalog.RecipeAvailable(v, r), "Classic recipe availability differs for " + r.Id);
            var other = World.Create(41);
            var fresh = other.Villages[0];
            foreach (var b in Catalog.Buildings)
                Check(Catalog.BuildingAvailable(fresh, b) == Catalog.BuildingAvailable(other, fresh, b), "Classic founding availability differs for " + b);
            foreach (var b in new[] { "beds", "nursery", "herb_garden", "water_bowl", "walls" })
            {
                string effect;
                DemoContent.FoundingServiceFactor(b, out effect);
                v.Buildings.Add(new Building { Id = w.Id("probe"), Kind = b, Completed = true });
                Check(DemoContent.ServiceFactor(w, v, b, effect) == 1, "Classic " + b + " gained a founding effect");
            }
        }

        // ---- Demo recipes ------------------------------------------------------------------------

        static void RecipeSeconds()
        {
            var w = DemoWorld();
            foreach (var recipe in DemoContent.AllRecipes(w))
                Check(recipe.Seconds >= 45 && recipe.Seconds <= 90, "Demo recipe " + recipe.Id + " takes " + recipe.Seconds + " s, outside 45 to 90");
            foreach (var recipe in Catalog.Recipes)
                Check(w.RecipeOf(recipe.Id) != null && !ReferenceEquals(w.RecipeOf(recipe.Id), recipe), "Demo worlds share the classic instance of " + recipe.Id);
            Check(Catalog.Recipes.All(r => r.Seconds >= 600), "Classic recipe seconds changed");
        }

        static void FoundingConversions()
        {
            var w = DemoWorld();
            void Conversion(string id, string input, double amount, string output, double produced)
            {
                var r = w.RecipeOf(id);
                Check(r != null && r.Inputs.Count == 1 && r.Inputs[0].Resource == input && r.Inputs[0].Amount == amount, "Demo " + id + " does not take " + amount + " " + input);
                Check(r.Outputs.Count == 1 && r.Outputs[0].Resource == output && r.Outputs[0].Amount == produced, "Demo " + id + " does not make " + produced + " " + output);
            }
            Conversion("logs_to_planks", "logs", 3, "planks", 1);
            Conversion("stone_to_blocks", "stone", 3, "blocks", 1);
            var tools = w.RecipeOf("planks_and_blocks_to_tools");
            Check(tools.Inputs.Count == 2 && World.Amount(tools.Inputs, "planks") == 2 && World.Amount(tools.Inputs, "blocks") == 2 && tools.ItemKind == "tool", "Demo tools do not take 2 planks and 2 blocks");
            Check(Catalog.Recipe("logs_to_planks").Inputs[0].Amount == 5 && Catalog.Recipe("stone_to_blocks").Inputs[0].Amount == 5, "Classic founding conversions changed");
        }

        static void MetalTiers()
        {
            var w = DemoWorld();
            foreach (var family in new[] { "weaponcraft", "armorcraft" })
            {
                var tiers = new[] { "preparation", "staples", "quality", "specialty", "masterwork" }.Select(t => w.RecipeOf(family + "_" + t)).ToList();
                for (int i = 1; i < tiers.Count; i++)
                {
                    Check(World.Amount(tiers[i].Inputs, "metal") > World.Amount(tiers[i - 1].Inputs, "metal"), family + " tier " + tiers[i].Id + " is not dearer than " + tiers[i - 1].Id);
                    Check(tiers[i].Quality >= tiers[i - 1].Quality, family + " tier " + tiers[i].Id + " is not better than " + tiers[i - 1].Id);
                }
            }
            Check(World.Amount(Catalog.Recipe("weaponcraft_preparation").Inputs, "metal") == 6, "Classic metal tiers changed");
        }

        static void LogsToMaterials()
        {
            var w = Fixture(out var v, out var c);
            w.PacingProfile = Pacing.DemoProfile;
            var recipe = w.RecipeOf(DemoContent.LogsToMaterials);
            Check(recipe != null && recipe.Building == "wood_cutter" && recipe.Founding, "Demo worlds have no founding wood cutter logs_to_materials recipe");
            Check(World.Amount(recipe.Inputs, "logs") == 2 && World.Amount(recipe.Outputs, "materials") == 3, "logs_to_materials is not 2 logs to 3 materials");
            var b = Station(w, v, "wood_cutter");
            World.Add(v.Stockpiles[0].Goods, "logs", 2);
            Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = DemoContent.LogsToMaterials, Edit = "add" });
            double materials = Goods(w, v, "materials");
            for (int i = 0; i < 300 && Goods(w, v, "materials") < materials + 3; i++)
                w.Step(1);
            Check(Goods(w, v, "materials") >= materials + 3 - 1e-9, "The wood cutter never turned 2 logs into 3 materials");
            Check(Goods(w, v, "logs") <= 1e-9, "The wood cutter did not consume the 2 logs");
            Valid(w);
        }

        static void DemoRecipeSpeed()
        {
            // The same planks order finishes in a tenth of the classic time and from three logs.
            double Run(bool demo, out double logs)
            {
                var w = Fixture(out var v, out var c);
                if (demo)
                    w.PacingProfile = Pacing.DemoProfile;
                var b = Station(w, v, "wood_cutter");
                World.Add(v.Stockpiles[0].Goods, "logs", 10);
                Act(w, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, RecipeId = "logs_to_planks", Edit = "add" });
                int t = 0;
                for (; t < 1800 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); t++)
                    w.Step(1);
                Check(v.Jobs.Any(j => j.Kind == "production" && j.Completed), "Planks never finished (demo=" + demo + ")");
                logs = 10 - Goods(w, v, "logs");
                return t;
            }
            double classic = Run(false, out var classicLogs), demoSeconds = Run(true, out var demoLogs);
            Check(Math.Abs(classicLogs - 5) < 1e-6 && Math.Abs(demoLogs - 3) < 1e-6, "Planks used " + classicLogs + " classic and " + demoLogs + " demo logs");
            Check(demoSeconds < classic / 4 && demoSeconds <= 120, "Demo planks took " + demoSeconds + " s against " + classic + " s classic");
        }

        // ---- The slice ---------------------------------------------------------------------------

        static void SliceSize()
        {
            int count = DemoContent.Slice.Count;
            Check(count >= 18 && count <= 24, "The demo slice has " + count + " studies, outside 18 to 24");
            Check(DemoContent.Slice.Distinct().Count() == count, "The demo slice repeats a study");
            Check(DemoContent.Slice.All(id => Catalog.Study(id) != null), "The demo slice names an unknown study");
            foreach (var role in Catalog.RoleStudies)
                Check(DemoContent.IsDemoVisibleStudy(role), "Officer study " + role + " is not in the demo slice");
            var w = DemoWorld();
            Check(DemoContent.Studies(w).Select(n => n.Id).SequenceEqual(DemoContent.Slice), "Demo worlds do not research the slice in order");
            Check(!DemoContent.IsDemoVisibleStudy("smelting") && !DemoContent.IsDemoVisibleStudy("rail"), "Full-game studies leaked into the demo slice");
        }

        static void SlicePrerequisites()
        {
            var w = DemoWorld();
            var seen = new HashSet<string>();
            foreach (var id in DemoContent.Slice)
            {
                var n = DemoContent.Study(w, id);
                foreach (var p in n.Prerequisites)
                {
                    Check(DemoContent.IsDemoVisibleStudy(p), id + " requires " + p + ", which is outside the demo slice");
                    Check(seen.Contains(p), id + " comes before its prerequisite " + p + " in the slice order");
                }
                seen.Add(id);
            }
            foreach (var id in DemoContent.FoundingStudies)
                Check(DemoContent.Slice[0] == id, "The founding study is not first in the slice");
            Check(Catalog.Study("carpentry_specialty").Prerequisites.SequenceEqual(new[] { "carpentry_bulk" }), "Classic prerequisites changed");
        }

        // The demo research model for an average player: one point per scholar-minute (W2-01's demo
        // research divisor of 60 at a work rate of 1), the Research Hut's two founding scholars at their
        // desks 85 % of the time (needs trips), research starting half a minute in, and every slice study
        // bought as soon as it is affordable, in slice order. School and Scholars' Guild raise the rate,
        // More Scholars adds a third seat. W2-INT-a measures the same curve in the running simulation.
        const double PointsPerScholarMinute = 1.0, Attendance = 0.85, StartMinutes = 0.5;

        internal static List<double> SliceUnlockMinutes(World w, Village v)
        {
            var owned = new HashSet<string>(DemoContent.FoundingStudies);
            double minutes = StartMinutes, points = 0, step = 1.0 / 600;
            var times = new List<double>();
            foreach (var n in DemoContent.Studies(w).Where(n => !owned.Contains(n.Id)))
            {
                while (points + 1e-9 < n.Cost)
                {
                    v.Research = owned.ToList();
                    double scholars = DemoContent.SlotsFor(w, v, "research_hut");
                    double rate = scholars * Attendance * PointsPerScholarMinute * Catalog.Effect(w, v, "researchRateMult");
                    points += rate * step;
                    minutes += step;
                }
                points -= n.Cost;
                owned.Add(n.Id);
                times.Add(minutes);
            }
            return times;
        }

        static void SlicePacing()
        {
            var w = DemoWorld();
            var times = SliceUnlockMinutes(w, w.Villages[0]);
            var gaps = times.Select((t, i) => t - (i == 0 ? 0 : times[i - 1])).ToList();
            Console.WriteLine("demo.slice " + string.Join(" ", DemoContent.Slice.Skip(1).Zip(times, (id, t) => id + "@" + t.ToString("0.0", CultureInfo.InvariantCulture))));
            for (int i = 0; i < gaps.Count; i++)
                Check(gaps[i] >= 1.5 && gaps[i] <= 2.5, DemoContent.Slice[i + 1] + " lands " + gaps[i].ToString("0.00", CultureInfo.InvariantCulture) + " min after the previous unlock, outside 1.5 to 2.5");
            Check(times.Last() >= 35 && times.Last() <= 45, "The last demo study lands at " + times.Last().ToString("0.0", CultureInfo.InvariantCulture) + " min, outside 35 to 45");
            Check(times.First() <= 2.5, "Basic Tools lands at " + times.First() + " min; the journey wants it at about 2");
        }

        static void SliceHasNoEmptyStudy()
        {
            var w = DemoWorld();
            foreach (var n in DemoContent.Studies(w).Where(n => !DemoContent.FoundingStudies.Contains(n.Id)))
            {
                bool visible = n.Payloads.Any(p => p.Kind == "unlock_building" || p.Kind == "unlock_recipe" || p.Kind == "unlock_capability"
                    || p.Kind == "modify" && (p.Id == "housingPerDen" || p.Id == "waterCarryCapacity" || p.Value >= 0.15)
                    || p.Kind == "modify_building" && p.Attribute == "worker_slots");
                Check(visible, n.Id + " has no visible effect in the demo: " + Csv(n.Payloads.Select(p => p.Kind + ":" + p.Id + ":" + p.Value)));
                Check(n.Cost > 0, n.Id + " is free in the demo");
            }
        }

        static void ResearchHutOwned()
        {
            var w = DemoWorld();
            Check(DemoContent.OwnedAtFounding(w, "research_hut") && !DemoContent.OwnedAtFounding(w, "basic_tools"), "Demo founding does not own exactly the Research Hut study");
            foreach (var n in DemoContent.Studies(w).Where(n => Catalog.Study(n.Id).Prerequisites.Contains("research_hut")))
                Check(!n.Prerequisites.Contains("research_hut"), n.Id + " still waits on the founding Research Hut study");
            // The first purchase needs nothing but research points, and already does something.
            var v = w.Villages[0];
            v.Research.Clear();
            var first = DemoContent.Study(w, DemoContent.Slice[1]);
            Check(first.Prerequisites.Count == 0, "The first demo purchase has prerequisites");
            v.Research.Add(first.Id);
            Check(Catalog.Effect(w, v, "huntYieldMult") >= 1.25, "Basic Tools is not a visible first purchase in the demo");
        }

        static void ResearchHutSlots()
        {
            var w = DemoWorld();
            var v = w.Villages[0];
            Check(DemoContent.SlotsFor(w, "research_hut") == 2 && DemoContent.SlotsFor(w, "school") == 1, "Demo Research Hut does not seat two scholars");
            v.Research = new List<string>();
            Check(DemoContent.SlotsFor(w, v, "research_hut") == 2, "Demo village Research Hut does not seat two scholars");
            v.Research.Add("research_hut_crews");
            Check(DemoContent.SlotsFor(w, v, "research_hut") == 3, "More Scholars does not add a third seat in the demo");
        }

        static void BuildingStudyUnlocksBaseRecipe()
        {
            var w = DemoWorld();
            foreach (var n in DemoContent.Studies(w))
                foreach (var building in n.Payloads.Where(p => p.Kind == "unlock_building").Select(p => p.Id))
                {
                    var hosted = DemoContent.AllRecipes(w).Where(r => r.Building == building).ToList();
                    if (hosted.Count == 0)
                        continue;
                    var v = w.Villages[0];
                    v.Research = new List<string>(DemoContent.FoundingStudies) { n.Id };
                    Check(Catalog.BuildingAvailable(w, v, building), n.Id + " does not unlock " + building + " in the demo");
                    var usable = hosted.Where(r => Catalog.RecipeAvailable(w, v, r) && DemoContent.IsFunctional(r)).ToList();
                    Check(usable.Count > 0, n.Id + " unlocks the " + building + " with nothing to make");
                    Check(usable.All(r => Catalog.RecipeAvailable(v, w.RecipeOf(r.Id))), "The village-only check disagrees on " + building + " recipes");
                }
            var classic = World.Create(41).Villages[0];
            classic.Research = new List<string> { "research_hut", "smithy" };
            Check(!Catalog.RecipeAvailable(classic, Catalog.Recipe("smithy_tool")), "Classic smithy gained a base recipe");
        }

        static void EveryStationHasBaseRecipe()
        {
            var w = DemoWorld();
            foreach (var station in DemoContent.AllRecipes(w).Select(r => r.Building).Distinct().Where(DemoContent.IsDemoVisibleBuilding))
                Check(DemoContent.Recipes(w).Any(r => r.Building == station), "Demo station " + station + " has no visible recipe");
        }

        static void NoDominatedRecipe()
        {
            var w = DemoWorld();
            var visible = DemoContent.Recipes(w).ToList();
            string Key(Recipe r) => r.Building + "|" + Csv(r.Inputs.Select(s => s.Resource).OrderBy(x => x)) + "|" + (r.ItemKind != "" ? r.ItemKind + ":" + r.Material : Csv(r.Outputs.Select(s => s.Resource).OrderBy(x => x)));
            double Made(Recipe r) => r.ItemKind != "" ? 1 : r.Outputs.Sum(s => s.Amount);
            double Used(Recipe r) => r.Inputs.Sum(s => s.Amount);
            foreach (var group in visible.GroupBy(Key))
                foreach (var a in group)
                    foreach (var b in group.Where(b => b != a))
                    {
                        bool noWorse = Made(b) / Used(b) >= Made(a) / Used(a) && Made(b) / b.Seconds >= Made(a) / a.Seconds && b.Quality >= a.Quality;
                        bool better = Made(b) / Used(b) > Made(a) / Used(a) || Made(b) / b.Seconds > Made(a) / a.Seconds || b.Quality > a.Quality;
                        Check(!(noWorse && better), a.Id + " is dominated by " + b.Id + " at the " + a.Building);
                    }
            Check(visible.Count > 0, "The demo shows no recipes");
        }

        static void FunctionlessHidden()
        {
            var w = DemoWorld();
            var decorative = new[] { "mug", "bowl", "toy", "trinket", "furniture", "clothing", "brick" };
            foreach (var r in DemoContent.AllRecipes(w))
            {
                bool junk = decorative.Contains(r.ItemKind) || r.ItemKind == "" && r.Outputs.All(s => s.Resource == "refined");
                Check(!junk || !DemoContent.IsFunctional(r) && !DemoContent.IsDemoVisible(r), r.Id + " makes nothing useful but is shown in the demo");
            }
            foreach (var id in new[] { "logs_to_planks", "stone_to_blocks", "planks_and_blocks_to_tools", DemoContent.LogsToMaterials, "herbal_poultice", "flour_to_food", "smithy_weapon" })
                Check(DemoContent.IsDemoVisibleRecipe(id), id + " is hidden in the demo");
            Check(!DemoContent.IsDemoVisibleRecipe("materials_to_refined") && !DemoContent.IsDemoVisibleRecipe("bone_mug"), "Functionless recipes are shown in the demo");
            Check(DemoContent.Recipes(w).All(r => r.Founding || DemoContent.Studies(w).Any(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == r.Id))), "A demo recipe has no unlock inside the slice");
        }

        static void BuildingsVisible()
        {
            foreach (var b in new[] { "den", "research_hut", "wood_cutter", "workshop", "sawmill", "smithy", "smelter", "mill", "field", "accounting_tent", "barracks", "beds", "walls" })
                Check(DemoContent.IsDemoVisibleBuilding(b), b + " is hidden in the demo");
            var w = DemoWorld();
            var v = w.Villages[0];
            v.Research = new List<string>(DemoContent.Slice);
            foreach (var b in Catalog.Buildings.Where(DemoContent.IsDemoVisibleBuilding))
                Check(Catalog.BuildingAvailable(w, v, b), b + " is shown in the demo but the slice never unlocks it");
        }

        static void StewardshipMerged()
        {
            var w = DemoWorld();
            foreach (var building in new[] { "den", "water_bowl", "beds", "herb_garden", "nursery", "walls", "food_storage", "elder_corner", "accounting_tent" })
            {
                var chain = new[] { "foundations", "workflow", "timing", "crews", "reinforcement", "mastery" }.Select(s => building + "_" + s).Where(id => Catalog.Study(id) != null && Catalog.Study(id).Payloads.Count == 1 && Catalog.Study(id).Payloads[0].Kind == "modify").ToList();
                Check(chain.Count >= 2, building + " has no stewardship chain");
                string effect = Catalog.Study(chain[0]).Payloads[0].Id;
                double merged = DemoContent.Study(w, chain[0]).Payloads.Where(p => p.Id == effect).Sum(p => p.Value);
                double classicTotal = chain.Sum(id => Catalog.Study(id).Payloads[0].Value);
                Check(merged >= 0.15 && merged >= classicTotal - 1e-9, building + " merged stewardship grants only " + merged);
                foreach (var later in chain.Skip(1))
                    Check(!DemoContent.Study(w, later).Payloads.Any(p => p.Id == effect), later + " still grants " + effect + " after the merge");
                Check(Catalog.Study(chain[0]).Payloads[0].Value < 0.15, "Classic " + chain[0] + " changed");
            }
            // The merged study does what it says in a running demo world.
            var v = w.Villages[0];
            v.Research = new List<string> { "den_insulation", "den_foundations" };
            Check(Catalog.Effect(w, v, "denStewardship") >= 1.15, "Snug Dens is weaker than +15 % in the demo");
        }

        static void ServiceFoundingEffects()
        {
            var w = DemoWorld();
            var v = w.Villages[0];
            foreach (var b in new[] { "beds", "nursery", "herb_garden", "water_bowl", "walls" })
            {
                double factor = DemoContent.FoundingServiceFactor(b, out var effect);
                Check(factor >= 1.15 && effect != "", b + " has no real founding effect");
                v.Buildings.RemoveAll(x => x.Kind == b);
                Check(DemoContent.ServiceFactor(w, v, b, effect) == 1, b + " helps before it is built");
                v.Buildings.Add(new Building { Id = w.Id("probe"), Kind = b, Completed = false });
                Check(DemoContent.ServiceFactor(w, v, b, effect) == 1, b + " helps while still a site");
                v.Buildings.Add(new Building { Id = w.Id("probe"), Kind = b, Completed = true });
                Check(DemoContent.ServiceFactor(w, v, b, effect) == factor, b + " founding effect is not applied once built");
                Check(DemoContent.ServiceFactor(w, v, b, "unrelated") == 1, b + " founding effect leaks into other services");
            }
        }

        static void DemoEffects()
        {
            var demo = DemoWorld();
            var classic = World.Create(41);
            Check(Catalog.Study(demo, "basic_tools").Cost == 3 && Catalog.Study(classic, "basic_tools").Cost == 5, "Study(World) does not follow the world's content");
            foreach (var w in new[] { demo, classic })
                w.Villages[0].Research = new List<string> { "research_hut", "basic_tools", "smithy" };
            Check(Math.Abs(Catalog.Effect(demo, demo.Villages[0], "huntYieldMult") - 1.25) < 1e-9, "Demo Basic Tools does not give +25 % hunting");
            Check(Math.Abs(Catalog.Effect(classic, classic.Villages[0], "huntYieldMult") - 1.1) < 1e-9, "Classic Basic Tools changed");
            Check(Catalog.BuildingAvailable(demo, demo.Villages[0], "smelter") && !Catalog.BuildingAvailable(classic, classic.Villages[0], "smelter"), "Only the demo Smithy study opens the Smelter");
            Check(Catalog.Owns(demo, demo.Villages[0], "unlock_recipe", "ore_to_metal") && !Catalog.Owns(classic, classic.Villages[0], "unlock_recipe", "ore_to_metal"), "Only the demo Smithy study unlocks smelting");
            Check(demo.RecipeOf("carpentry_specialty").Building == "sawmill" && Catalog.Recipe("carpentry_specialty").Building == "wood_cutter", "Fine planks did not move to the demo Sawmill");
        }

        // ---- Text --------------------------------------------------------------------------------

        static void PlayerVoiceText()
        {
            var w = DemoWorld();
            string[] jargon = { "identity", "identities", "finite", "silently", "payload", "Stewardship (", "(Foundations)", "_" };
            void Text(string id, string text, int max)
            {
                Check(!string.IsNullOrWhiteSpace(text) && text.Length <= max, id + " text is empty or longer than " + max + ": " + text);
                Check(text.All(ch => ch < 128 || ch == '—' || ch == '·'), id + " text has foreign characters: " + text);
                foreach (var word in jargon)
                    Check(text.IndexOf(word, StringComparison.OrdinalIgnoreCase) < 0, id + " text uses developer language '" + word + "': " + text);
            }
            var descriptions = new HashSet<string>();
            foreach (var id in DemoContent.Slice)
            {
                var n = Catalog.Study(id);
                Text(id, n.Name, 24);
                Text(id, n.Description, 120);
                Check(descriptions.Add(n.Description), id + " repeats another slice description");
            }
            foreach (var r in DemoContent.Recipes(w))
            {
                Text(r.Id, r.Name, 28);
                Check(char.IsUpper(r.Name[0]) && r.Name.Split(' ').Length >= 2, r.Id + " name is not a verb and an object: " + r.Name);
            }
            foreach (var id in new[] { "den_foundations", "den_workflow", "den_timing", "den_crews", "den_reinforcement", "den_mastery" })
                Check(Catalog.Study(id).Description.IndexOf("shelter more", StringComparison.OrdinalIgnoreCase) < 0 && Catalog.Study(id).Description.IndexOf("rest", StringComparison.OrdinalIgnoreCase) >= 0, id + " still promises room instead of rest");
            Check(Catalog.Research.All(n => n.Name.All(ch => ch < 128) && n.Description.All(ch => ch < 128 || ch == '—')), "A study still has non-English characters");
        }
    }
}
