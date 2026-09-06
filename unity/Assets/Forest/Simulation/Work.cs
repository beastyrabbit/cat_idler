using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static string JobLabor(string kind) => kind == "water" ? "fetch_water" : kind == "logs" || kind == "replant" ? "woodcut" : kind == "fibre" ? "forage" : kind == "fish" ? "fishing" : kind == "offering" ? "ritual" : kind == "expand" || kind == "road" || kind == "rail" || kind == "bridge" || kind == "dock" ? "build" : kind;
        private bool FreeWorker(Village v, Cat c) => c != null && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "" && !v.Routes.Any(r => r.CatId == c.Id);
        private Cat AvailableCat(Village v, string labor) => v.Cats.Where(c => c.Alive && c.AgeHours >= 12 && FreeWorker(v, c) && c.Migration != "arriving" && c.Migration != "departing")
            .OrderByDescending(c => (c.Preferences.Contains(labor) ? 100 : 0) + (c.Boosted ? 20 : 0) + Amount(c.Skills, labor)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
        private void StartJob(Village v, Cat c, Job j)
        {
            if (c.JobId != "" || v.Jobs.Any(job => !job.Completed && job.CatId == c.Id))
                throw new InvalidOperationException("Cat already owns active work");
            if (c.Cargo.Count > 0)
                Spill(v, c.Position, c.Cargo);
            if (j.Id == "")
                j.Id = Id("job");
            j.CatId = c.Id;
            j.Origin = c.Position;
            j.StartedAt = TimeSeconds;
            c.JobId = j.Id;
            c.Goal = j.Kind;
            c.Path.Clear();
            v.Jobs.Add(j);
        }
        private ActionResult CreateInputJob(Village v, Cat c, string kind, Int2 target, string resource, double amount, double work, string targetId)
        {
            c = c ?? AvailableCat(v, JobLabor(kind));
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available worker");
            string id = Id("job");
            if (!Reserve(v, id, resource, amount))
                return ActionResult.Fail("Required finite input unavailable");
            StartJob(v, c, new Job { Id = id, Kind = kind, TargetId = targetId, Position = target, Resource = resource, Amount = amount, RequiredWork = work, Phase = "fetch", Manual = true });
            return ActionResult.Ok(id);
        }
        private ActionResult Request(Village v, string kind, Cat c, bool manual, string resource = "")
        {
            kind = Snake(kind);
            switch (kind)
            {
                case "supply_food":
                case "leader_plan_hunt":
                case "hunt_expedition":
                    kind = "hunt";
                    break;
                case "supply_water":
                case "fetch_water":
                    kind = "water";
                    break;
                case "gather_logs":
                    kind = "logs";
                    break;
                case "forage_fibre":
                    kind = "fibre";
                    break;
                case "replant_tree":
                    kind = "replant";
                    break;
                case "explore":
                    kind = "scout";
                    break;
                case "train_warrior":
                    kind = "train";
                    break;
                case "expand_village":
                    kind = "expand";
                    break;
                case "leader_plan_house":
                case "build_house":
                    for (int z = -5; z < 5; z++)
                        for (int x = -5; x < 5; x++)
                        {
                            var p = new Int2(v.Center.X + x, v.Center.Z + z);
                            if (FreeSite(v, p, 2, 2) && FreeBuildingEntrance(v, p).HasValue)
                                return Plan(v, "den", p, c);
                        }
                    return ActionResult.Fail("No clear Den footprint");
                case "ritual":
                    return CreateInputJob(v, c, "offering", v.Center, "herbs", 5, 30, "");
                case "carry_offering":
                case "perform_offering":
                    return ActionResult.Fail("Use selectable OfferResource to create a physical offering");
            }
            if (!new[] { "hunt", "water", "logs", "quarry", "fibre", "fish", "scout", "train", "expand", "replant" }.Contains(kind))
                return ActionResult.Fail("Unknown physical job: " + kind);
            c = c ?? AvailableCat(v, JobLabor(kind));
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available worker");
            if (kind == "train")
            {
                var barracks = v.Buildings.Find(b => b.Kind == "barracks" && b.Completed);
                if (barracks == null)
                    return ActionResult.Fail("Completed Barracks required");
                StartJob(v, c, new Job { Kind = kind, Position = barracks.Position, RequiredWork = 120, Manual = manual });
                return ActionResult.Ok(c.JobId);
            }
            if (kind == "expand")
            {
                var pending = v.Jobs.Find(j => j.Kind == kind && !j.Completed);
                if (pending != null)
                {
                    if (pending.CatId != "")
                        return ActionResult.Fail("Expansion already active");
                    if (!CanConstructJob(v, pending))
                        return ActionResult.Fail("Foreign territory blocks expansion");
                    if (pending.OriginalKind != "expand_village" && ExpansionFootprintBlocked(v, v.Radius + 2, pending.Path))
                        return ActionResult.Fail("Existing footprint or entrance blocks expansion");
                    if (c.Cargo.Count > 0)
                        Spill(v, c.Position, c.Cargo);
                    pending.CatId = c.Id;
                    c.JobId = pending.Id;
                    c.Path.Clear();
                    return ActionResult.Ok(pending.Id);
                }
                int radius = v.Radius + 2;
                if (!CanModifyTerrain(v.Id, new Int2(v.Center.X - radius, v.Center.Z - radius), radius * 2 + 1, radius * 2 + 1))
                    return ActionResult.Fail("Foreign territory blocks expansion");
                if (ExpansionFootprintBlocked(v, radius))
                    return ActionResult.Fail("Existing footprint or entrance blocks expansion");
                var perimeter = new List<Int2>();
                for (int x = -radius; x <= radius; x++)
                {
                    perimeter.Add(new Int2(v.Center.X + x, v.Center.Z - radius));
                    if (x != 0)
                        perimeter.Add(new Int2(v.Center.X + x, v.Center.Z + radius));
                }
                for (int z = -radius + 1; z < radius; z++)
                {
                    perimeter.Add(new Int2(v.Center.X - radius, v.Center.Z + z));
                    perimeter.Add(new Int2(v.Center.X + radius, v.Center.Z + z));
                }
                perimeter.RemoveAll(p => FarmExterior(v, p, radius) || LayoutGate(v, p, radius));
                if (perimeter.Any(p => !v.Known.Contains(p) || !Walkable(p)))
                    return ActionResult.Fail("Survey a dry reachable outer perimeter first");
                var result = CreateInputJob(v, c, "expand", v.Center, "materials", perimeter.Count, 10, "");
                if (result.Success)
                    v.Jobs.Find(j => j.Id == result.EntityId).Path = perimeter;
                return result;
            }
            if (kind == "scout")
            {
                if (resource != "" && resource != "explore")
                {
                    var survey = new List<Int2>();
                    for (int surveyRadius = 2; surveyRadius <= 10; surveyRadius += 2)
                        foreach (var d in new[] { new Int2(surveyRadius, 0), new Int2(surveyRadius, surveyRadius), new Int2(0, surveyRadius), new Int2(-surveyRadius, surveyRadius), new Int2(-surveyRadius, 0), new Int2(-surveyRadius, -surveyRadius), new Int2(0, -surveyRadius), new Int2(surveyRadius, -surveyRadius) })
                            survey.Add(new Int2(c.Position.X + d.X, c.Position.Z + d.Z));
                    var start = survey.FirstOrDefault(p => Path(c.Position, p, v) != null);
                    if (start.Equals(default(Int2)) && !c.Position.Equals(default(Int2)))
                        return ActionResult.Fail("No reachable survey heading");
                    StartJob(v, c, new Job { Kind = "scout", Resource = resource == "wood" ? "logs" : resource, Position = start, Path = survey, RequiredWork = 15, Manual = manual });
                    return ActionResult.Ok(c.JobId);
                }
                int direction = (int)(Random() % 4), radius = 12 + v.Known.Count / 100 + (int)(Random() % 8);
                var offsets = new[] { new Int2(radius, 0), new Int2(0, radius), new Int2(-radius, 0), new Int2(0, -radius) };
                var target = new Int2(v.Center.X + offsets[direction].X, v.Center.Z + offsets[direction].Z);
                for (int attempt = 0; attempt < 4 && Path(c.Position, target) == null; attempt++)
                {
                    direction = (direction + 1) % 4;
                    target = new Int2(v.Center.X + offsets[direction].X, v.Center.Z + offsets[direction].Z);
                }
                if (Path(c.Position, target) == null)
                    return ActionResult.Fail("No reachable survey route");
                StartJob(v, c, new Job { Kind = "scout", Resource = resource, Position = target, RequiredWork = 15, Manual = manual });
                return ActionResult.Ok(c.JobId);
            }
            string wanted = kind == "hunt" ? "food" : kind == "quarry" ? (resource == "" ? "stone" : resource) : kind;
            Tile source = null;
            if (kind == "fish")
            {
                var spot = v.Stockpiles.Find(s => s.Kind == "fishing" && Neighbors(s.Position).Select(TileAt).Any(t => t.Water && FishStock(t) >= 1));
                if (spot == null)
                    return ActionResult.Fail("Designate a shoreline workplace beside replenished Fish habitat");
                source = TileAt(spot.Position);
                wanted = "fish";
            }
            else if (kind == "water")
            {
                source = v.Known.Select(TileAt).Where(t => t.Water).SelectMany(t => Neighbors(t.Position)).Where(p => Walkable(p) && Path(c.Position, p) != null).Select(TileAt).FirstOrDefault();
            }
            else if (kind == "replant")
                source = v.Known.Select(TileAt).Where(t => t.Resource == "logs" && t.Amount <= 0 && t.RegrowAt < 0).OrderBy(t => Int2.Distance(t.Position, c.Position)).FirstOrDefault(t => Path(c.Position, t.Position) != null);
            else
                source = v.Known.Select(TileAt).Where(t => SourceAmount(t, wanted) > 0 && Walkable(v, t.Position) && !v.Jobs.Any(j => !j.Completed && j.Kind == kind && j.Position.Equals(t.Position))).OrderByDescending(t => v.Stockpiles.Any(s => s.Kind == "zone_gather" && Contains(s.Position, s.Width, s.Depth, t.Position))).ThenBy(t => Int2.Distance(c.Position, t.Position)).FirstOrDefault(t => Path(c.Position, t.Position, v) != null);
            if (source == null)
                return ActionResult.Fail("No known reachable " + wanted + " source; scout first");
            if (v.Jobs.Any(j => !j.Completed && j.Kind == kind && j.Position.Equals(source.Position)))
                return ActionResult.Fail("Source work position claimed");
            StartJob(v, c, new Job { Kind = kind, Resource = wanted, Position = source.Position, RequiredWork = kind == "hunt" ? 120 : 60, Manual = manual });
            return ActionResult.Ok(c.JobId);
        }
        private IEnumerable<Int2> Neighbors(Int2 p)
        {
            yield return new Int2(p.X, p.Z + 1);
            yield return new Int2(p.X + 1, p.Z);
            yield return new Int2(p.X, p.Z - 1);
            yield return new Int2(p.X - 1, p.Z);
        }
        private ActionResult StartHaul(Village v, Cat c, Stockpile source)
        {
            c = c ?? AvailableCat(v, "haul");
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available carrier");
            var resource = source.Goods.FirstOrDefault(s => s.Amount > 0);
            if (resource == null)
            {
                var item = v.Items.Where(i => i.LocationId == source.Id).OrderBy(i => i.Id, StringComparer.Ordinal).FirstOrDefault();
                if (item == null)
                    return ActionResult.Fail("Empty source");
                var itemStorage = ItemHaulStorage(v, ItemResource(item.Kind), source.Position, source.Id);
                if (itemStorage == null)
                    return ActionResult.Fail("No accepting destination");
                if (Path(c.Position, source.Position, v) == null)
                    return ActionResult.Fail("Source unreachable");
                var haul = new Job { Id = Id("job"), Kind = "haul", Phase = "item_fetch", SourceId = source.Id, TargetId = itemStorage.Id, Position = source.Position, Resource = ItemResource(item.Kind), Amount = 1, ItemIds = new List<string> { item.Id } };
                StartJob(v, c, haul);
                item.LocationId = haul.Id;
                return ActionResult.Ok(haul.Id);
            }
            double amount = Math.Min(8, resource.Amount);
            var destination = Storage(v, resource.Resource, amount, source.Position);
            if (destination == null || destination.Id == source.Id)
                return ActionResult.Fail("No accepting destination");
            string id = Id("job");
            double free = resource.Amount - Reservations.Where(r => r.PileId == source.Id && r.Resource == resource.Resource).Sum(r => r.Amount);
            amount = Math.Min(amount, free);
            if (amount <= 0)
                return ActionResult.Fail("Cargo already claimed");
            Reservations.Add(new Reservation { OwnerId = id, VillageId = v.Id, PileId = source.Id, Resource = resource.Resource, Amount = amount });
            StartJob(v, c, new Job { Id = id, Kind = "haul", Phase = "fetch", SourceId = source.Id, TargetId = destination.Id, Position = destination.Position, Resource = resource.Resource, Amount = amount });
            return ActionResult.Ok(id);
        }
        private Stockpile ItemHaulStorage(Village v, string resource, Int2 from, string sourceId) => v.Stockpiles.Where(p => p.Kind == "storage" && p.Id != sourceId && HasRoom(v, p, resource, 1)).OrderBy(p => Int2.Distance(p.Position, from)).ThenBy(p => p.Id, StringComparer.Ordinal).FirstOrDefault(p => Path(from, p.Position, v) != null);
        private Stockpile Spill(Village v, Int2 p, List<Stack> goods)
        {
            if (goods.Count == 0)
                return null;
            var pile = v.Stockpiles.Find(s => s.Kind == "spill" && s.Position.Equals(p));
            if (pile == null)
            {
                pile = new Stockpile { Id = Id("spill"), Position = p, Width = 1, Depth = 1, Kind = "spill", Capacity = 1000000 };
                v.Stockpiles.Add(pile);
            }
            foreach (var s in goods)
                Add(pile.Goods, s.Resource, s.Amount);
            goods.Clear();
            return pile;
        }
        private void CancelWork(Village v, Cat c, bool preserveUnassignedCargo = false)
        {
            if (v.Accounting?.WorkerId == c.Id)
                v.Accounting = null;
            foreach (var route in v.Routes.Where(r => r.CatId == c.Id).ToArray())
                CancelRoute(v, route.Id);
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job != null)
            {
                bool preserve = job.Kind == "road" || job.Kind == "rail" || job.Kind == "expand";
                if (preserve)
                {
                    if (c.Cargo.Count > 0)
                    {
                        var quantities = c.Cargo.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
                        var spill = Spill(v, c.Position, c.Cargo);
                        job.SuspendedCargoPileId = spill.Id;
                        foreach (var s in quantities)
                            Reservations.Add(new Reservation { OwnerId = job.Id + ":resume", VillageId = v.Id, PileId = spill.Id, Resource = s.Resource, Amount = s.Amount });
                    }
                    job.CatId = "";
                    job.BlockedReason = "worker_required";
                }
                else
                {
                    if (c.Cargo.Count > 0)
                        Spill(v, c.Position, c.Cargo);
                    Release(job.Id);
                    Release(job.Id + ":resume");
                    if (job.Kind == "build")
                        Release(job.TargetId);
                    if (job.Local.Count > 0)
                        Spill(v, job.Position, job.Local);
                    foreach (var itemId in job.ItemIds)
                    {
                        var item = v.Items.Find(i => i.Id == itemId);
                        if (item != null)
                        {
                            var source = job.Phase == "item_fetch" ? v.Stockpiles.Find(p => p.Id == job.SourceId) : null;
                            if (source != null)
                            {
                                item.LocationId = source.Id;
                                continue;
                            }
                            var spill = new Stockpile { Id = Id("spill"), Kind = "spill", Width = 1, Depth = 1, Position = job.Phase == "item_fetch" ? job.Position : c.Position, Capacity = 10000 };
                            v.Stockpiles.Add(spill);
                            item.LocationId = spill.Id;
                        }
                    }
                    job.ItemIds.Clear();
                    job.Completed = true;
                    job.CatId = "";
                }
            }
            else if (!preserveUnassignedCargo && c.Cargo.Count > 0)
                Spill(v, c.Position, c.Cargo);
            foreach (var b in v.Buildings)
            {
                foreach (var slot in b.Slots.Where(s => s.CatId == c.Id))
                {
                    slot.CatId = "";
                    slot.JobId = "";
                    slot.BlockedReason = "worker_required";
                }
                b.ExtraWorkerIds.Remove(c.Id);
                if (b.WorkerId == c.Id)
                    b.WorkerId = b.Slots.FirstOrDefault(s => s.CatId != "")?.CatId ?? "";
            }
            foreach (var f in v.Farms)
                if (f.WorkerId == c.Id)
                    f.WorkerId = "";
            c.JobId = "";
            c.BuildingId = "";
            c.ResumeJobId = "";
            c.Path.Clear();
            c.Goal = "idle";
        }
        private void Finish(Village v, Cat c, Job j)
        {
            j.Completed = true;
            Release(j.Id);
            c.JobId = "";
            c.Path.Clear();
            c.Goal = "idle";
            c.BlockedReason = "";
            string labor = j.Kind == "production" ? Catalog.Recipe(j.RecipeId)?.Labor ?? "process" : JobLabor(j.Kind);
            if (Catalog.Labors.Contains(labor))
                Add(c.Skills, labor, 1);
            string role = labor == "hunt" ? "hunter" : labor == "build" || labor == "craft" ? "architect" : labor == "research" ? "healer" : labor == "fight" ? "warrior" : "";
            if (role != "")
            {
                Add(c.RoleExperience, role, 1);
                if (c.Specialization == "" && Amount(c.RoleExperience, role) >= 25)
                    c.Specialization = role;
            }
            foreach (var item in v.Items.Where(i => c.Equipment.Contains(i.Id) && i.Kind == "tool"))
                item.Condition = Math.Max(0, item.Condition - 0.1);
            Note(v, "completed", c.Name + " completed " + j.Kind, j.Id);
        }
        private bool CanConstructJob(Village v, Job j)
        {
            if (j.Kind == "expand")
            {
                int radius = v.Radius + 2;
                return j.OriginalKind == "expand_village" ? CanModifyTerrain(v.Id, j.Position) : CanModifyTerrain(v.Id, new Int2(v.Center.X - radius, v.Center.Z - radius), radius * 2 + 1, radius * 2 + 1);
            }
            if (j.Kind == "build")
            {
                var scaffold = v.Buildings.Find(b => b.Id == j.TargetId);
                return scaffold == null || CanModifyTerrain(v.Id, scaffold.Position, scaffold.Width, scaffold.Depth);
            }
            if (j.Kind == "road" || j.Kind == "rail" || j.Kind == "bridge" || j.Kind == "dock" || j.Kind == "wagon" || j.Kind == "vessel")
                return j.Path.All(p => CanModifyTerrain(v.Id, p)) && (j.Kind != "wagon" && j.Kind != "vessel" || CanModifyTerrain(v.Id, j.Position));
            return true;
        }
        private void TickJob(Village v, Cat c, Job j)
        {
            if (!CanConstructJob(v, j))
            {
                j.BlockedReason = c.BlockedReason = "foreign_territory";
                return;
            }
            if (j.BlockedReason == "foreign_territory")
                j.BlockedReason = c.BlockedReason = "";
            if (j.Kind == "expand" && j.OriginalKind != "expand_village")
            {
                if (ExpansionFootprintBlocked(v, v.Radius + 2, j.Path))
                {
                    j.BlockedReason = c.BlockedReason = "expansion_footprint_blocked";
                    return;
                }
                if (j.BlockedReason == "expansion_footprint_blocked")
                    j.BlockedReason = c.BlockedReason = "";
            }
            if (j.Kind == "road" || j.Kind == "rail")
            {
                if (j.Path.Any(p => !FreeInfrastructureFootprint(v, j.Kind, p)))
                {
                    j.BlockedReason = c.BlockedReason = "infrastructure_footprint_blocked";
                    return;
                }
                if (j.BlockedReason == "infrastructure_footprint_blocked")
                    j.BlockedReason = c.BlockedReason = "";
            }
            if (j.Kind == "build")
            {
                var scaffold = v.Buildings.Find(b => b.Id == j.TargetId);
                if (scaffold != null && !ConnectedEntrance(v, scaffold))
                {
                    j.BlockedReason = c.BlockedReason = "building_entrance_disconnected";
                    return;
                }
                if (j.BlockedReason == "building_entrance_disconnected")
                    j.BlockedReason = c.BlockedReason = "";
            }
            if (j.Kind == "production" && j.Phase != "output_delivery")
            {
                var station = v.Buildings.Find(b => b.Id == j.TargetId);
                var slot = station?.Slots.Find(s => s.CatId == c.Id);
                if (station?.Paused == true || slot?.Paused == true)
                {
                    j.BlockedReason = "queue_paused";
                    c.Goal = "production paused";
                    return;
                }
            }
            if (j.SuspendedCargoPileId != "")
            {
                if (c.Cargo.Count == 0 && Reservations.Any(r => r.OwnerId == j.Id + ":resume"))
                {
                    PickupClaim(v, c, j.Id + ":resume", j);
                    return;
                }
                if (!Reservations.Any(r => r.OwnerId == j.Id + ":resume"))
                    j.SuspendedCargoPileId = "";
            }
            c.Goal = j.Kind + " · " + j.Phase;
            if (j.Phase == "item_fetch")
            {
                if (!Move(c, j.Position, 1))
                    return;
                j.Phase = "output_delivery";
                c.Path.Clear();
                return;
            }
            if (j.Kind == "scout")
            {
                for (int z = -2; z <= 2; z++)
                    for (int x = -2; x <= 2; x++)
                    {
                        var p = new Int2(c.Position.X + x, c.Position.Z + z);
                        var observed = TileAt(p);
                        if (!c.ScoutNotes.Contains(p))
                            c.ScoutNotes.Add(p);
                        if (j.Resource != "" && j.Phase != "return" && ((j.Resource == "water" && observed.Water) || SourceAmount(observed, j.Resource) > 0))
                        {
                            j.Phase = "return";
                            j.TargetId = p.ToString();
                            c.Path.Clear();
                        }
                    }
                if (j.Phase == "return")
                {
                    if (Move(c, v.Center, 1))
                    {
                        foreach (var p in c.ScoutNotes)
                            if (!v.Known.Contains(p))
                                v.Known.Add(p);
                        foreach (var other in Villages.Where(other => other.Id != v.Id && c.ScoutNotes.Any(p => Int2.Distance(p, other.Center) <= other.Radius)))
                        {
                            if (!v.Contacts.Contains(other.Id))
                                v.Contacts.Add(other.Id);
                            if (!other.Contacts.Contains(v.Id))
                                other.Contacts.Add(v.Id);
                        }
                        Note(v, "discovery", c.ScoutNotes.Count + " notes delivered to shrine", c.Id);
                        c.ScoutNotes.Clear();
                        Finish(v, c, j);
                    }
                    return;
                }
                if (Move(c, j.Position, 1))
                {
                    if (j.Path.Count > 0 && j.PathIndex + 1 < j.Path.Count)
                    {
                        j.PathIndex++;
                        j.Position = j.Path[j.PathIndex];
                        c.Path.Clear();
                    }
                    else
                    {
                        j.Phase = "return";
                        c.Path.Clear();
                    }
                }
                if (TimeSeconds - j.StartedAt > 180)
                {
                    j.Phase = "return";
                    c.Path.Clear();
                }
                return;
            }
            if (j.Kind == "build")
            {
                var b = v.Buildings.Find(x => x.Id == j.TargetId) ?? ResolveScaffold(v, c, j);
                if (b == null)
                    return;
                if (b.Required.All(s => Amount(b.Inputs, s.Resource) >= s.Amount))
                {
                    if (!Move(c, b.Position, 1))
                        return;
                    j.Phase = "working";
                    b.Progress += WorkRate(v, c, "build", b.Kind);
                    if (b.Progress >= b.RequiredWork)
                    {
                        b.Inputs.Clear();
                        Commission(v, b);
                        Release(b.Id);
                        Finish(v, c, j);
                    }
                    return;
                }
                if (c.Cargo.Count > 0)
                {
                    if (!Move(c, b.Position, 1))
                        return;
                    foreach (var s in c.Cargo)
                        Add(b.Inputs, s.Resource, s.Amount);
                    c.Cargo.Clear();
                    return;
                }
                foreach (var input in b.Required)
                {
                    double held = Reservations.Where(r => r.OwnerId == b.Id && r.Resource == input.Resource).Sum(r => r.Amount);
                    double need = input.Amount - Amount(b.Inputs, input.Resource) - held;
                    if (need > 1e-8 && !Reserve(v, b.Id, input.Resource, need))
                    {
                        j.BlockedReason = "missing_scaffold_input";
                        return;
                    }
                }
                PickupClaim(v, c, b.Id, j);
                return;
            }
            if (j.Phase == "fetch")
            {
                if (c.Cargo.Count > 0)
                {
                    j.Phase = "input_delivery";
                    c.Path.Clear();
                    return;
                }
                if (Reservations.Any(r => r.OwnerId == j.Id))
                {
                    PickupClaim(v, c, j.Id, j);
                    return;
                }
                j.Phase = "input_delivery";
            }
            if (j.Phase == "input_delivery")
            {
                if (!Move(c, j.Position, 1))
                    return;
                foreach (var s in c.Cargo)
                    Add(j.Local, s.Resource, s.Amount);
                c.Cargo.Clear();
                if (Reservations.Any(r => r.OwnerId == j.Id))
                {
                    j.Phase = "fetch";
                    return;
                }
                if (j.Kind == "haul")
                {
                    j.Phase = "output_delivery";
                    foreach (var s in j.Local)
                        Add(c.Cargo, s.Resource, s.Amount);
                    j.Local.Clear();
                    return;
                }
                j.Phase = "working";
            }
            if (j.Phase == "travel")
            {
                if (!Move(c, j.Position, 1))
                    return;
                j.Phase = "working";
            }
            if (j.Phase == "working")
            {
                if (j.Kind == "production")
                {
                    var recipe = Catalog.Recipe(j.RecipeId);
                    if (recipe == null)
                    {
                        j.BlockedReason = "unknown_recipe";
                        return;
                    }
                    if (j.Reserved.Count == 0)
                        j.Reserved = recipe.Inputs.Select(s => new Stack(s.Resource, RecipeInput(v, recipe, s.Amount))).ToList();
                    var station = v.Buildings.Find(b => b.Id == j.TargetId);
                    foreach (var required in j.Reserved)
                    {
                        double missing = Math.Max(0, required.Amount - Amount(j.Local, required.Resource));
                        if (missing > 0 && station != null)
                        {
                            double fromStation = Math.Min(missing, Amount(station.Inputs, required.Resource));
                            if (fromStation > 0)
                            {
                                Add(station.Inputs, required.Resource, -fromStation);
                                Add(j.Local, required.Resource, fromStation);
                                missing -= fromStation;
                            }
                        }
                        if (missing > 1e-8)
                        {
                            double reserved = Reservations.Where(r => r.OwnerId == j.Id && r.Resource == required.Resource).Sum(r => r.Amount);
                            if (reserved + 1e-8 < missing && !Reserve(v, j.Id, required.Resource, missing - reserved))
                            {
                                j.BlockedReason = "missing_recipe_input";
                                return;
                            }
                            j.Phase = "fetch";
                            c.Path.Clear();
                            return;
                        }
                    }
                }
                if (j.Kind == "expand")
                {
                    if (j.OriginalKind == "expand_village")
                    {
                        if (!Move(c, j.Position, 1))
                            return;
                        j.Progress += WorkRate(v, c, "build", "");
                        if (j.Progress >= j.RequiredWork)
                        {
                            ClaimLegacyTile(v, j);
                            Finish(v, c, j);
                            var linked = v.Jobs.Find(job => job.Id == j.LinkedJobId && !job.Completed && job.CatId == "");
                            if (linked != null)
                            {
                                linked.CatId = c.Id;
                                c.JobId = linked.Id;
                            }
                        }
                        return;
                    }
                    if (j.PathIndex >= j.Path.Count)
                    {
                        j.Local.Clear();
                        Expand(v);
                        Finish(v, c, j);
                        return;
                    }
                    var target = j.Path[j.PathIndex];
                    var stand = Neighbors(target).Where(Walkable).OrderBy(p => Int2.Distance(p, c.Position)).FirstOrDefault(p => Path(c.Position, p) != null);
                    if (c.Cargo.Count == 0)
                    {
                        if (!Move(c, j.Position, 1))
                            return;
                        if (Amount(j.Local, "materials") < 1)
                        {
                            j.BlockedReason = "wall_materials_missing";
                            return;
                        }
                        Add(j.Local, "materials", -1);
                        Add(c.Cargo, "materials", 1);
                    }
                    if (!Move(c, stand, 1))
                        return;
                    j.Progress++;
                    if (j.Progress < 10)
                        return;
                    j.Progress = 0;
                    TileAt(target).Wall = true;
                    Add(c.Cargo, "materials", -1);
                    j.PathIndex++;
                    return;
                }
                if (j.Kind == "road" || j.Kind == "rail")
                {
                    if (j.PathIndex >= j.Path.Count)
                    {
                        Finish(v, c, j);
                        return;
                    }
                    var target = j.Path[j.PathIndex];
                    if (c.Cargo.Count == 0)
                    {
                        if (!Move(c, j.Position, 1))
                            return;
                        if (Amount(j.Local, j.Resource) < 1)
                        {
                            j.BlockedReason = "paving_input_missing";
                            return;
                        }
                        Add(j.Local, j.Resource, -1);
                        Add(c.Cargo, j.Resource, 1);
                    }
                    if (!Move(c, target, 1))
                        return;
                    j.Progress++;
                    if (j.Progress < 30)
                        return;
                    j.Progress = 0;
                    var tile = TileAt(target);
                    if (j.Kind == "road")
                    {
                        tile.Road = true;
                        tile.Dirt = false;
                    }
                    else
                        tile.Rail = true;
                    Add(c.Cargo, j.Resource, -1);
                    j.PathIndex++;
                    return;
                }
                if (!Move(c, j.Position, 1))
                    return;
                j.Progress += WorkRate(v, c, j.Kind == "production" ? Catalog.Recipe(j.RecipeId)?.Labor ?? "process" : JobLabor(j.Kind), j.Kind == "production" ? Catalog.Recipe(j.RecipeId)?.Building : "") * j.SpeedMultiplier;
                if (j.Progress < j.RequiredWork)
                    return;
                if (j.Kind == "production")
                {
                    var recipe = Catalog.Recipe(j.RecipeId);
                    foreach (var input in j.Reserved)
                        Add(j.Local, input.Resource, -input.Amount);
                    var inputStation = v.Buildings.Find(b => b.Id == j.TargetId);
                    if (inputStation != null)
                    {
                        foreach (var surplus in j.Local)
                            Add(inputStation.Inputs, surplus.Resource, surplus.Amount);
                        j.Local.Clear();
                    }
                    else
                        Spill(v, j.Position, j.Local);
                    foreach (var output in recipe.Outputs)
                        Add(j.Local, output.Resource, RecipeOutput(v, recipe, output.Amount));
                    if (recipe.ItemKind != "")
                    {
                        var item = new Item { Id = Id("item"), Kind = recipe.ItemKind, Material = recipe.Material, Quality = recipe.Quality, VillageId = v.Id, LocationId = j.Id, MaxCondition = 100 * Catalog.BuildingEffect(v, recipe.Building, "durability", 1) * (FamilyOwned(v, recipe, "preservation") ? 1.25 : 1) };
                        item.Condition = item.MaxCondition;
                        v.Items.Add(item);
                        j.ItemIds.Add(item.Id);
                    }
                    var station = v.Buildings.Find(b => b.Id == j.TargetId);
                    if (station != null)
                    {
                        var slot = station.Slots.Find(s => s.CatId == c.Id);
                        var queue = slot?.Queue ?? station.Queue;
                        if (queue.Count > 0 && queue[0].RecipeId == j.RecipeId)
                        {
                            var completed = queue[0];
                            queue.RemoveAt(0);
                            if (completed.Repeat)
                                queue.Add(completed);
                        }
                    }
                }
                else if (j.Kind == "offering")
                {
                    v.Blessings += j.Local.Sum(s => s.Amount) / 5 * Catalog.Effect(v, "shrineBlessingYield", 1);
                    j.Local.Clear();
                    Finish(v, c, j);
                    return;
                }
                else if (j.Kind == "mouse_farm")
                {
                    j.Local.Clear();
                    Add(j.Local, "food", 4 * Service(v, "mouse_farm", "mouseFarmFood", 1));
                }
                else if (j.Kind == "expand")
                {
                    j.Local.Clear();
                    Expand(v);
                    Finish(v, c, j);
                    return;
                }
                else if (j.Kind == "train")
                {
                    Add(c.Skills, "fight", 3);
                    Finish(v, c, j);
                    return;
                }
                else if (j.Kind == "replant")
                {
                    var t = TileAt(j.Position);
                    if (t.Resource == "logs" && t.Amount <= 0 && t.RegrowAt < 0)
                        t.RegrowAt = TimeSeconds + 86400;
                    Finish(v, c, j);
                    return;
                }
                else if (new[] { "road", "rail", "bridge", "dock", "wagon", "vessel" }.Contains(j.Kind))
                {
                    CompleteInfrastructure(v, j);
                    j.Local.Clear();
                    Finish(v, c, j);
                    return;
                }
                else
                {
                    var tile = TileAt(j.Position);
                    double quantity = j.Amount > 0 ? j.Amount : CarryCapacity(v, j.Resource) * (j.Kind == "hunt" ? Catalog.Effect(v, "huntYieldMult", 1) : j.Kind == "fibre" ? Catalog.Effect(v, "gatherYieldMult", 1) : j.Kind == "quarry" || j.Kind == "logs" ? Catalog.Effect(v, "materialYieldMult", 1) : 1) * j.YieldMultiplier;
                    if (j.Kind == "hunt")
                        quantity *= 1 + UpgradeLevel(v, "hunt_mastery") * 0.12;
                    if (j.Kind != "water" && j.Kind != "fish")
                    {
                        quantity = Math.Min(quantity, SourceAmount(tile, j.Resource));
                        TakeSource(tile, j.Resource, quantity);
                    }
                    if (j.Kind == "fish")
                    {
                        var habitat = Neighbors(j.Position).Select(TileAt).FirstOrDefault(t => t.Water && FishStock(t) > 0);
                        quantity = habitat == null ? 0 : Math.Min(8, FishStock(habitat));
                        if (habitat != null)
                            SetFish(habitat, FishStock(habitat) - quantity);
                    }
                    if (quantity > 0)
                        Add(j.Local, j.Resource, quantity);
                    if (j.Kind == "hunt" && quantity > 0)
                    {
                        Add(j.Local, "hide", quantity * 0.25);
                        Add(j.Local, "bone", quantity * 0.125);
                        tile.RegrowAt = TimeSeconds + 3600;
                    }
                }
                j.Phase = "output_delivery";
                c.Path.Clear();
            }
            if (j.Phase == "output_delivery")
            {
                if (c.Cargo.Count == 0 && j.Local.Count > 0)
                {
                    if (!Move(c, j.Position, 1))
                        return;
                    var s = j.Local[0];
                    double take = Math.Min(CarryCapacity(v, s.Resource), s.Amount);
                    Add(c.Cargo, s.Resource, take);
                    Add(j.Local, s.Resource, -take);
                }
                var cargo = c.Cargo.FirstOrDefault();
                var deliveredItem = cargo == null && j.ItemIds.Count > 0 ? v.Items.Find(i => i.Id == j.ItemIds[0]) : null;
                string resource = cargo?.Resource ?? (deliveredItem != null ? ItemResource(deliveredItem.Kind) : "");
                if (resource == "")
                {
                    Finish(v, c, j);
                    return;
                }
                var pile = j.Kind == "haul" && deliveredItem != null ? ItemHaulStorage(v, resource, c.Position, j.SourceId) : Storage(v, resource, cargo?.Amount ?? 1, c.Position);
                if (pile == null)
                {
                    j.BlockedReason = c.BlockedReason = "output_storage_full_or_unreachable";
                    return;
                }
                if (!Move(c, pile.Position, 1))
                    return;
                if (cargo != null)
                {
                    Add(pile.Goods, cargo.Resource, cargo.Amount);
                    c.Cargo.Remove(cargo);
                }
                if (deliveredItem != null)
                {
                    deliveredItem.LocationId = pile.Id;
                    j.ItemIds.Remove(deliveredItem.Id);
                }
                Add(c.Skills, "haul", 0.25);
                if (j.Local.Count == 0 && c.Cargo.Count == 0 && j.ItemIds.Count == 0)
                    Finish(v, c, j);
            }
        }
        private void PickupClaim(Village v, Cat c, string ownerId, Job j)
        {
            var claim = Reservations.FirstOrDefault(r => r.OwnerId == ownerId);
            if (claim == null)
                return;
            var pile = v.Stockpiles.Find(s => s.Id == claim.PileId);
            if (pile == null || Amount(pile.Goods, claim.Resource) < claim.Amount)
            {
                Release(ownerId);
                j.BlockedReason = "source_changed";
                return;
            }
            if (!Move(c, pile.Position, 1))
                return;
            double quantity = Math.Min(CarryCapacity(v, claim.Resource), claim.Amount);
            Add(pile.Goods, claim.Resource, -quantity);
            Add(c.Cargo, claim.Resource, quantity);
            claim.Amount -= quantity;
            if (claim.Amount <= 1e-9)
                Reservations.Remove(claim);
        }
        private Building ResolveScaffold(Village v, Cat c, Job j)
        {
            string kind = Snake(j.PendingBuildingKind == "" ? "den" : j.PendingBuildingKind);
            if (!Catalog.BuildingAvailable(v, kind) || kind == "shrine")
            {
                j.BlockedReason = "building_research_required";
                return null;
            }
            Int2? site = null;
            foreach (var p in v.Known.OrderBy(p => Int2.Distance(v.Center, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (FreeSite(v, p, 2, 2, kind == "field") && FreeBuildingEntrance(v, p).HasValue && Path(c.Position, p, v) != null)
                {
                    site = p;
                    break;
                }
            if (!site.HasValue)
            {
                j.BlockedReason = "site_required";
                return null;
            }
            int count = v.Buildings.Count(b => b.Kind == kind);
            double timber = 4 + count * 2, blocks = 2 + count;
            string timberKind = Available(v, "lumber") >= timber ? "lumber" : "planks";
            var bill = j.Reserved.Count > 0 ? j.Reserved.Select(s => new Stack(s.Resource, s.Amount)).ToList() : new List<Stack> { new Stack(timberKind, timber), new Stack("blocks", blocks) };
            if (bill.Any(s => Available(v, s.Resource) + Amount(j.Local, s.Resource) + Amount(c.Cargo, s.Resource) < s.Amount))
            {
                j.BlockedReason = "missing_scaffold_input";
                return null;
            }
            var scaffold = new Building { Id = Id(kind), Kind = kind, Position = site.Value, Entrance = FreeBuildingEntrance(v, site.Value).Value, HasEntrance = true, Required = bill, RequiredWork = Math.Max(1, j.RequiredWork), Progress = j.Progress, Inputs = j.Local };
            j.Local = new List<Stack>();
            v.Buildings.Add(scaffold);
            j.TargetId = scaffold.Id;
            j.Position = scaffold.Position;
            j.BlockedReason = "";
            return scaffold;
        }
        private double WorkRate(Village v, Cat c, string labor, string station)
        {
            double skill = Amount(c.Skills, labor), rate = (1 + Math.Min(0.333, skill / 300)) * UpgradeWorkRate(v, labor);
            string stat = labor == "hunt" || labor == "fishing" ? "hunting" : labor == "research" ? "medicine" : labor == "scout" ? "vision" : labor == "fight" ? "attack" : "building";
            rate *= 0.75 + Math.Clamp(Amount(c.Stats, stat), 0, 100) / 200;
            rate *= Catalog.Effect(v, labor == "research" ? "researchRate" : labor == "build" ? "constructionSpeed" : "productionRate", 1);
            if (!string.IsNullOrEmpty(station))
                rate /= Math.Max(0.1, Catalog.BuildingEffect(v, station, "cycle_time", 1));
            if (v.Items.Any(i => c.Equipment.Contains(i.Id) && i.Kind == "tool" && i.Condition > 0))
                rate *= 1.15;
            return Math.Clamp(rate, 0.1, 20);
        }
        private bool PendingExpansionWall(Village v, Int2 at) => v.Jobs.Any(j => !j.Completed && j.Kind == "expand" && j.OriginalKind != "expand_village" && j.Path.Contains(at));
        private Int2? FreeBuildingEntrance(Village v, Int2 position)
        {
            var expansions = v.Jobs.Where(j => !j.Completed && j.Kind == "expand" && j.OriginalKind != "expand_village").ToArray();
            var walls = expansions.SelectMany(j => j.Path).ToHashSet();
            var roads = ConnectedRoads(v, walls, expansions.Length > 0 ? v.Radius + 2 : 0);
            foreach (var at in EntranceCandidates(new Building { Position = position }).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (roads.Contains(at))
                    return at;
            return null;
        }
        private bool ExpansionFootprintBlocked(Village v, int radius, List<Int2> plannedWalls = null)
        {
            bool WallAt(Int2 p) => plannedWalls != null && plannedWalls.Contains(p) || Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)) == radius && !LayoutGate(v, p, radius) && !FarmExterior(v, p, radius);
            var walls = plannedWalls == null ? new HashSet<Int2>() : plannedWalls.ToHashSet();
            for (int offset = -radius; offset <= radius; offset++)
                foreach (var p in new[] { new Int2(v.Center.X + offset, v.Center.Z - radius), new Int2(v.Center.X + offset, v.Center.Z + radius), new Int2(v.Center.X - radius, v.Center.Z + offset), new Int2(v.Center.X + radius, v.Center.Z + offset) })
                    if (WallAt(p))
                        walls.Add(p);
            if (v.Jobs.Any(j => !j.Completed && (j.Kind == "road" || j.Kind == "rail") && j.Path.Any(walls.Contains)))
                return true;
            var currentRoads = ConnectedRoads(v);
            var roads = ConnectedRoads(v, walls, radius);
            bool WallCrosses(Int2 origin, int width, int depth)
            {
                for (int x = 0; x < width; x++)
                    for (int z = 0; z < depth; z++)
                        if (WallAt(new Int2(origin.X + x, origin.Z + z)))
                            return true;
                return false;
            }
            foreach (var building in v.Buildings)
            {
                if (WallCrosses(building.Position, building.Width, building.Depth))
                    return true;
                if (building.Kind != "shrine")
                {
                    var entrance = BuildingEntrance(v, building);
                    if (entrance.HasValue && (walls.Contains(entrance.Value) || currentRoads.Contains(entrance.Value) && !roads.Contains(entrance.Value)))
                        return true;
                }
            }
            return v.Stockpiles.Any(p => p.Kind != "spill" && !p.Kind.StartsWith("zone_", StringComparison.Ordinal) && WallCrosses(p.Position, p.Width, p.Depth));
        }
        private bool ExpansionFarmBoundary(Village v, int radius, Int2 from, Int2 to)
        {
            bool Inside(Int2 p) => Math.Abs(p.X - v.Center.X) <= radius && Math.Abs(p.Z - v.Center.Z) <= radius;
            return Inside(from) && Inside(to) && FarmExterior(v, from, radius) != FarmExterior(v, to, radius);
        }
        private void Expand(Village v)
        {
            v.Radius += 2;
            v.ClaimedTiles.Clear();
            v.AgriculturalTiles.Clear();
            v.BoundaryEdges.Clear();
            for (int z = -v.Radius; z <= v.Radius; z++)
                for (int x = -v.Radius; x <= v.Radius; x++)
                {
                    var p = new Int2(v.Center.X + x, v.Center.Z + z);
                    var t = TileAt(p);
                    if (FarmExterior(v, p, v.Radius))
                    {
                        t.ClaimId = "";
                        t.Wall = false;
                        v.AgriculturalTiles.Add(p);
                        continue;
                    }
                    v.ClaimedTiles.Add(p);
                    t.Water = t.Mountain = false;
                    t.Resource = "";
                    t.Amount = 0;
                    t.ClaimId = v.Id;
                    t.Biome = "meadow";
                    t.Wall = (Math.Abs(x) == v.Radius || Math.Abs(z) == v.Radius) && !LayoutGate(v, p, v.Radius);
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            foreach (var p in v.ClaimedTiles)
                foreach (var outside in Neighbors(p))
                    if (v.AgriculturalTiles.Contains(outside))
                        v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = outside });
            Note(v, "expansion", "Settlement grew to radius " + v.Radius);
        }
        private bool FarmExterior(Village v, Int2 p, int radius)
        {
            foreach (var farm in v.Farms)
            {
                if (Contains(farm.Position, farm.Width, farm.Depth, p))
                    return true;
                int left = farm.Position.X - v.Center.X, right = left + farm.Width - 1, bottom = farm.Position.Z - v.Center.Z, top = bottom + farm.Depth - 1;
                int x = p.X - v.Center.X, z = p.Z - v.Center.Z;
                int[] gaps = { radius + left, radius - right, radius + bottom, radius - top };
                int side = Array.IndexOf(gaps, gaps.Min());
                if (side == 0 && z >= bottom && z <= top && x <= left || side == 1 && z >= bottom && z <= top && x >= right || side == 2 && x >= left && x <= right && z <= bottom || side == 3 && x >= left && x <= right && z >= top)
                    return true;
            }
            return false;
        }
        private void ClaimLegacyTile(Village v, Job job)
        {
            var p = job.Position;
            if (!v.ClaimedTiles.Contains(p))
                v.ClaimedTiles.Add(p);
            if (!v.Known.Contains(p))
                v.Known.Add(p);
            var tile = TileAt(p);
            tile.ClaimId = v.Id;
            tile.Water = tile.Mountain = tile.Wall = false;
            if (job.AgriculturalExpansion)
            {
                if (!v.AgriculturalTiles.Contains(p))
                    v.AgriculturalTiles.Add(p);
            }
            else
            {
                tile.Resource = "";
                tile.Amount = 0;
                tile.Deposits.Clear();
                tile.Biome = "meadow";
            }
            v.BoundaryEdges.RemoveAll(e => e.From.Equals(p) || e.To.Equals(p));
            foreach (var next in Neighbors(p))
                if (!v.ClaimedTiles.Contains(next) && !(p.X == v.Center.X && p.Z > v.Center.Z && next.Z > p.Z))
                    v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = next });
            if (job.CompletionBoundaryEdges.Count > 0)
                v.BoundaryEdges = new List<BoundaryEdge>(job.CompletionBoundaryEdges);
            v.Radius = Math.Max(v.Radius, Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)));
            Note(v, "expansion", "Restored wall project claimed one tile", p.ToString());
        }
    }
}
