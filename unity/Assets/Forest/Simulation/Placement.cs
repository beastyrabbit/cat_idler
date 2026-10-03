using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Stable codes for placement checks and construction verbs, carried in <see cref="PlacementCheck.Code"/>,
    /// per-tile entries and <see cref="ActionResult.Code"/>. Values are UI-text keys: never rename one.
    /// </summary>
    public static class PlacementCode
    {
        // Per tile (the build ghost colours each tile by these).
        public const string Unmapped = "unmapped";
        public const string OutsideClaim = "outside_claim";
        public const string Occupied = "occupied";
        public const string Road = "road";
        public const string Water = "water";
        public const string Mountain = "mountain";
        public const string Wall = "wall";
        public const string PendingWall = "pending_wall";

        // Whole placement.
        public const string EntranceBlocked = "entrance_blocked";
        public const string ResearchLocked = "research_locked";
        public const string UnknownBuilding = "unknown_building";
        public const string InvalidArea = "invalid_area";
        public const string TooLong = "too_long";
        public const string NotConnected = "not_connected";
        public const string NoShore = "no_shore";
        public const string FieldRequired = "field_required";
        public const string Unreachable = "unreachable";
        public const string NoHandoff = "no_handoff";
        public const string InvalidZone = "invalid_zone";
        public const string InvalidResource = "invalid_resource";
        public const string AvoidBlocksAccess = "avoid_blocks_access";
        public const string BuilderBusy = "builder_busy";

        // Construction verbs (cancel and demolish).
        public const string NotUnderConstruction = "not_under_construction";
        public const string NotComplete = "not_complete";
        public const string ProtectedBuilding = "protected_building";
        public const string OccupiedDen = "occupied_den";
        public const string PileClaimed = "pile_claimed";
    }

    /// <summary>
    /// The side-effect-free answer to "may this be placed here?". The build ghost reads it every frame and the
    /// rules call the same function before they act, so the preview and the result cannot disagree.
    /// </summary>
    public sealed class PlacementCheck
    {
        public bool Ok;
        /// <summary>The first failure, in the order the action checks them; "" when <see cref="Ok"/>.</summary>
        public string Code = "";
        public string Kind = "";
        public Int2 Origin;
        public int Width = 1, Depth = 1, Rotation;
        /// <summary>Every footprint or path tile with its own code ("" when that tile is legal).</summary>
        public List<(Int2 Tile, string Code)> Tiles = new List<(Int2 Tile, string Code)>();
        /// <summary>The door tile a building would use (buildings only).</summary>
        public Int2? Entrance;
        public List<Stack> Cost = new List<Stack>();
        public double Work;
        public List<MaterialShortfall> MissingMaterials = new List<MaterialShortfall>();
        public bool BuilderAvailable;
    }

    public partial class World
    {
        /// <summary>Longest road or rail a line placement may cover, in tiles.</summary>
        public const int MaxLineTiles = 128;

        /// <summary>
        /// Checks a building placement without changing the world: per-tile codes, the entrance, the bill, missing
        /// materials and whether a builder is free. Plan calls it and acts only when it is <c>Ok</c>.
        /// </summary>
        public PlacementCheck CheckPlacement(Village v, string kind, Int2 origin, int rotation = 0)
        {
            kind = Snake(kind ?? "");
            rotation = (rotation % 4 + 4) % 4;
            var probe = new Building();
            int width = rotation % 2 == 0 ? probe.Width : probe.Depth, depth = rotation % 2 == 0 ? probe.Depth : probe.Width;
            var check = new PlacementCheck { Kind = kind, Origin = origin, Rotation = rotation, Width = width, Depth = depth };
            bool known = Catalog.Buildings.Contains(kind) && kind != "shrine";
            bool exterior = kind == "field";
            bool foreign = false;
            string first = "";
            for (int x = 0; x < width; x++)
                for (int z = 0; z < depth; z++)
                {
                    var at = new Int2(origin.X + x, origin.Z + z);
                    bool outside = !CanModifyTerrain(v.Id, at);
                    foreign |= outside;
                    string code = outside ? PlacementCode.OutsideClaim : SiteTileCode(v, at, exterior, GetTile);
                    if (first == "")
                        first = code;
                    check.Tiles.Add((at, code));
                }
            check.Entrance = FreeBuildingEntrance(v, origin, width, depth);
            check.BuilderAvailable = AvailableCat(v, "build") != null;
            if (known)
            {
                var bill = BuildCost(v, kind);
                check.Cost = bill.Inputs;
                check.Work = bill.Work;
                check.MissingMaterials = Shortfalls(v, bill.Inputs);
            }
            check.Code = !known ? PlacementCode.UnknownBuilding : !Catalog.BuildingAvailable(this, v, kind) ? PlacementCode.ResearchLocked : foreign ? PlacementCode.OutsideClaim : first != "" ? first : !check.Entrance.HasValue ? PlacementCode.EntranceBlocked : "";
            check.Ok = check.Code == "";
            return check;
        }

        /// <summary>
        /// Checks a road or rail line along the L path from <paramref name="from"/> to <paramref name="to"/>
        /// (<see cref="LineFootprint"/>) with the rules of the road and rail actions, without changing the world.
        /// </summary>
        public PlacementCheck CheckLine(Village v, string kind, Int2 from, Int2 to)
        {
            kind = Snake(kind ?? "");
            var path = LineFootprint(from, to);
            var check = new PlacementCheck { Kind = kind, Origin = from, Width = Math.Abs(to.X - from.X) + 1, Depth = Math.Abs(to.Z - from.Z) + 1 };
            bool line = kind == JobKind.Road || kind == JobKind.Rail;
            bool reached = path[path.Count - 1].Equals(to) && path.Count <= MaxLineTiles;
            string first = "";
            bool foreign = false;
            foreach (var at in path)
            {
                string code = line ? LineTileCode(v, kind, at) : "";
                bool outside = !CanModifyTerrain(v.Id, at);
                foreign |= outside;
                if (first == "")
                    first = code;
                check.Tiles.Add((at, code != "" ? code : outside ? PlacementCode.OutsideClaim : ""));
            }
            bool connected = kind != JobKind.Road || ConnectedToRoads(v, path);
            check.BuilderAvailable = AvailableCat(v, "build") != null;
            if (line)
            {
                var bill = LineCost(kind, path.Count);
                check.Cost = bill.Inputs;
                check.Work = bill.Work;
                check.MissingMaterials = Shortfalls(v, bill.Inputs);
            }
            check.Code = !line ? PlacementCode.UnknownBuilding : kind == JobKind.Rail && !Catalog.Owns(this, v, "unlock_capability", "rail_logistics") ? PlacementCode.ResearchLocked : !reached ? PlacementCode.TooLong : first != "" ? first : foreign ? PlacementCode.OutsideClaim : !connected ? PlacementCode.NotConnected : "";
            check.Ok = check.Code == "";
            return check;
        }

        /// <summary>
        /// Checks an area designation between two corners: <c>stockpile</c> (inside the claim), <c>gather</c>
        /// (outside it), <c>fishing</c> (one shore tile at <paramref name="from"/>), <c>zone_gather</c>,
        /// <c>zone_avoid</c> and <c>farm</c>, with the rules of the matching designate action. A farm also needs
        /// a path from the shrine, so it runs a path search.
        /// </summary>
        public PlacementCheck CheckArea(Village v, string kind, Int2 from, Int2 to)
        {
            kind = Snake(kind ?? "");
            var origin = new Int2(Math.Min(from.X, to.X), Math.Min(from.Z, to.Z));
            int width = Math.Abs(to.X - from.X) + 1, depth = Math.Abs(to.Z - from.Z) + 1;
            if (kind == "fishing")
            {
                origin = from;
                width = depth = 1;
            }
            var check = new PlacementCheck { Kind = kind, Origin = origin, Width = width, Depth = depth };
            check.BuilderAvailable = AvailableCat(v, "build") != null;
            bool zone = kind == "zone_gather" || kind == "zone_avoid";
            if (!zone && kind != "stockpile" && kind != "gather" && kind != "fishing" && kind != "farm")
            {
                check.Code = PlacementCode.InvalidZone;
                return check;
            }
            if (width > 16 || depth > 16)
            {
                check.Code = PlacementCode.InvalidArea;
                return check;
            }
            bool exterior = kind == "gather" || kind == "farm" || kind == "fishing" && GetTile(origin)?.ClaimId != v.Id;
            bool foreign = false;
            string first = "";
            for (int x = 0; x < width; x++)
                for (int z = 0; z < depth; z++)
                {
                    var at = new Int2(origin.X + x, origin.Z + z);
                    bool outside = !CanModifyTerrain(v.Id, at);
                    foreign |= outside;
                    // A zone is checked for mapping first and territory second; sites check territory first.
                    string own = zone ? (v.Known.Contains(at) ? "" : PlacementCode.Unmapped) : outside ? "" : SiteTileCode(v, at, exterior, GetTile);
                    string code = zone ? (own != "" ? own : outside ? PlacementCode.OutsideClaim : "") : outside ? PlacementCode.OutsideClaim : own;
                    if (first == "")
                        first = own;
                    check.Tiles.Add((at, code));
                }
            if (kind == "farm" && !v.Buildings.Any(b => b.Kind == "field" && b.Completed))
                check.Code = PlacementCode.FieldRequired;
            else if (zone)
                check.Code = first != "" ? first : foreign ? PlacementCode.OutsideClaim : kind == "zone_avoid" && AvoidCoversAccess(v, origin, width, depth) ? PlacementCode.AvoidBlocksAccess : "";
            else if (foreign)
                check.Code = PlacementCode.OutsideClaim;
            else if (first != "")
                check.Code = first;
            else if (kind == "fishing" && !Neighbors(origin).Any(n => GetTile(n)?.Water == true))
                check.Code = PlacementCode.NoShore;
            else if (kind == "farm" && Path(v.Center, origin) == null)
                check.Code = PlacementCode.Unreachable;
            else if (kind == "farm" && !FarmHandoff(v, origin, width, depth).HasValue)
                check.Code = PlacementCode.NoHandoff;
            check.Ok = check.Code == "";
            return check;
        }

        /// <summary>
        /// The L-shaped line from <paramref name="from"/> to <paramref name="to"/>, X first, then Z. Stops after
        /// <paramref name="maxTiles"/> tiles, so a line that is too long does not end at <paramref name="to"/>.
        /// </summary>
        public static List<Int2> LineFootprint(Int2 from, Int2 to, int maxTiles = MaxLineTiles + 1)
        {
            var path = new List<Int2> { from };
            var p = from;
            while (!p.Equals(to) && path.Count < maxTiles)
            {
                if (p.X != to.X)
                    p.X += Math.Sign(to.X - p.X);
                else
                    p.Z += Math.Sign(to.Z - p.Z);
                path.Add(p);
            }
            return path;
        }

        /// <summary>
        /// The shortest transport route a vehicle may run between two pile positions: over completed rail for
        /// <c>rail</c>, over water for <c>shipping</c> (the end may be dry). Reads existing tiles only; returns an
        /// empty list when no route exists.
        /// </summary>
        public List<Int2> PlanTransportPath(string mode, Int2 start, Int2 end)
        {
            var none = new List<Int2>();
            if (mode != "rail" && mode != "shipping" || start.Equals(end) || start.Level != end.Level)
                return none;
            bool Allowed(Int2 p)
            {
                var tile = GetTile(p);
                return tile != null && (mode == "rail" ? tile.Rail : p.Equals(end) || tile.Water);
            }
            if (mode == "rail" && !Allowed(start))
                return none;
            var parents = new Dictionary<Int2, Int2> { { start, start } };
            var queue = new Queue<Int2>();
            queue.Enqueue(start);
            while (queue.Count > 0 && parents.Count < 20000)
            {
                var at = queue.Dequeue();
                if (at.Equals(end))
                {
                    var path = new List<Int2>();
                    for (var p = at; !p.Equals(start); p = parents[p])
                        path.Add(p);
                    path.Add(start);
                    path.Reverse();
                    return path.Count <= 1024 ? path : none;
                }
                foreach (var next in Neighbors(at))
                    if (!parents.ContainsKey(next) && Allowed(next))
                    {
                        parents[next] = at;
                        queue.Enqueue(next);
                    }
            }
            return none;
        }

        /// <summary>
        /// The code of one footprint tile under the site rules (legal: ""). Reads the tile through
        /// <paramref name="read"/>: the placement check passes <c>GetTile</c> so it never creates terrain.
        /// The territory check is the caller's.
        /// </summary>
        private string SiteTileCode(Village v, Int2 at, bool exterior, Func<Int2, Tile> read)
        {
            var t = read(at);
            if (!v.Known.Contains(at) || t == null)
                return PlacementCode.Unmapped;
            if (!Walkable(at))
                return BarrierCode(t);
            if (RoadSurface(t) || t.Rail || ShrineRing(v, at))
                return PlacementCode.Road;
            if (v.Buildings.Any(b => b.HasEntrance && b.Entrance.Equals(at)))
                return PlacementCode.Occupied;
            if (PendingLine(v, at))
                return PlacementCode.Road;
            if (PendingExpansionWall(v, at))
                return PlacementCode.PendingWall;
            if (exterior ? t.ClaimId != "" : t.ClaimId != v.Id)
                return PlacementCode.OutsideClaim;
            if (v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, at)) || v.Stockpiles.Any(s => Contains(s.Position, s.Width, s.Depth, at)) || v.Farms.Any(f => Contains(f.Position, f.Width, f.Depth, at)))
                return PlacementCode.Occupied;
            return "";
        }

        /// <summary>The code of one road or rail tile under the line rules (legal: "").</summary>
        private string LineTileCode(Village v, string kind, Int2 at)
        {
            var t = GetTile(at);
            if (!v.Known.Contains(at) || t == null)
                return PlacementCode.Unmapped;
            if (!Walkable(at))
                return BarrierCode(t);
            if (!FreeInfrastructureFootprint(v, kind, at))
                return PlacementCode.Occupied;
            if (PendingExpansionWall(v, at))
                return PlacementCode.PendingWall;
            if (RoadSurface(t) || t.Rail || PendingLine(v, at))
                return PlacementCode.Road;
            return "";
        }

        private static string BarrierCode(Tile t) => t.Wall ? PlacementCode.Wall : t.Mountain ? PlacementCode.Mountain : PlacementCode.Water;

        private static bool PendingLine(Village v, Int2 at) => v.Jobs.Any(j => !j.Completed && (j.Kind == JobKind.Road || j.Kind == JobKind.Rail) && j.Path.Contains(at));

        private bool ConnectedToRoads(Village v, List<Int2> path)
        {
            var connected = ConnectedRoads(v);
            return path.Any(at => Neighbors(at).Any(connected.Contains));
        }

        /// <summary>True when an avoid zone over the area would cover a shrine road, an entrance or the shrine.</summary>
        private bool AvoidCoversAccess(Village v, Int2 p, int width, int depth) => ConnectedRoads(v).Any(at => Contains(p, width, depth, at)) || v.Buildings.Any(b => b.HasEntrance && Contains(p, width, depth, b.Entrance) || b.Kind == "shrine" && p.X < b.Position.X + b.Width && p.X + width > b.Position.X && p.Z < b.Position.Z + b.Depth && p.Z + depth > b.Position.Z);

        /// <summary>The tile beside a farm where its harvest is handed to haulers, or null when none is reachable.</summary>
        private Int2? FarmHandoff(Village v, Int2 p, int width, int depth)
        {
            foreach (var at in Neighbors(p))
                if (!Contains(p, width, depth, at) && CanModifyTerrain(v.Id, at) && Walkable(at) && Path(p, at) != null)
                    return at;
            return null;
        }
    }
}
