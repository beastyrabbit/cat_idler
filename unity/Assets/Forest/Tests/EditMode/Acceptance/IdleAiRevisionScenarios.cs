using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class IdleAiRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.idle_ai_staff_research_by_skill", () => SpecialistStaff("research_hut", "research", false));
            yield return new Scenario("regression.idle_ai_staff_craft_by_preference", () => SpecialistStaff("woodworking", "craft", true));
            yield return new Scenario("regression.idle_ai_staff_textile_by_preference", () => SpecialistStaff("clothier", "textile", true));
            yield return new Scenario("regression.idle_ai_replan_unfunded_automatic_recipe", () => ReplanRecipe(false, false));
            yield return new Scenario("regression.idle_ai_preserve_player_queue_takeover", () => ReplanRecipe(true, false));
            yield return new Scenario("regression.idle_ai_preserve_active_recipe_claim", () => ReplanRecipe(false, true));
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "idle-ai-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }

        static World Fixture(out Village village)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            store.Capacity = 100000;
            store.Position = new Int2(3, 2);
            village.LastLeaderResearch = 0;
            for (int i = 0; i < village.Cats.Count; i++)
            {
                var cat = village.Cats[i];
                cat.ControlledBy = i >= 1 && i <= 7 ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
                cat.Skills.Clear();
                cat.Preferences.Clear();
            }
            return world;
        }

        static Building Station(World world, Village village, string kind, Int2 position)
        {
            var building = new Building { Id = world.Id("idle-ai-station"), Kind = kind, Position = position, Completed = true };
            village.Buildings.Add(building);
            Check(world.Path(village.Center, position, village) != null, "AI fixture station is not reachable");
            return building;
        }

        static void Officer(World world, Village village, string role)
        {
            int index = Array.IndexOf(Catalog.Roles, role);
            if (!village.Research.Contains(Catalog.RoleStudies[index])) village.Research.Add(Catalog.RoleStudies[index]);
            if (!village.Buildings.Any(b => b.Kind == Catalog.RoleBuildings[index])) Station(world, village, Catalog.RoleBuildings[index], new Int2(-5, 1));
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = role, CatId = village.Cats[0].Id });
        }

        static void SpecialistStaff(string kind, string labor, bool preference)
        {
            var world = Fixture(out var village);
            var station = Station(world, village, kind, new Int2(1, 3));
            var generic = village.Cats[1];
            var specialist = village.Cats[6];
            World.Add(generic.Skills, "process", 250);
            World.Add(specialist.Skills, labor, 40);
            if (preference) Act(world, village, new GameAction { Kind = "SetCatLaborPreference", CatId = specialist.Id, Labor = labor, Enabled = true });
            World.Add(village.Stockpiles[0].Goods, "planks", 2);
            World.Add(village.Stockpiles[0].Goods, "blocks", 2);
            World.Add(village.Stockpiles[0].Goods, "fibre", 5);
            world.Step(1);
            Check(station.Slots.All(s => s.CatId == ""), "A specialist vacancy was staffed before its officer existed");
            Officer(world, village, kind == "research_hut" ? "loremaster" : kind == "clothier" ? "cloth_leader" : "forester");
            world.Step(10);
            Check(station.Slots.Count(s => s.CatId != "") == 1 && station.Slots[0].CatId == specialist.Id, "Officer ignored " + labor + " skill/preferences and assigned a generic processor");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") >= 4, "Specialist staffing consumed the founding idle reserve");
            if (kind == "research_hut")
            {
                double points = village.ResearchPoints, skill = World.Amount(specialist.Skills, labor);
                for (int second = 0; second < 60 && village.ResearchPoints == points; second++) world.Step(1);
                Check(village.ResearchPoints > points && World.Amount(specialist.Skills, labor) > skill, "The selected researcher did not do real research or gain its actual labor skill");
            }
            else
                Check(village.Jobs.Any(j => !j.Completed && j.CatId == specialist.Id && j.Kind == "production" && Catalog.Recipe(j.RecipeId).Labor == labor), "The selected specialist did not start its physical production labor");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ReplanRecipe(bool manual, bool active)
        {
            var world = Fixture(out var village);
            var station = Station(world, village, "clothier", new Int2(1, 3));
            Officer(world, village, "cloth_leader");
            var worker = village.Cats[1];
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            var slot = station.Slots.Single(s => s.CatId == worker.Id);
            // A persisted officer order can outlive its unclaimed supply: another consumer or spoilage can take it.
            slot.AutomatedBy = "cloth_leader";
            slot.Queue.Add(new QueueEntry { RecipeId = "fibre_to_thread", AutomatedBy = "cloth_leader" });
            World.Add(village.Stockpiles[0].Goods, "thread", 5);
            if (active) World.Add(village.Stockpiles[0].Goods, "fibre", 5);
            if (manual) Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "repeat", Index = 0, Enabled = true });
            world.Step(active ? 1 : 11);
            if (manual)
            {
                Check(slot.AutomatedBy == "" && slot.Queue.Count == 1 && slot.Queue[0].RecipeId == "fibre_to_thread" && slot.Queue[0].Repeat && worker.JobId == "", "Officer replaced the player's explicitly retained unfunded queue");
                Check(World.Amount(village.Stockpiles[0].Goods, "thread") == 5 && world.Reservations.Count == 0, "Manual queue takeover consumed or claimed the alternative's input");
                return;
            }
            string recipe = active ? "fibre_to_thread" : "fibre_to_cloth";
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == worker.Id && j.Kind == "production");
            Check(job != null && job.RecipeId == recipe, "Officer left a missing-input order stalled instead of choosing the funded cloth chain");
            string identity = job.Id;
            world.Step(20);
            Check(worker.JobId == identity && slot.Queue.Count == 1 && slot.Queue[0].RecipeId == recipe, "Officer replaced active production after it exclusively claimed its finite input");
            for (int second = 0; second < 700 && !job.Completed; second++) world.Step(1);
            Check(job.Completed, "Officer-selected physical production failed to finish");
            if (!active)
            {
                Check(world.Total(village, "cloth") == 1 && world.Total(village, "thread") == 0, "Replanned production did not consume the five finite Thread for exactly one delivered Cloth");
                Check(world.Reservations.Count == 0, "Completed alternative recipe retained a material claim");
            }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
