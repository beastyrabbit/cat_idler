using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;

namespace IdleCatForest.SaveImport;

public static partial class LegacyImport
{
    private sealed partial class Context
    {
        public void Farms(Village v, JToken row)
        {
            foreach (var saved in Entries(J(row, "farms")))
            {
                var rect = saved["rect"]; string phase = S(saved, "workPhase");
                var farm = new Farm { Id = Q(v.Id, S(saved, "id")), Position = new Int2((int)N(rect, "x1"), (int)N(rect, "y1")), Width = (int)(N(rect, "x2") - N(rect, "x1") + 1), Depth = (int)(N(rect, "y2") - N(rect, "y1") + 1), Crop = S(saved, "crop"), WorkerId = Q(v.Id, S(saved, "workerId")), Growth = N(saved, "growthHours") * 3600, Harvest = N(saved, "pendingOutput"), Fertility = N(saved, "fertility", 1), PlantedAt = Time(saved, "plantedAt"), Stage = S(saved, "stage", "soil"), Phase = phase switch { "waiting_for_worker" => "soil", "traveling" => "travel", "planting" => "planting", "tending" => "growing", "harvesting" => "harvesting", "hauling" => "hauling", "output_blocked" => "harvest_ready", _ => throw new NotSupportedException("Unsupported farm work phase.") } };
                farm.CycleSeconds = 86400; farm.YieldPerTile = farm.Crop == "grain" ? 2 : 1; farm.FertilityAffectsGrowth = true; farm.Handoff = farm.Position; v.Farms.Add(farm);
                if (farm.WorkerId != "") { var c = v.Cats.Find(c => c.Id == farm.WorkerId); if (c == null) throw new InvalidDataException("Farm worker is absent."); if (c.BuildingId == "") c.BuildingId = farm.Id; }
            }
        }
        public void Migration(Village v, JToken row)
        {
            var state = J(row, "migrationState"); if (state?["lastEvaluatedCohortBucket"]?.Type != JTokenType.Null && state?["lastEvaluatedCohortBucket"] != null) v.MigrationCohortBucket = (long)N(state, "lastEvaluatedCohortBucket");
            foreach (var migrant in Entries(state?["probationaryMigrants"]))
            {
                var c = v.Cats.Find(c => c.Id == Q(v.Id, S(migrant, "id"))); if (c == null) throw new InvalidDataException("Migrant identity is absent.");
                c.Migration = S(migrant, "phase", "probationary"); c.ProbationUntil = v.FoundedAt + N(migrant, "housingDeadlineGameMinute") * 60;
                if (migrant["routeExterior"]?.Type != JTokenType.Null && migrant["routeExterior"] != null) c.MigrationExterior = Point(migrant["routeExterior"]);
                v.LastMigration = Math.Max(v.LastMigration, v.FoundedAt + N(migrant, "arrivedGameMinute") * 60);
            }
        }
        public void Governance(Village v)
        {
            foreach (var row in Of("events", v)) v.Events.Add(new Event { Id = Q(v.Id, S(row, "id")), Time = Time(row, "timestamp"), Kind = S(row, "type"), Text = S(row, "message"), EntityId = Q(v.Id, S(row, "catId")), ActorName = S(row, "actorName") });
            foreach (var row in Of("elections", v))
            {
                var election = new HistoricElection { Id = Q(v.Id, S(row, "id")), Kind = S(row, "runtimeKind", S(row, "kind")), WinnerId = Q(v.Id, S(row, "winnerCatId")), TargetId = Q(v.Id, S(row, "targetCatId")), OpenedAt = Time(row, "startedAt"), ClosesAt = Time(row, "endsAt"), ResolvedAt = Time(row, "resolvedAt"), Candidates = Strings(J(row, "candidateCatIds")).Select(id => Q(v.Id, id)).ToList() };
                foreach (var vote in Of("votes", v).Where(vote => Q(v.Id, S(vote, "electionId")) == election.Id)) election.Votes.Add(new Ballot { PlayerId = S(vote, "playerId"), CatId = Q(v.Id, S(vote, "catId")), Weight = N(vote, "weight", 1) });
                v.ElectionHistory.Add(election); if (election.ResolvedAt < 0)
                {
                    var active = new Election { Id = election.Id, Kind = election.Kind, TargetId = election.TargetId != "" ? election.TargetId : election.Votes.FirstOrDefault()?.CatId ?? "", EndAt = election.ClosesAt, Votes = election.Votes };
                    if (election.Kind == "vote_kick") { if (v.KickPetition != null) throw new NotSupportedException("Multiple concurrent vote-kick petitions are not supported."); v.KickPetition = active; }
                    else { if (v.Election != null) throw new NotSupportedException("Multiple concurrent leadership elections are not supported."); v.Election = active; }
                }
                v.NextElection = Math.Max(v.NextElection, election.OpenedAt + 86400);
            }
            foreach (var row in Of("raiders", v)) v.Raids.Add(new Raid { Id = Q(v.Id, S(row, "id")), RaidId = Q(v.Id, S(row, "raidId")), Position = Point(J(row, "position")), Destination = J(row, "target")?.Type == JTokenType.Object ? Point(J(row, "target")) : null, Strength = N(row, "strength"), Defense = N(row, "defense"), Health = N(row, "hp"), Phase = S(row, "status") == "advancing" ? "approaching" : S(row, "status") });
            foreach (var row in Of("zones", v))
            {
                string kind = S(row, "kind"); var zone = new Stockpile { Id = Q(v.Id, S(row, "id")), Kind = "zone_" + kind, Position = new Int2((int)N(row, "x1"), (int)N(row, "y1")), Width = (int)(N(row, "x2") - N(row, "x1") + 1), Depth = (int)(N(row, "y2") - N(row, "y1") + 1), ExpiresAt = Time(row, "expiresAt"), Capacity = 0 }; v.Stockpiles.Add(zone);
            }
        }
        public void Accounting(Village v, JToken row)
        {
            var ledger = J(row, "stockLedger"); if (ledger?["pileReports"] is JObject reports) foreach (var property in reports.Properties())
            { var pile = v.Stockpiles.Find(p => p.Id == Q(v.Id, property.Name)); if (pile != null) { pile.Report = Goods(property.Value["reported"]); pile.CountedAt = Time(property.Value, "lastCounted"); } }
            var saved = ledger?["activeRound"]; if (saved?.Type == JTokenType.Object) v.Accounting = new AccountingRound { WorkerId = Q(v.Id, S(saved, "workerId")), BuildingId = Q(v.Id, S(saved, "tentId")), Phase = S(saved, "phase"), TargetId = Q(v.Id, S(saved, "targetStockpileId")), Pending = Strings(saved["pendingStockpileIds"]).Select(id => Q(v.Id, id)).ToList(), Unreachable = Strings(saved["unreachableStockpileIds"]).Select(id => Q(v.Id, id)).ToList(), DwellSeconds = N(saved, "dwellElapsedMs") / 1000, TopologySignature = saved["topologySignature"]?.Value<ulong>() ?? 0 };
            if (ledger?["stewardManagedPiles"] is JObject managed) foreach (var property in managed.Properties())
            { var savedPile = new StewardPile { PileId = Q(v.Id, property.Name), StationId = Q(v.Id, S(property.Value, "stationId")), Resource = S(property.Value, "resource"), Active = B(property.Value, "active") }; v.StewardPiles.Add(savedPile); var pile = v.Stockpiles.Find(p => p.Id == savedPile.PileId); if (pile != null) pile.ManagedBy = savedPile.Active ? "steward" : ""; }
        }
        public void Trader(Village v, JToken row)
        {
            var saved = J(row, "trader"); v.Trader.LastDepartedAt = Time(row, "lastTraderDepartedAt");
            v.Trader.NextAt = Math.Max(world.TimeSeconds, v.Trader.LastDepartedAt + 12 * 3600);
            if (saved == null || saved.Type == JTokenType.Null) return;
            var trader = v.Trader; trader.Id = Q(v.Id, S(saved, "id")); trader.Position = Point(saved["position"]); trader.Phase = S(saved, "state"); trader.Until = Time(saved, "departAt", Time(saved, "arrivedAt", world.TimeSeconds) + 900); trader.Coins = N(saved, "coin"); trader.Goods = Goods(saved["stock"]); trader.BlockedReason = B(saved, "routeBlocked") ? "blocked_route" : "";
            if (saved["routeExterior"]?.Type != JTokenType.Null && saved["routeExterior"] != null) trader.Exterior = Point(saved["routeExterior"]);
            if (saved["visitDestination"]?.Type != JTokenType.Null && saved["visitDestination"] != null) trader.VisitDestination = Point(saved["visitDestination"]);
            Items(v, saved["items"], trader.Items);
            if (trader.Phase != "trading") trader.Path = world.Path(trader.Position, trader.Phase == "arriving" ? trader.VisitDestination ?? v.Center : trader.Exterior ?? trader.Position) ?? new List<Int2>();
        }
        public void Trades(Village v, JToken row)
        {
            foreach (var saved in Entries(J(row, "villageTradeOffers"))) world.TradeOffers.Add(Trade(v, saved));
            foreach (var saved in Entries(J(row, "villageTradeCaravans")))
            {
                var trade = Trade(v, saved); var phase = S(saved, "phase"); trade.ActorId = Q(v.Id, S(saved, "actorId")); trade.X = N(saved["position"], "x"); trade.Z = N(saved["position"], "y"); trade.AcceptedAt = Time(saved, "acceptedAt"); trade.LastAdvancedAt = Time(saved, "lastAdvancedAt"); trade.PathIndex = (int)N(saved, "nextWaypoint"); trade.ReturnPath = Points(saved["returnRoute"]);
                trade.Status = phase switch { "outbound" => "outbound", "waiting_at_target" => "outbound", "returning" => "returning", "waiting_at_source" => "unloading", _ => throw new NotSupportedException("Unsupported caravan phase.") };
                trade.OfferedDelivered = phase == "returning" || phase == "waiting_at_source"; trade.Path = trade.OfferedDelivered ? new List<Int2>(trade.ReturnPath) : Points(saved["outboundRoute"]); if (phase == "waiting_at_target") trade.PathIndex = trade.Path.Count;
                Items(v, saved["offeredItems"], trade.OfferedItems); Items(v, saved["requestedItems"], trade.RequestedItems);
                foreach (var source in Entries(saved["offeredSources"])) trade.OfferedSources.Add(new Reservation { OwnerId = trade.Id, VillageId = trade.FromVillageId, PileId = Q(trade.FromVillageId, S(source, "stockpileId")), Resource = trade.Offered.Resource, Amount = N(source, "amount") });
                foreach (var source in Entries(saved["requestedSources"])) trade.RequestedSources.Add(new Reservation { OwnerId = trade.Id, VillageId = trade.ToVillageId, PileId = Q(trade.ToVillageId, S(source, "stockpileId")), Resource = trade.Requested.Resource, Amount = N(source, "amount") });
                world.TradeOffers.Add(trade);
            }
        }
        private static TradeOffer Trade(Village v, JToken row) => new TradeOffer { Id = Q(v.Id, S(row, "id")), FromVillageId = S(row, "fromColonyId"), ToVillageId = S(row, "toColonyId"), Offered = new IdleCatForest.Simulation.Stack(S(row, "offeredKind"), N(row, "offeredAmount")), Requested = new IdleCatForest.Simulation.Stack(S(row, "requestedKind"), N(row, "requestedAmount")) };
        public void Transport(Village v, JToken row)
        {
            var state = J(row, "transportState"); foreach (var p in Points(state?["trackTiles"])) world.TileAt(p).Rail = true;
            foreach (var dock in Entries(state?["docks"])) world.TileAt(Point(dock["landTile"])).Dock = true;
            foreach (var vehicle in Entries(state?["vehicles"])) v.Vehicles.Add(new Vehicle { Id = Q(v.Id, S(vehicle, "id")), Mode = S(vehicle, "mode"), Position = Point(vehicle["home"]), RouteId = Q(v.Id, S(vehicle, "assignedRouteId")) });
            foreach (var saved in Entries(state?["routes"]))
            {
                string phase = S(saved, "phase"); if (phase == "complete" || phase == "cancelled") continue;
                var r = new TransportRoute { Id = Q(v.Id, S(saved, "id")), Mode = S(saved, "mode"), SourceId = Q(v.Id, S(saved, "sourceStockpileId")), DestinationId = Q(v.Id, S(saved, "destinationStockpileId")), Resource = S(saved, "resource"), Amount = N(saved, "amount"), CatId = Q(v.Id, S(saved, "assignedCatId")), Phase = phase == "waiting_for_storage" ? "unloading" : phase, Path = Points(saved["path"]), PathIndex = (int)N(saved, "pathIndex"), SegmentProgress = N(saved, "segmentProgress"), VehicleId = Q(v.Id, S(saved, "vehicleId")), Position = Point(saved["position"]), Repeat = B(saved, "repeat") }; v.Routes.Add(r);
                var vehicle = v.Vehicles.Find(vehicle => vehicle.Id == r.VehicleId); var c = v.Cats.Find(c => c.Id == r.CatId); if (vehicle == null || c == null) throw new InvalidDataException("Active route vehicle or driver is absent."); vehicle.Position = r.Position; vehicle.RouteId = r.Id; c.BuildingId = r.Id; World.Add(vehicle.Cargo, r.Resource, N(saved, "cargoLoaded"));
            }
            foreach (var saved in Entries(state?["projects"]))
            {
                string phase = S(saved, "phase"); var path = Points(saved["tiles"]); var job = new Job { Id = Q(v.Id, S(saved, "id")), Kind = S(saved, "kind") switch { "track" => "rail", "dock" => "dock", "rolling_stock" => "wagon", "vessel" => "vessel", _ => throw new NotSupportedException("Unknown transport project.") }, CatId = Q(v.Id, S(saved, "assignedCatId")), Phase = phase == "fetching" ? "fetch" : "working", Completed = phase == "complete" || phase == "cancelled", Path = path, Position = path.FirstOrDefault(), RequiredWork = N(saved, "requiredWorkSeconds"), Progress = N(saved, "workDoneSeconds"), Local = Goods(saved["delivered"]), StartedAt = world.TimeSeconds };
                if (job.Completed) job.CatId = ""; else Claims(v, job.Id, saved["reservations"]); job.Resource = job.Local.FirstOrDefault()?.Resource ?? Entries(saved["reservations"]).Select(r => S(r, "kind")).FirstOrDefault() ?? ""; v.Jobs.Add(job);
            }
        }
    }
}
