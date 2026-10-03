using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Defense: raid consequences, equipment in combat and raid and merchant land movers.</summary>
    public static class DefenseScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.undefended_raid_has_consequences", RaidConsequences);
            yield return new Scenario("regression.functional_weapon_and_armor", Equipment);
            foreach (string mover in new[] { "merchant", "raid" })
                foreach (bool departing in new[] { false, true })
                {
                    string kind = mover;
                    bool returning = departing;
                    yield return new Scenario("regression.review_land_mover_" + kind + "_" + (returning ? "departing" : "arriving"), () => CachedLandMover(kind, returning));
                }
            yield return new Scenario("regression.review_land_mover_public_expansion_reroutes_merchant_and_raid", MerchantExpansionReroute);
            yield return new Scenario("defense.rally_sends_nearest_adults_without_direct_damage", RallySendsNearestAdults);
            yield return new Scenario("defense.captain_rallies_automatically", CaptainRallies);
            yield return new Scenario("defense.party_strength_drives_damage", PartyStrength);
            yield return new Scenario("defense.renewable_raid_trigger_needs_no_materials", RenewableTrigger);
            yield return new Scenario("defense.breach_casualty_dies_from_raid", BreachCasualty);
        }

        // ---- W2-09: rally, captain, parties and the renewable trigger (classic profile) ----

        static Raid HoldingRaid(World w, Village v, Int2 at, double health = 100000)
        {
            var raid = new Raid { Id = w.Id("held-raid"), Position = at, Health = health, Kind = "fox", Members = 3, MaxHealth = health, Strength = 30 };
            v.Raids.Add(raid);
            return raid;
        }

        static void RallySendsNearestAdults()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            foreach (var cat in v.Cats)
                cat.Skills.RemoveAll(s => s.Resource == "fight");
            var gate = new Int2(v.Center.X, v.Center.Z + v.Radius - 1);
            var raid = HoldingRaid(w, v, gate);
            double health = raid.Health;
            double minAge = w.CurrentPacing().WorkerMinAgeHours;
            var expected = v.Cats.Where(c => c.Alive && c.ControlledBy == "" && c.AgeHours >= minAge && c.Position.Level == 0 && c.Migration != "arriving" && c.Migration != "departing" && c.Health > 15 && !c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal) && !v.Routes.Any(r => r.CatId == c.Id))
                .OrderBy(c => Int2.Distance(c.Position, gate)).ThenBy(c => c.Id, StringComparer.Ordinal).Take(4).ToList();
            var result = Act(w, v, new GameAction { Kind = "DefendRaid" });
            Check(result.Code == "rallied" && raid.Health == health, "Rallying must not damage the raid directly: code=" + result.Code + " health=" + raid.Health);
            var rallied = v.Cats.Where(c => c.RalliedTo == raid.Id).ToList();
            Check(rallied.Count == 4 && expected.All(rallied.Contains), "The four nearest eligible adults must be rallied: " + string.Join(",", rallied.Select(c => c.Name)));
            Check(rallied.All(c => c.Goal == GoalKey.Defending && c.JobId == "" && c.BuildingId == "" && !v.Jobs.Any(j => !j.Completed && j.CatId == c.Id)), "Rallied cats must drop their work and defend");
            var again = w.Apply(Context(v), new GameAction { Kind = "DefendRaid" });
            Check(!again.Success && again.Code == "rally_cooldown", "A second rally inside the cooldown must be refused: " + again.Code);
            var start = rallied.ToDictionary(c => c, c => Int2.Distance(c.Position, gate));
            w.Step(4);
            Check(rallied.Count(c => Int2.Distance(c.Position, gate) < start[c] || Int2.Distance(c.Position, gate) <= 1) >= 3, "Rallied cats must physically move toward the raiders");
            w.Step(57);
            Check(w.Apply(Context(v), new GameAction { Kind = "DefendRaid" }).Success, "The rally must be available again after its 60 s cooldown");
            Valid(w);
        }

        static void CaptainRallies()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            var captain = v.Cats.First(c => c.Alive && c.OfficerRole == "");
            captain.OfficerRole = "captain";
            v.Officers.Add(new Officer { Role = "captain", CatId = captain.Id });
            var raid = HoldingRaid(w, v, new Int2(v.Center.X, v.Center.Z + v.Radius - 1));
            w.Step(1.5);
            Check(v.Cats.Count(c => c.RalliedTo == raid.Id) > 0, "A Captain must rally defenders against an active raid without a click");
        }

        static (double damage, double health) StrengthBattle(double strength)
        {
            var w = Fixture(out var v, out var c);
            c.Position = v.Center;
            c.X = 0;
            c.Z = 0;
            c.Skills.Add(new Stack("fight", 10));
            c.BuildingId = "";
            var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Strength = strength, Health = 1000, MaxHealth = 1000, Members = 3, Kind = "fox" };
            v.Raids.Add(raid);
            w.Step(30);
            return (1000 - raid.Health, 100 - c.Health);
        }

        static void PartyStrength()
        {
            var weak = StrengthBattle(30);
            var strong = StrengthBattle(60);
            // Health regeneration offsets both sides equally, so the doubled strength shows in the difference.
            double regen = 30 * World.Create(41).CurrentPacing().HealthRegenPerSecond;
            Check(weak.health > 0 && Math.Abs((strong.health + regen) / (weak.health + regen) - 2) < 0.05, "Raider strength must drive the injury it deals: weak=" + weak.health + " strong=" + strong.health);
            Check(Math.Abs(strong.damage - weak.damage) < 1e-6, "Raider strength must not change the defender's own blows");
        }

        static void RenewableTrigger()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            var pace = w.CurrentPacing();
            foreach (var pile in v.Stockpiles)
                pile.Goods.RemoveAll(s => s.Resource == "materials" || s.Resource == "refined");
            v.FoundedAt = -(pace.RaidGraceSeconds + 10);
            Check(v.Cats.Count(c => c.Alive) >= pace.RaidMinCats, "Fixture: the founding colony must be large enough to draw raiders");
            var spawn = typeof(World).GetMethod("MaybeSpawnRaid", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            for (int i = 0; i < 400 && v.Raids.Count == 0; i++)
                spawn.Invoke(w, new object[] { v });
            Check(v.Raids.Count == 1, "Game time and colony size alone must eventually draw a raid, without stored materials");
            var raid = v.Raids[0];
            Check(raid.Members >= 2 && raid.Strength > 0 && raid.MaxHealth == raid.Health, "Natural raids arrive as a party: members=" + raid.Members);
        }

        static void BreachCasualty()
        {
            var w = Fixture(out var v, out var c);
            c.Skills.RemoveAll(s => s.Resource == "fight");
            c.ControlledBy = "";
            c.Position = v.Center;
            c.X = v.Center.X;
            c.Z = v.Center.Z;
            c.Health = 5;
            var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Health = 1000 };
            v.Raids.Add(raid);
            w.Step(1);
            Check(!c.Alive && v.Events.Any(e => e.Kind == "death" && e.EntityId == c.Id && e.Text.Contains("raid")), "A cat killed in a classic raid breach must die from the raid, not from needs");
        }
        static void RaidConsequences()
        {
            var w = Fixture(out var v, out var c);
            World.Add(v.Stockpiles[0].Goods, "materials", 500);
            v.Raids.Add(new Raid { Id = w.Id("raid"), Position = new Int2(0, 5), Strength = 100, Health = 100 });
            var health = v.Cats.Sum(x => x.Health);
            w.Step(1800);
            Check(v.Raids.Count == 0 || v.Cats.Sum(x => x.Health) < health - 1 || Goods(w, v, "materials") < 500, "Undefended raid remains inert without consequences");
        }
        static (double damage, double health) Battle(bool equipped)
        {
            var w = Fixture(out var v, out var c);
            c.Position = v.Center;
            c.X = 0;
            c.Z = 0;
            c.Skills.Add(new Stack("fight", 10));
            c.BuildingId = "";
            var raid = new Raid { Id = w.Id("raid"), Position = v.Center, Strength = 50, Health = 1000 };
            v.Raids.Add(raid);
            foreach (var kind in new[] { "weapon", "armor" })
                v.Items.Add(new Item { Id = w.Id(kind), Kind = kind, Material = "metal", Quality = 4, VillageId = v.Id, LocationId = v.Stockpiles[0].Id });
            if (equipped)
                foreach (var item in v.Items.ToArray())
                    Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = item.Id });
            w.Step(30);
            return (1000 - raid.Health, c.Health);
        }
        static void Equipment()
        {
            var plain = Battle(false);
            var equipped = Battle(true);
            Check(equipped.damage > plain.damage * 1.1, "Weapon does not improve actual combat damage");
            Check(equipped.health > plain.health, "Armor does not reduce actual combat injury");
        }
        static void MerchantExpansionReroute()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            c.Skills.RemoveAll(s => s.Resource == "fight");
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            v.Coins = 10;
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            World.Add(v.Stockpiles[0].Goods, "planks", 10);
            World.Add(v.Stockpiles[0].Goods, "blocks", 5);
            for (int x = -13; x <= 13; x++)
                for (int z = -13; z <= 16; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || z >= 10)
                        tile.Wall = tile.Water = tile.Mountain = false;
                    if (z >= 10)
                    {
                        tile.Road = false;
                        tile.ClaimId = "";
                    }
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(0, 10)).Road = true;
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(0, 11), To = new Int2(0, 12) });
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(1, 12), To = new Int2(1, 13) });
            void Complete(GameAction action)
            {
                var id = Act(w, v, action).EntityId;
                var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < 1000 && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Public merchant fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(0, 11), End = new Int2(0, 15), CatId = c.Id });
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(1, 11), End = new Int2(1, 12), CatId = c.Id });
            var fieldId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(-2, 15), CatId = c.Id }).EntityId;
            var field = v.Buildings.Single(b => b.Id == fieldId);
            for (int tick = 0; tick < 1000 && !field.Completed; tick++)
                w.Step(1);
            Check(field.Completed, "Public merchant Field did not complete");
            Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(1, 10), End = new Int2(3, 10) });
            double materials = Goods(w, v, "materials");
            var expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId;
            var expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 / World.SimulationStepSeconds && expansion.PathIndex < expansion.Path.Count; tick++)
                w.Step(World.SimulationStepSeconds);
            var trader = v.Trader;
            Check(!expansion.Completed && expansion.PathIndex == expansion.Path.Count && w.TimeSeconds < trader.NextAt, "Merchant fixture did not reach the pre-cutover checkpoint before its natural visit");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            while (w.TimeSeconds < trader.NextAt)
            {
                Act(w, v, new GameAction { Kind = "KeepCatControl", CatId = c.Id });
                w.Step(Math.Min(20, trader.NextAt - w.TimeSeconds));
            }
            var from = new Int2(1, 10);
            var to = new Int2(0, 10);
            int crossing = trader.Path.FindIndex(p => p.Equals(from));
            Check(trader.Phase == "arriving" && trader.Position.Equals(new Int2(0, 12)) && crossing >= 0 && crossing + 1 < trader.Path.Count && trader.Path[crossing + 1].Equals(to), "Automatic merchant did not cache the future public farm-fence crossing: phase=" + trader.Phase + " position=" + trader.Position + " path=" + string.Join(";", trader.Path) + " wall=" + w.TileAt(new Int2(-1, 11)).Wall + " radius=" + v.Radius);
            string identity = trader.Id;
            long visits = v.TraderVisitCount;
            double coins = trader.Coins;
            var cargo = trader.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
            var originalPath = trader.Path;
            // Inject a finite active threat before the public cutover, using ordinary pathfinding.
            var raid = new Raid { Id = w.Id("farm-fence-raid"), Position = trader.Position, Path = w.Path(trader.Position, v.Center), Health = 100, Loot = new List<Stack> { new Stack("logs", 8) } };
            v.Raids.Add(raid);
            var raidPath = raid.Path;
            Check(raidPath != null && raidPath.Contains(from) && raidPath[raidPath.IndexOf(from) + 1].Equals(to), "Active raid did not cache the same future farm-fence edge");
            double food = Goods(w, v, "food");
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            w.Step(1);
            Check(expansion.Completed && v.Radius == 11 && w.Walkable(from) && w.Walkable(to) && !w.Crossable(from, to), "Public expansion did not close the cached walkable merchant edge");
            bool stopped = false, rerouted = false, raidRerouted = false;
            for (int tick = 0; tick < 100 / World.SimulationStepSeconds && (trader.Phase != "trading" || raid.Phase != "departing"); tick++)
            {
                var before = trader.Position;
                var raidBefore = raid.Position;
                double progress = trader.Progress, x = trader.X, z = trader.Z;
                w.Step(World.SimulationStepSeconds);
                Check(Math.Sqrt((trader.X - x) * (trader.X - x) + (trader.Z - z) * (trader.Z - z)) <= 0.8 * World.SimulationStepSeconds + 1e-8, "Merchant rerouting teleported its authoritative position");
                Check(before.Equals(trader.Position) || Int2.Distance(before, trader.Position) == 1 && w.Crossable(before, trader.Position), "Automatic merchant crossed the completed public farm fence");
                Check(raidBefore.Equals(raid.Position) || Int2.Distance(raidBefore, raid.Position) == 1 && w.Crossable(raidBefore, raid.Position), "Active raid crossed the completed public farm fence");
                if (before.Equals(from) && before.Equals(trader.Position) && trader.BlockedReason == "blocked_route")
                {
                    stopped = true;
                    Check(trader.Progress <= progress && progress - trader.Progress <= 0.8 * World.SimulationStepSeconds + 1e-8, "Blocked merchant advanced across the closed edge or teleported during retreat");
                }
                rerouted |= !ReferenceEquals(originalPath, trader.Path);
                raidRerouted |= !ReferenceEquals(raidPath, raid.Path);
                Check(v.Raids.Contains(raid), "Farm fence removed the active raid identity");
                Near(World.Amount(raid.Loot, "logs"), 8, "Farm-fence raid reroute changed its finite loot");
                Check(trader.Id == identity && v.TraderVisitCount == visits && trader.Coins == coins && trader.Goods.Count == cargo.Length, "Merchant reroute replaced its visit, purse or finite cargo");
                foreach (var stack in cargo)
                    Near(World.Amount(trader.Goods, stack.Resource), stack.Amount, "Public fence changed merchant cargo");
            }
            Check(stopped && rerouted && trader.Phase == "trading" && trader.Position.Equals(v.Center) && trader.Exterior.Value.Equals(new Int2(0, 12)), "Blocked merchant did not lawfully reroute the same visit to its shrine");
            Near(Goods(w, v, "materials"), materials - expansion.Path.Count, "Merchant interruption changed the public expansion bill");
            Check(raidRerouted && raid.Phase == "departing" && raid.Position.Equals(v.Center) && v.Events.Count(e => e.EntityId == raid.Id && e.Kind == "raid_breach") == 1, "Active raid did not lawfully reroute to one physical shrine breach");
            Near(Goods(w, v, "food"), food - 20, "Rerouted public-fence raid did not debit exactly one finite theft");
            Near(World.Amount(raid.Loot, "food"), 20, "Rerouted public-fence raid did not retain the stolen food");
            double logs = Goods(w, v, "logs"), stock = World.Amount(trader.Goods, "logs");
            Act(w, v, new GameAction { Kind = "BuyResource", Resource = "logs", Amount = 1 });
            Near(Goods(w, v, "logs"), logs + 1, "Recovered merchant could not supply a public purchase");
            Near(World.Amount(trader.Goods, "logs"), stock - 1, "Recovered merchant purchase did not debit finite stock");
            Near(v.Coins, 8, "Recovered merchant purchase charged the wrong coin");
            Near(trader.Coins, coins + 2, "Recovered merchant purchase did not credit its original purse");
            Valid(w);
        }
        static void CachedLandMover(string kind, bool departing)
        {
            var w = Fixture(out var v, out var c);
            c.ControlledBy = "fixture-held";
            c.ControlLeaseUntil = 10000000;
            var start = new Int2(-6, 1);
            var exit = new Int2(0, v.Radius + 3);
            var destination = departing ? exit : v.Center;
            foreach (var p in new[] { new Int2(-7, 1), new Int2(-6, 0), new Int2(-6, 2) })
                w.TileAt(p).Wall = true;
            var path = w.Path(start, destination);
            Check(path != null && path.Count > 1 && path[0].Equals(new Int2(-5, 1)), "Land mover fixture did not cache its only physical exit");
            var edge = new BoundaryEdge { From = start, To = path[0] };
            double food = Goods(w, v, "food");
            var trader = v.Trader;
            var item = new Item { Id = w.Id("merchant-tool"), Kind = "tool", LocationId = "trader", Quality = 3, Condition = 57, MaxCondition = 88 };
            var raid = new Raid { Id = w.Id("cached-raid"), Phase = departing ? "departing" : "approaching", Position = start, Path = new List<Int2>(path), Health = 100, Loot = new List<Stack> { new Stack("logs", 8) } };
            if (kind == "merchant")
            {
                trader.Id = w.Id("cached-merchant");
                trader.Phase = departing ? "departing" : "arriving";
                trader.Position = start;
                trader.Exterior = exit;
                trader.VisitDestination = v.Center;
                trader.Path = new List<Int2>(path);
                trader.Goods.Add(new Stack("logs", 8));
                trader.Items.Add(item);
                trader.Coins = 17;
            }
            else
                v.Raids.Add(raid);
            w.Step(1);
            Check((kind == "merchant" ? trader.Position : raid.Position).Equals(start), "Land mover advanced before its movement budget was ready");
            v.BoundaryEdges.Add(edge);
            string identity = kind == "merchant" ? trader.Id : raid.Id;
            // A closed edge requires a bounded physical retreat from its fractional position.
            for (int tick = 0; tick < 30; tick++)
            {
                double x = kind == "merchant" ? trader.X : raid.X, z = kind == "merchant" ? trader.Z : raid.Z;
                w.Step(0.1);
                double dx = (kind == "merchant" ? trader.X : raid.X) - x, dz = (kind == "merchant" ? trader.Z : raid.Z) - z;
                Check(Math.Sqrt(dx * dx + dz * dz) <= (kind == "merchant" ? 0.08 : 0.05) + 1e-8, "Blocked mover teleported during its return to the last safe anchor");
            }
            Check((kind == "merchant" ? trader.Position : raid.Position).Equals(start), "Cached " + kind + " " + (departing ? "departure" : "arrival") + " crossed a newly blocked edge");
            if (kind == "merchant")
            {
                Check(trader.BlockedReason == "blocked_route" && trader.Progress == 0 && trader.X == start.X && trader.Z == start.Z && trader.PathIndex == 0 && trader.Phase == (departing ? "departing" : "arriving") && trader.Id == identity, "Blocked merchant did not stop at its safe anchor with the same phase and visit identity");
                Check(trader.Items.Count == 1 && trader.Items[0] == item && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Blocked merchant changed exact cargo identity or condition");
                Near(World.Amount(trader.Goods, "logs"), 8, "Blocked merchant lost its finite goods");
                Near(trader.Coins, 17, "Blocked merchant changed its purse");
                Check(trader.LastDepartedAt < 0, "Blocked merchant completed its departure");
            }
            else
            {
                Check(v.Raids.Single().Id == identity && raid.Phase == (departing ? "departing" : "approaching"), "Blocked raid changed identity or phase");
                Near(World.Amount(raid.Loot, "logs"), 8, "Blocked raid lost finite loot");
                Near(Goods(w, v, "food"), food, "Blocked raid looted through the edge");
            }
            v.BoundaryEdges.Remove(edge);
            bool Finished() => kind == "merchant" ? trader.Phase == (departing ? "absent" : "trading") : departing ? !v.Raids.Contains(raid) : raid.Phase == "departing";
            for (int tick = 0; tick < 100 && !Finished(); tick++)
            {
                var before = kind == "merchant" ? trader.Position : raid.Position;
                w.Step(1);
                var after = kind == "merchant" ? trader.Position : raid.Position;
                Check(before.Equals(after) || Int2.Distance(before, after) == 1 && w.Crossable(before, after), "Recovered land mover skipped a physical edge");
            }
            Check(Finished(), "Cleared edge did not resume the original " + kind + " journey");
            if (kind == "merchant")
            {
                Check(trader.Id == identity && trader.Position.Equals(destination), "Recovered merchant changed visit identity or missed its destination");
                Near(World.Amount(trader.Goods, "logs"), 8, "Recovered merchant changed finite goods");
                Near(trader.Coins, 17, "Recovered merchant changed its purse");
                if (departing)
                    Check(trader.Items.Count == 0 && trader.LastDepartedAt > 4 && trader.NextAt > w.TimeSeconds && !v.Items.Any(i => i.Id == item.Id), "Departing merchant did not remove its external cargo once at the exit");
                else
                    Check(trader.Items.Single() == item && item.Condition == 57 && trader.Until > w.TimeSeconds, "Arriving merchant did not retain its exact cargo for trade");
            }
            else if (departing)
                Check(v.Events.Count(e => e.EntityId == identity && e.Kind == "raid_loss") == 1 && Goods(w, v, "logs") == 0, "Escaping raid duplicated its finite stolen cargo or departure");
            else
            {
                Near(Goods(w, v, "food"), food - 20, "Recovered raid did not debit one finite shrine theft");
                Near(World.Amount(raid.Loot, "food"), 20, "Recovered raid did not retain the stolen goods");
                Check(v.Events.Count(e => e.EntityId == identity && e.Kind == "raid_breach") == 1, "Recovered raid looted more than once");
            }
            Valid(w);
        }
    }
}
