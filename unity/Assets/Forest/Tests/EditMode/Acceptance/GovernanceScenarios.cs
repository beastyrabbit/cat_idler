using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Territory between villages: foreign claims reject or recheck building, expansion and designation.</summary>
    public static class GovernanceScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (var infrastructure in new[] { "road", "rail", "bridge", "dock", "wagon", "vessel" })
            {
                string kind = infrastructure;
                yield return new Scenario("regression.territory_rejects_foreign_" + kind, () => ForeignInfrastructure(kind, false));
                yield return new Scenario("regression.territory_rejects_stale_building_" + kind, () => ForeignInfrastructure(kind, true));
                yield return new Scenario("regression.territory_rechecks_pending_" + kind, () => PendingForeignInfrastructure(kind));
            }
            yield return new Scenario("regression.territory_expansion_rejects_foreign_ring", () => ForeignExpansion(false, false));
            yield return new Scenario("regression.territory_expansion_rejects_foreign_interior", () => ForeignExpansion(true, false));
            yield return new Scenario("regression.territory_expansion_rejects_stale_building", () => ForeignExpansion(true, true));
            yield return new Scenario("regression.territory_expansion_rechecks_claims", PendingForeignExpansion);
            yield return new Scenario("regression.territory_legacy_tile_rechecks_claims", () => LegacyForeignExpansion(false));
            yield return new Scenario("regression.territory_legacy_tile_rechecks_stale_building", () => LegacyForeignExpansion(true));
            yield return new Scenario("regression.territory_founding_skips_foreign_deposit_site", ForeignFounding);
            yield return new Scenario("regression.territory_recovery_preserves_foreign_land_and_pending_state", ForeignRecovery);
            yield return new Scenario("regression.territory_zone_rejects_foreign_footprint", () => ForeignDesignation(false));
            yield return new Scenario("regression.territory_fishing_rejects_foreign_claim", () => ForeignDesignation(true));
            yield return new Scenario("regression.territory_scaffold_rejects_foreign_footprint", () => ForeignScaffold(false));
            yield return new Scenario("regression.territory_scaffold_rechecks_claims", () => ForeignScaffold(true));
        }
        static World TerritoryFixture(out Village v, out Cat c, out Village foreign, out PlayerContext actor)
        {
            var w = Fixture(out v, out c);
            c.BuildingId = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            foreach (var resource in new[] { "materials", "metal", "lumber", "planks", "blocks" })
                World.Add(v.Stockpiles[0].Goods, resource, 1000);
            var owner = new PlayerContext { PlayerId = "terrain-owner-b" };
            var founded = w.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Private terrain fixture" });
            Check(founded.Success, "Second identity could not found its private village");
            foreign = w.Village(founded.EntityId);
            foreach (var cat in foreign.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
            }
            foreign.LastLeaderResearch = w.TimeSeconds;
            actor = new PlayerContext { PlayerId = "terrain-owner-a", VillageId = v.Id };
            Check(w.CanControl(owner, foreign) && !w.CanControl(actor, foreign), "Fixture identities do not establish private-village ownership");
            return w;
        }
        static GameAction InfrastructureAction(string kind, string catId, Int2 at) => kind == "road" || kind == "rail" ? new GameAction { Kind = kind == "road" ? "BuildRoad" : "DesignateRail", CatId = catId, Position = at, End = at } : new GameAction { Kind = kind == "bridge" ? "BuildBridge" : kind == "dock" ? "BuildDock" : "BuildTransportVehicle", CatId = catId, Position = at, Mode = kind == "vessel" ? "shipping" : "rail" };
        static void InfrastructureSite(World w, Village v, string kind, Int2 at)
        {
            foreach (var p in new[] { at, new Int2(at.X - 1, at.Z), new Int2(at.X + 1, at.Z), new Int2(at.X, at.Z - 1), new Int2(at.X, at.Z + 1) })
            {
                var t = w.TileAt(p);
                t.Wall = t.Water = t.Mountain = t.Road = t.Rail = t.Dock = t.Bridge = false;
                if (!v.Known.Contains(p))
                    v.Known.Add(p);
            }
            if (kind == "road")
                w.TileAt(new Int2(at.X - 1, at.Z)).Road = true;
            if (kind == "bridge")
                w.TileAt(at).Water = true;
            if (kind == "dock")
                w.TileAt(new Int2(at.X + 1, at.Z)).Water = true;
            if (kind == "wagon")
                w.TileAt(at).Rail = true;
            if (kind == "vessel")
                w.TileAt(at).Dock = true;
        }
        static void ForeignInfrastructure(string kind, bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(1, 2);
            InfrastructureSite(w, v, kind, at);
            w.TileAt(at).ClaimId = staleBuilding ? v.Id : foreign.Id;
            if (staleBuilding)
                Station(w, foreign, "den", at);
            string terrain = TerrainState(w.TileAt(at));
            int jobs = v.Jobs.Count, reservations = w.Reservations.Count;
            var result = w.Apply(actor, InfrastructureAction(kind, c.Id, at));
            Check(!result.Success, "Public " + kind + " accepted another village's " + (staleBuilding ? "building footprint with stale ClaimId" : "claimed terrain"));
            Check(TerrainState(w.TileAt(at)) == terrain && v.Jobs.Count == jobs && w.Reservations.Count == reservations, "Rejected foreign construction changed terrain or reserved inputs");
            if (!staleBuilding)
            {
                w.TileAt(at).ClaimId = "";
                foreign.Stockpiles.Add(new Stockpile { Id = w.Id("foreign-zone-hint"), Kind = "zone_gather", Position = at, Width = 1, Depth = 1 });
                result = w.Apply(actor, InfrastructureAction(kind, c.Id, at));
                Check(result.Success, "Unclaimed exterior " + kind + " became unavailable: " + result.Error);
                var job = v.Jobs.Single(j => j.Id == result.EntityId);
                for (int tick = 0; tick < 500 && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Allowed unclaimed " + kind + " did not complete physically");
                Valid(w);
            }
        }
        static void PendingForeignInfrastructure(string kind)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(1, 2);
            InfrastructureSite(w, v, kind, at);
            var result = w.Apply(actor, InfrastructureAction(kind, c.Id, at));
            Check(result.Success, "Could not plan own terrain before ownership change");
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            for (int tick = 0; tick < 200 && job.Phase != "working"; tick++)
                w.Step(1);
            Check(job.Phase == "working" && !job.Completed, "Fixture must change ownership during funded construction");
            w.TileAt(at).ClaimId = foreign.Id;
            string terrain = TerrainState(w.TileAt(at));
            double progress = job.Progress;
            var quantities = new[] { "materials", "metal", "lumber" }.Select(r => Goods(w, v, r)).ToArray();
            w.Step(World.SimulationStepSeconds);
            Check(job.BlockedReason == "foreign_territory" && job.Progress == progress, "Foreign ownership did not stop construction in the first actor quantum");
            w.Step(180 - World.SimulationStepSeconds);
            Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.Progress == progress, "Pending " + kind + " advanced after its site became foreign");
            Check(TerrainState(w.TileAt(at)) == terrain && v.Vehicles.Count == 0, "Pending " + kind + " changed foreign terrain or created a vehicle there");
            for (int i = 0; i < quantities.Length; i++)
                Near(Goods(w, v, new[] { "materials", "metal", "lumber" }[i]), quantities[i], "Blocked foreign construction consumed its retained inputs");
            Valid(w);
            w.TileAt(at).ClaimId = v.Id;
            for (int tick = 0; tick < 500 && !job.Completed; tick++)
                w.Step(1);
            Check(job.Completed && job.BlockedReason != "foreign_territory", "Construction could not resume when its own site became available");
            Valid(w);
        }
        static void ForeignExpansion(bool interior, bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            PrepareExpansion(w, v);
            var at = new Int2(v.Radius + (interior ? 1 : 2), 1);
            w.TileAt(at).ClaimId = staleBuilding ? "" : foreign.Id;
            if (staleBuilding)
                Station(w, foreign, "den", at);
            string terrain = TerrainState(w.TileAt(at));
            int radius = v.Radius;
            var result = w.Apply(actor, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            Check(!result.Success, "Expansion accepted a foreign " + (staleBuilding ? "building" : interior ? "interior claim" : "ring claim"));
            Check(v.Radius == radius && TerrainState(w.TileAt(at)) == terrain && v.Jobs.Count == 0 && w.Reservations.Count == 0, "Rejected expansion changed ownership or reserved inputs");
            Valid(w);
        }
        static void PendingForeignExpansion()
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            PrepareExpansion(w, v);
            var result = w.Apply(actor, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id });
            Check(result.Success, "Could not plan clear expansion");
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            for (int tick = 0; tick < 1800 && job.PathIndex == 0; tick++)
                w.Step(1);
            Check(job.PathIndex > 0, "Expansion never paid for a physical perimeter segment");
            var at = new Int2(v.Radius + 1, 1);
            w.TileAt(at).ClaimId = foreign.Id;
            string terrain = TerrainState(w.TileAt(at));
            int radius = v.Radius, completed = job.PathIndex;
            double goods = Goods(w, v, "materials");
            w.Step(180);
            Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.PathIndex == completed && v.Radius == radius, "Pending expansion advanced across a newly foreign interior claim");
            Check(TerrainState(w.TileAt(at)) == terrain && job.Path.Take(completed).All(p => w.TileAt(p).Wall), "Blocked expansion overwrote foreign ownership or lost paid segments");
            Near(Goods(w, v, "materials"), goods, "Blocked expansion consumed retained materials");
            Valid(w);
        }
        static void LegacyForeignExpansion(bool staleBuilding)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(1, 2);
            w.TileAt(at).ClaimId = staleBuilding ? v.Id : foreign.Id;
            if (staleBuilding)
                Station(w, foreign, "den", at);
            var job = new Job { Id = w.Id("imported-expand"), Kind = "expand", OriginalKind = "expand_village", Phase = "working", CatId = c.Id, Position = at, Progress = 9, RequiredWork = 10 };
            v.Jobs.Add(job);
            c.JobId = job.Id;
            string terrain = TerrainState(w.TileAt(at));
            int radius = v.Radius, claimed = v.ClaimedTiles.Count;
            w.Step(5);
            Check(!job.Completed && job.BlockedReason == "foreign_territory" && job.Progress == 9, "Imported single-tile expansion advanced on foreign property");
            Check(TerrainState(w.TileAt(at)) == terrain && v.Radius == radius && v.ClaimedTiles.Count == claimed, "Imported single-tile expansion rewrote foreign terrain or boundaries");
            Valid(w);
        }
        static void ForeignFounding()
        {
            var w = World.Create(41);
            var existing = w.Villages[0];
            var owner = new PlayerContext { PlayerId = "terrain-founding-owner" };
            uint hash = World.Hash(owner.PlayerId);
            var first = new Int2(80 + (int)(hash % 8) * 80, 80 + (int)((hash / 8) % 8) * 80);
            var deposit = new Int2(first.X - 12, first.Z - 4);
            var tile = w.TileAt(deposit);
            tile.ClaimId = existing.Id;
            tile.Resource = "gem";
            tile.Amount = 17;
            string before = TerrainState(tile);
            var result = w.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Protected founding" });
            Check(result.Success, "Could not choose a safe alternative founding site");
            Check(w.Village(result.EntityId).Center.Equals(new Int2(first.X + 80, first.Z)), "Founding did not skip the deterministic candidate whose starter deposit crosses foreign land");
            Check(TerrainState(tile) == before && w.GetTile(first) == null, "Founding overwrote foreign land or generated tiles while checking a rejected candidate");
            Valid(w);
        }
        static void ForeignRecovery()
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(v.Center.X + 2, v.Center.Z + v.Radius + 2);
            w.TileAt(at).ClaimId = foreign.Id;
            foreach (var cat in v.Cats)
                cat.Alive = false;
            var scaffold = new Building { Id = w.Id("pending-scaffold"), Kind = "wood_cutter", Position = new Int2(-5, 1) };
            v.Buildings.Add(scaffold);
            w.Reservations.Add(new Reservation { OwnerId = scaffold.Id, VillageId = v.Id, PileId = v.Stockpiles[0].Id, Resource = "materials", Amount = 1 });
            var trade = new TradeOffer { Id = w.Id("pending-trade"), FromVillageId = v.Id, ToVillageId = foreign.Id, Status = "offered" };
            w.TradeOffers.Add(trade);
            string terrain = TerrainState(w.TileAt(at));
            int run = v.Run;
            w.Step(120);
            Check(ReferenceEquals(w.Village(v.Id), v) && v.Run == run && !v.Cats.Any(cat => cat.Alive), "Conflicting extinction recovery silently replaced the colony");
            Check(TerrainState(w.TileAt(at)) == terrain && w.Reservations.Any(r => r.OwnerId == scaffold.Id) && trade.Status == "offered", "Blocked recovery changed foreign terrain or destroyed pending ownership");
            Check(v.Events.Count(e => e.Kind == "recovery_blocked") == 1, "Blocked recovery must report the conflict once rather than repeat every check");
            Valid(w);
        }
        static void ForeignDesignation(bool fishing)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(1, 2);
            InfrastructureSite(w, v, "dock", at);
            if (fishing)
                w.TileAt(at).ClaimId = foreign.Id;
            else
                Station(w, foreign, "den", at);
            int piles = v.Stockpiles.Count;
            var result = w.Apply(actor, new GameAction { Kind = fishing ? "DesignateFishingSpot" : "CreateZone", Resource = "gather", Position = at, End = at });
            Check(!result.Success && v.Stockpiles.Count == piles, "Placed work site bypassed foreign terrain ownership");
            Valid(w);
        }
        static void ForeignScaffold(bool pending)
        {
            var w = TerritoryFixture(out var v, out var c, out var foreign, out var actor);
            var at = new Int2(-5, 1);
            if (!pending)
                Station(w, foreign, "den", at);
            var result = w.Apply(actor, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", CatId = c.Id, Position = at });
            if (!pending)
            {
                Check(!result.Success && w.Reservations.Count == 0, "New scaffold occupied a foreign building footprint under stale ClaimId");
                return;
            }
            Check(result.Success, "Could not plan an own-territory scaffold");
            var scaffold = v.Buildings.Single(b => b.Id == result.EntityId);
            var job = v.Jobs.Single(j => j.TargetId == scaffold.Id);
            w.TileAt(at).ClaimId = foreign.Id;
            double timber = Goods(w, v, "lumber"), blocks = Goods(w, v, "blocks");
            int claims = w.Reservations.Count;
            w.Step(180);
            Check(!scaffold.Completed && scaffold.Progress == 0 && scaffold.Inputs.Count == 0 && job.BlockedReason == "foreign_territory", "Scaffold consumed inputs or advanced on newly foreign terrain");
            Check(w.Reservations.Count == claims, "Blocked scaffold discarded its finite material claims");
            Near(Goods(w, v, "lumber"), timber, "Blocked scaffold consumed timber");
            Near(Goods(w, v, "blocks"), blocks, "Blocked scaffold consumed blocks");
            Valid(w);
        }
    }
}
