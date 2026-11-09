using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ProgressionRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.second_progression_farmer_owns_mouse_farm_automation", () => MouseFarmOfficer("farmer", true));
            yield return new Scenario("regression.second_progression_steward_does_not_automate_mouse_farm", () => MouseFarmOfficer("steward", false));
            yield return new Scenario("regression.second_progression_crew_study_expands_officer_staffing", () => ResearchCrews(false, false));
            yield return new Scenario("regression.second_progression_officer_crews_preserve_manual_slot", () => ResearchCrews(true, false));
            yield return new Scenario("regression.second_progression_officer_crews_retain_idle_reserve", () => ResearchCrews(false, true));
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static PlayerContext Context(Village village) => new PlayerContext { PlayerId = "progression-acceptance", VillageId = village.Id };
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(Context(village), action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }

        static World Fixture(int availableWorkers, out Village village)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            store.Capacity = 100000;
            store.Position = new Int2(3, 2);
            village.ResearchPoints = 1000;
            village.LastLeaderResearch = 0;
            for (int i = 0; i < village.Cats.Count; i++)
            {
                var cat = village.Cats[i];
                cat.ControlledBy = i < availableWorkers ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
            }
            return world;
        }

        static Building Station(World world, Village village, string kind, Int2 position)
        {
            var building = new Building { Id = world.Id("progression-station"), Kind = kind, Position = position, Completed = true };
            village.Buildings.Add(building);
            Check(world.Path(village.Center, position, village) != null, "Progression station must be reachable: " + kind);
            return building;
        }

        static void AddAncestors(Village village, string study)
        {
            foreach (var prerequisite in Catalog.Study(study).Prerequisites)
            {
                AddAncestors(village, prerequisite);
                if (!village.Research.Contains(prerequisite)) village.Research.Add(prerequisite);
            }
        }

        static void MouseFarmOfficer(string role, bool automated)
        {
            var world = Fixture(7, out var village);
            village.Research.Add(role == "farmer" ? "irrigation" : "basic_tools");
            Station(world, village, role == "farmer" ? "field" : "workshop", new Int2(1, 3));
            var farm = Station(world, village, "mouse_farm", new Int2(-5, 1));
            World.Add(village.Stockpiles[0].Goods, "grain", 1);
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = role, CatId = village.Cats[0].Id });
            world.Step(10);
            var staffed = farm.Slots.Where(s => s.CatId != "").ToArray();
            if (!automated)
            {
                Check(staffed.Length == 0, "Steward crossed the Farmer's food-production boundary by staffing a mouse farm");
                Check(!village.Jobs.Any(j => !j.Completed && j.TargetId == farm.Id), "Steward started unauthorized food production");
                return;
            }
            Check(staffed.Length == 1 && staffed[0].AutomatedBy == "farmer", "Assigned Farmer did not staff the reachable mouse farm");
            for (int second = 0; second < 700 && !village.Jobs.Any(j => j.Kind == "mouse_farm" && j.Completed); second++) world.Step(1);
            Check(village.Jobs.Any(j => j.Kind == "mouse_farm" && j.Completed && j.TargetId == farm.Id), "Farmer staffing did not complete the physical grain-to-food chain");
            Check(World.Amount(village.Stockpiles[0].Goods, "grain") == 0 && world.Reservations.All(r => r.Resource != "grain"), "Completed mouse-farm cycle retained or duplicated its finite grain input");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ResearchCrews(bool manualFirst, bool limitedLabor)
        {
            var world = Fixture(limitedLabor ? 5 : 7, out var village);
            AddAncestors(village, "research_hut_crews");
            var hut = Station(world, village, "research_hut", new Int2(1, 3));
            var busy = limitedLabor ? null : village.Cats[6];
            var manualWorkplace = village.Buildings.First(b => b.Kind == "den");
            if (busy != null) Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = manualWorkplace.Id, CatId = busy.Id });
            var manual = village.Cats[1];
            if (manualFirst)
            {
                Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = hut.Id, CatId = manual.Id });
                Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = hut.Id, CatId = manual.Id, Edit = "pause", Enabled = true });
            }
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "loremaster", CatId = village.Cats[0].Id });
            world.Step(10);
            Check(hut.Slots.Count(s => s.CatId != "") == 1, "Before the crew study, the Research Hut must expose exactly one staffed position");
            Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = "research_hut_crews" });
            Check(Catalog.BuildingEffect(village, "research_hut", "worker_slots", 1) == 2, "Crew study did not unlock its second real work position");
            world.Step(10);
            int expected = limitedLabor ? 1 : 2;
            Check(hut.Slots.Count(s => s.CatId != "") == expected, "Loremaster ignored an unlocked crew position or consumed the colony's idle reserve");
            Check(hut.Slots.Where(s => s.CatId != "").All(s => village.Cats.Single(c => c.Id == s.CatId).ControlledBy == "") && (busy == null || busy.BuildingId == manualWorkplace.Id), "Officer crew staffing took a controlled cat or displaced existing manual work");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") >= 4, "Officer staffing consumed the founding population's four-cat idle reserve");
            if (limitedLabor) return;
            var automatic = hut.Slots.Where(s => s.AutomatedBy == "loremaster" && s.CatId != "").ToArray();
            Check(automatic.Length == (manualFirst ? 1 : 2), "New crew position lost its officer ownership");
            foreach (var slot in automatic)
            {
                var cat = village.Cats.Single(c => c.Id == slot.CatId);
                cat.Position = hut.Position;
                cat.X = hut.Position.X;
                cat.Z = hut.Position.Z;
                cat.Path.Clear();
            }
            double points = village.ResearchPoints;
            world.Step(1);
            Check(village.ResearchPoints > points && automatic.All(s => World.Amount(village.Cats.Single(c => c.Id == s.CatId).Skills, "research") > 0), "Automatically filled crew positions did not perform real research");
            var automaticIds = automatic.Select(s => s.CatId).ToArray();
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "loremaster" });
            Check(!hut.Slots.Any(s => s.AutomatedBy == "loremaster" || automaticIds.Contains(s.CatId)) && automaticIds.All(id => village.Cats.Single(c => c.Id == id).BuildingId == ""), "Officer vacancy retained its automatic crew");
            if (manualFirst)
            {
                var manualSlot = hut.Slots.Single(s => s.CatId == manual.Id);
                Check(manualSlot.Paused && manualSlot.AutomatedBy == "" && manual.BuildingId == hut.Id, "Officer staffing or vacancy changed the player's paused manual position");
            }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
