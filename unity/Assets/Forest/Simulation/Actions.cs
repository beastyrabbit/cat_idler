using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        public ActionResult Apply(PlayerContext context, GameAction action)
        {
            if (context == null || action == null)
                return ActionResult.Fail("Missing action or identity");
            if (!Finite(action.Amount) || !Finite(action.OtherAmount) || Math.Abs((long)action.Position.X) > 1000000 || Math.Abs((long)action.Position.Z) > 1000000 || Math.Abs((long)action.End.X) > 1000000 || Math.Abs((long)action.End.Z) > 1000000 || action.Accepts == null || action.Path == null || action.Path.Count > 1024 || action.Path.Any(p => Math.Abs((long)p.X) > 1000000 || Math.Abs((long)p.Z) > 1000000))
                return ActionResult.Fail("Invalid finite action arguments");
            string kind = (action.Kind ?? "").Replace("_", "").ToLowerInvariant();
            if (kind == "ensure")
                return ActionResult.Ok();
            if (string.IsNullOrWhiteSpace(context.PlayerId))
                return ActionResult.Fail("Authenticated identity required");
            var player = Players.Find(p => p.Id == context.PlayerId);
            if (player == null)
            {
                player = new Player { Id = context.PlayerId };
                Players.Add(player);
            }
            if (kind == "presence")
                return ActionResult.Ok();
            if (kind == "foundvillage")
            {
                if (Villages.Any(v => v.OwnerId == context.PlayerId && !v.Communal))
                    return ActionResult.Fail("One personal village per identity");
                if (string.IsNullOrWhiteSpace(action.Name) || action.Name.Length > 48)
                    return ActionResult.Fail("Village name requires 1–48 characters");
                uint hash = Hash(context.PlayerId);
                var center = new Int2(80 + (int)(hash % 8) * 80, 80 + (int)((hash / 8) % 8) * 80);
                while (Villages.Any(v => Int2.Distance(v.Center, center) < 48) || !CanFoundAt("", center, false))
                {
                    center.X += 80;
                    if (center.X > 1000000)
                        return ActionResult.Fail("No unclaimed founding site available");
                }
                var v = Found(Id("village"), action.Name.Trim(), context.PlayerId, center, false);
                Villages.Add(v);
                player.PersonalVillageId = player.SelectedVillageId = v.Id;
                return new ActionResult { Success = true, EntityId = v.Id, VillageId = v.Id };
            }
            if (kind == "joinvillage")
            {
                var target = Village(string.IsNullOrEmpty(action.TargetId) ? action.OtherVillageId : action.TargetId);
                if (!CanControl(context, target))
                    return ActionResult.Fail("Village ownership denied");
                player.SelectedVillageId = target.Id;
                return new ActionResult { Success = true, VillageId = target.Id };
            }
            var village = Village(string.IsNullOrEmpty(context.VillageId) ? player.SelectedVillageId : context.VillageId);
            if (!CanControl(context, village))
                return ActionResult.Fail("Village ownership denied");
            var cat = village.Cats.Find(c => c.Id == action.CatId && c.Alive);
            var building = village.Buildings.Find(b => b.Id == (string.IsNullOrEmpty(action.BuildingId) ? action.TargetId : action.BuildingId));
            switch (kind)
            {
                case "settestacceleration":
                case "advancetime":
                case "settestrngseed":
                    if (!context.IsDeveloper)
                        return ActionResult.Fail("Developer control disabled");
                    if (kind == "advancetime")
                        Step(Math.Clamp(action.Amount, 0, 86400));
                    else if (kind == "settestrngseed")
                        RandomState = unchecked((uint)action.Amount);
                    else
                        return ActionResult.Fail("Explicit AdvanceTime replaces implicit accelerated clocks");
                    return ActionResult.Ok();
                case "requestjob":
                    return Request(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, cat, true, action.Resource);
                case "dispatchscout":
                    return Request(village, "scout", cat, true, action.Resource);
                case "boost":
                    return BoostJob(village, village.Jobs.Find(j => j.Id == action.TargetId && !j.Completed));
                case "boostcat":
                    if (cat == null)
                        return ActionResult.Fail("Living cat required");
                    cat.Boosted = action.Enabled;
                    return ActionResult.Ok();
                case "setcatlaborpreference":
                    if (cat == null || !Catalog.Labors.Contains(action.Labor))
                        return ActionResult.Fail("Living cat and maintained labor required");
                    cat.Preferences.Remove(action.Labor);
                    if (action.Enabled)
                        cat.Preferences.Add(action.Labor);
                    return ActionResult.Ok();
                case "planbuilding":
                    return Plan(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, action.Position, cat);
                case "assignworker":
                    return Assign(village, cat, building, action);
                case "assignofficer":
                    int role = Array.IndexOf(Catalog.Roles, action.Role);
                    if (role < 0 || cat == null)
                        return ActionResult.Fail("Unknown role or living cat");
                    if (!village.Research.Contains(Catalog.RoleStudies[role]) || !village.Buildings.Any(b => b.Kind == Catalog.RoleBuildings[role] && b.Completed))
                        return ActionResult.Fail("Office requires completed " + Catalog.RoleBuildings[role] + " and " + Catalog.RoleStudies[role]);
                    if (village.Officers.Any(o => o.CatId == cat.Id && o.Role != action.Role))
                        return ActionResult.Fail("Cat already holds an office");
                    foreach (var replaced in village.Officers.Where(o => o.Role == action.Role))
                    {
                        var previous = Cat(replaced.CatId);
                        if (previous != null)
                            previous.OfficerRole = "";
                    }
                    village.Officers.RemoveAll(o => o.Role == action.Role);
                    village.Officers.Add(new Officer { Role = action.Role, CatId = cat.Id });
                    cat.OfficerRole = action.Role;
                    return ActionResult.Ok();
                case "unassignofficer":
                    var holder = village.Officers.Find(o => o.Role == action.Role);
                    if (holder == null)
                        return ActionResult.Fail("Office already vacant");
                    var h = Cat(holder.CatId);
                    if (h != null)
                        h.OfficerRole = "";
                    village.Officers.Remove(holder);
                    StopOfficerWork(village, action.Role);
                    return ActionResult.Ok();
                case "researchnode":
                case "unlocknode":
                    return Research(village, action.NodeId, kind == "unlocknode");
                case "purchaseupgrade":
                    return PurchaseUpgrade(village, !string.IsNullOrEmpty(action.Name) ? action.Name : !string.IsNullOrEmpty(action.Resource) ? action.Resource : action.NodeId);
                case "editproductionqueue":
                case "editproductionworkslot":
                    return EditQueue(village, building, cat, action, kind == "editproductionworkslot");
                case "designatestockpile":
                case "designategatherspot":
                case "designatefishingspot":
                    return DesignatePile(village, action, kind);
                case "removestockpile":
                case "removegatherspot":
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
                case "haulgatherspot":
                    var source = village.Stockpiles.Find(s => s.Id == action.TargetId && (s.Goods.Any(g => g.Amount > 0) || village.Items.Any(i => i.LocationId == s.Id)));
                    if (source == null)
                        return ActionResult.Fail("No source goods");
                    return StartHaul(village, cat, source);
                case "designatefarm":
                    return DesignateFarm(village, action);
                case "clearfarm":
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
                case "createzone":
                    if (action.Resource != "avoid" && action.Resource != "gather")
                        return ActionResult.Fail("Zone kind must be avoid or gather");
                    return DesignatePile(village, action, "zone");
                case "removezone":
                    return village.Stockpiles.RemoveAll(s => s.Id == action.TargetId && s.Kind.StartsWith("zone_", StringComparison.Ordinal)) > 0 ? ActionResult.Ok() : ActionResult.Fail("Unknown zone");
                case "offertithe":
                    if (Total(village, "food") < village.Cats.Count(c => c.Alive) * 6 + 10)
                        return ActionResult.Fail("Food reserve protected");
                    if (!Spend(village, "food", 10))
                        return ActionResult.Fail("Food claimed");
                    village.Blessings += 1;
                    Note(village, "tithe", "Food tithe earned one blessing");
                    return ActionResult.Ok();
                case "offermaterials":
                case "offerresource":
                    string offering = kind == "offermaterials" ? "materials" : action.Resource;
                    if (!new[] { "food", "herbs", "materials" }.Contains(offering))
                        return ActionResult.Fail("Offering must be food, herbs, or materials");
                    if (Total(village, offering) < (offering == "food" ? village.Cats.Count(c => c.Alive) * 6 : 10) + 5)
                        return ActionResult.Fail("Survival and construction reserve protected");
                    return CreateInputJob(village, cat, "offering", village.Center, offering, 5, 10, "");
                case "trainwarrior":
                    return Request(village, "train", cat, true);
                case "defendraid":
                    if (village.Raids.Count == 0)
                        return ActionResult.Fail("No active raid");
                    village.Raids[0].Health -= 6;
                    return ActionResult.Ok();
                case "castvote":
                    if (village.Election == null || cat == null)
                        return ActionResult.Fail("Open election and living candidate required");
                    village.Election.Votes.RemoveAll(b => b.PlayerId == context.PlayerId);
                    village.Election.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = cat.Id });
                    return ActionResult.Ok();
                case "requestvotekick":
                    if (!village.Cats.Any(c => c.Id == village.LeaderId && c.Alive))
                        return ActionResult.Fail("Living leader required");
                    if (village.KickPetition == null)
                        village.KickPetition = new Election { Id = Id("petition"), Kind = "vote_kick", TargetId = village.LeaderId, EndAt = TimeSeconds + 600 };
                    if (village.KickPetition.EndAt <= TimeSeconds)
                        return ActionResult.Fail("Petition closed");
                    if (!village.KickPetition.Votes.Any(b => b.PlayerId == context.PlayerId))
                        village.KickPetition.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = village.KickPetition.TargetId });
                    return ActionResult.Ok(village.KickPetition.Id);
                case "buildroad":
                case "designaterail":
                    return PlanLinear(village, action, kind == "buildroad" ? "road" : "rail", cat);
                case "buildbridge":
                    return CreateInfrastructure(village, action, cat, "bridge", "lumber", 4);
                case "builddock":
                    return CreateInfrastructure(village, action, cat, "dock", "lumber", 8);
                case "buildtransportvehicle":
                    return CreateInfrastructure(village, action, cat, action.Mode == "shipping" ? "vessel" : "wagon", action.Mode == "shipping" ? "lumber" : "metal", 8);
                case "createtransportroute":
                    return CreateRoute(village, action, cat);
                case "canceltransportroute":
                    return CancelRoute(village, action.TargetId);
                case "offervillagetrade":
                    return OfferTrade(village, action);
                case "acceptvillagetrade":
                    return AcceptTrade(village, action.TargetId);
                case "cancelvillagetrade":
                    return CancelTrade(village, action.TargetId);
                case "buyresource":
                    return Buy(village, action);
                case "sellgoods":
                    return Sell(village, action);
                case "equipitem":
                case "unequipitem":
                case "repairitem":
                    return Equipment(village, cat, action, kind);
                case "entercatcontrol":
                case "entercat":
                    if (cat == null)
                        return ActionResult.Fail("Living cat required");
                    if (cat.ControlledBy != "" && cat.ControlledBy != context.PlayerId && cat.ControlLeaseUntil > TimeSeconds)
                        return ActionResult.Fail("Cat controlled by another player");
                    foreach (var other in village.Cats.Where(c => c.ControlledBy == context.PlayerId))
                    {
                        other.ControlledBy = "";
                        other.Goal = "idle";
                    }
                    CancelWork(village, cat, preserveUnassignedCargo: true);
                    cat.ControlledBy = context.PlayerId;
                    cat.ControlLeaseUntil = TimeSeconds + 30;
                    cat.Goal = "player_control";
                    return ActionResult.Ok(cat.Id);
                case "leavecatcontrol":
                case "leavecat":
                    if (cat == null || cat.ControlledBy != context.PlayerId)
                        return ActionResult.Fail("Control lease required");
                    cat.ControlledBy = "";
                    cat.Goal = "idle";
                    return ActionResult.Ok(cat.Id);
                case "keepcatcontrol":
                    if (cat == null || cat.ControlledBy != context.PlayerId || cat.ControlLeaseUntil <= TimeSeconds)
                        return ActionResult.Fail("Existing control lease required");
                    cat.ControlLeaseUntil = TimeSeconds + 30;
                    return ActionResult.Ok();
                case "movecat":
                    if (cat == null || cat.ControlledBy != context.PlayerId)
                        return ActionResult.Fail("Control lease required");
                    if (village.Routes.Any(r => r.CatId == cat.Id && r.CancelRequested && r.PathIndex > 0))
                        return ActionResult.Fail("Vessel is physically returning this cat to its dock");
                    if (Math.Abs(action.Position.X) + Math.Abs(action.Position.Z) != 1)
                        return ActionResult.Fail("Movement must be one cardinal direction");
                    var next = new Int2(cat.Position.X + action.Position.X, cat.Position.Z + action.Position.Z);
                    if (!Walkable(village, next) || !Crossable(cat.Position, next))
                        return ActionResult.Fail("Blocked route");
                    cat.Path = new List<Int2> { next };
                    cat.ControlLeaseUntil = TimeSeconds + 30;
                    return ActionResult.Ok();
                case "interactcat":
                    if (cat == null || cat.ControlledBy != context.PlayerId)
                        return ActionResult.Fail("Control lease required");
                    return Interact(village, cat, action);
                default:
                    return ActionResult.Fail("Unknown action: " + action.Kind);
            }
        }
        private ActionResult Research(Village v, string id, bool blessings)
        {
            var node = Catalog.Study(id);
            if (node == null)
                return ActionResult.Fail("Unknown study");
            if (v.Research.Contains(id))
                return ActionResult.Fail("Already researched");
            if (node.Prerequisites.Any(p => !v.Research.Contains(p)))
                return ActionResult.Fail("Missing prerequisites");
            if (blessings && Catalog.Research.IndexOf(node) >= 24)
                return ActionResult.Fail("Blessings purchase original studies only");
            if ((blessings ? v.Blessings : v.ResearchPoints) < node.Cost)
                return ActionResult.Fail("Insufficient " + (blessings ? "blessings" : "research points"));
            if (blessings)
                v.Blessings -= node.Cost;
            else
                v.ResearchPoints -= node.Cost;
            v.Research.Add(id);
            Note(v, "research", node.Name, id);
            return ActionResult.Ok(id);
        }
        private ActionResult EditQueue(Village v, Building b, Cat c, GameAction a, bool exact)
        {
            if (b == null || !b.Completed)
                return ActionResult.Fail("Completed station required");
            if (exact && (c == null || !b.Slots.Any(s => s.CatId == c.Id)))
                return ActionResult.Fail("Exact staffed slot required");
            var slot = exact ? b.Slots.Find(s => s.CatId == c.Id) : b.Slots.FirstOrDefault();
            var queue = slot?.Queue ?? b.Queue;
            if (slot != null)
                slot.AutomatedBy = "";
            foreach (var entry in queue)
                entry.AutomatedBy = "";
            string edit = (a.Edit ?? "").Replace("_", "").ToLowerInvariant();
            if (edit == "add")
            {
                var r = Catalog.Recipe(a.RecipeId);
                if (r == null || r.Building != b.Kind || !Catalog.RecipeAvailable(v, r))
                    return ActionResult.Fail("Recipe locked or wrong station");
                if (queue.Count >= 64)
                    return ActionResult.Fail("Queue limit reached");
                queue.Add(new QueueEntry { RecipeId = r.Id, Repeat = a.Repeat });
            }
            else if (edit == "setpaused" || edit == "pause")
            {
                if (slot != null)
                    slot.Paused = a.Enabled;
                else
                    b.Paused = a.Enabled;
            }
            else
            {
                if (a.Index < 0 || a.Index >= queue.Count)
                    return ActionResult.Fail("Queue index out of range");
                if (edit == "remove")
                    queue.RemoveAt(a.Index);
                else if (edit == "setrepeat" || edit == "repeat")
                    queue[a.Index].Repeat = a.Enabled;
                else if (edit == "move")
                {
                    int to = a.Index + (a.Direction < 0 ? -1 : 1);
                    if (to < 0 || to >= queue.Count)
                        return ActionResult.Fail("Queue edge");
                    var q = queue[a.Index];
                    queue.RemoveAt(a.Index);
                    queue.Insert(to, q);
                }
                else
                    return ActionResult.Fail("Unknown queue edit");
            }
            if (slot != null && b.Slots.IndexOf(slot) == 0)
                b.Queue = slot.Queue;
            return ActionResult.Ok();
        }
        private ActionResult Assign(Village v, Cat c, Building b, GameAction a)
        {
            if (c == null || c.ControlledBy != "")
                return ActionResult.Fail("Available living cat required");
            if (v.Routes.Any(r => r.CatId == c.Id && r.Mode == "shipping" && r.PathIndex > 0))
                return ActionResult.Fail("Return the vessel to its dock before reassigning its driver");
            if (b == null && a.TargetId != "")
            {
                var farm = v.Farms.Find(f => f.Id == a.TargetId);
                if (farm != null)
                {
                    var previous = v.Cats.Find(x => x.Id == farm.WorkerId);
                    if (previous != null && previous.Id != c.Id)
                        CancelWork(v, previous);
                    CancelWork(v, c);
                    farm.WorkerId = c.Id;
                    c.BuildingId = farm.Id;
                    return ActionResult.Ok();
                }
            }
            if (b != null && !b.Completed)
            {
                if (v.Jobs.Any(j => !j.Completed && j.TargetId == b.Id))
                    return ActionResult.Fail("Scaffold already has a builder");
                CancelWork(v, c);
                StartJob(v, c, new Job { Kind = "build", TargetId = b.Id, Position = b.Position, RequiredWork = b.RequiredWork, Manual = true });
                return ActionResult.Ok();
            }
            if (b != null && b.Slots.Count(s => s.CatId != "" && s.CatId != c.Id) >= (int)Catalog.BuildingEffect(v, b.Kind, "worker_slots", 1))
                return ActionResult.Fail("All work positions staffed");
            CancelWork(v, c);
            if (b == null)
                return ActionResult.Ok();
            var vacancy = b.Slots.FirstOrDefault(s => s.CatId == "");
            if (vacancy != null)
            {
                vacancy.CatId = c.Id;
                vacancy.AutomatedBy = "";
            }
            else
                b.Slots.Add(new WorkSlot { CatId = c.Id, Queue = b.Slots.Count == 0 ? b.Queue : new List<QueueEntry>() });
            if (b.WorkerId == "")
                b.WorkerId = c.Id;
            else
                b.ExtraWorkerIds.Add(c.Id);
            c.BuildingId = b.Id;
            if (b.Kind == "field")
            {
                var plot = v.Farms.FirstOrDefault(f => f.WorkerId == "");
                if (plot != null)
                    plot.WorkerId = c.Id;
            }
            return ActionResult.Ok();
        }
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
                    if (v.Jobs.Any(j => !j.Completed && (j.Kind == "road" || j.Kind == "rail") && j.Path.Contains(at)))
                        return false;
                    if (exterior ? t.ClaimId != "" : t.ClaimId != v.Id)
                        return false;
                    if (v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, at)) || v.Stockpiles.Any(s => Contains(s.Position, s.Width, s.Depth, at)) || v.Farms.Any(f => Contains(f.Position, f.Width, f.Depth, at)))
                        return false;
                }
            return true;
        }
        private static bool Contains(Int2 p, int w, int d, Int2 at) => at.X >= p.X && at.X < p.X + w && at.Z >= p.Z && at.Z < p.Z + d;
        private ActionResult Plan(Village v, string kind, Int2 p, Cat c)
        {
            kind = Snake(kind);
            if (!Catalog.Buildings.Contains(kind) || kind == "shrine")
                return ActionResult.Fail("Unknown constructible building");
            if (!Catalog.BuildingAvailable(v, kind))
                return ActionResult.Fail("Research unlock required");
            if (!FreeSite(v, p, 2, 2, kind == "field"))
                return ActionResult.Fail("Occupied, unmapped, or invalid footprint");
            var entry = BuildingEntrance(v, new Building { Position = p });
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
        private ActionResult Equipment(Village v, Cat c, GameAction a, string kind)
        {
            var item = v.Items.Find(i => i.Id == a.TargetId);
            if (item == null)
                return ActionResult.Fail("Exact item identity required");
            if (kind == "repairitem")
            {
                string station = item.Material == "metal" ? "smithy" : item.Material == "fibre" ? "clothier" : item.Material == "leather" ? "tannery" : "woodworking";
                if (!v.Stockpiles.Any(p => p.Id == item.LocationId) || !v.Buildings.Any(b => b.Kind == station && b.Completed && b.Slots.Any(s => Cat(s.CatId)?.Alive == true)))
                    return ActionResult.Fail("Stored item and staffed repair station required");
                string material = item.Material == "wood" ? "planks" : item.Material;
                if (!Spend(v, material, 1))
                    return ActionResult.Fail("Matching repair material required");
                item.Condition = item.MaxCondition;
                return ActionResult.Ok();
            }
            if (c == null)
                return ActionResult.Fail("Living bearer required");
            if (kind == "equipitem")
            {
                if (!new[] { "tool", "weapon", "armor" }.Contains(item.Kind) || item.Condition <= 0 || !v.Stockpiles.Any(p => p.Id == item.LocationId) || v.Items.Any(i => i.LocationId == c.Id && i.Kind == item.Kind))
                    return ActionResult.Fail("Unavailable or occupied equipment slot");
                item.LocationId = c.Id;
                c.Equipment.Add(item.Id);
            }
            else
            {
                if (item.LocationId != c.Id || !c.Equipment.Contains(item.Id))
                    return ActionResult.Fail("Item not equipped by this cat");
                var storage = Storage(v, ItemResource(item.Kind), 1, c.Position);
                if (storage == null)
                    return ActionResult.Fail("No reachable equipment storage");
                item.LocationId = storage.Id;
                c.Equipment.Remove(item.Id);
            }
            return ActionResult.Ok();
        }
        private static string ItemResource(string kind) => kind == "tool" ? "tools" : kind == "weapon" ? "weapons" : kind == "armor" ? "armor" : kind + "s";
        private ActionResult Interact(Village v, Cat c, GameAction a)
        {
            if (a.Mode == "eat" || a.Mode == "drink" || a.Mode == "rest")
                return ControlledNeed(v, c, a.Mode);
            var pile = v.Stockpiles.Find(s => s.Id == a.TargetId);
            if (pile != null)
            {
                if (Int2.Distance(c.Position, pile.Position) > 1)
                    return ActionResult.Fail("Walk to the pile first");
                if (c.Cargo.Count > 0)
                {
                    if (c.Cargo.Any(s => !HasRoom(v, pile, s.Resource, s.Amount)) || c.Cargo.Where(s => !pile.ResourceLimits.Any(limit => limit.Resource == s.Resource)).Any(s => PileLoad(v, pile) + c.Cargo.Sum(g => g.Amount) > ResourceCapacity(v, pile, s.Resource)))
                        return ActionResult.Fail("Storage full or incompatible");
                    foreach (var s in c.Cargo)
                        Add(pile.Goods, s.Resource, s.Amount);
                    c.Cargo.Clear();
                    return ActionResult.Ok();
                }
                string resource = a.Resource;
                double amount = Math.Min(8, Math.Max(1, a.Amount));
                double free = Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
                if (free < amount)
                    return ActionResult.Fail("Unclaimed goods unavailable");
                Add(pile.Goods, resource, -amount);
                Add(c.Cargo, resource, amount);
                return ActionResult.Ok();
            }
            if (Int2.Distance(c.Position, v.Center) <= 1 && c.Cargo.Count > 0)
            {
                foreach (var s in c.Cargo.ToArray())
                    if (new[] { "food", "herbs", "materials" }.Contains(s.Resource))
                    {
                        v.Blessings += s.Amount / 5;
                        c.Cargo.Remove(s);
                    }
                return ActionResult.Ok();
            }
            return ActionResult.Fail("No interactable target within reach");
        }
        public static string Snake(string value) => string.Concat((value ?? "").Select((ch, i) => char.IsUpper(ch) ? (i > 0 ? "_" : "") + char.ToLowerInvariant(ch) : ch.ToString())).Replace(" ", "_");
    }
}
