using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Construction: founding layout, roads, doors, enclosures, expansion and placement.</summary>
    public static class ConstructionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (bool personal in new[] { false, true })
            {
                bool own = personal;
                yield return new Scenario("regression.layout_shrine_ring_" + (own ? "personal" : "communal"), () => FoundingLayout(own, false));
                yield return new Scenario("regression.layout_building_roads_" + (own ? "personal" : "communal"), () => FoundingLayout(own, true));
            }
            yield return new Scenario("regression.layout_four_exits_reach_finite_sources", FoundingExits);
            yield return new Scenario("regression.layout_disconnected_building_rejected", DisconnectedBuilding);
            foreach (string obstruction in new[] { "field", "entrance", "gather" })
            {
                string kind = obstruction;
                yield return new Scenario("regression.review_expansion_preserves_" + kind, () => ExpansionFootprint(kind, false));
            }
            yield return new Scenario("regression.review_expansion_rechecks_field", () => ExpansionFootprint("field", true));
            yield return new Scenario("regression.review_expansion_resumes_after_gather_removal", () => ExpansionFootprint("gather", true));
            foreach (string site in new[] { "footprint", "entrance", "alternate_entrance", "gather" })
            {
                string placement = site;
                yield return new Scenario("regression.review_expansion_reserves_" + placement, () => PendingExpansionPlacement(placement));
            }
            foreach (string order in new[] { "field_first", "expansion_first", "alternate_route", "loaded_conflict" })
            {
                string sequence = order;
                yield return new Scenario("regression.review_expansion_upstream_road_" + sequence, () => ExpansionRoadConnection(sequence));
            }
            foreach (string order in new[] { "field_first", "expansion_first", "alternate_route", "loaded_conflict" })
            {
                string sequence = order;
                yield return new Scenario("regression.review_expansion_farm_edge_" + sequence, () => ExpansionFarmEdge(sequence));
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
                yield return new Scenario("regression.review_expansion_respects_linear_" + (track ? "rail" : "road"), () => PendingExpansionLinear(track, false));
            }
            yield return new Scenario("regression.layout_shrine_ring_reserved", ShrineRingReserved);
            yield return new Scenario("regression.layout_pending_entrance_preserves_work", PendingEntrance);
            yield return new Scenario("regression.layout_new_building_uses_door", BuildingDoor);
            yield return new Scenario("regression.layout_isolated_road_cannot_extend", IsolatedRoad);
            yield return new Scenario("regression.layout_paid_road_opens_building", PaidBuildingRoad);
            yield return new Scenario("regression.layout_legacy_geometry_and_roads_preserved", LegacyLayout);
            yield return new Scenario("regression.layout_avoid_cannot_close_shrine_access", AvoidShrineAccess);
            foreach (bool rail in new[] { false, true })
            {
                bool track = rail;
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_reserves_footprint", () => PendingRoadFootprint(track, false));
                yield return new Scenario("regression.layout_pending_" + (track ? "rail" : "road") + "_rechecks_footprint", () => PendingRoadFootprint(track, true));
            }
            yield return new Scenario("regression.buildable_founding_services", BuildableServices);
            yield return new Scenario("regression.communal_and_personal_enclosures", Enclosures);
            yield return new Scenario("regression.expansion_interruption_preserves_perimeter", ExpansionInterruption);
        }
        static HashSet<Int2> LayoutRoads(World w, Village v)
        {
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            bool Inside(Building b, Int2 p) => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth;
            var seen = new HashSet<Int2>();
            var pending = new Queue<Int2>();
            for (int x = 0; x < shrine.Width; x++)
                for (int z = 0; z < shrine.Depth; z++)
                    pending.Enqueue(new Int2(shrine.Position.X + x, shrine.Position.Z + z));
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                {
                    var next = new Int2(at.X + offset.X, at.Z + offset.Z);
                    var tile = w.GetTile(next);
                    if (tile != null && tile.Road && w.Walkable(v, next) && !v.Buildings.Any(b => Inside(b, next)) && !seen.Contains(next))
                    {
                        seen.Add(next);
                        pending.Enqueue(next);
                    }
                }
            }
            return seen;
        }
        static void FoundingLayout(bool personal, bool roads)
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            if (personal)
                v = w.Village(Act(w, v, new GameAction { Kind = "FoundVillage", Name = "Layout home" }).EntityId);
            var shrine = v.Buildings.Single(b => b.Kind == "shrine");
            if (!roads)
            {
                Check(shrine.Width == 3 && shrine.Depth == 3 && shrine.Position.Equals(new Int2(v.Center.X - 1, v.Center.Z - 1)), "Shrine must occupy exactly the centered3×3tiles");
                for (int z = -2; z <= 2; z++)
                    for (int x = -2; x <= 2; x++)
                    {
                        var p = new Int2(v.Center.X + x, v.Center.Z + z);
                        var t = w.TileAt(p);
                        Check(t.Road == (Math.Max(Math.Abs(x), Math.Abs(z)) == 2), "Shrine road ring is incomplete or crosses the shrine at " + p);
                        if (t.Road)
                            Check(!v.Buildings.Any(b => p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth) && !v.Stockpiles.Any(s => p.X >= s.Position.X && p.X < s.Position.X + s.Width && p.Z >= s.Position.Z && p.Z < s.Position.Z + s.Depth), "Shrine road overlaps a physical footprint");
                    }
                Check(v.Cats.Count(c => c.Alive) == (personal ? 15 : 30) && v.Buildings.Count(b => b.Kind == "den") == (personal ? 3 : 6), "Founding population or housing changed");
                return;
            }
            var network = LayoutRoads(w, v);
            foreach (var b in v.Buildings.Where(b => b.Kind != "shrine"))
            {
                bool adjacent = network.Any(p => p.X >= b.Position.X && p.X < b.Position.X + b.Width && (p.Z == b.Position.Z - 1 || p.Z == b.Position.Z + b.Depth) || p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth && (p.X == b.Position.X - 1 || p.X == b.Position.X + b.Width));
                Check(adjacent, "Founding building lacks an entrance on shrine-connected road: " + b.Kind + " " + b.Position);
                Check(w.Path(v.Center, b.Position, v) != null, "Founding building is physically unreachable: " + b.Kind);
            }
        }
        static void FoundingExits()
        {
            foreach (int seed in new[] { 7, 41, 127 })
            {
                var w = World.Create(seed);
                Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Exit home" });
                foreach (var v in w.Villages)
                {
                    foreach (var d in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                    {
                        var gate = new Int2(v.Center.X + d.X * v.Radius, v.Center.Z + d.Z * v.Radius);
                        var outside = new Int2(gate.X + d.X, gate.Z + d.Z);
                        Check(!w.TileAt(gate).Wall && w.TileAt(gate).Road && w.Walkable(v, outside) && w.Path(v.Center, outside, v) != null, "Founding gate is not a real traversable exit: " + gate);
                    }
                    foreach (string resource in new[] { "food", "logs", "stone", "fibre", "ore", "clay", "sand", "gem" })
                        Check(w.Tiles.Where(t => t.Resource == resource && t.Amount == 200 && Int2.Distance(t.Position, v.Center) < 30).Any(t => w.Path(v.Center, t.Position, v) != null), "Finite founding source is isolated: " + resource + " seed=" + seed);
                }
            }
        }
        static void DisconnectedBuilding()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var at = new Int2(-6, -5);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= -2))
                tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            int buildings = v.Buildings.Count, claims = w.Reservations.Count;
            double timber = Goods(w, v, "planks");
            var denied = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id });
            Check(!denied.Success, "Disconnected paving allowed a new building without a shrine-connected entrance");
            Check(v.Buildings.Count == buildings && w.Reservations.Count == claims && Goods(w, v, "planks") == timber, "Rejected layout changed property or claims");
        }
        static void ExpansionFootprint(string kind, bool pending)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", kind == "entrance" ? 1002 : 1001);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int radius = v.Radius, outer = radius + 2;
            var at = new Int2(outer + (kind == "entrance" ? 1 : 0), 1);
            var entry = kind == "entrance" ? new Int2(outer, 1) : new Int2(outer, 0);
            for (int x = outer; x <= outer + 2; x++)
                for (int z = 0; z <= 2; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            for (int x = radius; x < outer; x++)
                w.TileAt(new Int2(x, 0)).Road = true;
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer, 0), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId);
            for (int tick = 0; tick < 600 && !road.Completed; tick++)
                w.Step(1);
            Check(road.Completed && w.TileAt(entry).Road, "Public road did not connect the exterior work site");
            Near(Goods(w, v, "materials"), 1000, "Exterior access road did not consume its exact bill");
            Job expansion = null;
            Cat builder = c;
            if (pending)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                for (int tick = 0; tick < 1800 && expansion.PathIndex == 0; tick++)
                    w.Step(1);
                Check(expansion.PathIndex > 0, "Expansion must have paid perimeter work before the competing building");
                builder = v.Cats[1];
                builder.ControlledBy = "";
            }
            string obstacleId;
            if (pending)
            {
                // Explicit loaded-state fault: older saves could already contain work on the reserved perimeter.
                if (kind == "gather")
                {
                    var pile = new Stockpile { Id = w.Id("loaded-gather"), Kind = "gather", Position = at, Width = 1, Depth = 1 };
                    v.Stockpiles.Add(pile);
                    obstacleId = pile.Id;
                }
                else
                {
                    var field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                    v.Buildings.Add(field);
                    obstacleId = field.Id;
                }
            }
            else if (kind == "gather")
                obstacleId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = at, End = at }).EntityId;
            else
            {
                obstacleId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id }).EntityId;
                var field = v.Buildings.Single(b => b.Id == obstacleId);
                Check(field.Entrance.Equals(entry), "Field did not use the intended exterior entrance");
                if (!pending)
                {
                    for (int tick = 0; tick < 600 && !field.Completed; tick++)
                        w.Step(1);
                    Check(field.Completed && w.Path(v.Center, field.Position, v) != null, "Public exterior Field did not complete with reachable work position");
                }
            }
            int claims = w.Reservations.Count, jobs = v.Jobs.Count;
            double materials = Goods(w, v, "materials");
            string terrain = TerrainState(w.TileAt(at)), entryTerrain = TerrainState(w.TileAt(entry));
            if (pending)
            {
                int paid = expansion.PathIndex;
                double progress = expansion.Progress;
                string held = string.Join("|", w.Reservations.Where(r => r.OwnerId.StartsWith(expansion.Id, StringComparison.Ordinal)).Select(r => r.OwnerId + ":" + r.Amount));
                w.Step(3);
                Check(!expansion.Completed && expansion.BlockedReason == "expansion_footprint_blocked" && expansion.PathIndex == paid && expansion.Progress == progress, "Pending expansion advanced after an exterior Field occupied its wall route");
                Check(expansion.Path.Take(paid).All(p => w.TileAt(p).Wall), "Blocked expansion discarded paid wall segments");
                Check(held == string.Join("|", w.Reservations.Where(r => r.OwnerId.StartsWith(expansion.Id, StringComparison.Ordinal)).Select(r => r.OwnerId + ":" + r.Amount)), "Blocked expansion changed its held input claims");
            }
            else
            {
                var denied = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                Check(!denied.Success && denied.Error.Contains("footprint"), "Expansion accepted a wall through an existing " + kind);
                Check(v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected expansion created work or reserved materials");
            }
            Check(v.Radius == radius && TerrainState(w.TileAt(at)) == terrain && TerrainState(w.TileAt(entry)) == entryTerrain, "Expansion changed the protected work position or entrance");
            Near(Goods(w, v, "materials"), materials, "Conflicting expansion consumed materials");
            Valid(w);
            if (pending && kind == "gather")
            {
                Act(w, v, new GameAction { Kind = "RemoveGatherSpot", TargetId = obstacleId });
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                    w.Step(1);
                Check(expansion.Completed && v.Radius == outer && expansion.BlockedReason == "", "Removing the obstruction did not resume the same expansion");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) })
                    Check(!w.TileAt(gate).Wall, "Resumed expansion closed a required cardinal gate");
                Check(expansion.Path.All(p => w.TileAt(p).Wall), "Resumed expansion lost a paid perimeter segment");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Resumed expansion consumed more than its finite perimeter bill");
                Check(w.Reservations.Count == 0, "Completed expansion retained claims");
                Valid(w);
            }
        }
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
        static void ExpansionRoadConnection(string order)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1003);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            int outer = v.Radius + 2;
            var at = new Int2(outer + 1, 2);
            var entry = new Int2(outer + 1, 1);
            var crossing = new Int2(outer, 1);
            for (int x = outer - 1; x <= outer + 2; x++)
                for (int z = 0; z <= 3; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(outer - 1, 0)).Road = true;
            if (order == "alternate_route")
            {
                w.TileAt(new Int2(outer, 0)).Road = true;
                w.TileAt(new Int2(outer + 1, 0)).Road = true;
            }
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(outer - 1, 1), End = entry, CatId = c.Id }).EntityId;
            var road = v.Jobs.Single(j => j.Id == roadId);
            for (int tick = 0; tick < 600 && !road.Completed; tick++)
                w.Step(1);
            Check(road.Completed && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Public road did not connect the distant Field entrance");
            Near(Goods(w, v, "materials"), 1000, "Access road changed its finite material bill");
            bool expansionFirst = order == "expansion_first" || order == "loaded_conflict";
            Job expansion = null;
            Building field = null;
            Cat builder = c;
            if (expansionFirst)
            {
                var planned = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
                expansion = v.Jobs.Single(j => j.Id == planned.EntityId);
                Check(expansion.Path.Contains(crossing) && !expansion.Path.Contains(entry), "Fixture must reserve an upstream road, not the doorway");
                builder = v.Cats[1];
                builder.ControlledBy = "";
            }
            if (order == "loaded_conflict")
            {
                // Existing save conflict: a completed Field whose road crosses already planned wall work.
                field = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                v.Buildings.Add(field);
                double materials = Goods(w, v, "materials"), progress = expansion.Progress;
                int claims = w.Reservations.Count;
                w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field's upstream road did not pause the existing expansion before material movement");
                Near(Goods(w, v, "materials"), materials, "Blocked upstream road consumed retained expansion materials");
                Check(!w.TileAt(crossing).Wall, "Blocked expansion walled the Field's upstream road");
                Valid(w);
                return;
            }
            int reservations = w.Reservations.Count, jobs = v.Jobs.Count, buildings = v.Buildings.Count;
            var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
            if (order == "expansion_first")
            {
                Check(!placement.Success, "Field placement accepted a road that pending expansion will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected disconnected Field changed structures, jobs or claims");
                Near(Goods(w, v, "planks"), 4, "Rejected Field consumed timber");
                Near(Goods(w, v, "blocks"), 2, "Rejected Field consumed blocks");
                Valid(w);
                return;
            }
            Check(placement.Success, "Initial exterior Field was not connected: " + placement.Error);
            field = v.Buildings.Single(b => b.Id == placement.EntityId);
            for (int tick = 0; tick < 600 && !field.Completed; tick++)
                w.Step(1);
            Check(field.Completed, "Public exterior Field construction did not complete");
            reservations = w.Reservations.Count;
            jobs = v.Jobs.Count;
            var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted an upstream wall that disconnects an existing Field");
                Check(v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected expansion created jobs or claims");
                Near(Goods(w, v, "materials"), 1000, "Rejected expansion consumed materials");
            }
            else
            {
                Check(expanded.Success, "Alternate road through the required gate did not allow expansion");
                expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
                for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                    w.Step(1);
                Check(expansion.Completed && w.TileAt(crossing).Wall && w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Completed expansion lost the Field's alternate shrine-connected road");
                foreach (var gate in new[] { new Int2(0, outer), new Int2(0, -outer), new Int2(outer, 0), new Int2(-outer, 0) })
                    Check(!w.TileAt(gate).Wall, "Alternate access created a missing cardinal gate");
                Near(Goods(w, v, "materials"), 1000 - expansion.Path.Count, "Expansion with alternate access changed its wall bill");
            }
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry), "Existing Field lost its shrine-connected doorway");
            Valid(w);
        }
        static void ExpansionFarmEdge(string order)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            PrepareExpansion(w, v);
            World.Add(v.Stockpiles[0].Goods, "materials", 1020);
            World.Add(v.Stockpiles[0].Goods, "planks", 10);
            World.Add(v.Stockpiles[0].Goods, "blocks", 5);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            for (int x = 10; x <= 15; x++)
                for (int z = 0; z <= 6; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = false;
                    tile.ClaimId = "";
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(10, 0)).Road = true;
            void Road(Int2 from, Int2 to)
            {
                var id = Act(w, v, new GameAction { Kind = "BuildRoad", Position = from, End = to, CatId = c.Id }).EntityId;
                var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < 1000 && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Farm-edge fixture road did not complete");
            }
            Building Field(Int2 position)
            {
                var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = position, CatId = c.Id }).EntityId;
                var building = v.Buildings.Single(b => b.Id == id);
                for (int tick = 0; tick < 1000 && !building.Completed; tick++)
                    w.Step(1);
                Check(building.Completed, "Farm-edge fixture Field did not complete");
                return building;
            }
            Road(new Int2(11, 0), new Int2(13, 0));
            var existing = Field(new Int2(13, 1));
            Road(new Int2(11, 1), new Int2(11, 3));
            Road(new Int2(12, 3), new Int2(12, 3));
            if (order == "alternate_route")
            {
                Road(new Int2(12, 1), new Int2(12, 1));
                Road(new Int2(12, 2), new Int2(12, 2));
            }
            var at = new Int2(12, 4);
            var entry = new Int2(12, 3);
            var edgeFrom = new Int2(11, 0);
            var edgeTo = new Int2(11, 1);
            Building target = null;
            if (order == "field_first" || order == "alternate_route")
                target = Field(at);
            Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 1), End = new Int2(10, 3) });
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Crossable(edgeFrom, edgeTo), "Farm-edge fixture must start with a connected road across the future fence");
            int claims = w.Reservations.Count, jobs = v.Jobs.Count, radius = v.Radius;
            double materials = Goods(w, v, "materials");
            var expanded = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            if (order == "field_first")
            {
                Check(!expanded.Success, "Expansion accepted a future farm fence that disconnects an existing Field");
                Check(v.Jobs.Count == jobs && w.Reservations.Count == claims && v.Radius == radius && v.BoundaryEdges.Count == 0, "Rejected farm-edge expansion changed work, claims or fences");
                Near(Goods(w, v, "materials"), materials, "Rejected farm-edge expansion consumed materials");
                Valid(w);
                return;
            }
            Check(expanded.Success, "Clear farm-edge expansion was rejected: " + expanded.Error);
            var expansion = v.Jobs.Single(j => j.Id == expanded.EntityId);
            Check(!expansion.Path.Contains(edgeFrom) && !expansion.Path.Contains(edgeTo) && !expansion.Path.Contains(entry), "Farm-edge fixture accidentally relies on a future wall tile");
            if (order == "expansion_first")
            {
                var builder = v.Cats[1];
                builder.ControlledBy = "";
                claims = w.Reservations.Count;
                jobs = v.Jobs.Count;
                int buildings = v.Buildings.Count;
                double timber = Goods(w, v, "planks"), blocks = Goods(w, v, "blocks");
                var placement = w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "field", Position = at, CatId = builder.Id });
                Check(!placement.Success, "New Field accepted a road that the pending farm fence will disconnect");
                Check(v.Buildings.Count == buildings && v.Jobs.Count == jobs && w.Reservations.Count == claims, "Rejected Field changed structures, jobs or claims");
                Near(Goods(w, v, "planks"), timber, "Rejected Field consumed timber");
                Near(Goods(w, v, "blocks"), blocks, "Rejected Field consumed blocks");
                Valid(w);
                return;
            }
            if (order == "loaded_conflict")
            {
                // Saved-state conflict from a prior version: the later Field already exists.
                target = new Building { Id = w.Id("loaded-field"), Kind = "field", Position = at, Entrance = entry, HasEntrance = true, Completed = true };
                v.Buildings.Add(target);
                claims = w.Reservations.Count;
                double progress = expansion.Progress;
                w.Step(3);
                Check(expansion.BlockedReason == "expansion_footprint_blocked" && expansion.Progress == progress && expansion.PathIndex == 0 && w.Reservations.Count == claims, "Loaded Field did not pause expansion before its future farm fence cut the road");
                Near(Goods(w, v, "materials"), materials, "Blocked farm-edge expansion consumed materials");
                Check(w.Crossable(edgeFrom, edgeTo), "Blocked expansion changed the farm fence");
                Valid(w);
                return;
            }
            for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                w.Step(1);
            Check(expansion.Completed && v.Radius == radius + 2 && !w.Crossable(edgeFrom, edgeTo) && w.TileAt(edgeFrom).Road && w.TileAt(edgeTo).Road && !w.TileAt(edgeTo).Wall, "Expansion did not create the expected edge fence while retaining its road tiles");
            Check(w.BuildingEntrance(v, new Building { Position = at }).Equals(entry) && w.Path(v.Center, target.Position, v) != null && w.BuildingEntrance(v, new Building { Position = existing.Position }).Equals(existing.Entrance), "Alternate road did not preserve both Fields across the completed farm fence");
            Near(Goods(w, v, "materials"), materials - expansion.Path.Count, "Farm-edge expansion changed its exact wall bill");
            Check(w.Reservations.Count == 0, "Completed farm-edge expansion retained claims");
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
        static void PendingExpansionLinear(bool rail, bool expansionFirst)
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
        static void ShrineRingReserved()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-3, -3), CatId = c.Id }).Success, "Construction occupied the shrine's required road ring");
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
        static void BuildingDoor()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var id = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id }).EntityId;
            var building = v.Buildings.Single(b => b.Id == id);
            Check(building.HasEntrance, "New construction did not preserve its selected doorway");
            var path = w.Path(v.Center, building.Position, v);
            Check(path != null && path.Contains(building.Entrance), "Physical worker path bypassed the selected doorway");
            var closed = new Int2(building.Position.X - 1, building.Position.Z);
            Check(!closed.Equals(building.Entrance) && !w.Crossable(closed, building.Position), "Building exterior allowed a wall crossing away from its door");
        }
        static void IsolatedRoad()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "materials", 100);
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                tile.Road = false;
            w.TileAt(new Int2(-6, -3)).Road = true;
            Check(!LayoutRoads(w, v).Contains(new Int2(-6, -3)), "Test paving is connected instead of isolated");
            Check(!w.Apply(Context(v), new GameAction { Kind = "BuildRoad", Position = new Int2(-6, -2), End = new Int2(-5, -2), CatId = c.Id }).Success, "Isolated paving bypassed shrine-road connectivity");
        }
        static void PaidBuildingRoad()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            foreach (var tile in w.Tiles.Where(t => t.Position.X <= -3 && t.Position.Z <= 0))
                tile.Road = false;
            World.Add(v.Stockpiles[0].Goods, "materials", 4);
            World.Add(v.Stockpiles[0].Goods, "planks", 4);
            World.Add(v.Stockpiles[0].Goods, "blocks", 2);
            var at = new Int2(-6, -4);
            Check(!w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).Success, "Unpaved site was accepted before its access road existed");
            var roadId = Act(w, v, new GameAction { Kind = "BuildRoad", Position = new Int2(-3, -2), End = new Int2(-6, -2), CatId = c.Id }).EntityId;
            Check(!w.TileAt(new Int2(-6, -2)).Road, "Road appeared before physical work");
            for (int tick = 0; tick < 600 && !v.Jobs.Single(j => j.Id == roadId).Completed; tick++)
                w.Step(1);
            Check(v.Jobs.Single(j => j.Id == roadId).Completed, "Paid access road did not finish");
            Near(Goods(w, v, "materials"), 0, "Road did not consume its exact four materials");
            var buildingId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = at, CatId = c.Id }).EntityId;
            w.Step(600);
            var building = v.Buildings.Single(b => b.Id == buildingId);
            Check(building.Completed && LayoutRoads(w, v).Contains(building.Entrance), "Road did not enable a connected completed building");
            Near(Goods(w, v, "planks"), 0, "Building timber bill changed");
            Near(Goods(w, v, "blocks"), 0, "Building block bill changed");
            Valid(w);
        }
        static void LegacyLayout()
        {
            var w = new World { Seed = 41 };
            var v = new Village { Id = "legacy-layout", Communal = true, Center = new Int2(0, 0), Radius = 12 };
            w.Villages.Add(v);
            var shrine = new Building { Id = "saved-shrine", Kind = "shrine", Position = new Int2(3, 3), Width = 3, Depth = 3, Completed = true };
            var old = new Building { Id = "saved-workshop", Kind = "wood_cutter", Position = new Int2(8, 4), Width = 2, Depth = 2, Completed = true };
            v.Buildings.AddRange(new[] { shrine, old });
            for (int z = 0; z <= 12; z++)
                for (int x = 0; x <= 12; x++)
                {
                    var at = new Int2(x, z);
                    v.Known.Add(at);
                    v.ClaimedTiles.Add(at);
                    w.Tiles.Add(new Tile { Position = at, ClaimId = v.Id, Biome = "meadow" });
                }
            foreach (var at in new[] { new Int2(6, 4), new Int2(7, 4), new Int2(6, 3), new Int2(7, 3), new Int2(8, 3) })
                w.TileAt(at).Overlay = "road_built";
            var cat = new Cat { Id = "saved-cat", VillageId = v.Id, Position = new Int2(6, 3), X = 6, Z = 3 };
            v.Cats.Add(cat);
            v.Stockpiles.Add(new Stockpile { Id = "saved-store", Position = new Int2(1, 1), Goods = new List<Stack> { new Stack("planks", 20), new Stack("blocks", 20) } });
            var terrain = string.Join("|", w.Tiles.Select(t => t.Position + ":" + t.Overlay + ":" + TerrainState(t)));
            Check(w.BuildingEntrance(v, old).Equals(new Int2(8, 3)), "Imported road_built network was not rooted at the actual saved shrine footprint");
            var result = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(8, 1), CatId = cat.Id });
            Check(v.LayoutVersion == 0 && !old.HasEntrance && old.Position.Equals(new Int2(8, 4)) && shrine.Position.Equals(new Int2(3, 3)), "Inspecting or extending a legacy layout relocated saved property");
            Check(w.Tiles.Count == 169 && terrain == string.Join("|", w.Tiles.Select(t => t.Position + ":" + t.Overlay + ":" + TerrainState(t))), "Legacy compatibility silently paved or rewrote saved terrain");
            Check(v.Buildings.Single(b => b.Id == result.EntityId).HasEntrance && v.Cats.Single().Id == "saved-cat", "New connected extension lost persisted layout or cat identity");
        }
        static void AvoidShrineAccess()
        {
            var w = World.Create(7);
            var v = w.Villages[0];
            foreach (var at in new[] { v.Center, new Int2(v.Center.X, v.Center.Z + 2), v.Buildings.First(b => b.HasEntrance).Entrance })
                Check(!w.Apply(Context(v), new GameAction { Kind = "CreateZone", Resource = "avoid", Position = at, End = at }).Success, "Avoid zone severed shrine or building access");
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
        static void BuildableServices()
        {
            foreach (var kind in new[] { "den", "workshop", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "mouse_farm" })
            {
                var w = Fixture(out var v, out var c);
                World.Add(v.Stockpiles[0].Goods, "planks", 100);
                World.Add(v.Stockpiles[0].Goods, "blocks", 100);
                c.BuildingId = "";
                Act(w, v, new GameAction { Kind = "PlanBuilding", Name = kind, Position = new Int2(-5, 1), CatId = c.Id });
                w.Step(500);
                Check(v.Buildings.Any(b => b.Kind == kind && b.Position.Equals(new Int2(-5, 1)) && b.Completed), kind + " cannot complete physical construction");
                Valid(w);
            }
        }
        static void Enclosures()
        {
            var w = World.Create(7);
            Check(w.Villages[0].Radius == 9, "Communal enclosure must fit its30cat/6Den blueprint at radius9");
            var made = Act(w, w.Villages[0], new GameAction { Kind = "FoundVillage", Name = "Personal camp" });
            var v = w.Village(made.EntityId);
            Check(v.Radius == 6 && v.Cats.Count(c => c.Alive) == 15, "Personal founding footprint/population changed");
        }
        static void ExpansionInterruption()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            int radius = v.Radius, newRadius = radius + 2;
            foreach (var t in w.Tiles.Where(t => Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius))
            {
                t.Wall = t.Water = t.Mountain = false;
                t.Resource = "";
                t.Amount = 0;
            }
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            for (int i = 0; i < 1200 && !w.Tiles.Any(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)); i++)
                w.Step(1);
            var built = w.Tiles.Where(t => t.Wall && (Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius)).Select(t => t.Position).ToArray();
            Check(built.Length > 0, "Expansion never built its first real segment");
            Check(v.Radius == radius, "Expansion cut over before perimeter completion");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            w.Step(12000);
            Check(v.Radius == newRadius, "Interrupted expansion cannot resume to complete its wall");
            Check(built.All(p => w.TileAt(p).Wall), "Expansion interruption lost completed segments");
            Check(!w.TileAt(new Int2(0, newRadius)).Wall, "Expansion closed the new south gate");
            Valid(w);
        }
    }
}
