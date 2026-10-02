using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Long seeded campaign twins (explicit; excluded from the default runs).</summary>
    public static class CampaignScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (int seed in new[] { 7, 41, 127 })
            {
                int captured = seed;
                yield return new Scenario("campaign.fresh_48h_seed_" + seed, () => Campaign(captured, 48, false));
                yield return new Scenario("campaign.established_72h_seed_" + seed, () => Campaign(captured, 72, true));
                yield return new Scenario("campaign.shared_personal_48h_seed_" + seed, () => Campaign(captured, 48, false, true));
            }
        }
        static World CampaignWorld(int seed, bool established, bool personal)
        {
            var w = World.Create(seed);
            if (personal)
                Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Personal acceptance camp" });
            if (established)
            {
                var v = w.Villages[0];
                v.ResearchPoints = 1000;
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                World.Add(v.Stockpiles[0].Goods, "logs", 100);
                var b = Station(w, v, "wood_cutter", new Int2(-4, 1));
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = v.Cats[20].Id, BuildingId = b.Id });
                var recipe = Catalog.Recipes.First(r => r.Building == b.Kind && r.Founding);
                Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe.Id, Repeat = true });
            }
            return w;
        }
        static void Campaign(int seed, int hours, bool established, bool personal = false)
        {
            var a = CampaignWorld(seed, established, personal);
            var b = CampaignWorld(seed, established, personal);
            var founding = a.Villages.ToDictionary(v => v.Id, v => v.Cats.Select(c => c.Id).ToArray());
            for (int hour = 0; hour < hours; hour++)
            {
                a.Step(3600);
                for (int minute = 0; minute < 60; minute++)
                    b.Step(60);
                Valid(a);
                Valid(b);
                Check(Fingerprint(a) == Fingerprint(b), "Partition/twin divergence seed=" + seed + " hour=" + (hour + 1));
                foreach (var v in a.Villages)
                {
                    if (v.Run != 1)
                        throw new InvalidOperationException("Accidental extinction reset village=" + v.Id + " seed=" + seed + " hour=" + (hour + 1) + CampaignState(v));
                    if (!founding[v.Id].All(id => v.Cats.Any(c => c.Id == id && c.Alive)))
                        throw new InvalidOperationException("Founding adult lost village=" + v.Id + " seed=" + seed + " hour=" + (hour + 1) + CampaignState(v));
                }
            }
        }
        static string CampaignState(Village v)
        {
            string StackText(IEnumerable<Stack> goods) => string.Join(",", goods.Select(s => s.Resource + "=" + s.Amount.ToString("F2", System.Globalization.CultureInfo.InvariantCulture)));
            return " piles=[" + string.Join(";", v.Stockpiles.Select(p => p.Id + "@" + p.Position.X + "," + p.Position.Z + ":" + StackText(p.Goods))) + "] cats=[" + string.Join(";", v.Cats.Where(c => !c.Alive || c.Health < 50 || c.Thirst < 15).Select(c => c.Id + " alive=" + c.Alive + " hp=" + c.Health + " thirst=" + c.Thirst + " hunger=" + c.Hunger + " pos=" + c.Position.X + "," + c.Position.Z + " goal=" + c.Goal + " blocked=" + c.BlockedReason + " path=" + c.Path.Count + " job=" + c.JobId + " cargo=" + StackText(c.Cargo))) + "] events=[" + string.Join(";", v.Events.Where(e => e.Kind == "death" || e.Kind == "recovery").Select(e => e.Time + ":" + e.Text)) + "]";
        }
    }
}
