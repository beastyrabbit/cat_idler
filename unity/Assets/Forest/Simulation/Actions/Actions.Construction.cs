using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandlePlanBuilding(Village village, GameAction action, Cat cat) => Plan(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, action.Position, cat);
        private ActionResult HandleDesignatePile(Village village, GameAction action, string kind) => DesignatePile(village, action, kind);
        private ActionResult HandleRemovePile(Village village, GameAction action)
        {
            var pile = village.Stockpiles.Find(s => s.Id == action.TargetId);
            if (pile == null)
                return ActionResult.Fail("Unknown pile");
            if (Reservations.Any(r => r.PileId == pile.Id) || village.Jobs.Any(j => !j.Completed && j.Phase == "item_fetch" && j.SourceId == pile.Id && j.ItemIds.Count > 0))
                return ActionResult.Fail("Pile has active claims");
            if (pile.Goods.Count > 0 || village.Items.Any(i => i.LocationId == pile.Id))
            {
                pile.Kind = "spill";
                pile.Accepts.Clear();
                pile.ExpiresAt = -1;
            }
            else
                village.Stockpiles.Remove(pile);
            return ActionResult.Ok();
        }
        private ActionResult HandleDesignateFarm(Village village, GameAction action) => DesignateFarm(village, action);
        private ActionResult HandleClearFarm(Village village, GameAction action)
        {
            var farm = village.Farms.Find(f => f.Id == action.TargetId);
            if (farm == null)
                return ActionResult.Fail("Unknown farm");
            if (farm.Harvest > 0)
                Spill(village, farm.Position, new List<Stack> { new Stack(farm.Crop, farm.Harvest) });
            if (!string.IsNullOrEmpty(farm.WorkerId))
            {
                var fc = Cat(farm.WorkerId);
                if (fc != null)
                    CancelWork(village, fc);
            }
            village.Farms.Remove(farm);
            return ActionResult.Ok();
        }
        private ActionResult HandleCreateZone(Village village, GameAction action)
        {
            if (action.Resource != "avoid" && action.Resource != "gather")
                return ActionResult.Fail("Zone kind must be avoid or gather");
            return DesignatePile(village, action, "zone");
        }
        private ActionResult HandleRemoveZone(Village village, GameAction action) => village.Stockpiles.RemoveAll(s => s.Id == action.TargetId && s.Kind.StartsWith("zone_", StringComparison.Ordinal)) > 0 ? ActionResult.Ok() : ActionResult.Fail("Unknown zone");
        private ActionResult HandleBuildRoad(Village village, GameAction action, string kind, Cat cat) => PlanLinear(village, action, kind == "buildroad" ? "road" : "rail", cat);
        private ActionResult HandleBuildBridge(Village village, GameAction action, Cat cat) => CreateInfrastructure(village, action, cat, "bridge", "lumber", 4);
        private ActionResult HandleBuildDock(Village village, GameAction action, Cat cat) => CreateInfrastructure(village, action, cat, "dock", "lumber", 8);
        private bool FreeSite(Village v, Int2 p, int width, int depth, bool exterior = false)
        {
            if (width < 1 || depth < 1 || width > 16 || depth > 16)
                return false;
            if (!CanModifyTerrain(v.Id, p, width, depth))
                return false;
            for (int x = 0; x < width; x++)
                for (int z = 0; z < depth; z++)
                {
                    var at = new Int2(p.X + x, p.Z + z);
                    var t = TileAt(at);
                    if (!v.Known.Contains(at) || !Walkable(at) || RoadSurface(t) || t.Rail || ShrineRing(v, at) || v.Buildings.Any(b => b.HasEntrance && b.Entrance.Equals(at)))
                        return false;
                    if (v.Jobs.Any(j => !j.Completed && (j.Kind == "road" || j.Kind == "rail") && j.Path.Contains(at)) || PendingExpansionWall(v, at))
                        return false;
                    if (exterior ? t.ClaimId != "" : t.ClaimId != v.Id)
                        return false;
                    if (v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, at)) || v.Stockpiles.Any(s => Contains(s.Position, s.Width, s.Depth, at)) || v.Farms.Any(f => Contains(f.Position, f.Width, f.Depth, at)))
                        return false;
                }
            return true;
        }
        private static bool Contains(Int2 p, int w, int d, Int2 at) => at.Level == p.Level && at.X >= p.X && at.X < p.X + w && at.Z >= p.Z && at.Z < p.Z + d;
        private ActionResult Plan(Village v, string kind, Int2 p, Cat c)
        {
            kind = Snake(kind);
            if (!Catalog.Buildings.Contains(kind) || kind == "shrine")
                return ActionResult.Fail("Unknown constructible building");
            if (!Catalog.BuildingAvailable(v, kind))
                return ActionResult.Fail("Research unlock required");
            if (!FreeSite(v, p, 2, 2, kind == "field"))
                return ActionResult.Fail("Occupied, unmapped, or invalid footprint");
            var entry = FreeBuildingEntrance(v, p);
            if (!entry.HasValue)
                return ActionResult.Fail("Building entrance must touch the shrine-connected road network");
            int count = v.Buildings.Count(b => b.Kind == kind);
            double timber = 4 + count * 2, blocks = 2 + count;
            string timberKind = Available(v, "lumber") >= timber ? "lumber" : "planks";
            if (Available(v, timberKind) < timber || Available(v, "blocks") < blocks)
                return ActionResult.Fail("Construction requires " + timber + " timber and " + blocks + " blocks");
            c = c ?? AvailableCat(v, "build");
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available builder");
            var b = new Building { Id = Id(kind), Kind = kind, Position = p, Entrance = entry.Value, HasEntrance = true, RequiredWork = 120 + count * 30 };
            b.Required.Add(new Stack(timberKind, timber));
            b.Required.Add(new Stack("blocks", blocks));
            if (!Reserve(v, b.Id, timberKind, timber) || !Reserve(v, b.Id, "blocks", blocks))
            {
                Release(b.Id);
                return ActionResult.Fail("Construction inputs claimed");
            }
            v.Buildings.Add(b);
            StartJob(v, c, new Job { Kind = "build", TargetId = b.Id, Position = p, RequiredWork = b.RequiredWork, Manual = true });
            return ActionResult.Ok(b.Id);
        }
        private ActionResult DesignatePile(Village v, GameAction a, string kind)
        {
            int width = Math.Abs(a.End.X - a.Position.X) + 1, depth = Math.Abs(a.End.Z - a.Position.Z) + 1;
            var p = new Int2(Math.Min(a.End.X, a.Position.X), Math.Min(a.End.Z, a.Position.Z));
            if (kind == "designatefishingspot")
            {
                p = a.Position;
                width = depth = 1;
                if (!FreeSite(v, p, width, depth, GetTile(p)?.ClaimId != v.Id))
                    return ActionResult.Fail("Invalid fishing area");
                if (!Walkable(p) || !new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) }.Any(d => TileAt(new Int2(p.X + d.X, p.Z + d.Z)).Water))
                    return ActionResult.Fail("Fishing needs a walkable shore");
            }
            else if (kind == "zone")
            {
                if (width > 16 || depth > 16 || Enumerable.Range(0, width).Any(x => Enumerable.Range(0, depth).Any(z => !v.Known.Contains(new Int2(p.X + x, p.Z + z)))))
                    return ActionResult.Fail("Zone must be a bounded mapped area");
            }
            else if (!FreeSite(v, p, width, depth, kind != "designatestockpile"))
                return ActionResult.Fail("Invalid stockpile area");
            if (a.Accepts.Any(r => !Catalog.Resources.Contains(r) || r == "blessings"))
                return ActionResult.Fail("Invalid stored resource");
            if (!CanModifyTerrain(v.Id, p, width, depth))
                return ActionResult.Fail("Foreign territory cannot host this work site");
            if (kind == "zone" && a.Resource == "avoid" && (ConnectedRoads(v).Any(at => Contains(p, width, depth, at)) || v.Buildings.Any(b => b.HasEntrance && Contains(p, width, depth, b.Entrance) || b.Kind == "shrine" && p.X < b.Position.X + b.Width && p.X + width > b.Position.X && p.Z < b.Position.Z + b.Depth && p.Z + depth > b.Position.Z)))
                return ActionResult.Fail("Avoid zones cannot cover shrine roads or building entrances");
            var pile = new Stockpile { Id = Id("pile"), Position = p, Width = width, Depth = depth, Capacity = width * depth * 100, Kind = kind == "zone" ? "zone_" + a.Resource : kind == "designatestockpile" ? "storage" : kind == "designatefishingspot" ? "fishing" : "gather", Accepts = new List<string>(a.Accepts) };
            if (kind == "designategatherspot")
                pile.Accepts = new List<string> { a.Resource };
            v.Stockpiles.Add(pile);
            return ActionResult.Ok(pile.Id);
        }
        private ActionResult DesignateFarm(Village v, GameAction a)
        {
            string crop = string.IsNullOrEmpty(a.Resource) ? "grain" : a.Resource;
            if (crop == "herb")
                crop = "herbs";
            if (!new[] { "grain", "herbs", "catnip" }.Contains(crop))
                return ActionResult.Fail("Unknown crop");
            if (!v.Buildings.Any(b => b.Kind == "field" && b.Completed))
                return ActionResult.Fail("Completed Field required");
            var p = new Int2(Math.Min(a.Position.X, a.End.X), Math.Min(a.Position.Z, a.End.Z));
            int w = Math.Abs(a.Position.X - a.End.X) + 1, d = Math.Abs(a.Position.Z - a.End.Z) + 1;
            if (!FreeSite(v, p, w, d, true) || Path(v.Center, p) == null)
                return ActionResult.Fail("Farm requires reachable mapped exterior ground");
            var handoff = Neighbors(p).FirstOrDefault(at => !Contains(p, w, d, at) && CanModifyTerrain(v.Id, at) && Walkable(at) && Path(p, at) != null);
            if (handoff.Equals(default(Int2)) && !p.Equals(default(Int2)))
                return ActionResult.Fail("No adjacent physical harvest handoff");
            var f = new Farm { Id = Id("farm"), Position = p, Width = w, Depth = d, Crop = crop, Handoff = handoff };
            v.Farms.Add(f);
            return ActionResult.Ok(f.Id);
        }
    }
}
