using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandlePlanBuilding(Village village, GameAction action, Cat cat) => Plan(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, action.Position, cat, action.Direction);
        private ActionResult HandleDesignatePile(Village village, GameAction action, string kind) => DesignatePile(village, action, kind);
        private ActionResult HandleRemovePile(Village village, GameAction action)
        {
            var pile = village.Stockpiles.Find(s => s.Id == action.TargetId);
            if (pile == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Unknown pile");
            if (Reservations.Any(r => r.PileId == pile.Id) || village.Jobs.Any(j => !j.Completed && j.Phase == JobPhase.ItemFetch && j.SourceId == pile.Id && j.ItemIds.Count > 0))
                return ActionResult.Fail(PlacementCode.PileClaimed, "Pile has active claims");
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
                return ActionResult.Fail(ResultCode.UnknownTarget, "Unknown farm");
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
                return ActionResult.Fail(PlacementCode.InvalidZone, "Zone kind must be avoid or gather");
            return DesignatePile(village, action, "zone");
        }
        private ActionResult HandleRemoveZone(Village village, GameAction action) => village.Stockpiles.RemoveAll(s => s.Id == action.TargetId && s.Kind.StartsWith("zone_", StringComparison.Ordinal)) > 0 ? ActionResult.Ok() : ActionResult.Fail(ResultCode.UnknownTarget, "Unknown zone");

        /// <summary>Road and rail: the line check names the first problem and missing materials, then the job is planned.</summary>
        private ActionResult HandleBuildRoad(Village village, GameAction action, string kind, Cat cat)
        {
            string line = kind == "buildroad" ? JobKind.Road : JobKind.Rail;
            var check = CheckLine(village, line, action.Position, action.End);
            if (!check.Ok)
                return PlacementFailure(check, check.Code == PlacementCode.ResearchLocked ? "Rail research required" : check.Code == PlacementCode.OutsideClaim ? "Foreign territory cannot be modified" : check.Code == PlacementCode.NotConnected ? "Road must join shrine road network" : "Route must be mapped, free, dry, and unclaimed");
            if (check.MissingMaterials.Count > 0)
                return MissingMaterialFailure(check.MissingMaterials[0]);
            return PlanLinear(village, action, line, cat);
        }
        private ActionResult HandleBuildBridge(Village village, GameAction action, Cat cat)
        {
            var bill = InfrastructureCost("bridge").Inputs[0];
            return CreateInfrastructure(village, action, cat, "bridge", bill.Resource, bill.Amount);
        }
        private ActionResult HandleBuildDock(Village village, GameAction action, Cat cat)
        {
            var bill = InfrastructureCost("dock").Inputs[0];
            return CreateInfrastructure(village, action, cat, "dock", bill.Resource, bill.Amount);
        }

        /// <summary>
        /// Cancels a building that is still under construction: every builder stops (keeping nothing), all claims
        /// are released, the staged and carried materials go back to the nearest storage with room (a spill at
        /// the site when none has), and the scaffold is removed. Router kind: <c>CancelConstruction</c>.
        /// </summary>
        private ActionResult HandleCancelConstruction(Village village, GameAction action)
        {
            var b = TargetBuilding(village, action);
            if (b == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Unknown building");
            if (b.Kind == "shrine")
                return ActionResult.Fail(PlacementCode.ProtectedBuilding, "The shrine cannot be removed");
            if (b.Completed)
                return ActionResult.Fail(PlacementCode.NotUnderConstruction, "Only a building under construction can be cancelled");
            var refund = new List<Stack>();
            foreach (var s in b.Inputs)
                Add(refund, s.Resource, s.Amount);
            foreach (var job in ScaffoldJobs(village, b).ToArray())
            {
                var builder = village.Cats.Find(x => x.Id == job.CatId && job.CatId != "");
                if (builder != null)
                {
                    foreach (var s in builder.Cargo)
                        Add(refund, s.Resource, s.Amount);
                    builder.Cargo.Clear();
                    builder.JobId = "";
                    if (builder.ResumeJobId == job.Id)
                        builder.ResumeJobId = "";
                    builder.Path.Clear();
                    builder.Goal = GoalKey.Idle;
                    builder.BlockedReason = "";
                }
                foreach (var s in job.Local)
                    Add(refund, s.Resource, s.Amount);
                job.Local.Clear();
                Release(job.Id);
                Release(job.Id + ":resume");
                job.Completed = true;
                job.CatId = "";
                job.BlockedReason = "";
            }
            Release(b.Id);
            village.Buildings.Remove(b);
            ReturnToStorage(village, b.Position, refund);
            return ActionResult.Ok(ResultCode.Cancelled, b.Id, RefundArgs(b, refund));
        }

        /// <summary>
        /// Demolishes a completed building and returns half of its bill to the nearest storage. Refused for the
        /// shrine, a den whose beds are taken, and a station whose store has claims on it. Workers stop, station
        /// goods go back to storage and the station's own store becomes a spill if it still holds anything.
        /// Router kind: <c>Demolish</c>.
        /// </summary>
        private ActionResult HandleDemolish(Village village, GameAction action)
        {
            var b = TargetBuilding(village, action);
            if (b == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Unknown building");
            if (b.Kind == "shrine")
                return ActionResult.Fail(PlacementCode.ProtectedBuilding, "The shrine cannot be removed");
            if (!b.Completed)
                return ActionResult.Fail(PlacementCode.NotComplete, "Cancel construction instead of demolishing");
            if (b.Kind == "den")
            {
                int beds = (village.Buildings.Count(x => x.Kind == "den" && x.Completed) - 1) * DenCapacity(village);
                if (village.Cats.Any(c => c.Alive && c.BedId == b.Id) || village.Cats.Count(c => c.Alive && c.BedId != "") + village.Cats.Count(c => c.Alive && c.PregnantUntil > 0) > beds)
                    return ActionResult.Fail(PlacementCode.OccupiedDen, "Cats sleep in this den");
            }
            var stores = village.Stockpiles.Where(s => s.ManagedBy == b.Id).ToArray();
            if (stores.Any(s => Reservations.Any(r => r.PileId == s.Id) || village.Jobs.Any(j => !j.Completed && j.Phase == JobPhase.ItemFetch && j.SourceId == s.Id)))
                return ActionResult.Fail(PlacementCode.PileClaimed, "Goods in this building are being fetched");
            foreach (var c in village.Cats.Where(c => c.Alive && (c.BuildingId == b.Id || b.WorkerId == c.Id || b.ExtraWorkerIds.Contains(c.Id) || b.Slots.Any(s => s.CatId == c.Id) || village.Jobs.Any(j => !j.Completed && j.CatId == c.Id && j.TargetId == b.Id))).ToArray())
                CancelWork(village, c);
            foreach (var job in village.Jobs.Where(j => !j.Completed && j.TargetId == b.Id).ToArray())
            {
                Release(job.Id);
                job.Completed = true;
                job.CatId = "";
            }
            if (village.Accounting?.BuildingId == b.Id)
                village.Accounting = null;
            foreach (var store in stores)
            {
                if (store.Goods.Count > 0 || village.Items.Any(i => i.LocationId == store.Id))
                {
                    store.Kind = "spill";
                    store.ManagedBy = "";
                    store.Accepts.Clear();
                    store.ExpiresAt = -1;
                }
                else
                    village.Stockpiles.Remove(store);
            }
            village.Buildings.Remove(b);
            var refund = DemolishRefund(village, b);
            var held = new List<Stack>();
            foreach (var s in b.Inputs.Concat(b.Outputs))
                Add(held, s.Resource, s.Amount);
            b.Inputs.Clear();
            b.Outputs.Clear();
            ReturnToStorage(village, b.Position, held);
            ReturnToStorage(village, b.Position, refund);
            return ActionResult.Ok(ResultCode.Demolished, b.Id, RefundArgs(b, refund));
        }

        private static Building TargetBuilding(Village village, GameAction action)
        {
            string id = string.IsNullOrEmpty(action.BuildingId) ? action.TargetId : action.BuildingId;
            return village.Buildings.Find(x => x.Id == id);
        }

        private static (string key, string value)[] RefundArgs(Building b, List<Stack> refund) => new[] { ("kind", b.Kind) }.Concat(refund.Select(s => (s.Resource, s.Amount.ToString(CultureInfo.InvariantCulture)))).ToArray();

        /// <summary>
        /// Puts goods into the nearest reachable storage with room, one resource at a time; what fits nowhere is
        /// left as a spill at <paramref name="from"/>.
        /// </summary>
        private void ReturnToStorage(Village v, Int2 from, List<Stack> goods)
        {
            var leftover = new List<Stack>();
            foreach (var s in goods.Where(s => s.Amount > 1e-9))
            {
                var pile = Storage(v, s.Resource, s.Amount, from);
                if (pile != null)
                    Add(pile.Goods, s.Resource, s.Amount);
                else
                    Add(leftover, s.Resource, s.Amount);
            }
            Spill(v, from, leftover);
        }

        /// <summary>
        /// The site rules for a footprint, with today's tile reads (<c>TileAt</c>). Housing automation and scaffold
        /// resolution use it; the placement check applies the same rules through <see cref="SiteTileCode"/>.
        /// </summary>
        private bool FreeSite(Village v, Int2 p, int width, int depth, bool exterior = false)
        {
            if (width < 1 || depth < 1 || width > 16 || depth > 16)
                return false;
            if (!CanModifyTerrain(v.Id, p, width, depth))
                return false;
            for (int x = 0; x < width; x++)
                for (int z = 0; z < depth; z++)
                    if (SiteTileCode(v, new Int2(p.X + x, p.Z + z), exterior, TileAt) != "")
                        return false;
            return true;
        }
        private static bool Contains(Int2 p, int w, int d, Int2 at) => at.Level == p.Level && at.X >= p.X && at.X < p.X + w && at.Z >= p.Z && at.Z < p.Z + d;

        /// <summary>
        /// Plans a building at <paramref name="p"/>. The placement check decides legality. Classic worlds then
        /// need the full bill and a free builder now (today's rule). Demo worlds always place the scaffold: it
        /// claims what it can and either starts with a free builder (<c>planned</c>) or waits for a builder or for
        /// materials (<c>queued</c>). A named cat that is busy is refused in both profiles.
        /// </summary>
        private ActionResult Plan(Village v, string kind, Int2 p, Cat c, int rotation = 0)
        {
            kind = Snake(kind);
            var check = CheckPlacement(v, kind, p, rotation);
            if (!check.Ok)
                return PlacementFailure(check, PlacementMessage(check.Code));
            var bill = BuildCost(v, kind);
            if (!DemoConstruction)
            {
                if (check.MissingMaterials.Count > 0)
                    return MissingMaterialFailure(check.MissingMaterials[0]);
                c = c ?? AvailableCat(v, "build");
                if (!FreeWorker(v, c))
                    return ActionResult.Fail(PlacementCode.BuilderBusy, "No available builder");
                var site = NewScaffold(kind, p, check, bill);
                if (site.Required.Any(s => !Reserve(v, site.Id, s.Resource, s.Amount)))
                {
                    Release(site.Id);
                    return ActionResult.Fail(ResultCode.MissingMaterial, "Construction inputs claimed");
                }
                v.Buildings.Add(site);
                StartJob(v, c, new Job { Kind = JobKind.Build, TargetId = site.Id, Position = p, RequiredWork = site.RequiredWork, Manual = true });
                return ActionResult.Ok(ResultCode.Planned, site.Id, ("kind", kind));
            }
            if (c != null && !FreeWorker(v, c))
                return ActionResult.Fail(PlacementCode.BuilderBusy, "No available builder");
            var b = NewScaffold(kind, p, check, bill);
            v.Buildings.Add(b);
            if (!SecureScaffoldInputs(v, b))
            {
                b.BlockedReason = Blocked.MissingScaffoldInput;
                return ActionResult.Ok(ResultCode.Queued, b.Id, ("kind", kind), ("reason", Blocked.MissingScaffoldInput));
            }
            var builder = c ?? AvailableCat(v, "build");
            StartScaffoldJob(v, b, builder);
            return builder != null ? ActionResult.Ok(ResultCode.Planned, b.Id, ("kind", kind)) : ActionResult.Ok(ResultCode.Queued, b.Id, ("kind", kind), ("reason", Blocked.WorkerRequired));
        }

        private Building NewScaffold(string kind, Int2 p, PlacementCheck check, BuildBill bill) => new Building { Id = Id(kind), Kind = kind, Position = p, Width = check.Width, Depth = check.Depth, Entrance = check.Entrance.Value, HasEntrance = true, RequiredWork = bill.Work, Required = bill.Inputs.Select(s => new Stack(s.Resource, s.Amount)).ToList() };

        private static string PlacementMessage(string code)
        {
            switch (code)
            {
                case PlacementCode.UnknownBuilding:
                    return "Unknown constructible building";
                case PlacementCode.ResearchLocked:
                    return "Research unlock required";
                case PlacementCode.EntranceBlocked:
                    return "Building entrance must touch the shrine-connected road network";
                case PlacementCode.OutsideClaim:
                    return "Site is outside the village claim";
                case PlacementCode.Unmapped:
                    return "Site is not mapped yet";
                case PlacementCode.Road:
                    return "Site is on a road";
                case PlacementCode.PendingWall:
                    return "Site is on a planned fence";
                case PlacementCode.Water:
                    return "Site is in water";
                case PlacementCode.Mountain:
                    return "Site is on rock";
                case PlacementCode.Wall:
                    return "Site is on a fence";
                default:
                    return "Site is occupied";
            }
        }

        /// <summary>A coded placement failure that names the first failing tile when there is one.</summary>
        private static ActionResult PlacementFailure(PlacementCheck check, string message)
        {
            var bad = check.Tiles.FirstOrDefault(t => t.Code != "");
            if (bad.Code == null)
                return ActionResult.Fail(check.Code, message);
            return ActionResult.Fail(check.Code, message, ("x", bad.Tile.X.ToString(CultureInfo.InvariantCulture)), ("z", bad.Tile.Z.ToString(CultureInfo.InvariantCulture)), ("tile", bad.Code));
        }

        private ActionResult DesignatePile(Village v, GameAction a, string kind)
        {
            bool fishing = kind == "designatefishingspot";
            string area = fishing ? "fishing" : kind == "zone" ? "zone_" + a.Resource : kind == "designatestockpile" ? "stockpile" : "gather";
            var check = CheckArea(v, area, a.Position, fishing ? a.Position : a.End);
            if (!check.Ok)
                return PlacementFailure(check, AreaMessage(area, check.Code));
            if (a.Accepts.Any(r => !Catalog.Resources.Contains(r) || r == "blessings"))
                return ActionResult.Fail(PlacementCode.InvalidResource, "Invalid stored resource");
            int width = check.Width, depth = check.Depth;
            var pile = new Stockpile { Id = Id("pile"), Position = check.Origin, Width = width, Depth = depth, Capacity = width * depth * 100, Kind = kind == "zone" ? "zone_" + a.Resource : kind == "designatestockpile" ? "storage" : fishing ? "fishing" : "gather", Accepts = new List<string>(a.Accepts) };
            if (kind == "designategatherspot")
                pile.Accepts = new List<string> { a.Resource };
            v.Stockpiles.Add(pile);
            return ActionResult.Ok(pile.Id);
        }

        private static string AreaMessage(string area, string code)
        {
            if (code == PlacementCode.OutsideClaim && area.StartsWith("zone_", StringComparison.Ordinal))
                return "Foreign territory cannot host this work site";
            switch (area)
            {
                case "fishing":
                    return code == PlacementCode.NoShore ? "Fishing needs a walkable shore" : "Invalid fishing area";
                case "zone_avoid":
                case "zone_gather":
                    return code == PlacementCode.AvoidBlocksAccess ? "Avoid zones cannot cover shrine roads or building entrances" : "Zone must be a bounded mapped area";
                case "farm":
                    return code == PlacementCode.FieldRequired ? "Completed Field required" : code == PlacementCode.NoHandoff ? "No adjacent physical harvest handoff" : "Farm requires reachable mapped exterior ground";
                default:
                    return "Invalid stockpile area";
            }
        }

        private ActionResult DesignateFarm(Village v, GameAction a)
        {
            string crop = string.IsNullOrEmpty(a.Resource) ? "grain" : a.Resource;
            if (crop == "herb")
                crop = "herbs";
            if (!new[] { "grain", "herbs", "catnip" }.Contains(crop))
                return ActionResult.Fail(PlacementCode.InvalidResource, "Unknown crop");
            var check = CheckArea(v, "farm", a.Position, a.End);
            if (!check.Ok)
                return PlacementFailure(check, AreaMessage("farm", check.Code));
            var f = new Farm { Id = Id("farm"), Position = check.Origin, Width = check.Width, Depth = check.Depth, Crop = crop, Handoff = FarmHandoff(v, check.Origin, check.Width, check.Depth).Value };
            v.Farms.Add(f);
            return ActionResult.Ok(f.Id);
        }
    }
}
