using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private void TickVillage(Village v)
        {
            foreach (var pending in v.Jobs.Where(j => !j.Completed && j.CatId == "" && (j.AutomatedBy == "" || j.AutomatedBy == "leader" || HasOfficer(v, j.AutomatedBy))).ToArray())
            {
                var worker = AvailableCat(v, JobLabor(pending.Kind));
                if (worker == null)
                    break;
                if (worker.Cargo.Count > 0)
                    Spill(v, worker.Position, worker.Cargo);
                pending.CatId = worker.Id;
                worker.JobId = pending.Id;
                worker.Goal = pending.Kind;
            }
            if ((long)TimeSeconds % 60 == 0)
                foreach (var tile in Tiles.Where(t => t.RegrowAt >= 0 && t.RegrowAt <= TimeSeconds).ToArray())
                {
                    tile.Amount = tile.Resource == "logs" ? 40 : 30;
                    tile.RegrowAt = -1;
                }
            foreach (var c in v.Cats.Where(c => c.Alive).ToArray())
            {
                if (c.Health <= 0)
                {
                    Die(v, c, "needs");
                    continue;
                }
                double resilience = Math.Max(0.45, 1 - UpgradeLevel(v, "resilience") * 0.08);
                c.AgeHours += (c.AgeHours < 24 ? Service(v, "nursery", "kittenGrowth", 1) : 1) / 3600;
                c.Hunger = Math.Max(0, c.Hunger - 20.0 / 3600 * resilience);
                c.Thirst = Math.Max(0, c.Thirst - 30.0 / 3600 * resilience / Catalog.Effect(v, "waterEfficiency", 1) / Service(v, "water_bowl", "waterStewardship", 1));
                c.Rest = Math.Max(0, c.Rest - 12.0 / 3600 / Service(v, "den", "denStewardship", 1));
                if (c.Health < 100 && c.Hunger > 50 && c.Thirst > 50)
                    c.Health = Math.Min(100, c.Health + 0.002 * Catalog.Effect(v, "healthRecovery", 1));
                if (c.Hunger <= 0)
                    c.Health -= 2.0 / 60;
                if (c.Thirst <= 0)
                    c.Health -= 3.0 / 60;
                if (c.Health <= 0)
                {
                    Die(v, c, "needs");
                    continue;
                }
                if (c.Migration == "arriving" || c.Migration == "departing")
                {
                    Int2 destination = c.Migration == "arriving" ? v.Center : c.MigrationExterior ?? new Int2(v.Center.X, v.Center.Z + v.Radius + 3);
                    if (Move(c, destination, 1))
                    {
                        if (c.Migration == "arriving")
                        {
                            c.Migration = "probationary";
                            c.ProbationUntil = TimeSeconds + 36 * 3600;
                        }
                        else
                        {
                            c.Alive = false;
                            v.MigrationDepartures++;
                            Note(v, "migration", c.Name + " left the colony", c.Id);
                        }
                    }
                    continue;
                }
                if (c.ControlledBy != "")
                {
                    if (c.ControlLeaseUntil <= TimeSeconds)
                    {
                        c.ControlledBy = "";
                        c.Path.Clear();
                        c.Goal = "idle";
                    }
                    else
                        continue;
                }
                if (TickNeeds(v, c))
                    continue;
                var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
                if (job != null)
                    TickJob(v, c, job);
                else if (c.JobId != "")
                    c.JobId = "";
                else if (c.BuildingId == "")
                {
                    if (c.Path.Count > 0)
                        Move(c, c.Path[c.Path.Count - 1], 1);
                    else if ((long)TimeSeconds % 20 == Hash(c.Id) % 20)
                    {
                        int x = (int)(Hash(c.Id + ":" + (long)TimeSeconds) % 9) - 4, z = (int)(Hash(c.Id + ":z:" + (long)TimeSeconds) % 9) - 4;
                        var target = new Int2(v.Center.X + x, v.Center.Z + z);
                        c.Path = Path(c.Position, target, v) ?? new List<Int2>();
                        c.Goal = "resting at home";
                    }
                }
            }
            TickStations(v);
            TickFarms(v);
            TickAccounting(v);
            TickTransport(v);
            TickTrader(v);
            if ((long)TimeSeconds % 10 == 0)
                Direct(v);
            Governance(v);
            if ((long)TimeSeconds % 60 == 0)
            {
                Population(v);
                Spoilage(v);
                v.Jobs.RemoveAll(j => j.Completed && TimeSeconds - j.StartedAt > 600);
            }
            TickRaids(v);
        }
        private bool TickNeeds(Village v, Cat c)
        {
            if (c.Goal.StartsWith("need_", StringComparison.Ordinal))
            {
                if (c.Goal == "need_sleep")
                {
                    var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
                    if (den == null)
                    {
                        c.Goal = "idle";
                        return false;
                    }
                    if (Move(c, den.Position, 1))
                        c.Rest = Math.Min(100, c.Rest + 0.15 * Catalog.Effect(v, "restRecovery", 1));
                    if (c.Rest >= 95)
                        Resume(c);
                    return true;
                }
                string resource = c.Goal == "need_drink" ? "water" : c.Goal == "need_heal" ? "medicine" : "food";
                if (c.Cargo.Count == 0)
                {
                    var choices = c.Goal == "need_drink" ? (!c.PersonalBrewUsed && Available(v, "water") >= 0.75 ? new[] { "brew", "water" } : new[] { "water" }) : c.Goal == "need_heal" ? new[] { "medicine", "herbs" } : new[] { "fish", "preserves", "food" };
                    Stockpile pile = null;
                    foreach (var choice in choices)
                    {
                        resource = choice;
                        double serving = resource == "brew" ? 0.25 : resource == "water" && c.PersonalBrewUsed ? 0.75 : 1;
                        pile = v.Stockpiles.Where(s => s.Kind == "storage" && Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount) >= serving).OrderBy(s => Int2.Distance(c.Position, s.Position)).FirstOrDefault(s => Path(c.Position, s.Position, v) != null);
                        if (pile != null)
                            break;
                    }
                    if (pile == null)
                    {
                        c.BlockedReason = "need_supply_missing";
                        Resume(c);
                        return false;
                    }
                    if (!Move(c, pile.Position, 1))
                        return true;
                    double wanted = resource == "brew" ? 0.25 : resource == "water" && c.PersonalBrewUsed ? 0.75 : 1;
                    double free = Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
                    if (free < wanted)
                    {
                        Resume(c);
                        return false;
                    }
                    Add(pile.Goods, resource, -wanted);
                    Add(c.Cargo, resource, wanted);
                    return true;
                }
                if (!Move(c, v.Center, 1))
                    return true;
                if (c.Goal == "need_drink")
                {
                    var serving = c.Cargo.Find(s => s.Resource == "water" || s.Resource == "brew");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(serving.Resource == "brew" ? 0.25 : c.PersonalBrewUsed ? 0.75 : 1, serving.Amount);
                    c.Thirst = Math.Min(100, c.Thirst + 50 * dose);
                    Add(c.Cargo, serving.Resource, -dose);
                    if (serving.Resource == "brew")
                    {
                        c.PersonalBrewUsed = true;
                        c.Path.Clear();
                        return true;
                    }
                    c.PersonalBrewUsed = false;
                }
                else if (c.Goal == "need_heal")
                {
                    var serving = c.Cargo.Find(s => s.Resource == "herbs" || s.Resource == "medicine");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(1, serving.Amount);
                    c.Health = Math.Min(100, c.Health + 25 * dose * Service(v, "herb_garden", "herbMedicineEfficacy", 1));
                    Add(c.Cargo, serving.Resource, -dose);
                }
                else
                {
                    var serving = c.Cargo.Find(s => s.Resource == "food" || s.Resource == "fish" || s.Resource == "preserves");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(1, serving.Amount);
                    c.Hunger = Math.Min(100, c.Hunger + 50 * dose);
                    Add(c.Cargo, serving.Resource, -dose);
                }
                Resume(c);
                return true;
            }
            if (c.Cargo.Count > 0)
            {
                if (c.Thirst >= 15 && c.Hunger >= 15 && c.Health >= 20)
                    return false;
                bool providesNeed = c.Thirst < 15 && Amount(c.Cargo, "water") > 0 || c.Thirst >= 15 && c.Hunger < 15 && c.Cargo.Any(s => new[] { "food", "fish", "preserves" }.Contains(s.Resource));
                if (!providesNeed)
                {
                    var suspended = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
                    var quantities = c.Cargo.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
                    var pile = Spill(v, c.Position, c.Cargo);
                    if (suspended != null && pile != null)
                    {
                        suspended.SuspendedCargoPileId = pile.Id;
                        foreach (var s in quantities)
                            Reservations.Add(new Reservation { OwnerId = suspended.Id + ":resume", VillageId = v.Id, PileId = pile.Id, Resource = s.Resource, Amount = s.Amount });
                    }
                }
            }
            string need = c.Thirst < 35 ? "need_drink" : c.Hunger < 35 ? "need_eat" : c.Health < 40 ? "need_heal" : c.Rest < 20 && c.BedId != "" ? "need_sleep" : "";
            if (need == "")
                return false;
            c.ResumeJobId = c.JobId;
            c.Goal = need;
            c.Path.Clear();
            return true;
        }
        private void Resume(Cat c)
        {
            c.JobId = c.ResumeJobId;
            c.ResumeJobId = "";
            c.Path.Clear();
            c.Goal = "idle";
        }
        private void Die(Village v, Cat c, string reason)
        {
            CancelWork(v, c);
            Spill(v, c.Position, c.Cargo);
            c.Alive = false;
            c.DeathUnixMilliseconds = EpochUnixMs + (long)(TimeSeconds * 1000);
            c.ControlledBy = "";
            c.PregnantUntil = -1;
            c.PregnancyMateId = "";
            c.BedId = "";
            foreach (var item in v.Items.Where(i => i.LocationId == c.Id))
            {
                var pile = new Stockpile { Id = Id("spill"), Kind = "spill", Position = c.Position, Width = 1, Depth = 1, Capacity = 10000 };
                v.Stockpiles.Add(pile);
                item.LocationId = pile.Id;
            }
            c.Equipment.Clear();
            foreach (var officer in v.Officers.Where(o => o.CatId == c.Id).ToArray())
            {
                var successor = v.Cats.Where(x => x.Alive && x.Id != c.Id && x.OfficerRole == "").OrderBy(x => x.Id, StringComparer.Ordinal).FirstOrDefault();
                if (successor == null)
                    v.Officers.Remove(officer);
                else
                {
                    officer.CatId = successor.Id;
                    successor.OfficerRole = officer.Role;
                }
            }
            if (v.LeaderId == c.Id)
                v.LeaderId = v.Cats.Where(x => x.Alive).OrderBy(x => x.Id, StringComparer.Ordinal).FirstOrDefault()?.Id ?? "";
            Note(v, "death", c.Name + " died from " + reason, c.Id);
        }
        private void Population(Village v)
        {
            var alive = v.Cats.Where(c => c.Alive).ToArray();
            if (alive.Length == 0)
            {
                if (!CanFoundAt(v.Id, v.Center, v.Communal))
                {
                    if (!v.Events.Any(e => e.Kind == "recovery_blocked"))
                        Note(v, "recovery_blocked", "Refounding blocked by foreign territory", v.Id);
                    return;
                }
                foreach (var r in Reservations.Where(r => r.VillageId == v.Id).ToArray())
                    Reservations.Remove(r);
                foreach (var trade in TradeOffers.Where(t => t.FromVillageId == v.Id || t.ToVillageId == v.Id).ToArray())
                    CancelTrade(v, trade.Id);
                var replacement = Found(v.Id, v.Name, v.OwnerId, v.Center, v.Communal);
                replacement.Run = v.Run + 1;
                replacement.Blessings = v.Blessings;
                replacement.Research = new List<string>(v.Research);
                Villages[Villages.IndexOf(v)] = replacement;
                Note(replacement, "recovery", "Colony refounded after extinction");
                return;
            }
            int beds = v.Buildings.Count(b => b.Completed && b.Kind == "den") * DenCapacity(v);
            foreach (var c in alive)
            {
                if (c.BedId == "" && c.Migration != "arriving" && c.Migration != "departing")
                {
                    var den = v.Buildings.Where(b => b.Completed && b.Kind == "den").FirstOrDefault(b => v.Cats.Count(x => x.Alive && x.BedId == b.Id) < DenCapacity(v));
                    int reserved = alive.Count(x => x.PregnantUntil > 0);
                    if (den != null && alive.Count(x => x.BedId != "") + reserved < beds)
                    {
                        c.BedId = den.Id;
                        c.Migration = "resident";
                        c.ProbationUntil = -1;
                    }
                    else if (c.ProbationUntil >= 0 && TimeSeconds >= c.ProbationUntil)
                    {
                        CancelWork(v, c);
                        c.Migration = "departing";
                        c.Goal = "leaving";
                    }
                }
                if (c.PregnantUntil > 0 && TimeSeconds >= c.PregnantUntil)
                {
                    var kitten = NewCat(v, "Kitten " + NextId, c.Position);
                    kitten.AgeHours = 0;
                    kitten.BirthUnixMilliseconds = EpochUnixMs + (long)(TimeSeconds * 1000);
                    kitten.ParentIds.Add(c.Id);
                    if (c.PregnancyMateId != "")
                        kitten.ParentIds.Add(c.PregnancyMateId);
                    var mate = v.Cats.Find(x => x.Id == c.PregnancyMateId);
                    foreach (var stat in kitten.Stats)
                    {
                        double maternal = Amount(c.Stats, stat.Resource), paternal = mate == null ? maternal : Amount(mate.Stats, stat.Resource);
                        stat.Amount = Math.Clamp((maternal + paternal) * 0.5 + (Roll() - 0.5) * 16, 0, 100);
                    }
                    var den = v.Buildings.Where(b => b.Completed && b.Kind == "den").FirstOrDefault(b => v.Cats.Count(x => x.Alive && x.BedId == b.Id) < DenCapacity(v));
                    if (den != null)
                    {
                        kitten.BedId = den.Id;
                        v.Cats.Add(kitten);
                        c.PregnantUntil = -1;
                        c.PregnancyMateId = "";
                        Note(v, "birth", kitten.Name + " was born", kitten.Id);
                    }
                }
                if (c.AgeHours >= (c.Id == v.LeaderId || c.OfficerRole == "loremaster" ? 288 : 240) && Roll() < 0.0002 / Service(v, "elder_corner", "elderProtection", 1))
                    Die(v, c, "old age");
            }
            if ((long)TimeSeconds % 3600 == 0 && TimeSeconds - v.FoundedAt >= 36 * 3600 && Total(v, "food") >= alive.Length * 2.5 && Total(v, "water") >= alive.Length * 2.5)
            {
                int reservations = alive.Count(c => c.PregnantUntil > 0);
                if (alive.Length + reservations < beds)
                {
                    var mother = alive.FirstOrDefault(c => c.AgeHours >= 24 && c.AgeHours < 240 && c.PregnantUntil < 0);
                    var mate = alive.FirstOrDefault(c => c != mother && c.AgeHours >= 24 && c.AgeHours < 240);
                    if (mother != null && mate != null && Roll() < 0.001 + Math.Min(0.5, v.Blessings * 0.02) * 0.0007)
                    {
                        mother.PregnantUntil = TimeSeconds + 18 * 3600;
                        mother.PregnancyMateId = mate.Id;
                        Note(v, "pregnancy", mother.Name + " reserved a kitten bed", mother.Id);
                    }
                }
            }
            if (TimeSeconds - v.FoundedAt >= 30 * 3600 && TimeSeconds - v.LastMigration >= 12 * 3600)
            {
                v.LastMigration = TimeSeconds;
                if (Total(v, "food") >= alive.Length * 4 && Total(v, "water") >= alive.Length * 5 && alive.Length < 200)
                {
                    var migrant = NewCat(v, "Wanderer " + NextId, new Int2(v.Center.X, v.Center.Z + v.Radius + 3));
                    migrant.Migration = "arriving";
                    v.Cats.Add(migrant);
                    Note(v, "migration", migrant.Name + " approaches the gate", migrant.Id);
                }
            }
        }
        private bool HasOfficer(Village v, string role) => v.Officers.Any(o => o.Role == role && v.Cats.Any(c => c.Id == o.CatId && c.Alive));
        private void Emergency(Village v, string kind)
        {
            if (v.Jobs.Any(j => j.Kind == kind && !j.Completed))
                return;
            var worker = v.Cats.Where(c => c.Alive && c.ControlledBy == "" && c.AgeHours >= 12 && c.Migration != "arriving" && c.Migration != "departing" && !v.Jobs.Any(j => j.CatId == c.Id && !j.Completed && (j.Kind == "hunt" || j.Kind == "water")))
                .OrderByDescending(c => c.Health + c.Thirst + c.Hunger).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
            if (worker == null)
                return;
            CancelWork(v, worker);
            var result = Request(v, kind, worker, false);
            if (result.Success)
                Note(v, "emergency", worker.Name + " preempted work for physical " + kind, worker.Id);
        }
        private void Direct(Village v)
        {
            int alive = v.Cats.Count(c => c.Alive), hunts = (int)Math.Ceiling(alive * 6.0 / 15), waters = (int)Math.Ceiling(alive * 2.0 / 15);
            if (Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive * 6 && v.Jobs.Count(j => j.Kind == "hunt" && !j.Completed) < hunts)
            {
                var result = Request(v, "hunt", null, false);
                if (!result.Success && Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive)
                    Emergency(v, "hunt");
            }
            if (Total(v, "water") < alive * 8 && v.Jobs.Count(j => j.Kind == "water" && !j.Completed) < waters)
            {
                var result = Request(v, "water", null, false);
                if (!result.Success && Total(v, "water") < alive)
                    Emergency(v, "water");
            }
            if (!v.Jobs.Any(j => j.Kind == "scout" && !j.Completed) && ((long)TimeSeconds % 600 == 0 || !v.Known.Any(p => TileAt(p).Resource == "logs")))
                Request(v, "scout", null, false, "logs");
            if (HasOfficer(v, "forester"))
            {
                foreach (var resource in new[] { "logs", "stone" })
                    if (Total(v, resource) < 40 && !v.Jobs.Any(j => j.Resource == resource && !j.Completed))
                        MarkOfficerJob(v, Request(v, resource == "stone" ? "quarry" : "logs", null, false), "forester");
                if ((long)TimeSeconds % 600 == 0)
                    MarkOfficerJob(v, Request(v, "replant", null, false), "forester");
            }
            if (HasOfficer(v, "farmer"))
            {
                if (Total(v, "fibre") < 10 && !v.Jobs.Any(j => j.Kind == "fibre" && !j.Completed))
                    MarkOfficerJob(v, Request(v, "fibre", null, false), "farmer");
                foreach (var f in v.Farms.Where(f => f.WorkerId == ""))
                {
                    var c = AvailableCat(v, "farm");
                    if (c != null)
                    {
                        f.WorkerId = c.Id;
                        f.AutomatedBy = "farmer";
                        c.BuildingId = f.Id;
                    }
                }
            }
            if (HasOfficer(v, "steward"))
            {
                foreach (var pile in v.Stockpiles.Where(s => new[] { "gather", "spill", "farm_output", "fishing" }.Contains(s.Kind) && (s.Goods.Count > 0 || v.Items.Any(i => i.LocationId == s.Id))).ToArray())
                {
                    if (!v.Jobs.Any(j => j.SourceId == pile.Id && !j.Completed))
                        MarkOfficerJob(v, StartHaul(v, null, pile), "steward");
                }
            }
            StaffOfficers(v);
            PlanHousing(v);
            if (HasOfficer(v, "loremaster") && v.Blessings < 10 && Total(v, "materials") > 30 && !v.Jobs.Any(j => j.Kind == "offering" && !j.Completed))
                MarkOfficerJob(v, CreateInputJob(v, null, "offering", v.Center, "materials", 5, 30, ""), "loremaster");
            if (TimeSeconds - v.LastLeaderResearch >= 86400 && v.Cats.Any(c => c.Id == v.LeaderId && c.Alive))
            {
                var node = Catalog.Research.Where(n => !v.Research.Contains(n.Id) && n.Cost <= v.ResearchPoints && n.Prerequisites.All(v.Research.Contains)).OrderBy(n => n.LeaderPriority).ThenBy(n => n.Id, StringComparer.Ordinal).FirstOrDefault();
                if (node != null && Research(v, node.Id, false).Success)
                    v.LastLeaderResearch = TimeSeconds;
            }
        }
        private void TickStations(Village v)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed))
                foreach (var slot in b.Slots.ToArray())
                {
                    var c = v.Cats.Find(c => c.Id == slot.CatId && c.Alive);
                    if (c == null || c.ControlledBy != "" || c.JobId != "" || c.Goal.StartsWith("need_", StringComparison.Ordinal) || slot.Paused || b.Paused)
                        continue;
                    if (b.Kind == "research_hut" || b.Kind == "school")
                    {
                        c.Goal = "research";
                        if (Move(c, b.Position, 1))
                        {
                            v.ResearchPoints += WorkRate(v, c, "research", b.Kind) * Catalog.Effect(v, "researchRateMult", 1) * Catalog.BuildingEffect(v, b.Kind, "output", 1) / 600;
                            Add(c.Skills, "research", 1.0 / 3600);
                        }
                        continue;
                    }
                    if (b.Kind == "accounting_tent")
                        continue;
                    var outputItems = v.Items.Where(i => i.LocationId == b.Id && i.StationCompartment == "local_output").ToArray();
                    if (b.Outputs.Count > 0 || outputItems.Length > 0)
                    {
                        if (outputItems.Length > 0 && !Move(c, b.Position, 1))
                            continue;
                        var delivery = new Job { Kind = "production", Phase = "output_delivery", TargetId = b.Id, Position = b.Position, Local = b.Outputs.Select(s => new Stack(s.Resource, s.Amount)).ToList() };
                        b.Outputs.Clear();
                        StartJob(v, c, delivery);
                        slot.JobId = delivery.Id;
                        foreach (var item in outputItems)
                        {
                            item.LocationId = delivery.Id;
                            delivery.ItemIds.Add(item.Id);
                        }
                        continue;
                    }
                    if (b.Kind == "mouse_farm")
                    {
                        if (Available(v, "grain") < 1)
                        {
                            b.BlockedReason = "missing_grain";
                            continue;
                        }
                        string mouseJob = Id("job");
                        if (!Reserve(v, mouseJob, "grain", 1))
                            continue;
                        StartJob(v, c, new Job { Id = mouseJob, Kind = "mouse_farm", Phase = "fetch", Position = b.Position, TargetId = b.Id, RequiredWork = 600 });
                        continue;
                    }
                    var queue = slot.Queue;
                    var recipe = queue.Count > 0 ? Catalog.Recipe(queue[0].RecipeId) : null;
                    if (recipe == null)
                    {
                        slot.BlockedReason = b.BlockedReason = "queue_empty";
                        continue;
                    }
                    if (!Catalog.RecipeAvailable(v, recipe))
                    {
                        slot.BlockedReason = b.BlockedReason = "recipe_locked";
                        continue;
                    }
                    if (recipe.Inputs.Any(s => Available(v, s.Resource) + Amount(b.Inputs, s.Resource) < RecipeInput(v, recipe, s.Amount)))
                    {
                        slot.BlockedReason = b.BlockedReason = "missing_input";
                        continue;
                    }
                    if (recipe.Outputs.Any(s => Storage(v, s.Resource, s.Amount, b.Position) == null))
                    {
                        slot.BlockedReason = b.BlockedReason = "output_storage_full";
                        continue;
                    }
                    double localLoad = v.Jobs.Where(j => !j.Completed && j.Kind == "production" && j.TargetId == b.Id).Sum(j => j.Local.Sum(s => s.Amount));
                    if (localLoad + recipe.Inputs.Sum(s => RecipeInput(v, recipe, s.Amount)) > StationCapacity(v, b.Kind))
                    {
                        slot.BlockedReason = b.BlockedReason = "station_local_full";
                        continue;
                    }
                    string id = Id("job");
                    bool funded = true;
                    foreach (var input in recipe.Inputs)
                    {
                        double missing = Math.Max(0, RecipeInput(v, recipe, input.Amount) - Amount(b.Inputs, input.Resource));
                        if (missing > 0 && !Reserve(v, id, input.Resource, missing))
                        {
                            funded = false;
                            break;
                        }
                    }
                    if (!funded)
                    {
                        Release(id);
                        slot.BlockedReason = b.BlockedReason = "input_claimed";
                        continue;
                    }
                    var production = new Job { Id = id, Kind = "production", RecipeId = recipe.Id, Position = b.Position, TargetId = b.Id, Phase = "fetch", RequiredWork = RecipeDuration(v, recipe), Reserved = recipe.Inputs.Select(s => new Stack(s.Resource, RecipeInput(v, recipe, s.Amount))).ToList() };
                    foreach (var input in production.Reserved)
                    {
                        double take = Math.Min(input.Amount, Amount(b.Inputs, input.Resource));
                        if (take > 0)
                        {
                            Add(b.Inputs, input.Resource, -take);
                            Add(production.Local, input.Resource, take);
                        }
                    }
                    StartJob(v, c, production);
                    slot.JobId = id;
                    slot.BlockedReason = b.BlockedReason = "";
                }
        }
        private void TickFarms(Village v)
        {
            foreach (var f in v.Farms)
            {
                var c = v.Cats.Find(c => c.Id == f.WorkerId && c.Alive);
                if (c == null)
                {
                    f.BlockedReason = "worker_required";
                    continue;
                }
                if (c.JobId != "" || c.ControlledBy != "" || c.Goal.StartsWith("need_", StringComparison.Ordinal))
                    continue;
                c.Goal = "farm · " + f.Phase;
                if (f.Harvest > 0 || c.Cargo.Count > 0)
                {
                    if (c.Cargo.Count == 0)
                    {
                        if (!Move(c, f.Position, 1))
                        {
                            f.BlockedReason = "blocked_harvest_route";
                            continue;
                        }
                        double take = Math.Min(8, f.Harvest);
                        Add(c.Cargo, f.Crop, take);
                        f.Harvest -= take;
                        f.Phase = "hauling";
                        continue;
                    }
                    if (!Move(c, f.Handoff, 1))
                    {
                        f.BlockedReason = "blocked_handoff_route";
                        continue;
                    }
                    var pile = v.Stockpiles.Find(s => s.ManagedBy == f.Id);
                    if (pile == null)
                    {
                        pile = new Stockpile { Id = Id("harvest"), ManagedBy = f.Id, Kind = "farm_output", Position = f.Handoff, Width = 1, Depth = 1, Capacity = 100 };
                        v.Stockpiles.Add(pile);
                    }
                    if (pile.Goods.Sum(s => s.Amount) + c.Cargo.Sum(s => s.Amount) > pile.Capacity)
                    {
                        f.BlockedReason = "handoff_full";
                        continue;
                    }
                    foreach (var cargo in c.Cargo)
                        Add(pile.Goods, cargo.Resource, cargo.Amount);
                    c.Cargo.Clear();
                    Add(c.Skills, "haul", 0.25);
                    f.BlockedReason = "";
                    continue;
                }
                if (!Move(c, f.Position, 1))
                {
                    f.BlockedReason = "blocked_route";
                    continue;
                }
                f.BlockedReason = "";
                f.Growth += WorkRate(v, c, "farm", "field") * Service(v, "field", "fieldStewardship", 1) * (f.FertilityAffectsGrowth ? Math.Max(0, f.Fertility) : 1);
                Add(c.Skills, "farm", 1.0 / 3600);
                double cycle = Math.Max(1, f.CycleSeconds), fraction = f.Growth / cycle;
                f.Phase = fraction < 1.0 / 12 ? "soil" : fraction < 0.25 ? "sprout" : fraction < 0.5 ? "growing" : fraction < 0.75 ? "mature" : "flowering";
                if (f.Growth >= cycle)
                {
                    f.Harvest = f.YieldPerTile * f.Width * f.Depth * (f.FertilityAffectsGrowth ? 1 : Math.Max(0, f.Fertility)) * Catalog.Effect(v, "farmYield", 1) * Catalog.Effect(v, "farmYieldMult", 1) * Catalog.BuildingEffect(v, "field", "output", 1);
                    f.Growth = 0;
                    f.Phase = "harvesting";
                }
            }
        }
        private void TickAccounting(Village v)
        {
            var tent = v.Buildings.Find(b => b.Kind == "accounting_tent" && b.Completed && b.Slots.Any(s => s.CatId != ""));
            if (tent == null || !HasOfficer(v, "accountant"))
            {
                v.Accounting = null;
                return;
            }
            var c = v.Cats.Find(c => c.Id == tent.Slots.First(s => s.CatId != "").CatId && c.Alive);
            if (c == null || c.ControlledBy != "" || c.JobId != "" || c.Goal.StartsWith("need_", StringComparison.Ordinal))
                return;
            var round = v.Accounting;
            if (round == null || round.WorkerId != c.Id || round.BuildingId != tent.Id)
            {
                round = new AccountingRound { WorkerId = c.Id, BuildingId = tent.Id, Phase = "returning" };
                v.Accounting = round;
            }
            if (round.Phase == "waiting")
            {
                round.DwellSeconds += 1;
                if (round.DwellSeconds < 60)
                    return;
                round.Phase = "returning";
                round.DwellSeconds = 0;
            }
            if (round.Phase == "returning" || round.Phase == "return_to_tent" || round.Phase == "")
            {
                c.Goal = "accounting · returning to tent";
                if (!Move(c, tent.Position, 1))
                    return;
                round.Pending = v.Stockpiles.Where(s => s.Kind == "storage").OrderBy(s => Int2.Distance(tent.Position, s.Position)).ThenBy(s => s.Id, StringComparer.Ordinal).Select(s => s.Id).ToList();
                round.Unreachable.Clear();
                round.TargetId = "";
                round.Phase = "visiting";
            }
            if (round.TargetId == "")
            {
                while (round.Pending.Count > 0)
                {
                    string id = round.Pending[0];
                    round.Pending.RemoveAt(0);
                    var next = v.Stockpiles.Find(s => s.Id == id);
                    if (next == null)
                        continue;
                    if (Path(c.Position, next.Position, v) == null)
                    {
                        round.Unreachable.Add(id);
                        continue;
                    }
                    round.TargetId = id;
                    round.DwellSeconds = 0;
                    break;
                }
                if (round.TargetId == "")
                {
                    if (Move(c, tent.Position, 1))
                    {
                        round.Phase = "waiting";
                        round.DwellSeconds = 0;
                    }
                    return;
                }
            }
            var pile = v.Stockpiles.Find(s => s.Id == round.TargetId);
            if (pile == null)
            {
                round.TargetId = "";
                return;
            }
            c.Goal = "accounting · " + pile.Id;
            if (!Move(c, pile.Position, 1))
            {
                if (c.BlockedReason == "blocked_route")
                {
                    round.Unreachable.Add(pile.Id);
                    round.TargetId = "";
                }
                return;
            }
            round.Phase = "counting";
            round.DwellSeconds += Service(v, "accounting_tent", "accountingSpeed", 1);
            tent.Progress = round.DwellSeconds;
            if (round.DwellSeconds >= 5)
            {
                pile.Report = pile.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToList();
                foreach (var kind in new[] { "tools", "weapons", "armor" })
                {
                    double amount = v.Items.Count(i => i.LocationId == pile.Id && i.Kind == EquipmentKind(kind));
                    if (amount > 0)
                        Add(pile.Report, kind, amount);
                }
                pile.CountedAt = TimeSeconds;
                tent.Progress = 0;
                round.DwellSeconds = 0;
                round.TargetId = "";
                round.Phase = "visiting";
            }
        }
        private void Governance(Village v)
        {
            if (v.Election == null && TimeSeconds >= v.NextElection)
                v.Election = new Election { Id = Id("election"), EndAt = TimeSeconds + 1800 };
            if (v.Election != null && TimeSeconds >= v.Election.EndAt)
            {
                var winner = v.Election.Votes.Where(b => v.Cats.Any(c => c.Id == b.CatId && c.Alive)).GroupBy(b => b.CatId).OrderByDescending(g => g.Sum(ballot => ballot.Weight)).ThenBy(g => g.Key, StringComparer.Ordinal).FirstOrDefault()?.Key ?? v.Cats.FirstOrDefault(c => c.Alive)?.Id;
                if (winner != null)
                    v.LeaderId = winner;
                v.Election = null;
                v.NextElection = TimeSeconds + 86400;
                Note(v, "election", "Leader term began", v.LeaderId);
            }
            if (v.KickPetition != null && TimeSeconds >= v.KickPetition.EndAt)
            {
                var petition = v.KickPetition;
                v.KickPetition = null;
                bool passed = petition.TargetId == v.LeaderId && petition.Votes.Where(b => b.CatId == petition.TargetId).Select(b => b.PlayerId).Distinct().Count() >= 5;
                if (passed)
                {
                    v.LeaderId = v.Cats.Where(c => c.Alive && c.Id != petition.TargetId && c.Migration != "arriving" && c.Migration != "departing").OrderByDescending(c => Amount(c.Stats, "leadership")).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault()?.Id ?? "";
                    if (v.Election == null)
                        v.Election = new Election { Id = Id("election"), Kind = "snap", EndAt = TimeSeconds + 1800 };
                    Note(v, "election", "Five players removed the leader; interim term began", v.LeaderId);
                }
                else
                    Note(v, "election", "Leader removal petition expired without five signatures");
            }
            if ((long)TimeSeconds % 3600 == 0 && TimeSeconds - v.FoundedAt > 8 * 3600 && v.Cats.Count(c => c.Alive) >= 20 && Total(v, "materials") + Total(v, "refined") > 250 && v.Raids.Count == 0 && Roll() < 0.1)
            {
                v.Raids.Add(new Raid { Id = Id("raid"), Position = new Int2(v.Center.X, v.Center.Z + v.Radius - 1) });
                Note(v, "raid", "Raiders approach the south gate");
            }
        }
    }
}
