using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Needs: drinking, eating and processed needs, including needs that interrupt carried cargo; staggered
    /// per-cat needs, servings taken at the source after a short dwell, sleeping beside the den, open water,
    /// spilled food, blocked trips with backoff and the delivery leg before a critical need.
    /// </summary>
    public static class NeedsScenarios
    {
        static readonly string[] Profiles = { Pacing.ClassicProfile, Pacing.DemoProfile };

        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.cargo_critical_need_conservation", CargoNeed);
            yield return new Scenario("regression.preserved_food_is_edible", PreservedFood);
            yield return new Scenario("regression.emergency_staffed_water", EmergencyWater);
            yield return new Scenario("regression.medicine_and_brew_consumption", ProcessedNeeds);
            yield return new Scenario("needs.demo_needs_staggered", DemoStaggered);
            yield return new Scenario("needs.classic_needs_not_reseeded", ClassicNotReseeded);
            yield return new Scenario("needs.busy_with_need_and_rest_rate", BusyAndRestRate);
            foreach (var profile in Profiles)
            {
                string p = profile;
                yield return new Scenario("needs.drink_dwells_at_source." + p, () => DwellAtSource(p, GoalKey.NeedDrink));
                yield return new Scenario("needs.eat_dwells_at_source." + p, () => DwellAtSource(p, GoalKey.NeedEat));
                yield return new Scenario("needs.route_avoids_shrine." + p, () => RouteAvoidsShrine(p));
                yield return new Scenario("needs.sleep_beside_den." + p, () => SleepBesideDen(p));
                yield return new Scenario("needs.drink_at_water_edge." + p, () => DrinkAtWaterEdge(p));
                yield return new Scenario("needs.eat_from_spill." + p, () => EatFromLoosePile(p, "spill"));
                yield return new Scenario("needs.eat_from_farm_output." + p, () => EatFromLoosePile(p, "farm_output"));
                yield return new Scenario("needs.blocked_trip_backs_off." + p, () => BlockedTripBacksOff(p));
                yield return new Scenario("needs.short_delivery_finishes_first." + p, () => DeliveryBeforeNeed(p, false));
                yield return new Scenario("needs.long_delivery_deposits_in_storage." + p, () => DeliveryBeforeNeed(p, true));
                yield return new Scenario("needs.night_raises_sleep_threshold." + p, () => NightSleep(p));
            }
        }

        static World ProfileFixture(string profile, out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            w.PacingProfile = profile;
            return w;
        }

        static void NoWater(World w)
        {
            foreach (var t in w.Tiles.Where(t => t.Water))
                t.Water = false;
        }

        static IEnumerable<Int2> Neighbors(Int2 p)
        {
            yield return new Int2(p.X, p.Z + 1, p.Level);
            yield return new Int2(p.X + 1, p.Z, p.Level);
            yield return new Int2(p.X, p.Z - 1, p.Level);
            yield return new Int2(p.X - 1, p.Z, p.Level);
        }

        static bool Beside(Int2 at, int width, int depth, Int2 p) => !LivelinessScenarios.Inside(at, width, depth, p) && Neighbors(p).Any(n => LivelinessScenarios.Inside(at, width, depth, n));

        /// <summary>
        /// Demo worlds seed every fresh cat's needs, metabolism and thresholds from its id, so need trips
        /// start spread out: never more than five of thirty in any ten-second window.
        /// </summary>
        static void DemoStaggered()
        {
            var w = LivelinessScenarios.ProfileWorld(Pacing.DemoProfile, out var v);
            var pace = w.CurrentPacing();
            w.Step(World.SimulationStepSeconds);
            var cats = v.Cats.Where(c => c.Alive).ToList();
            Check(cats.Count == 30 && cats.All(c => c.LifeSeeded), "Demo cats were not seeded on their first tick");
            Check(cats.All(c => c.Thirst >= 54.9 && c.Thirst <= 100 && c.Hunger >= 54.9 && c.Hunger <= 100 && c.Rest >= 54.9 && c.Rest <= 100), "Seeded needs left the 55 to 100 range");
            Check(cats.Select(c => Math.Round(c.Thirst, 2)).Distinct().Count() >= 25 && cats.Select(c => Math.Round(c.Hunger, 2)).Distinct().Count() >= 25, "Seeded needs are not per cat: thirst " + string.Join(",", cats.Select(c => c.Thirst.ToString("F2", System.Globalization.CultureInfo.InvariantCulture))));
            Check(cats.All(c => c.Metabolism >= pace.MetabolismMin && c.Metabolism <= pace.MetabolismMax) && cats.Select(c => c.Metabolism).Distinct().Count() >= 25, "Metabolism is not seeded per cat within its range");
            var thresholds = cats.Select(c => w.NeedThreshold(c, GoalKey.NeedDrink)).ToList();
            Check(thresholds.All(t => Math.Abs(t - pace.DrinkThreshold) <= pace.NeedThresholdJitter + 1e-9) && thresholds.Distinct().Count() >= 25, "Drink thresholds are not jittered per cat");
            double thirst = cats[0].Thirst;
            w.Step(World.SimulationStepSeconds);
            Check(cats[0].Thirst < thirst && cats[0].Thirst > thirst - 1, "A seeded cat was seeded again");

            // Run at least demo-like decay (240 thirst and 160 hunger per hour), whatever this profile's values are.
            double extraThirst = Math.Max(0, 240 - pace.ThirstDecayPerHour) / 3600, extraHunger = Math.Max(0, 160 - pace.HungerDecayPerHour) / 3600;
            var starts = new List<double>();
            var wasBusy = cats.ToDictionary(c => c.Id, World.BusyWithNeed);
            for (int second = 0; second < 600; second++)
            {
                foreach (var c in cats)
                {
                    c.Thirst = Math.Max(0, c.Thirst - extraThirst * c.Metabolism);
                    c.Hunger = Math.Max(0, c.Hunger - extraHunger * c.Metabolism);
                }
                for (int tick = 0; tick < 20; tick++)
                {
                    w.Step(World.SimulationStepSeconds);
                    foreach (var c in cats)
                    {
                        bool busy = World.BusyWithNeed(c);
                        if (busy && !wasBusy[c.Id])
                            starts.Add(w.TimeSeconds);
                        wasBusy[c.Id] = busy;
                    }
                }
            }
            int worst = starts.Select(t => starts.Count(o => o >= t && o < t + 10)).DefaultIfEmpty(0).Max();
            Console.WriteLine("needs.stagger starts=" + starts.Count + " worst_10s_window=" + worst);
            Check(starts.Count >= 15, "Too few need trips to judge the stagger: " + starts.Count);
            Check(worst <= 5, worst + " cats started a need trip within one ten-second window");
            Check(cats.All(c => c.Alive), "A cat died during the stagger run");
            Valid(w);
        }

        /// <summary>Classic worlds keep every need, threshold and decay exactly as before.</summary>
        static void ClassicNotReseeded()
        {
            var w = LivelinessScenarios.ProfileWorld(Pacing.ClassicProfile, out var v);
            var pace = w.CurrentPacing();
            w.Step(World.SimulationStepSeconds);
            foreach (var c in v.Cats)
            {
                Check(!c.LifeSeeded && c.Metabolism == 1, "Classic cat was seeded");
                Near(c.Thirst, 100 - World.SimulationStepSeconds * pace.ThirstDecayPerHour / 3600, "Classic thirst changed");
                Near(c.Hunger, 100 - World.SimulationStepSeconds * pace.HungerDecayPerHour / 3600, "Classic hunger changed");
                Near(w.NeedThreshold(c, GoalKey.NeedDrink), pace.DrinkThreshold, "Classic drink threshold jittered");
                Near(w.NeedThreshold(c, GoalKey.NeedEat), pace.EatThreshold, "Classic eat threshold jittered");
                Near(w.NeedThreshold(c, GoalKey.NeedSleep), pace.SleepThreshold, "Classic daytime sleep threshold changed");
            }
        }

        static void BusyAndRestRate()
        {
            var w = Fixture(out var v, out var c);
            foreach (var goal in new[] { GoalKey.NeedDrink, GoalKey.NeedEat, GoalKey.NeedHeal, GoalKey.NeedSleep })
            {
                c.Goal = goal;
                Check(World.BusyWithNeed(c), goal + " is not busy with a need");
            }
            foreach (var goal in new[] { GoalKey.Idle, GoalKey.IdleNap, GoalKey.Research, "hunt · working", GoalKey.PlayerRest })
            {
                c.Goal = goal;
                Check(!World.BusyWithNeed(c), goal + " counted as a need");
            }
            Check(!World.BusyWithNeed(null), "A missing cat counted as busy");
            var pace = w.CurrentPacing();
            Near(w.RestRate(v), pace.SleepRecoveryPerSecond, "Rest rate without a Nest Hall");
        }

        /// <summary>
        /// A thirsty or hungry cat walks beside the store, lingers NeedDwellMin..Max seconds, then takes
        /// exactly one serving there; it never stands inside a footprint on the way.
        /// </summary>
        static void DwellAtSource(string profile, string need)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var pace = w.CurrentPacing();
            var pile = v.Stockpiles[0];
            string resource = need == GoalKey.NeedDrink ? "water" : "food";
            if (need == GoalKey.NeedDrink)
                c.Thirst = 1;
            else
                c.Hunger = 1;
            double Level() => need == GoalKey.NeedDrink ? c.Thirst : c.Hunger;
            double stored = World.Amount(pile.Goods, resource), arrivedAt = -1, servedAt = -1, last = Level();
            Int2 servedAtTile = default;
            for (int tick = 0; tick < 1200 && servedAt < 0; tick++)
            {
                w.Step(World.SimulationStepSeconds);
                Check(!LivelinessScenarios.InFootprint(v, c.Position), "Need trip entered a footprint at " + c.Position);
                if (arrivedAt < 0 && c.Goal == need && c.HasNeedSpot && c.Position.Equals(c.NeedSpot) && LivelinessScenarios.AtCentre(c))
                {
                    arrivedAt = w.TimeSeconds;
                    Near(World.Amount(pile.Goods, resource), stored, "The serving was taken before the dwell");
                }
                if (Level() > last + 10)
                {
                    servedAt = w.TimeSeconds;
                    servedAtTile = c.Position;
                }
                last = Level();
            }
            Check(arrivedAt >= 0 && servedAt >= 0, "The " + resource + " trip never arrived or was never served");
            double dwell = servedAt - arrivedAt;
            Check(dwell >= pace.NeedDwellMin - 1e-6 && dwell <= pace.NeedDwellMax + World.SimulationStepSeconds + 1e-6, "Dwell before the serving was " + dwell + " s");
            Check(Beside(pile.Position, pile.Width, pile.Depth, servedAtTile), "Served at " + servedAtTile + ", not beside the store");
            Check(!servedAtTile.Equals(v.Center), "Served at the village centre");
            Near(World.Amount(pile.Goods, resource), stored - pace.NeedServing, "The trip did not take exactly one serving from the store");
            Check(!World.BusyWithNeed(c) && c.Cargo.Count == 0, "The cat did not finish its trip empty-pawed");
            Valid(w);
        }

        /// <summary>A store on the far side of the shrine is reached around it, not across it.</summary>
        static void RouteAvoidsShrine(string profile)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var pile = v.Stockpiles[0];
            pile.Position = new Int2(0, 3);
            c.Position = new Int2(0, -2);
            c.X = c.Position.X;
            c.Z = c.Position.Z;
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            var direct = w.Path(c.Position, new Int2(0, 2), v);
            Check(direct != null && direct.Any(p => LivelinessScenarios.Inside(shrine.Position, shrine.Width, shrine.Depth, p)), "Fixture no longer tempts a route across the shrine");
            c.Thirst = 1;
            for (int tick = 0; tick < 1200 && c.Thirst < 30; tick++)
            {
                w.Step(World.SimulationStepSeconds);
                Check(!LivelinessScenarios.InFootprint(v, c.Position), "Need trip crossed a footprint at " + c.Position);
            }
            Check(c.Thirst > 30, "The cat never drank from the store beyond the shrine");
        }

        /// <summary>Five bed-mates sleep on the den's doorstep and the free tiles beside it, never inside, at most two per tile.</summary>
        static void SleepBesideDen(string profile)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var den = v.Buildings.Single(b => b.Id == c.BedId);
            var sleepers = v.Cats.Where(o => o.BedId == den.Id).ToList();
            Check(sleepers.Count == 5, "Fixture den does not hold five cats");
            foreach (var o in sleepers)
            {
                o.ControlledBy = "";
                o.ControlLeaseUntil = 0;
                o.BuildingId = "fixture-held";
                o.Rest = 1;
            }
            var rested = new HashSet<string>();
            var restBefore = sleepers.ToDictionary(o => o.Id, o => o.Rest);
            for (int second = 0; second < 150; second++)
            {
                w.Step(1);
                var tiles = new Dictionary<Int2, int>();
                foreach (var o in sleepers)
                {
                    Check(!LivelinessScenarios.InFootprint(v, o.Position), o.Name + " walked into a footprint at " + o.Position);
                    if (o.Goal != GoalKey.NeedSleep || o.Path.Count > 0 || !LivelinessScenarios.AtCentre(o))
                        continue;
                    if (o.Rest > restBefore[o.Id] + 1e-9)
                    {
                        rested.Add(o.Id);
                        Check(o.Position.Equals(den.Entrance) || Beside(den.Position, den.Width, den.Depth, o.Position), o.Name + " slept at " + o.Position + ", not at the den");
                        tiles[o.Position] = tiles.TryGetValue(o.Position, out int n) ? n + 1 : 1;
                    }
                    restBefore[o.Id] = o.Rest;
                }
                Check(tiles.All(e => e.Value <= 2), "More than two cats slept on one tile");
            }
            Check(rested.Count == 5, "Only " + rested.Count + " of five bed-mates slept");
            Check(sleepers.Select(o => o.NeedSpot).Distinct().Count() >= 3, "Sleepers piled onto too few tiles");
        }

        /// <summary>With no water stored, a thirsty cat drinks at the pond's edge; stored water is untouched.</summary>
        static void DrinkAtWaterEdge(string profile)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water");
            c.Thirst = 1;
            double last = c.Thirst;
            Int2 drankAt = default;
            bool drank = false;
            for (int tick = 0; tick < 2400 && !drank; tick++)
            {
                w.Step(World.SimulationStepSeconds);
                Check(!LivelinessScenarios.InFootprint(v, c.Position), "Water trip entered a footprint at " + c.Position);
                if (c.Thirst > last + 10)
                {
                    drank = true;
                    drankAt = c.Position;
                }
                last = c.Thirst;
            }
            Check(drank, "A thirsty cat with no stored water never drank at known open water");
            Check(!w.TileAt(drankAt).Water && Neighbors(drankAt).Any(n => w.GetTile(n)?.Water == true), "Drank at " + drankAt + ", not at a water edge");
            Near(Goods(w, v, "water"), 0, "Drinking at the pond created or consumed stored water");
            Check(c.Thirst > 35 && c.Alive, "Pond drink did not restore thirst");
            Valid(w);
        }

        /// <summary>With no food stored, a hungry cat eats from a spill or a farm's harvest pile.</summary>
        static void EatFromLoosePile(string profile, string kind)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "food");
            var loose = new Stockpile { Id = w.Id("loose-food"), Kind = kind, Position = new Int2(5, 5), Width = 1, Depth = 1, Capacity = 100 };
            World.Add(loose.Goods, "food", 3);
            v.Stockpiles.Add(loose);
            c.Hunger = 1;
            w.Step(60);
            Check(c.Hunger > 35, "A hungry cat ignored food in a " + kind + " pile");
            // Spoilage nibbles at loose food every upkeep tick.
            Near(World.Amount(loose.Goods, "food"), 2, "Eating did not take one serving from the " + kind + " pile", 0.01);
            Valid(w);
        }

        /// <summary>
        /// A need trip whose route closes ends after NeedTripBackoffSeconds instead of waiting forever,
        /// and each further failure doubles the delay before the next attempt.
        /// </summary>
        static void BlockedTripBacksOff(string profile)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var pace = w.CurrentPacing();
            NoWater(w);
            var pile = v.Stockpiles[0];
            c.Thirst = 1;
            for (int tick = 0; tick < 100 && !(c.Goal == GoalKey.NeedDrink && c.HasNeedSpot && c.Path.Count > 0); tick++)
                w.Step(World.SimulationStepSeconds);
            Check(c.Goal == GoalKey.NeedDrink && c.HasNeedSpot, "The drink trip never started");
            // Wall the store in: its tiles and every tile beside it.
            var walled = new List<Tile>();
            for (int x = pile.Position.X - 1; x <= pile.Position.X + pile.Width; x++)
                for (int z = pile.Position.Z - 1; z <= pile.Position.Z + pile.Depth; z++)
                {
                    var t = w.TileAt(new Int2(x, z));
                    if (!t.Wall && !new Int2(x, z).Equals(c.Position))
                    {
                        t.Wall = true;
                        walled.Add(t);
                    }
                }
            double blockedAt = w.TimeSeconds;
            for (int tick = 0; tick < 200 && World.BusyWithNeed(c); tick++)
                w.Step(World.SimulationStepSeconds);
            double abortedAt = w.TimeSeconds;
            Check(!World.BusyWithNeed(c), "A blocked need trip never gave up");
            Check(abortedAt - blockedAt <= pace.NeedTripBackoffSeconds + 1 + 1e-6, "A blocked need trip waited " + (abortedAt - blockedAt) + " s");
            double firstDelay = c.NextNeedAttemptAt - Math.Floor(abortedAt);
            Near(firstDelay, pace.NeedTripBackoffSeconds, "First retry delay");
            // Next attempt: still no reachable supply, so the delay doubles.
            for (int tick = 0; tick < 400 && !World.BusyWithNeed(c); tick++)
                w.Step(World.SimulationStepSeconds);
            Check(World.BusyWithNeed(c) && w.TimeSeconds + 1e-6 >= Math.Floor(abortedAt) + firstDelay, "The retry did not wait for its delay");
            for (int tick = 0; tick < 200 && World.BusyWithNeed(c); tick++)
                w.Step(World.SimulationStepSeconds);
            double secondFailure = w.TimeSeconds;
            Near(c.NextNeedAttemptAt - Math.Floor(secondFailure), 2 * pace.NeedTripBackoffSeconds, "Second retry delay did not double");
            // Open the store again: the cat drinks on its next attempt.
            foreach (var t in walled)
                t.Wall = false;
            for (int tick = 0; tick < 1200 && c.Thirst < 30; tick++)
                w.Step(World.SimulationStepSeconds);
            Check(c.Thirst > 30, "The cat did not drink once the store reopened");
            Check(c.NeedBackoff == 0, "A served trip did not reset the backoff");
            Valid(w);
        }

        /// <summary>
        /// A carrier whose need turns critical finishes a short delivery leg first; on a long leg it stores
        /// the cargo in the nearest storage instead of spilling it, and its job reclaims it afterwards.
        /// </summary>
        static void DeliveryBeforeNeed(string profile, bool longLeg)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var store = v.Stockpiles[0];
            var destination = new Stockpile { Id = w.Id("delivery-target"), Position = longLeg ? new Int2(-8, -6) : new Int2(6, 0), Width = 2, Depth = 2, Capacity = 1000, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(destination);
            var job = new Job { Id = w.Id("delivery-haul"), Kind = JobKind.Haul, Phase = JobPhase.InputDelivery, CatId = c.Id, Resource = "logs", Amount = 8, Position = destination.Position, TargetId = destination.Id, SourceId = "fixture-source" };
            v.Jobs.Add(job);
            c.JobId = job.Id;
            c.Cargo.Add(new Stack("logs", 8));
            c.Thirst = 10;
            bool drank = false;
            double last = c.Thirst;
            for (int tick = 0; tick < 2400 && !(drank && job.Completed); tick++)
            {
                w.Step(World.SimulationStepSeconds);
                Check(!v.Stockpiles.Any(s => s.Kind == "spill" && World.Amount(s.Goods, "logs") > 0), "Work cargo was spilled before a need trip");
                Near(Goods(w, v, "logs"), 8, "Work cargo was lost or duplicated");
                if (c.Thirst > last + 10)
                {
                    drank = true;
                    if (!longLeg)
                        Check(World.Amount(destination.Goods, "logs") >= 8 - 1e-9, "The cat drank before finishing a short delivery leg");
                    else
                        Check(World.Amount(destination.Goods, "logs") < 1e-9, "The long leg was finished before the critical drink");
                }
                last = c.Thirst;
            }
            Check(drank, "The carrier never drank");
            Check(job.Completed && World.Amount(destination.Goods, "logs") >= 8 - 1e-9, "The suspended delivery did not finish after the drink");
            Valid(w);
        }

        /// <summary>At night the sleep threshold rises by NightSleepBias, so a moderately tired cat beds down.</summary>
        static void NightSleep(string profile)
        {
            var w = ProfileFixture(profile, out var v, out var c);
            var pace = w.CurrentPacing();
            double day = w.NeedThreshold(c, GoalKey.NeedSleep);
            c.Rest = day + pace.NightSleepBias / 2;
            w.Step(2);
            Check(c.Goal != GoalKey.NeedSleep, "A cat above its daytime threshold went to sleep by day");
            var night = ProfileFixture(profile, out var nv, out var nc);
            night.TimeSeconds = (DayClock.NightStartHour * 60 - DayClock.StartMinutes + 30) * 60 / pace.DayScale;
            Check(DayClock.IsNight(night), "Fixture clock is not at night");
            Near(night.NeedThreshold(nc, GoalKey.NeedSleep) - day, pace.NightSleepBias, "Night bias");
            nc.Rest = day + pace.NightSleepBias / 2;
            night.Step(2);
            Check(nc.Goal == GoalKey.NeedSleep, "A tired cat did not bed down at night");
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
