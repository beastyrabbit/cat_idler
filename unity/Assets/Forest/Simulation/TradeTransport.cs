using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult OfferTrade(Village v, GameAction a)
        {
            var other = Village(a.OtherVillageId);
            if (other == null || other.Id == v.Id || !v.Contacts.Contains(other.Id) || !other.Contacts.Contains(v.Id))
                return ActionResult.Fail("Mutual shrine-delivered contact required");
            if (!Catalog.Resources.Contains(a.Resource) || !Catalog.Resources.Contains(a.OtherResource) || a.Resource == "blessings" || a.OtherResource == "blessings" || a.Amount <= 0 || a.OtherAmount <= 0 || a.Amount > 1000000 || a.OtherAmount > 1000000)
                return ActionResult.Fail("Invalid finite barter");
            if (TradeOffers.Count(t => t.FromVillageId == v.Id && t.Status == "offered") >= 32)
                return ActionResult.Fail("Open offer limit reached");
            var trade = new TradeOffer { Id = Id("trade"), FromVillageId = v.Id, ToVillageId = other.Id, Offered = new Stack(a.Resource, a.Amount), Requested = new Stack(a.OtherResource, a.OtherAmount) };
            TradeOffers.Add(trade);
            return ActionResult.Ok(trade.Id);
        }
        private static string EquipmentKind(string resource) => resource == "tools" ? "tool" : resource == "weapons" ? "weapon" : resource == "armor" ? "armor" : "";
        private bool CanEscrow(Village v, Stack s)
        {
            string kind = EquipmentKind(s.Resource);
            return kind == "" ? Available(v, s.Resource) >= s.Amount : s.Amount == (int)s.Amount && v.Items.Count(i => i.Kind == kind && v.Stockpiles.Any(p => p.Id == i.LocationId && p.Kind == "storage")) >= s.Amount;
        }
        private void Escrow(Village v, Stack s, List<Item> items)
        {
            string kind = EquipmentKind(s.Resource);
            if (kind == "")
            {
                if (!Spend(v, s.Resource, s.Amount))
                    throw new InvalidOperationException("Trade escrow race");
                return;
            }
            foreach (var item in v.Items.Where(i => i.Kind == kind && v.Stockpiles.Any(p => p.Id == i.LocationId && p.Kind == "storage")).OrderBy(i => i.Id, StringComparer.Ordinal).Take((int)s.Amount).ToArray())
            {
                items.Add(item);
                v.Items.Remove(item);
                item.LocationId = "escrow";
            }
        }
        private ActionResult AcceptTrade(Village receiver, string id)
        {
            var t = TradeOffers.Find(t => t.Id == id && t.Status == "offered" && t.ToVillageId == receiver.Id);
            if (t == null)
                return ActionResult.Fail("Offer not addressed to selected village");
            var sender = Village(t.FromVillageId);
            if (sender == null)
                return ActionResult.Fail("Source village missing");
            if (!CanEscrow(sender, t.Offered) || !CanEscrow(receiver, t.Requested))
                return ActionResult.Fail("Unclaimed barter goods unavailable");
            var path = Path(sender.Center, receiver.Center);
            if (path == null)
                return ActionResult.Fail("No physical gate-to-gate land route");
            Escrow(sender, t.Offered, t.OfferedItems);
            Escrow(receiver, t.Requested, t.RequestedItems);
            t.Path = path;
            t.ReturnPath = new List<Int2>(path);
            t.ReturnPath.Reverse();
            t.ReturnPath.Add(sender.Center);
            t.PathIndex = 0;
            t.Progress = 0;
            t.X = sender.Center.X;
            t.Z = sender.Center.Z;
            t.AcceptedAt = t.LastAdvancedAt = TimeSeconds;
            t.Status = "outbound";
            return ActionResult.Ok(t.Id);
        }
        private bool DepositTrade(Village target, Stack goods, List<Item> items, Int2 at, bool force)
        {
            var pile = Storage(target, goods.Resource, goods.Amount, at);
            if (pile == null && force)
            {
                pile = new Stockpile { Id = Id("escrow_return"), Position = at, Kind = "spill", Capacity = Math.Max(1000000, goods.Amount), Width = 1, Depth = 1 };
                target.Stockpiles.Add(pile);
            }
            if (pile == null)
                return false;
            if (items.Count > 0)
            {
                foreach (var item in items)
                {
                    item.VillageId = target.Id;
                    item.LocationId = pile.Id;
                    target.Items.Add(item);
                }
                items.Clear();
            }
            else
                Add(pile.Goods, goods.Resource, goods.Amount);
            return true;
        }
        private ActionResult CancelTrade(Village controller, string id)
        {
            var t = TradeOffers.Find(t => t.Id == id);
            if (t == null || (t.FromVillageId != controller.Id && t.ToVillageId != controller.Id))
                return ActionResult.Fail("Trade controller required");
            if (t.Status == "completed" || t.Status == "cancelled")
                return ActionResult.Fail("Trade already closed");
            if (t.OfferedDelivered)
                return ActionResult.Fail("The outward delivery is complete. Payment must finish returning.");
            if (t.Status != "offered")
            {
                var from = Village(t.FromVillageId);
                var to = Village(t.ToVillageId);
                if (from != null && !t.OfferedDelivered)
                    DepositTrade(from, t.Offered, t.OfferedItems, from.Center, true);
                if (to != null)
                    DepositTrade(to, t.Requested, t.RequestedItems, to.Center, true);
            }
            t.Status = "cancelled";
            return ActionResult.Ok();
        }
        private void TickTrades()
        {
            foreach (var t in TradeOffers.Where(t => t.Status == "outbound" || t.Status == "returning" || t.Status == "unloading"))
            {
                if (t.Status == "unloading")
                {
                    var source = Village(t.FromVillageId);
                    var target = Village(t.ToVillageId);
                    if (source == null || target == null)
                        continue;
                    // Both destinations must have room before either side receives credit.
                    if (Storage(source, t.Requested.Resource, t.Requested.Amount, source.Center) == null || !t.OfferedDelivered && Storage(target, t.Offered.Resource, t.Offered.Amount, target.Center) == null)
                        continue;
                    DepositTrade(source, t.Requested, t.RequestedItems, source.Center, false);
                    if (!t.OfferedDelivered)
                    {
                        DepositTrade(target, t.Offered, t.OfferedItems, target.Center, false);
                        t.OfferedDelivered = true;
                    }
                    t.Status = "completed";
                    Note(source, "trade", "Caravan returned and barter completed", t.Id);
                    Note(target, "trade", "Barter delivery completed", t.Id);
                    continue;
                }
                if (t.PathIndex < t.Path.Count && !Walkable(t.Path[t.PathIndex]))
                    continue;
                t.Progress += 0.8;
                if (t.Progress >= 1)
                {
                    t.Progress -= 1;
                    if (t.PathIndex < t.Path.Count)
                    {
                        t.X = t.Path[t.PathIndex].X;
                        t.Z = t.Path[t.PathIndex].Z;
                    }
                    t.PathIndex++;
                }
                t.LastAdvancedAt = TimeSeconds;
                if (t.PathIndex < t.Path.Count)
                    continue;
                if (t.Status == "outbound")
                {
                    if (t.ReturnPath.Count == 0)
                    {
                        t.ReturnPath = new List<Int2>(t.Path);
                        t.ReturnPath.Reverse();
                        t.ReturnPath.Add(Village(t.FromVillageId).Center);
                    }
                    t.Path = new List<Int2>(t.ReturnPath);
                    t.PathIndex = 0;
                    t.Status = "returning";
                }
                else
                    t.Status = "unloading";
            }
        }
        private ActionResult Buy(Village v, GameAction a)
        {
            if (v.Trader.Phase != "trading")
                return ActionResult.Fail("Trader must reach the shrine");
            if (a.Amount <= 0 || a.Amount > 1000000 || !Catalog.Resources.Contains(a.Resource) || a.Resource == "blessings")
                return ActionResult.Fail("Invalid purchase");
            if (Amount(v.Trader.Goods, a.Resource) < a.Amount)
                return ActionResult.Fail("Trader sold out");
            double cost = a.Amount * 2;
            if (v.Coins < cost)
                return ActionResult.Fail("Insufficient coin");
            var pile = Storage(v, a.Resource, a.Amount, v.Center);
            if (pile == null)
                return ActionResult.Fail("Storage unavailable");
            Add(v.Trader.Goods, a.Resource, -a.Amount);
            Add(pile.Goods, a.Resource, a.Amount);
            v.Coins -= cost;
            v.Trader.Coins += cost;
            return ActionResult.Ok();
        }
        private ActionResult Sell(Village v, GameAction a)
        {
            if (v.Trader.Phase != "trading")
                return ActionResult.Fail("Trader must reach the shrine");
            var item = v.Items.Find(i => i.Id == a.TargetId && v.Stockpiles.Any(p => p.Id == i.LocationId && p.Kind == "storage"));
            if (item == null)
                return ActionResult.Fail("Exact stored item required");
            double price = (item.Quality + 1) * 4 * Catalog.Effect(v, "tradeValue", 1);
            if (v.Trader.Coins < price || item.Weight > 20 || v.Trader.Items.Sum(i => i.Weight) + item.Weight > 100)
                return ActionResult.Fail("Trader purse or wagon capacity exhausted");
            v.Items.Remove(item);
            item.LocationId = "trader";
            v.Trader.Items.Add(item);
            v.Trader.Coins -= price;
            v.Coins += price;
            return ActionResult.Ok(item.Id);
        }
        private void TickTrader(Village v)
        {
            var t = v.Trader;
            if (t.Phase == "absent")
            {
                if (TimeSeconds < t.NextAt)
                    return;
                t.Id = Id("trader");
                v.TraderVisitCount++;
                t.Exterior = new Int2(v.Center.X, v.Center.Z + v.Radius + 3);
                t.VisitDestination = v.Center;
                t.Position = t.Exterior.Value;
                t.Path = Path(t.Position, t.VisitDestination.Value) ?? new List<Int2>();
                t.PathIndex = 0;
                t.Phase = "arriving";
                t.Coins = 1000;
                t.Items.Clear();
                t.Goods.Clear();
                foreach (var resource in new[] { "food", "water", "herbs", "materials", "logs", "planks", "blocks", "ore", "metal", "fibre", "grain" })
                    t.Goods.Add(new Stack(resource, 20 + Random() % 40));
            }
            if (t.Phase == "trading")
            {
                if (TimeSeconds < t.Until)
                    return;
                t.Path = Path(t.Position, t.Exterior ?? new Int2(v.Center.X, v.Center.Z + v.Radius + 3)) ?? new List<Int2>();
                t.PathIndex = 0;
                t.Phase = "departing";
            }
            if (t.Path.Count == 0 || t.PathIndex < t.Path.Count && !Walkable(t.Path[t.PathIndex]))
            {
                t.BlockedReason = "blocked_route";
                if ((long)TimeSeconds % 10 == 0)
                {
                    t.Path = Path(t.Position, t.Phase == "arriving" ? t.VisitDestination ?? v.Center : t.Exterior ?? new Int2(v.Center.X, v.Center.Z + v.Radius + 3)) ?? new List<Int2>();
                    t.PathIndex = 0;
                }
                return;
            }
            t.Progress += 0.8;
            if (t.Progress < 1)
                return;
            t.Progress -= 1;
            if (t.PathIndex < t.Path.Count)
                t.Position = t.Path[t.PathIndex++];
            if (t.PathIndex < t.Path.Count)
                return;
            if (t.Phase == "arriving")
            {
                t.Phase = "trading";
                t.Until = TimeSeconds + 900;
                t.BlockedReason = "";
                Note(v, "trader", "Finite merchant wagon reached the shrine");
            }
            else
            {
                t.Phase = "absent";
                t.LastDepartedAt = TimeSeconds;
                t.NextAt = TimeSeconds + 12 * 3600;
                t.Items.Clear();
            }
        }
        private bool FreeInfrastructureFootprint(Village v, string kind, Int2 p) => !v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, p)) && (kind != "road" || RoadSpace(v, p));
        private ActionResult PlanLinear(Village v, GameAction a, string kind, Cat cat)
        {
            if (kind == "rail" && !Catalog.Owns(v, "unlock_capability", "rail_logistics"))
                return ActionResult.Fail("Rail research required");
            var path = new List<Int2>();
            var p = a.Position;
            path.Add(p);
            while (!p.Equals(a.End) && path.Count < 129)
            {
                if (p.X != a.End.X)
                    p.X += Math.Sign(a.End.X - p.X);
                else
                    p.Z += Math.Sign(a.End.Z - p.Z);
                path.Add(p);
            }
            if (path.Count > 128 || path.Any(at => !v.Known.Contains(at) || !Walkable(at) || !FreeInfrastructureFootprint(v, kind, at) || PendingExpansionWall(v, at) || RoadSurface(TileAt(at)) || TileAt(at).Rail || v.Jobs.Any(j => !j.Completed && (j.Kind == "road" || j.Kind == "rail") && j.Path.Contains(at))))
                return ActionResult.Fail("Route must be mapped, free, dry, and unclaimed");
            if (path.Any(at => !CanModifyTerrain(v.Id, at)))
                return ActionResult.Fail("Foreign territory cannot be modified");
            var connected = kind == "road" ? ConnectedRoads(v) : null;
            if (kind == "road" && !path.Any(at => Neighbors(at).Any(connected.Contains)))
                return ActionResult.Fail("Road must join shrine road network");
            var result = CreateInputJob(v, cat, kind, path[0], kind == "rail" ? "metal" : "materials", path.Count, 30 * path.Count, "");
            if (result.Success)
                v.Jobs.Find(j => j.Id == result.EntityId).Path = path;
            return result;
        }
        private ActionResult CreateInfrastructure(Village v, GameAction a, Cat c, string kind, string resource, double amount)
        {
            if (!v.Known.Contains(a.Position))
                return ActionResult.Fail("Mapped infrastructure site required");
            if (!CanModifyTerrain(v.Id, a.Position))
                return ActionResult.Fail("Foreign territory cannot be modified");
            if (kind == "bridge" && (!TileAt(a.Position).Water || !Neighbors(a.Position).Any(Walkable)))
                return ActionResult.Fail("Bridge needs a narrow reachable crossing");
            if (kind == "dock" && (!Walkable(a.Position) || !Neighbors(a.Position).Any(p => TileAt(p).Water)))
                return ActionResult.Fail("Dock needs a dry shoreline");
            if (kind == "wagon" && (!Catalog.Owns(v, "unlock_capability", "rail_logistics") || !TileAt(a.Position).Rail))
                return ActionResult.Fail("Rail and completed track required");
            if ((kind == "dock" || kind == "vessel") && !Catalog.Owns(v, "unlock_capability", "water_travel"))
                return ActionResult.Fail("Shipping research required");
            if (kind == "vessel" && !TileAt(a.Position).Dock)
                return ActionResult.Fail("Completed dock required");
            var position = kind == "bridge" ? Neighbors(a.Position).First(Walkable) : a.Position;
            var result = CreateInputJob(v, c, kind, position, resource, amount, 120, "");
            if (result.Success)
                v.Jobs.Find(j => j.Id == result.EntityId).Path = new List<Int2> { a.Position };
            return result;
        }
        private void CompleteInfrastructure(Village v, Job j)
        {
            if (j.Kind == "wagon" || j.Kind == "vessel")
            {
                v.Vehicles.Add(new Vehicle { Id = Id(j.Kind), Mode = j.Kind == "vessel" ? "shipping" : "rail", Position = j.Position });
                return;
            }
            foreach (var p in j.Path)
            {
                var t = TileAt(p);
                if (j.Kind == "road")
                {
                    t.Road = true;
                    t.Dirt = false;
                }
                else if (j.Kind == "rail")
                    t.Rail = true;
                else if (j.Kind == "dock")
                    t.Dock = true;
                else if (j.Kind == "bridge")
                    t.Bridge = true;
            }
        }
        private ActionResult CreateRoute(Village v, GameAction a, Cat c)
        {
            var source = v.Stockpiles.Find(s => s.Id == a.TargetId);
            var destination = v.Stockpiles.Find(s => s.Id == a.BuildingId);
            if (source == null || destination == null || source.Id == destination.Id || c == null || c.JobId != "" || c.BuildingId != "" || c.ControlledBy != "")
                return ActionResult.Fail("Distinct piles and available living driver required");
            if (a.Amount <= 0 || a.Amount > 40 || !Catalog.Resources.Contains(a.Resource) || a.Resource == "blessings" || EquipmentKind(a.Resource) != "" && a.Amount != (int)a.Amount || a.Path.Count < 2 || a.Path.Count > 1024)
                return ActionResult.Fail("Invalid cargo or route");
            if (!a.Path[0].Equals(source.Position) || !a.Path[a.Path.Count - 1].Equals(destination.Position))
                return ActionResult.Fail("Route must connect exact pile locations");
            if (a.Mode != "rail" && a.Mode != "shipping")
                return ActionResult.Fail("Unknown transport mode");
            if (a.Path.Where((p, i) => i > 0 && Int2.Distance(p, a.Path[i - 1]) != 1).Any())
                return ActionResult.Fail("Route waypoints must be adjacent");
            if (a.Mode == "rail" && a.Path.Any(p => !TileAt(p).Rail))
                return ActionResult.Fail("Continuous completed track required");
            if (a.Mode == "shipping" && a.Path.Skip(1).Take(a.Path.Count - 2).Any(p => !TileAt(p).Water))
                return ActionResult.Fail("Vessel requires water waypoints");
            var vehicle = v.Vehicles.Find(vehicle => vehicle.Mode == a.Mode && vehicle.RouteId == "" && vehicle.Position.Equals(source.Position));
            if (vehicle == null)
                return ActionResult.Fail("Free physical vehicle must be at the route source");
            var route = new TransportRoute { Id = Id("route"), Mode = a.Mode, CatId = c.Id, VehicleId = vehicle.Id, SourceId = source.Id, DestinationId = destination.Id, Resource = a.Resource, Amount = a.Amount, Path = new List<Int2>(a.Path), Repeat = a.Repeat };
            v.Routes.Add(route);
            vehicle.RouteId = route.Id;
            c.BuildingId = route.Id;
            return ActionResult.Ok(route.Id);
        }
        private ActionResult CancelRoute(Village v, string id)
        {
            var route = v.Routes.Find(r => r.Id == id);
            if (route == null)
                return ActionResult.Fail("Unknown route");
            var vehicle = v.Vehicles.Find(x => x.Id == route.VehicleId);
            bool accessibleWreck = vehicle != null && !v.Cats.Any(c => c.Id == route.CatId && c.Alive) && Walkable(v, vehicle.Position);
            if (route.Mode == "shipping" && vehicle != null && !accessibleWreck && route.Path.Count > 0 &&
                (!vehicle.Position.Equals(route.Path[0]) || vehicle.Cargo.Count > 0 || vehicle.ItemIds.Count > 0))
            {
                route.CancelRequested = true;
                route.Repeat = false;
                route.Phase = "returning_to_dock";
                int index = route.Path.FindIndex(p => p.Equals(vehicle.Position));
                if (index >= 0)
                    route.PathIndex = index;
                return ActionResult.Ok(route.Id);
            }
            if (vehicle != null)
            {
                var spill = Spill(v, vehicle.Position, vehicle.Cargo);
                if (vehicle.ItemIds.Count > 0)
                {
                    if (spill == null)
                    {
                        spill = new Stockpile { Id = Id("spill"), Position = vehicle.Position, Kind = "spill", Width = 1, Depth = 1, Capacity = 10000 };
                        v.Stockpiles.Add(spill);
                    }
                    foreach (var item in v.Items.Where(i => vehicle.ItemIds.Contains(i.Id)))
                        item.LocationId = spill.Id;
                    vehicle.ItemIds.Clear();
                }
                vehicle.RouteId = "";
            }
            var cat = v.Cats.Find(c => c.Id == route.CatId);
            if (cat != null)
            {
                cat.BuildingId = "";
                cat.Path.Clear();
            }
            v.Routes.Remove(route);
            return ActionResult.Ok();
        }
        private void ReturnVessel(Village v, TransportRoute route, Vehicle vehicle, Cat driver)
        {
            if (driver == null)
            {
                if (Walkable(v, vehicle.Position))
                    CancelRoute(v, route.Id);
                else
                    route.BlockedReason = "driver_required · build bridge access to salvage wreck";
                return;
            }
            driver.Goal = driver.ControlledBy != "" ? "player_control · returning to dock" : "returning vessel to dock";
            if (route.PathIndex > 0)
            {
                int next = route.PathIndex - 1;
                var tile = TileAt(route.Path[next]);
                if (next > 0 && !tile.Water && !tile.Dock)
                {
                    route.BlockedReason = "return_route_blocked";
                    return;
                }
                route.PathIndex = next;
                vehicle.Position = route.Path[next];
                driver.Position = vehicle.Position;
                driver.X = vehicle.Position.X;
                driver.Z = vehicle.Position.Z;
                return;
            }
            var source = v.Stockpiles.Find(p => p.Id == route.SourceId);
            if (source == null)
            {
                source = new Stockpile { Id = Id("shore_return"), Position = vehicle.Position, Kind = "spill", Width = 1, Depth = 1, Capacity = 1000000 };
                v.Stockpiles.Add(source);
                route.SourceId = source.Id;
            }
            double load = vehicle.Cargo.Sum(s => s.Amount) + vehicle.ItemIds.Count;
            if (!HasRoom(v, source, route.Resource, load))
            {
                route.BlockedReason = "return_storage_full";
                return;
            }
            foreach (var stack in vehicle.Cargo)
                Add(source.Goods, stack.Resource, stack.Amount);
            vehicle.Cargo.Clear();
            foreach (var item in v.Items.Where(i => vehicle.ItemIds.Contains(i.Id)))
                item.LocationId = source.Id;
            vehicle.ItemIds.Clear();
            vehicle.RouteId = "";
            if (driver.BuildingId == route.Id)
                driver.BuildingId = "";
            driver.Path.Clear();
            driver.Goal = driver.ControlledBy != "" ? "player_control" : "idle";
            v.Routes.Remove(route);
        }
        private void TickTransport(Village v)
        {
            foreach (var route in v.Routes.ToArray())
            {
                var c = v.Cats.Find(c => c.Id == route.CatId && c.Alive);
                var vehicle = v.Vehicles.Find(x => x.Id == route.VehicleId);
                if (route.CancelRequested && vehicle != null)
                {
                    ReturnVessel(v, route, vehicle, c);
                    continue;
                }
                if (c == null || vehicle == null)
                {
                    CancelRoute(v, route.Id);
                    continue;
                }
                if (c.ControlledBy != "" || c.Goal.StartsWith("need_", StringComparison.Ordinal))
                    continue;
                var source = v.Stockpiles.Find(s => s.Id == route.SourceId);
                var destination = v.Stockpiles.Find(s => s.Id == route.DestinationId);
                if (source == null || destination == null)
                {
                    CancelRoute(v, route.Id);
                    continue;
                }
                c.Goal = route.Mode + " · " + route.Phase;
                if (route.Phase == "boarding")
                {
                    if (Move(c, source.Position, 1))
                    {
                        vehicle.Position = c.Position;
                        route.Phase = "loading";
                    }
                    continue;
                }
                if (route.Phase == "loading")
                {
                    string kind = EquipmentKind(route.Resource);
                    if (kind != "")
                    {
                        var items = v.Items.Where(i => i.LocationId == source.Id && i.Kind == kind).OrderBy(i => i.Id, StringComparer.Ordinal).Take((int)route.Amount).ToArray();
                        if (items.Length < route.Amount)
                        {
                            route.BlockedReason = "missing_input";
                            continue;
                        }
                        foreach (var item in items)
                        {
                            item.LocationId = vehicle.Id;
                            vehicle.ItemIds.Add(item.Id);
                        }
                    }
                    else
                    {
                        double free = Amount(source.Goods, route.Resource) - Reservations.Where(r => r.PileId == source.Id && r.Resource == route.Resource).Sum(r => r.Amount);
                        if (free < route.Amount)
                        {
                            route.BlockedReason = "missing_input";
                            continue;
                        }
                        Add(source.Goods, route.Resource, -route.Amount);
                        Add(vehicle.Cargo, route.Resource, route.Amount);
                    }
                    route.Phase = "outbound";
                    route.PathIndex = 0;
                    continue;
                }
                if (route.Phase == "unloading")
                {
                    if (!HasRoom(v, destination, route.Resource, vehicle.Cargo.Sum(s => s.Amount) + vehicle.ItemIds.Count))
                    {
                        route.BlockedReason = "storage_full";
                        continue;
                    }
                    foreach (var stack in vehicle.Cargo)
                        Add(destination.Goods, stack.Resource, stack.Amount);
                    vehicle.Cargo.Clear();
                    foreach (var item in v.Items.Where(i => vehicle.ItemIds.Contains(i.Id)))
                        item.LocationId = destination.Id;
                    vehicle.ItemIds.Clear();
                    route.Phase = "returning";
                    continue;
                }
                int next = route.PathIndex + (route.Phase == "returning" ? -1 : 1);
                if (next < 0)
                {
                    if (route.Repeat)
                    {
                        route.Phase = "loading";
                        route.PathIndex = 0;
                    }
                    else
                        CancelRoute(v, route.Id);
                    continue;
                }
                if (next >= route.Path.Count)
                {
                    route.Phase = "unloading";
                    continue;
                }
                var tile = TileAt(route.Path[next]);
                if (route.Mode == "rail" ? !tile.Rail || !Walkable(v, route.Path[next]) || !Crossable(vehicle.Position, route.Path[next]) : (!tile.Water && !tile.Dock && next != 0 && next != route.Path.Count - 1))
                {
                    route.BlockedReason = "route_blocked";
                    continue;
                }
                route.PathIndex = next;
                vehicle.Position = route.Path[next];
                c.Position = vehicle.Position;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                route.BlockedReason = "";
                Add(c.Skills, "haul", 1.0 / 3600);
            }
        }
    }
}
