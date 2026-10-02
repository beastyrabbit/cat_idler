using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Needs: drinking, eating and processed needs, including needs that interrupt carried cargo.</summary>
    public static class NeedsScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.cargo_critical_need_conservation", CargoNeed);
            yield return new Scenario("regression.preserved_food_is_edible", PreservedFood);
            yield return new Scenario("regression.emergency_staffed_water", EmergencyWater);
            yield return new Scenario("regression.medicine_and_brew_consumption", ProcessedNeeds);
        }
        static void CargoNeed()
        {
            var w = Fixture(out var v, out var c);
            c.Thirst = 1;
            c.Cargo.Add(new Stack("logs", 8));
            v.Stockpiles[0].Accepts.Add("water");
            World.Add(v.Stockpiles[0].Goods, "water", 10);
            var j = new Job { Id = w.Id("job"), Kind = "logs", CatId = c.Id, Resource = "logs", Phase = "output_delivery", Position = c.Position };
            v.Jobs.Add(j);
            c.JobId = j.Id;
            w.Step(60);
            Check(c.Alive && c.Thirst > 35, "Cargo prevented drinking available water");
            Near(Goods(w, v, "logs"), 8, "Interrupted cargo lost/duplicated");
            Valid(w);
        }
        static void PreservedFood()
        {
            var w = Fixture(out var v, out var c);
            c.Hunger = 1;
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "food");
            World.Add(v.Stockpiles[0].Goods, "preserves", 8);
            w.Step(60);
            Check(c.Hunger > 35, "Preserved food cannot satisfy hunger");
            Check(Goods(w, v, "preserves") < 8, "Eating did not debit finite serving");
        }
        static void EmergencyWater()
        {
            var w = Fixture(out var v, out var c);
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water");
            var station = Station(w, v, "research_hut");
            c.Thirst = 1;
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            w.Step(240);
            Check(c.Alive && c.Thirst > 15, "No emergency fetch from staffed colony");
            Check(Goods(w, v, "water") > 0 || c.Thirst > 35, "No physical water recovery");
        }
        static void ProcessedNeeds()
        {
            var w = Fixture(out var v, out var c);
            c.Health = 10;
            c.Hunger = 100;
            c.Thirst = 100;
            World.Add(v.Stockpiles[0].Goods, "medicine", 5);
            w.Step(30);
            Check(c.Health > 30 && Goods(w, v, "medicine") < 5, "Medicine has no finite healing consumer");
            w = Fixture(out v, out c);
            c.Thirst = 1;
            World.Add(v.Stockpiles[0].Goods, "brew", 5);
            w.Step(30);
            Check(c.Thirst > 35, "Brew followed by clean water did not satisfy thirst");
            Near(Goods(w, v, "brew"), 4.75, "One drink must consume exactly one quarter Brew serving");
            Near(Goods(w, v, "water"), 999.25, "Brew drink must finish with three quarters clean Water");
        }
    }
}
