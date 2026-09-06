using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;
using Stack = IdleCatForest.Simulation.Stack;

namespace IdleCatForest.SaveImport;

public static partial class LegacyImport
{
    private sealed partial class Context
    {
        private void Claims(Village v, string owner, JToken entries)
        { foreach (var r in Entries(entries)) world.Reservations.Add(new Reservation { OwnerId = owner, VillageId = v.Id, PileId = Q(v.Id, S(r, "sourceStockpileId")), Resource = S(r, "kind"), Amount = N(r, "amount") }); }
        private List<Stack> TakeLocal(Village v, string raw)
        {
            var pile = v.Stockpiles.Find(p => p.Id == Q(v.Id, raw)); if (pile == null) return new List<Stack>();
            var result = pile.Goods; pile.Goods = new List<Stack>(); v.Stockpiles.Remove(pile); return result;
        }
        private static void Merge(List<Stack> to, IEnumerable<Stack> from)
        { foreach (var s in from) World.Add(to, s.Resource, s.Amount); }
        private static string Kind(string kind) => kind switch
        { "supply_food" or "leader_plan_hunt" or "hunt_expedition" => "hunt", "supply_water" or "fetch_water" => "water", "gather_logs" => "logs", "forage_fibre" => "fibre", "build_house" or "leader_plan_house" => "build", "build_road" => "road", "replant_tree" => "replant", "explore" => "scout", "train_warrior" => "train", "expand_village" => "expand", "ritual" or "carry_offering" or "perform_offering" => "offering", "haul_gather_spot" => "haul", "quarry" or "fish" => kind, _ => throw new NotSupportedException("Unsupported legacy job kind.") };
        public void Jobs(Village v)
        {
            foreach (var row in Of("jobs", v))
            {
                var original = S(row, "kind"); var meta = J(row, "metadata"); var status = S(row, "status"); var cat = v.Cats.Find(c => c.Id == Q(v.Id, S(row, "assignedCatId")));
                var j = new Job { Id = Q(v.Id, S(row, "id")), OriginalKind = original, Kind = Kind(original), CatId = cat?.Id ?? "", Requester = S(row, "requestedByType", "player"), Manual = S(row, "requestedByType") == "player", StartedAt = Time(row, "startedAt", Time(row, "createdAt", world.TimeSeconds)), RequiredWork = N(row, "baseDurationSec", 1), SpeedMultiplier = N(row, "speedMultiplier", 1), YieldMultiplier = N(row, "yieldMultiplier", 1), Amount = N(row, "yieldMultiplier"), BoostCount = (int)N(row, "clickTimeReducedSec"), CompletionTime = Time(row, "completedAt"), Completed = status == "completed" || status == "failed" || status == "cancelled", Origin = cat?.Position ?? v.Center, Position = meta?["site"]?.Type == JTokenType.Object ? Point(meta["site"]) : cat?.Position ?? v.Center, Accepted = B(meta, "accepted") };
                if (j.Completed) j.CatId = "";
                double end = Time(row, "endsAt"); j.Progress = end >= 0 ? Math.Clamp(j.RequiredWork - (end - world.TimeSeconds) * j.SpeedMultiplier, 0, j.RequiredWork) : 0;
                j.Phase = j.Accepted ? "working" : "travel"; j.Resource = j.Kind switch { "hunt" => "food", "water" => "water", "logs" => "logs", "quarry" => "materials", "fibre" => "fibre", "fish" => "fish", _ => "" };
                switch (S(meta, "kind", "none"))
                {
                    case "construction":
                        j.TargetId = Q(v.Id, S(meta, "buildingId")); j.PendingBuildingKind = S(meta, "buildingType", "den"); var b = v.Buildings.Find(b => b.Id == j.TargetId); if (b != null) j.Position = b.Position;
                        else if (!j.Completed) { j.TargetId = ""; j.BlockedReason = "site_required"; }
                        break;
                    case "roadConstruction": j.Path = Points(meta["tiles"]); j.PathIndex = (int)N(meta, "nextTile"); j.Progress = N(meta, "workMs") / 1000; j.Resource = "materials"; j.Phase = "fetch"; Claims(v, j.Id, meta["reservations"]); break;
                    case "expansion":
                        j.Position = Point(meta["target"]); j.LinkedJobId = Q(v.Id, S(meta, "sourceBuildJobId")); j.Progress = N(meta, "wallWorkMs") / 1000;
                        var expansion = derived?[v.Id]?["Expansions"]?[Raw(v.Id, j.Id)]; j.AgriculturalExpansion = B(expansion, "Agricultural");
                        foreach (var edge in Entries(expansion?["BoundaryEdges"])) j.CompletionBoundaryEdges.Add(new BoundaryEdge { From = Point(edge["From"]), To = Point(edge["To"]) }); break;
                    case "gatherHaul": j.SourceId = Q(v.Id, S(meta, "stockpileId")); PrepareHaul(v, j); break;
                    case "stockpileHaul": j.SourceId = Q(v.Id, S(meta, "sourceStockpileId")); j.TargetId = Q(v.Id, S(meta, "destinationStockpileId")); j.Resource = S(meta, "resourceKind"); Merge(j.Local, TakeLocal(v, S(meta, "transitId"))); if (j.Local.Count > 0) j.Phase = "output_delivery"; else PrepareHaul(v, j); break;
                    case "offeringCarry":
                    case "offeringRitual":
                        j.Resource = S(meta, "resourceKind", "materials"); j.Position = v.Center; Merge(j.Local, TakeLocal(v, S(meta, "escrowId"))); j.Amount = N(meta, "amount", N(meta, "delivered"));
                        if (S(meta, "kind") == "offeringCarry") { j.SourceId = Q(v.Id, S(meta, "sourceStockpileId")); var p = v.Stockpiles.Find(p => p.Id == j.SourceId); double amount = Math.Min(World.Amount(p?.Goods ?? new List<Stack>(), j.Resource), Math.Max(0, N(row, "yieldMultiplier") - World.Amount(j.Local, j.Resource))); if (amount > 0) { world.Reservations.Add(new Reservation { OwnerId = j.Id, VillageId = v.Id, PileId = p.Id, Resource = j.Resource, Amount = amount }); j.Phase = "fetch"; } }
                        break;
                    case "scout": j.ScoutMission = S(meta, "mission"); j.Resource = j.ScoutMission == "explore" ? "" : j.ScoutMission; j.Position = Point(meta?["destination"] ?? meta?["target"]); if (B(meta, "found")) j.Phase = "return"; break;
                    case "none": case "site": case "hauling": break;
                    default: throw new NotSupportedException("Unsupported legacy job metadata.");
                }
                v.Jobs.Add(j);
            }
        }
        private void PrepareHaul(Village v, Job j)
        {
            if (j.Completed) return; var source = v.Stockpiles.Find(p => p.Id == j.SourceId); if (source == null) throw new InvalidDataException("Haul source is absent.");
            var goods = source.Goods.FirstOrDefault(s => s.Amount > 0); if (goods == null) { j.Completed = true; j.CatId = ""; return; }
            j.Resource = goods.Resource; j.Amount = Math.Min(8, goods.Amount); j.Position = source.Position; j.Phase = "fetch";
            world.Reservations.Add(new Reservation { OwnerId = j.Id, VillageId = v.Id, PileId = source.Id, Resource = j.Resource, Amount = j.Amount });
        }
        public void Stations(Village v)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed))
            {
                var raw = Raw(v.Id, b.Id); var inputs = TakeLocal(v, "station-input:" + raw); var outputs = TakeLocal(v, "station-output:" + raw);
                // Functional output counters mirror identities at the building's local-output compartment.
                foreach (var pair in new[] { ("tools", "tool"), ("weapons", "weapon"), ("armor", "armor") })
                { double units = v.Items.Count(i => i.Kind == pair.Item2 && i.LocationKind == "station" && i.LocationId == b.Id && i.StationCompartment == "local_output"); World.Add(outputs, pair.Item1, -Math.Min(units, World.Amount(outputs, pair.Item1))); }
                // Inputs are station-owned and only transferred to one slot once. Other slots compete for the remainder.
                Merge(b.Inputs, inputs); Merge(b.Outputs, outputs);
                foreach (var slot in b.Slots.Where(s => s.CatId != ""))
                {
                    var c = v.Cats.Find(c => c.Id == slot.CatId); if (c == null) throw new InvalidDataException("Station worker is absent.");
                    if (v.Jobs.Any(j => !j.Completed && j.CatId == c.Id)) continue;
                    var recipe = Catalog.Recipe(slot.Queue.FirstOrDefault()?.RecipeId ?? b.Queue.FirstOrDefault()?.RecipeId ?? "");
                    bool outgoing = c.ImportedCargoSource.StartsWith("station-out|", StringComparison.Ordinal);
                    if (outgoing)
                    { var job = StationJob(v, b, slot, c, recipe); job.Phase = "output_delivery"; continue; }
                    bool inputCargo = c.ImportedCargoSource.StartsWith("station-in|", StringComparison.Ordinal);
                    // TickStations adopts station output after physical pickup; already-carried cargo resumes separately.
                    if (!inputCargo && slot.Progress <= 0 && (b.Outputs.Count > 0 || v.Items.Any(i => i.LocationId == b.Id && i.StationCompartment == "local_output"))) continue;
                    if (recipe == null) continue;
                    if (slot.Progress > 0 || inputCargo || recipe.Inputs.All(s => World.Amount(b.Inputs, s.Resource) >= s.Amount))
                    {
                        var job = StationJob(v, b, slot, c, recipe); job.Phase = inputCargo ? "input_delivery" : "working";
                        foreach (var s in recipe.Inputs) { double take = Math.Min(s.Amount, World.Amount(b.Inputs, s.Resource)); if (take > 0) { World.Add(b.Inputs, s.Resource, -take); World.Add(job.Local, s.Resource, take); } }
                        job.Progress = Math.Clamp(slot.Progress, 0, recipe.Seconds);
                    }
                }
            }
        }
        private Job StationJob(Village v, Building b, WorkSlot slot, Cat c, Recipe recipe)
        {
            var j = new Job { Id = Q(v.Id, "import-station-" + Raw(v.Id, b.Id) + "-" + Raw(v.Id, c.Id)), Kind = "production", OriginalKind = "station", CatId = c.Id, TargetId = b.Id, RecipeId = recipe?.Id ?? "", Position = b.Position, Origin = c.Position, RequiredWork = recipe?.Seconds ?? 1, StartedAt = world.TimeSeconds }; slot.JobId = j.Id; v.Jobs.Add(j); return j;
        }
        public void Cargo(Village v)
        {
            foreach (var c in v.Cats.Where(c => c.Cargo.Count > 0 || c.ImportedCargoSource.StartsWith("personal-need|", StringComparison.Ordinal)))
            {
                var marker = c.ImportedCargoSource; var parts = marker.Split('|');
                if (marker.StartsWith("personal-need|", StringComparison.Ordinal))
                {
                    var encoded = marker.Substring("personal-need|".Length); var saved = encoded.StartsWith("{", StringComparison.Ordinal) ? IdleCatForest.Authority.WireJson.Decode<JObject>(encoded) : null;
                    var task = saved == null ? encoded.Split('|')[0] : S(saved, "task"); c.Goal = task switch { "eat" => "need_eat", "drink" => "need_drink", "sleep" => "need_sleep", _ => throw new NotSupportedException("Unsupported personal need continuation.") };
                    c.PersonalBrewUsed = B(saved, "brewUsed"); c.ResumeJobId = v.Jobs.Find(j => !j.Completed && j.CatId == c.Id)?.Id ?? ""; continue;
                }
                // The hidden transit pile mirrors the on-paw amount in the old aggregate. Move it rather than credit it twice.
                if (parts.Length >= 3 && (parts[0] == "station-in" || parts[0] == "construction-in"))
                {
                    var hidden = v.Stockpiles.Find(p => p.Id == Q(v.Id, parts[2])); if (hidden != null) foreach (var cargo in c.Cargo) World.Add(hidden.Goods, cargo.Resource, -Math.Min(cargo.Amount, World.Amount(hidden.Goods, cargo.Resource)));
                }
                var job = v.Jobs.Find(j => !j.Completed && j.CatId == c.Id);
                if (job == null) { job = new Job { Id = Q(v.Id, "import-cargo-" + Raw(v.Id, c.Id)), Kind = "haul", OriginalKind = "loose_cargo", CatId = c.Id, Position = c.Position, Origin = c.Position, Phase = "output_delivery", StartedAt = world.TimeSeconds }; v.Jobs.Add(job); }
                else if (marker.StartsWith("station-out|", StringComparison.Ordinal) || marker.StartsWith("farm-out|", StringComparison.Ordinal) || marker.StartsWith("steward-haul|", StringComparison.Ordinal)) job.Phase = "output_delivery";
                else if (marker.StartsWith("station-in|", StringComparison.Ordinal)) job.Phase = "input_delivery";
                var carriedItems = v.Items.Where(i => i.LocationKind == "carrier" && i.LocationId == c.Id).ToArray();
                if (parts.Length >= 4 && parts[3].StartsWith("item:", StringComparison.Ordinal))
                { var id = Q(v.Id, parts[3].Substring(5)); var item = carriedItems.FirstOrDefault(i => i.Id == id); if (item == null) throw new InvalidDataException("Carried item identity is absent."); carriedItems = new[] { item }; }
                else if (parts[0] != "station-out") carriedItems = Array.Empty<Item>();
                if (carriedItems.Length > 0)
                {
                    foreach (var item in carriedItems) { job.ItemIds.Add(item.Id); item.LocationId = job.Id; }
                    // Legacy Carrying has one scalar kind/amount: here it only projects the exact cargo, including Refined variants.
                    c.Cargo.Clear();
                }
            }
            foreach (var p in v.Stockpiles.Where(p => p.Kind == "legacy_local").ToArray())
            { if (p.Goods.Count == 0) v.Stockpiles.Remove(p); else if (Raw(v.Id, p.Id).StartsWith("construction-input:", StringComparison.Ordinal)) { var b = v.Buildings.Find(b => Raw(v.Id, p.Id) == "construction-input:" + Raw(v.Id, b.Id)); if (b == null) throw new InvalidDataException("Construction local owner is absent.");/* deliveredLumber etc already own the exact material */v.Stockpiles.Remove(p); } else { p.Kind = "spill"; p.ResourceLimits.Clear(); p.Capacity = Math.Max(p.Capacity, p.Goods.Sum(s => s.Amount)); v.Events.Add(new Event { Time = world.TimeSeconds, Kind = "migration", Text = "Unassigned transit goods remain at their saved position in a recoverable pile.", EntityId = p.Id }); } }
        }
        public void Items(Village v, JToken token, List<Item> target)
        {
            var entries = token is JArray ? Entries(token) : Entries(token?["instances"]);
            foreach (var row in entries)
            {
                var spec = S(row, "item").Split(':'); if (spec.Length != 3) throw new InvalidDataException("Normalized item key is invalid.");
                var location = row["location"]; var kind = S(location, "kind", "legacy_treasury"); var item = new Item { Id = Q(v.Id, S(row, "id")), Kind = spec[0], Material = spec[1], Quality = int.Parse(spec[2], System.Globalization.CultureInfo.InvariantCulture), Condition = N(row, "durability"), MaxCondition = N(row, "maxDurability", 1), VillageId = v.Id, LocationKind = kind, StationCompartment = S(location, "compartment"), Credited = row["credited"] == null || B(row, "credited"), AutomaticallyIssued = B(row, "autoIssued"), ActiveJobId = Q(v.Id, S(row, "activeJobId")) };
                int grams = item.Kind switch { "mug" => 400, "bowl" => 500, "furniture" => 8000, "tool" => 2500, "weapon" => 3000, "armor" => 6000, "clothing" => 800, "trinket" => 300, "toy" => 500, "brick" => 2000, _ => throw new NotSupportedException("Unknown item kind.") };
                int material = item.Material switch { "fibre" => 40, "leather" => 70, "gem" => 80, "sand" => 110, "bone" => 90, "wood" => 100, "clay" => 120, "metal" => 150, "stone" => 180, _ => throw new NotSupportedException("Unknown item material.") };
                item.Weight = Math.Max(1, grams * material * new[] { 110, 105, 100, 95, 90 }[Math.Clamp(item.Quality, 0, 4)] / 10000) / 1000d;
                item.LocationId = kind switch { "stockpile" => Q(v.Id, S(location, "stockpile_id")), "station" => Q(v.Id, S(location, "building_id")), "equipped" or "carrier" => Q(v.Id, S(location, "cat_id")), "trader" => "trader", "caravan" => "escrow", "legacy_treasury" => v.Stockpiles.FirstOrDefault(p => p.Kind == "storage")?.Id ?? throw new InvalidDataException("Legacy item treasury has no storage."), _ => throw new NotSupportedException("Unsupported item location.") };
                if (kind == "equipped") { var cat = v.Cats.Find(c => c.Id == item.LocationId); if (cat == null) throw new InvalidDataException("Equipped item owner is absent."); cat.Equipment.Add(item.Id); }
                target.Add(item);
            }
            // Functional scalar counters are a legacy compatibility projection of these exact units.
            foreach (var p in v.Stockpiles) foreach (var pair in new[] { ("tools", "tool"), ("weapons", "weapon"), ("armor", "armor") })
            { double units = target.Count(i => i.Kind == pair.Item2 && i.LocationId == p.Id); if (units > 0) World.Add(p.Goods, pair.Item1, -Math.Min(units, World.Amount(p.Goods, pair.Item1))); }
        }
    }
}
