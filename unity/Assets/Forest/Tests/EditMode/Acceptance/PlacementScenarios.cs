using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Placement: the side-effect-free placement API (buildings, lines, areas, transport paths) against the
    /// actions it guards, reserved footprints of pending work, and expansion of a fresh demo colony. The
    /// review_expansion_reserves_*, review_fishing_placement_* and layout_pending_* cases moved here from
    /// ConstructionScenarios with unchanged names.
    /// </summary>
    public static class PlacementScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (string site in new[] { "footprint", "entrance", "alternate_entrance", "gather" })
            {
                string placement = site;
                yield return new Scenario("regression.review_expansion_reserves_" + placement, () => PendingExpansionPlacement(placement));
            }
            foreach (string shore in new[] { "road", "entrance", "pending_wall", "duplicate", "unmapped", "legal_owned", "legal_exterior" })
            {
                string placement = shore;
                yield return new Scenario("regression.review_fishing_placement_" + placement, () => FishingPlacement(placement));
            }
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.review_expansion_reserves_linear_" + (track ? "rail" : "road"), () => PendingExpansionLinear(track, true));
            }
            yield return new Scenario("regression.layout_pending_entrance_preserves_work", PendingEntrance);
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_reserves_footprint", () => PendingRoadFootprint(track, false));
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_rechecks_footprint", () => PendingRoadFootprint(track, true));
            }
            yield return new Scenario("placement.check_agrees_with_plan_demo", () => CheckAgreesWithPlan(true, 500));
            yield return new Scenario("placement.check_agrees_with_plan_classic", () => CheckAgreesWithPlan(false, 200));
            yield return new Scenario("placement.checks_are_side_effect_free", ChecksAreSideEffectFree);
            yield return new Scenario("placement.tile_codes_name_each_problem", TileCodes);
            yield return new Scenario("placement.line_agrees_with_road_actions", LineAgreesWithRoads);
            yield return new Scenario("placement.line_footprint_is_x_then_z", LineFootprintShape);
            yield return new Scenario("placement.transport_path_follows_rail_and_water", TransportPath);
            yield return new Scenario("placement.area_agrees_with_designations", AreaAgreesWithDesignations);
            yield return new Scenario("placement.fresh_demo_colony_expands_without_clearing", FreshDemoExpansion);
            yield return new Scenario("placement.classic_expansion_keeps_survey_rule", ClassicExpansionSurvey);
        }

        // ----- moved from ConstructionScenarios (names unchanged) -----

        static void PendingExpansionPlacement(string site)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2;
            bool doorway = site == "entrance" || site == "alternate_entrance";
            int row = site == "alternate_entrance" ? 2 : 1;
            var at = new Int2(outer + (doorway ? 1 : 0), row);
            var blockedEntry = new Int2(outer, row);
            var alternate = new Int2(outer + 3, row);
            for (int x = outer; x <= outer + 3; x++)
                for (int z = 0; z <= row + 1; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            for (int x = v.Radius; x <= outer; x++)
                w.TileAt(new Int2(x, 0)).Road = true;
            if (doorway)
                for (int z = 1; z <= row; z++)
                    w.TileAt(new Int2(outer, z)).Road = true;
            if (site == "alternate_entrance")
            {
                for (int x = outer + 1; x <= alternate.X; x++)
                    w.TileAt(new Int2(x, 0)).Road = true;
                for (int z = 1; z <= row; z++)
                    w.TileAt(new Int2(alternate.X, z)).Road = true;
                Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(blockedEntry), "Fixture must prefer the doorway that expansion reserves");
            }
            var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            var expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
            var builder = v.Cats[1];
            builder.ControlledBy = "";
            int claims = w.Reservations.Count, buildings = v.Buildings.Count, piles = v.Stockpiles.Count, jobs = v.Jobs.Count;
            double materials = Goods(w, v, "materials"), timber = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
            var action = site == "gather" ? new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = at, End = at } : new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id };
            var result = w.Apply(Context(v), action);
            if (site == "alternate_entrance")
            {
                Check(result.Success, "Reserved first doorway prevented selection of another connected entrance");
                var field = v.Buildings.Single(b => b.Id == result.EntityId);
                Check(field.Entrance.Equals(alternate) && !expansion.Path.Contains(field.Entrance), "Field chose the reserved expansion wall instead of the alternate entrance");
                for (int tick = 0; tick < 12000 && (!expansion.Completed || !field.Completed); tick++)
                    w.Step(1);
                Check(field.Completed && expansion.Completed && w.Path(v.Center, field.Position, v) != null, "Field with an alternate doorway did not remain reachable through completed expansion");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Concurrent expansion changed its exact material bill");
                Near(Goods(w, v, "planks"), 0, "Concurrent Field changed its timber bill");
                Near(Goods(w, v, "blocks"), 0, "Concurrent Field changed its block bill");
            }
            else
            {
                Check(!result.Success, "Public " + site + " placement occupied a pending expansion wall");
                Check(v.Buildings.Count == buildings && v.Stockpiles.Count == piles && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected placement changed structures, work or claims");
                Near(Goods(w, v, "materials"), materials, "Rejected placement consumed expansion materials");
                Near(Goods(w, v, "planks"), timber, "Rejected placement consumed timber");
                Near(Goods(w, v, "blocks"), blocks, "Rejected placement consumed blocks");
            }
            Valid(w);
        }
        static void FishingPlacement(string placement)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            var at = placement == "road" ? new Int2(2, 1) : placement == "entrance" ? v.Buildings.First(b => b.Kind == "den").Entrance : placement == "pending_wall" ? new Int2(v.Radius + 2, 1) : placement == "legal_owned" ? new Int2(4, -5) : new Int2(12, 5);
            var tile = w.TileAt(at);
            tile.Wall = tile.Water = tile.Mountain = false;
            if (placement != "road" && placement != "entrance")
                tile.Road = tile.Rail = false;
            if (!v.Known.Contains(at))
                v.Known.Add(at);
            if (placement == "unmapped")
                v.Known.Remove(at);
            var water = w.TileAt(new Int2(at.X + 1, at.Z));
            water.Wall = water.Mountain = false;
            water.Water = true;
            water.Resource = "fish";
            water.Amount = water.FishCapacity = 24;
            if (placement == "pending_wall")
                Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            var action = new GameAction { Kind = "DesignateFishingSpot", Position = at, End = at };
            if (placement == "duplicate")
                Act(w, v, action);
            int piles = v.Stockpiles.Count, jobs = v.Jobs.Count, claims = w.Reservations.Count;
            string terrain = TerrainState(tile);
            double materials = Goods(w, v, "materials");
            var result = w.Apply(Context(v), action);
            bool legal = placement == "legal_owned" || placement == "legal_exterior";
            if (legal)
            {
                Check(result.Success, "Legal " + placement + " fishing shore was rejected: " + result.Error);
                Check(v.Stockpiles.Count == piles + 1 && v.Stockpiles.Single(p => p.Id == result.EntityId).Kind == "fishing", "Legal fishing designation did not create its physical shore pile");
            }
            else
            {
                Check(!result.Success, "Fishing designation accepted " + placement + " shore");
                Check(v.Stockpiles.Count == piles && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected fishing designation changed piles, work or claims");
            }
            Check(TerrainState(tile) == terrain, "Fishing designation changed its underlying terrain");
            Near(Goods(w, v, "materials"), materials, "Fishing designation changed held materials");
            Valid(w);
        }
        /// <summary>Shared with ConstructionScenarios (review_expansion_respects_linear_*).</summary>
        internal static void PendingExpansionLinear(bool rail, bool expansionFirst)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            World.Add(v.Stockpiles[0].Goods, "metal", 10);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2;
            var at = new Int2(outer, 1);
            var tile = w.TileAt(at);
            tile.Road = tile.Rail = false;
            w.TileAt(new Int2(outer, 0)).Road = true;
            var wall = new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id };
            var line = new GameAction { Kind = rail ? "DesignateRail" : "BuildRoad", Position = at, End = at, CatId = c.Id };
            Act(w, v, expansionFirst ? wall : line);
            var builder = v.Cats[1];
            builder.ControlledBy = "";
            int jobs = v.Jobs.Count, claims = w.Reservations.Count;
            double materials = Goods(w, v, "materials"), metal = Goods(w, v, "metal");
            var second = expansionFirst ? line : wall;
            second.CatId = builder.Id;
            var result = w.Apply(Context(v), second);
            Check(!result.Success, expansionFirst ? "Linear infrastructure accepted a reserved expansion wall" : "Expansion accepted a reserved linear infrastructure path");
            Check(v.Jobs.Count == jobs && w.Reservations.Count == claims && !tile.Road && !tile.Rail, "Rejected linear work changed jobs, claims or terrain");
            Near(Goods(w, v, "materials"), materials, "Rejected road consumed expansion materials");
            Near(Goods(w, v, "metal"), metal, "Rejected rail consumed metal");
            Valid(w);
        }
        static void PendingEntrance()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var building = v.Buildings.Single(b => b.Id == id);
            var entrance = w.BuildingEntrance(v, building);
            Check(entrance.HasValue, "Test scaffold has no connected entry");
            var tile = w.TileAt(entrance.Value);
            tile.Road = false;
            var job = v.Jobs.Single(j => j.TargetId == id);
            double goods = Goods(w, v, "planks"), progress = building.Progress;
            string claims = string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Amount));
            w.Step(3);
            Check(job.BlockedReason == "building_entrance_disconnected" && !job.Completed && building.Progress == progress, "Scaffold progressed without its connected entrance");
            Near(Goods(w, v, "planks"), goods, "Blocked entry consumed construction goods");
            Check(string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Amount)) == claims, "Blocked entry changed held claims");
            tile.Road = true;
            w.Step(600);
            Check(building.Completed && job.Completed, "Repairing the existing road did not resume the same scaffold");
        }
        static void PendingRoadFootprint(bool rail, bool changed)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var builder = v.Cats[1];
            builder.ControlledBy = "";
            builder.BuildingId = "";
            string resource = rail ? "metal" : "materials";
            World.Add(v.Stockpiles[0].Goods, resource, 2);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-5, 1);
            var roadId = Act(w, v, new GameAction { Kind = rail ? "DesignateRail" : "BuildRoad", Position = at, End = new Int2(-4, 1), CatId = c.Id }).EntityId;
            var job = v.Jobs.Single(j => j.Id == roadId);
            int count = v.Buildings.Count;
            double goods = Goods(w, v, resource), timber = Goods(w, v, "planks");
            var claims = string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount));
            if (!changed)
            {
                var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = builder.Id });
                Check(!result.Success, "Second builder occupied a reserved " + job.Kind + " path before paving completed");
                Check(v.Buildings.Count == count && builder.JobId == "" && claims == string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount)), "Rejected footprint changed ownership or claims");
                Near(Goods(w, v, resource), goods, "Rejected footprint consumed road inputs");
                Near(Goods(w, v, "planks"), timber, "Rejected footprint consumed timber");
                return;
            }
            builder.ControlledBy = "fixture-held";
            // Explicit imported-state fault: old saves can contain work under a later structure.
            var blocking = new Building { Id = "imported-overlap", Kind = "den", Position = at, Completed = true };
            v.Buildings.Add(blocking);
            w.Step(3);
            Check(job.BlockedReason == "infrastructure_footprint_blocked" && !job.Completed && job.Progress == 0 && job.PathIndex == 0, "Pending " + job.Kind + " advanced through an occupied footprint");
            Check(claims == string.Join("|", w.Reservations.Select(r => r.OwnerId + ":" + r.Resource + ":" + r.Amount)), "Blocked infrastructure changed held input claims");
            Near(Goods(w, v, resource), goods, "Blocked infrastructure consumed inputs");
            v.Buildings.Remove(blocking);
            for (int tick = 0; tick < 600 && !job.Completed; tick++)
                w.Step(1);
            Check(job.Completed && job.Path.All(p => rail ? w.TileAt(p).Rail : w.TileAt(p).Road), "Cleared footprint did not resume the same infrastructure work");
            Near(Goods(w, v, resource), 0, "Resumed infrastructure did not consume exact finite inputs");
            Valid(w);
        }

        // ----- the placement API -----

        static readonly string[] Kinds = { "den", "workshop", "wood_cutter", "field", "research_hut", "smithy", "shrine", "no_such_building" };

        /// <summary>
        /// CheckPlacement predicts PlanBuilding on random tiles and kinds: same legality, same first failure code,
        /// and on success the same entrance and bill. Demo plans never fail for a missing builder or materials;
        /// classic plans also need both, which the check reports separately.
        /// </summary>
        static void CheckAgreesWithPlan(bool demo, int samples)
        {
            var w = Fixture(out var v, out var c);
            if (demo)
                w.PacingProfile = "demo";
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", demo ? 100000 : 30);
            World.Add(v.Stockpiles[0].Goods, "blocks", demo ? 100000 : 15);
            uint state = 20261002;
            int Next(int n)
            {
                state = unchecked(state * 1664525u + 1013904223u);
                return (int)((state >> 8) % (uint)n);
            }
            var codes = new HashSet<string>();
            int placed = 0;
            for (int i = 0; i < samples; i++)
            {
                string kind = Kinds[Next(Kinds.Length)];
                var at = new Int2(Next(27) - 13, Next(27) - 13);
                var check = w.CheckPlacement(v, kind, at);
                var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = kind, Position = at });
                bool expected = check.Ok && (demo || check.MissingMaterials.Count == 0 && check.BuilderAvailable);
                Check(result.Success == expected, "Placement check disagreed with Plan for " + kind + " at " + at + ": check=" + check.Ok + "/" + check.Code + " plan=" + result.Success + "/" + result.Code + " " + result.Error);
                if (!check.Ok)
                {
                    Check(result.Code == check.Code && check.Code != "", "Plan failed with a different code than the check: " + result.Code + " vs " + check.Code);
                    codes.Add(check.Code);
                    continue;
                }
                if (!result.Success)
                {
                    Check(result.Code == (check.MissingMaterials.Count > 0 ? ResultCode.MissingMaterial : PlacementCode.BuilderBusy), "Classic plan failed for an unpredicted reason: " + result.Code);
                    codes.Add(result.Code);
                    continue;
                }
                placed++;
                var b = v.Buildings.Single(x => x.Id == result.EntityId);
                Check(b.Kind == kind && b.Position.Equals(at) && b.HasEntrance && b.Entrance.Equals(check.Entrance.Value), "Plan placed a different footprint or entrance than the check showed");
                Check(string.Join(",", b.Required.Select(s => s.Resource + ":" + s.Amount)) == string.Join(",", check.Cost.Select(s => s.Resource + ":" + s.Amount)), "Plan charged a different bill than the check showed");
                Check(check.Tiles.Count == 4 && check.Tiles.All(t => t.Code == ""), "A legal placement reported a bad tile");
            }
            Check(placed >= (demo ? 5 : 1) && codes.Count >= 4, "Agreement sample too narrow: placed=" + placed + " codes=" + string.Join(",", codes));
            Valid(w);
        }

        static void ChecksAreSideEffectFree()
        {
            var w = Fixture(out var v, out var c);
            w.PacingProfile = "demo";
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 3);
            var before = Fingerprint(w);
            for (int x = -16; x <= 16; x += 2)
                for (int z = -16; z <= 16; z += 3)
                {
                    var at = new Int2(x, z);
                    foreach (var kind in Kinds)
                        w.CheckPlacement(v, kind, at, (x + z) & 3);
                    w.CheckLine(v, "road", at, new Int2(x + 4, z + 3));
                    w.CheckLine(v, "rail", at, new Int2(x - 3, z));
                    foreach (var area in new[] { "stockpile", "gather", "fishing", "zone_avoid", "zone_gather" })
                        w.CheckArea(v, area, at, new Int2(x + 1, z + 2));
                }
            Check(Fingerprint(w) == before, "A placement check changed the world (tiles, claims or entities)");
        }

        static void TileCodes()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            string Code(Int2 at) => w.CheckArea(v, "stockpile", at, at).Tiles.Single().Code;
            var den = v.Buildings.First(b => b.Kind == "den");
            var water = new Int2(-6, 1);
            w.TileAt(water).Water = true;
            var rock = new Int2(-6, 2);
            w.TileAt(rock).Mountain = true;
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            var pendingWall = v.Jobs.Single(j => j.Kind == "expand").Path.First();
            var unclaimed = new Int2(v.Radius + 1, 3);
            Check(Code(new Int2(2, 1)) == PlacementCode.Road, "A road tile was not reported as road: " + Code(new Int2(2, 1)));
            Check(Code(den.Position) == PlacementCode.Occupied && Code(den.Entrance) != "", "Den footprint or door was not reported as taken");
            Check(Code(water) == PlacementCode.Water && Code(rock) == PlacementCode.Mountain, "Water or rock was not named: " + Code(water) + "/" + Code(rock));
            Check(Code(new Int2(60, 60)) == PlacementCode.Unmapped, "An unmapped tile was not reported");
            Check(Code(pendingWall) == PlacementCode.PendingWall, "A planned fence tile was not reported: " + Code(pendingWall));
            Check(w.CheckArea(v, "gather", pendingWall, pendingWall).Code == PlacementCode.PendingWall, "A planned fence did not block an exterior site");
            Check(Code(unclaimed) == PlacementCode.OutsideClaim, "An interior site outside the claim was accepted: " + Code(unclaimed));
            var locked = w.CheckPlacement(v, "smithy", new Int2(-5, 1));
            Check(!locked.Ok && locked.Code == PlacementCode.ResearchLocked, "A locked building was not reported as research_locked: " + locked.Code);
            var result = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "den", Position = water });
            Check(!result.Success && result.Code == PlacementCode.Water && result.Args["x"] == water.X.ToString(System.Globalization.CultureInfo.InvariantCulture) && result.Args["tile"] == PlacementCode.Water, "Plan did not name the water tile it refused: " + result.Code);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= -2))
                tile.Road = false;
            var cutOff = w.CheckPlacement(v, "den", new Int2(-6, -5));
            Check(!cutOff.Ok && cutOff.Code == PlacementCode.EntranceBlocked && !cutOff.Entrance.HasValue && cutOff.Tiles.All(t => t.Code == ""), "A disconnected site was not reported as entrance_blocked: " + cutOff.Code);
            var good = w.CheckPlacement(v, "den", new Int2(-5, 1));
            Check(good.Ok && good.Entrance.HasValue && good.Cost.Count == 2 && good.Work > 0, "A legal den site was refused: " + good.Code);
        }

        static void LineAgreesWithRoads()
        {
            foreach (var (kind, from, to, research) in new[] { ("road", new Int2(-3, -2), new Int2(-6, -2), false), ("road", new Int2(-6, -6), new Int2(-6, -8), false), ("road", new Int2(-3, 0), new Int2(-3, 1), false), ("road", new Int2(-3, -2), new Int2(200, -2), false), ("rail", new Int2(-5, 1), new Int2(-4, 1), false), ("rail", new Int2(-5, 1), new Int2(-4, 1), true), ("road", new Int2(4, 5), new Int2(4, 7), false) })
            {
                var w = Fixture(out var v, out var c);
                c.BuildingId = "";
                if (research)
                    v.Research = Catalog.Research.Select(n => n.Id).ToList();
                foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                    tile.Road = false;
                World.Add(v.Stockpiles[0].Goods, "materials", 3);
                World.Add(v.Stockpiles[0].Goods, "metal", 10);
                var check = w.CheckLine(v, kind, from, to);
                var result = w.Apply(Context(v), new GameAction { Kind = kind == "road" ? "BuildRoad" : "DesignateRail", Position = from, End = to, CatId = c.Id });
                bool expected = check.Ok && check.MissingMaterials.Count == 0 && check.BuilderAvailable;
                Check(result.Success == expected, "Line check disagreed with " + kind + " " + from + "->" + to + ": check=" + check.Code + " missing=" + check.MissingMaterials.Count + " result=" + result.Code + " " + result.Error);
                if (!check.Ok)
                    Check(result.Code == check.Code, "Line action failed with a different code: " + result.Code + " vs " + check.Code);
                else if (!result.Success)
                    Check(result.Code == ResultCode.MissingMaterial && result.Args["resource"] == check.MissingMaterials[0].Resource, "Missing line materials were not named");
                else
                    Check(v.Jobs.Single(j => j.Id == result.EntityId).Path.SequenceEqual(check.Tiles.Select(t => t.Tile)), "Planned line differs from the checked footprint");
            }
        }

        static void LineFootprintShape()
        {
            var line = World.LineFootprint(new Int2(0, 0), new Int2(3, -2));
            Check(line.SequenceEqual(new[] { new Int2(0, 0), new Int2(1, 0), new Int2(2, 0), new Int2(3, 0), new Int2(3, -1), new Int2(3, -2) }), "Line footprint is not the L path, X first");
            var single = World.LineFootprint(new Int2(5, 5), new Int2(5, 5));
            Check(single.Count == 1 && single[0].Equals(new Int2(5, 5)), "A one-tile line is not one tile");
            var longLine = World.LineFootprint(new Int2(0, 0), new Int2(300, 0));
            Check(longLine.Count == World.MaxLineTiles + 1 && !longLine.Last().Equals(new Int2(300, 0)), "A too-long line was not cut where the rules stop");
        }

        static void TransportPath()
        {
            var w = Fixture(out var v, out var c);
            for (int z = 5; z <= 9; z++)
                w.TileAt(new Int2(3, z)).Rail = true;
            var rail = w.PlanTransportPath("rail", new Int2(3, 5), new Int2(3, 9));
            Check(rail.Count == 5 && rail.First().Equals(new Int2(3, 5)) && rail.Last().Equals(new Int2(3, 9)) && rail.All(p => w.TileAt(p).Rail) && rail.Zip(rail.Skip(1), (a, b) => Int2.Distance(a, b)).All(d => d == 1), "Rail transport path is not a continuous track between the piles");
            w.TileAt(new Int2(3, 7)).Rail = false;
            Check(w.PlanTransportPath("rail", new Int2(3, 5), new Int2(3, 9)).Count == 0, "Rail transport path crossed a gap in the track");
            for (int x = 20; x <= 24; x++)
                w.TileAt(new Int2(x, 0)).Water = true;
            var tiles = w.Tiles.Count;
            var ship = w.PlanTransportPath("shipping", new Int2(19, 0), new Int2(25, 0));
            Check(ship.Count == 0 || ship.Skip(1).Take(ship.Count - 2).All(p => w.TileAt(p).Water), "Shipping path left the water");
            w.TileAt(new Int2(19, 0));
            w.TileAt(new Int2(25, 0));
            ship = w.PlanTransportPath("shipping", new Int2(19, 0), new Int2(25, 0));
            Check(ship.Count == 7 && ship.Skip(1).Take(5).All(p => w.TileAt(p).Water), "Shipping path did not follow the water");
            Check(w.PlanTransportPath("cart", new Int2(3, 5), new Int2(3, 9)).Count == 0, "An unknown transport mode produced a path");
        }

        static void AreaAgreesWithDesignations()
        {
            var cases = new[] { ("DesignateStockpile", "", new Int2(-5, 1), new Int2(-4, 2)), ("DesignateStockpile", "", new Int2(14, 10), new Int2(15, 11)), ("DesignateStockpile", "", new Int2(-6, 4), new Int2(12, 5)), ("DesignateGatherSpot", "logs", new Int2(14, 10), new Int2(14, 10)), ("DesignateGatherSpot", "logs", new Int2(-5, 1), new Int2(-5, 1)), ("CreateZone", "avoid", new Int2(-5, 1), new Int2(-4, 2)), ("CreateZone", "avoid", new Int2(-1, 2), new Int2(1, 2)), ("CreateZone", "gather", new Int2(40, 40), new Int2(41, 41)), ("DesignateFishingSpot", "", new Int2(12, 8), new Int2(12, 8)), ("DesignateFishingSpot", "", new Int2(14, 12), new Int2(14, 12)), ("DesignateFishingSpot", "", new Int2(-6, 4), new Int2(-6, 4)), ("DesignateFarm", "grain", new Int2(14, 10), new Int2(15, 11)), ("DesignateFarm", "grain", new Int2(-5, 1), new Int2(-4, 2)) };
            var legal = 0;
            var outcomes = new List<string>();
            foreach (var (kind, resource, from, to) in cases)
            {
                var w = Fixture(out var v, out var c);
                Exterior(w, v);
                var water = w.TileAt(new Int2(13, 8));
                water.Water = true;
                water.Resource = "fish";
                water.Amount = water.FishCapacity = 24;
                if (kind == "DesignateFarm")
                    Station(w, v, "field", new Int2(-7, -7));
                string area = kind == "DesignateStockpile" ? "stockpile" : kind == "DesignateGatherSpot" ? "gather" : kind == "CreateZone" ? "zone_" + resource : kind == "DesignateFishingSpot" ? "fishing" : "farm";
                var check = w.CheckArea(v, area, from, to);
                var result = w.Apply(Context(v), new GameAction { Kind = kind, Resource = resource, Position = from, End = to });
                Check(result.Success == check.Ok, "Area check disagreed with " + kind + " " + from + "->" + to + ": check=" + check.Code + " result=" + result.Code + " " + result.Error);
                if (!check.Ok)
                    Check(result.Code == check.Code, "Designation failed with a different code: " + result.Code + " vs " + check.Code);
                else
                    legal++;
                outcomes.Add(kind + from + ":" + (check.Ok ? "ok" : check.Code));
            }
            Check(legal >= 4, "Area agreement sample has too few legal designations: " + string.Join(" ", outcomes));
        }

        /// <summary>
        /// A freshly founded demo colony expands without any tile being cleared first: the founding pond on the
        /// new ring is a natural barrier that stays water, every other ring tile becomes fence, the four gates stay
        /// open and the bill is exactly one material per fence tile.
        /// </summary>
        static void FreshDemoExpansion()
        {
            var w = World.Create(41);
            w.PacingProfile = "demo";
            var v = w.Villages[0];
            var builder = v.Cats[0];
            foreach (var other in v.Cats.Where(x => x != builder))
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
            }
            int radius = v.Radius, outer = radius + 2;
            var pond = new Int2(v.Center.X + 2, v.Center.Z + outer);
            Check(w.TileAt(pond).Water && !w.Walkable(pond), "Fixture must start with the founding pond on the next ring");
            double materials = Goods(w, v, "materials");
            var result = (ActionResult)ConstructionScenarios.Invoke(w, "PlanExpansion", v, builder);
            Check(result.Success, "Fresh demo colony could not expand without clearing tiles: " + result.Code + " " + result.Error);
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            Check(!job.Path.Contains(pond) && job.Path.Count > 0 && job.Path.Count + 4 < 8 * outer, "Expansion planned fence on the natural barrier");
            Check(!job.Position.Equals(v.Center) && Int2.Distance(job.Position, job.Path[0]) <= 2 && Math.Max(Math.Abs(job.Position.X - v.Center.X), Math.Abs(job.Position.Z - v.Center.Z)) < outer, "Demo expansion did not stage just inside the first segment");
            for (int tick = 0; tick < 6000 && !job.Completed; tick++)
                w.Step(1);
            Check(job.Completed && v.Radius == outer, "Fresh demo expansion did not complete");
            Check(w.TileAt(pond).Water && !w.TileAt(pond).Wall && w.TileAt(pond).ClaimId == v.Id, "The pond barrier was drained or walled");
            Check(job.Path.All(p => w.TileAt(p).Wall), "A planned fence segment is missing");
            foreach (var gate in new[] { new Int2(v.Center.X, v.Center.Z + outer), new Int2(v.Center.X, v.Center.Z - outer), new Int2(v.Center.X + outer, v.Center.Z), new Int2(v.Center.X - outer, v.Center.Z) })
                Check(!w.TileAt(gate).Wall, "Expansion closed a gate at " + gate);
            Near(Goods(w, v, "materials"), materials - job.Path.Count, "Expansion did not charge one material per fence tile");
            Check(w.Reservations.Count == 0, "Completed expansion kept claims");
            Valid(w);
        }

        static void ClassicExpansionSurvey()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            var result = (ActionResult)ConstructionScenarios.Invoke(w, "PlanExpansion", v, v.Cats[0]);
            Check(!result.Success && result.Code == PlacementCode.Water && result.Error.Contains("Survey"), "Classic expansion lost today's survey rule for the founding pond: " + result.Code);
            var routed = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = v.Cats[0].Id });
            Check(!routed.Success, "Classic RequestJob expand accepted a ring through the founding pond");
        }
    }
}
