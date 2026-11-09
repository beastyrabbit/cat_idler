using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;
using SimStack = IdleCatForest.Simulation.Stack;

public static class PrimaryQueueScenarios
{
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("hourly primary queue matches uninterrupted Farmer handback after durable restart", () =>
        {
            var uninterrupted = World.Create(41);
            var village = uninterrupted.Villages.Single();
            var player = new PlayerContext { PlayerId = "synthetic-primary-queue", VillageId = village.Id };
            void Act(GameAction action)
            {
                var result = uninterrupted.Apply(player, action);
                check(result.Success, action.Kind + "/" + action.Edit + " rejected: " + result.Error);
            }

            // Follow the renewable Grain fixture, but fund just the next recipe
            // decision. Keep the ordinary colony, needs, beds, terrain and clock rate.
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.Clear();
            village.LastLeaderResearch = 0;
            village.Stockpiles.Add(new Stockpile
            {
                Id = uninterrupted.Id("primary-queue-provisions"),
                Position = new Int2(3, 2),
                Width = 1,
                Depth = 1,
                Capacity = 1500,
                Accepts = new List<string> { "preserves", "water", "fibre" },
                Goods = new List<SimStack> { new SimStack("preserves", 600), new SimStack("water", 600), new SimStack("fibre", 10) }
            });
            var grain = new Stockpile
            {
                Id = uninterrupted.Id("primary-queue-grain"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Capacity = 100,
                Accepts = new List<string> { "grain" },
                Goods = new List<SimStack> { new SimStack("grain", 4) }
            };
            var flour = new Stockpile
            {
                Id = uninterrupted.Id("primary-queue-flour"),
                Position = new Int2(3, -3),
                Width = 1,
                Depth = 1,
                Capacity = 64,
                Accepts = new List<string> { "flour" }
            };
            village.Stockpiles.AddRange(new[] { grain, flour });
            string[] studies =
            {
                "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers",
                "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation"
            };
            village.ResearchPoints = studies.Sum(id => Catalog.Study(id).Cost);
            foreach (string study in studies) Act(new GameAction { Kind = "ResearchNode", NodeId = study });
            check(Math.Abs(village.ResearchPoints) < 1e-8, "fixture did not spend its exact legal research budget");
            village.Buildings.Add(new Building { Id = uninterrupted.Id("primary-queue-field"), Kind = "field", Position = new Int2(6, 4), Completed = true });
            var mill = new Building { Id = uninterrupted.Id("primary-queue-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            village.Buildings.Add(mill);
            var worker = village.Cats[3];
            Act(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[1].Id });
            Act(new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = worker.Id });
            void Edit(string edit, bool enabled = true) => Act(new GameAction
            {
                Kind = "EditProductionWorkSlot",
                BuildingId = mill.Id,
                CatId = worker.Id,
                Edit = edit,
                Enabled = enabled
            });
            var slot = mill.Slots.Single();
            Edit("pause");
            Edit("delegate");
            check(slot.Paused && slot.AutomatedBy == "farmer", "public setup did not retain the paused Farmer position");
            Edit("pause", false); // The UI's Resume action takes manual control.
            check(!slot.Paused && slot.AutomatedBy == "", "public Resume did not return the same position to manual control");
            Edit("delegate");

            var recipe = Catalog.Recipe("grain_to_flour");
            check(!player.IsDeveloper && uninterrupted.Players.Single().Id == player.PlayerId
                && village.Cats.Count == 30 && village.Cats.All(c => c.Alive && c.ControlledBy == "" && c.Cargo.Count == 0
                    && c.Hunger == 100 && c.Thirst == 100 && c.Rest == 100 && c.Health == 100),
                "fixture lost its ordinary player or healthy, uncontrolled founding colony");
            check(worker.BuildingId == mill.Id && mill.WorkerId == worker.Id && slot.CatId == worker.Id
                && slot.AutomatedBy == "farmer" && !slot.Paused && !mill.Paused && slot.Queue.Count == 0 && mill.Queue.Count == 0,
                "public handback changed the worker, pause or empty orders before the future Farmer choice");
            check(Catalog.RecipeAvailable(village, recipe) && recipe.Inputs.Single().Resource == "grain"
                && uninterrupted.RecipeInput(village, recipe, recipe.Inputs.Single().Amount) == 4
                && World.Amount(grain.Goods, "grain") == 4 && uninterrupted.Available(village, "grain") == 4
                && uninterrupted.Total(village, "flour") == 0,
                "future Farmer recipe is locked, unfunded or already credited");
            check(uninterrupted.Path(worker.Position, grain.Position, village) != null
                && uninterrupted.Path(grain.Position, mill.Position, village) != null
                && uninterrupted.Path(mill.Position, flour.Position, village) != null
                && uninterrupted.HasRoom(village, flour, "flour", uninterrupted.RecipeOutput(village, recipe, recipe.Outputs.Single().Amount)),
                "funded recipe lacks a reachable exact Grain pile or accepting Flour destination");
            check(village.Jobs.Count == 0 && uninterrupted.Reservations.Count == 0, "fixture unexpectedly started or claimed work");

            // Anchor a fresh fixture one second before the normal ten-second
            // planning boundary. No warmup, accelerated work or post-action clone.
            uninterrupted.TimeSeconds = 9;
            AuthorityRuntime.ValidateWorld(uninterrupted);
            string before = WireJson.Encode(uninterrupted);
            string directory = Path.Combine(Path.GetTempPath(), "forest-primary-queue-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(directory);
            try
            {
                string path = Path.Combine(directory, "world.json");
                SaveStore.Save(path, uninterrupted, requireNew: true);
                var restored = SaveStore.Load<World>(path);
                check(WireJson.Encode(uninterrupted) == before && WireJson.Encode(restored) == before,
                    "durable twins differ before the future Farmer choice");
                AuthorityRuntime.ValidateWorld(restored);

                uninterrupted.Step(1);
                restored.Step(1);
                foreach (var twin in new[] { uninterrupted, restored })
                {
                    var v = twin.Village(village.Id);
                    var b = v.Buildings.Single(candidate => candidate.Id == mill.Id);
                    var s = b.Slots.Single();
                    check(twin.TimeSeconds == 10 && twin.SimulationTimeSeconds == 10 && s.CatId == worker.Id
                        && b.WorkerId == worker.Id && twin.Cat(worker.Id).BuildingId == b.Id
                        && s.AutomatedBy == "farmer" && !s.Paused && !b.Paused,
                        "one normal decision changed the retained worker or Farmer handback");
                    check(s.Queue.Count == 1 && s.Queue[0].RecipeId == "grain_to_flour" && s.Queue[0].AutomatedBy == "farmer"
                        && !s.Queue[0].Repeat && v.Jobs.Count == 0 && twin.Reservations.Count == 0
                        && World.Amount(v.Stockpiles.Single(p => p.Id == grain.Id).Goods, "grain") == 4,
                        "one normal decision did not populate the funded real slot queue before work starts");
                    AuthorityRuntime.ValidateWorld(twin);
                }

                string continuousJson = WireJson.Encode(uninterrupted), restoredJson = WireJson.Encode(restored);
                string mirrorPath = "$.Villages[0].Buildings[" + village.Buildings.IndexOf(mill) + "].Queue";
                // Read-only diagnostics. Neither serialized world nor any real
                // queue is replaced, omitted, sorted or normalized for comparison.
                var differences = Differences(JToken.Parse(continuousJson), JToken.Parse(restoredJson), "$").ToArray();
                check(differences.All(p => p == mirrorPath || p.StartsWith(mirrorPath + "[", StringComparison.Ordinal)),
                    "unexpected authority drift beyond the legacy primary Building.Queue: " + string.Join(", ", differences));
                check(continuousJson == restoredJson,
                    "legacy primary Building.Queue alias drift after Farmer handback: " + string.Join(", ", differences)
                    + "; uninterrupted=" + WireJson.Encode(mill.Queue)
                    + "; restored=" + WireJson.Encode(restored.Village(village.Id).Buildings.Single(b => b.Id == mill.Id).Queue));
            }
            finally
            {
                // Delete only the uniquely owned disposable test directory.
                Directory.Delete(directory, recursive: true);
            }
        });
    }

    static IEnumerable<string> Differences(JToken left, JToken right, string path)
    {
        if (JToken.DeepEquals(left, right)) yield break;
        if (left is JObject a && right is JObject b)
        {
            foreach (string name in a.Properties().Select(p => p.Name).Union(b.Properties().Select(p => p.Name)))
                foreach (string difference in Differences(a[name], b[name], path + "." + name))
                    yield return difference;
        }
        else if (left is JArray x && right is JArray y && x.Count == y.Count)
        {
            for (int i = 0; i < x.Count; i++)
                foreach (string difference in Differences(x[i], y[i], path + "[" + i + "]"))
                    yield return difference;
        }
        else yield return path;
    }
}
