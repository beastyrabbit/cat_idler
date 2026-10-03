using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Continuous stepping: movement and work budgets, tick partitions, failed paths and fractional rates.</summary>
    public static class CoreScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (string mode in new[] { "cadence", "recovery", "destination", "start", "partition" })
            {
                string check = mode;
                yield return new Scenario("regression.continuous_failed_path_" + check, () => ContinuousFailedPath(check));
            }
            yield return new Scenario("regression.continuous_npc_movement_and_needs", ContinuousNpc);
            yield return new Scenario("regression.continuous_stationary_work_rate", ContinuousWork);
            yield return new Scenario("regression.continuous_shared_movement_work_budget", ContinuousBudget);
            yield return new Scenario("regression.continuous_direct_turn_retraces_edge", ContinuousDirectTurn);
            foreach (string ground in new[] { "grass", "road", "forest" })
            {
                string terrain = ground;
                yield return new Scenario("regression.continuous_segment_rate_turn_" + terrain, () => ContinuousTurnRate(terrain));
            }
            yield return new Scenario("regression.continuous_segment_rate_remaining_budget", ContinuousTurnRemainingBudget);
            yield return new Scenario("regression.continuous_segment_rate_fractional_reboard", ContinuousReboardRate);
            yield return new Scenario("regression.continuous_missing_supply_preserves_work_route", ContinuousMissingSupply);
            yield return new Scenario("regression.continuous_fractional_partition", ContinuousPartition);
            foreach (string activity in new[] { "research", "farm", "sleep" })
            {
                string kind = activity;
                yield return new Scenario("regression.continuous_" + kind + "_rate", () => ContinuousRate(kind));
            }
            yield return new Scenario("regression.continuous_combat_shares_cat_budget", ContinuousCombatBudget);
            yield return new Scenario("regression.continuous_rail_cancel_mid_edge", () => ContinuousTransportInterruption(false));
            yield return new Scenario("regression.continuous_shipping_needs_and_cancel_mid_edge", () => ContinuousTransportInterruption(true));
            yield return new Scenario("regression.continuous_legacy_caravan_positive", () => ContinuousLegacyCaravan(1));
            yield return new Scenario("regression.continuous_legacy_caravan_negative", () => ContinuousLegacyCaravan(-1));
            foreach (string mover in new[] { "merchant", "raid", "caravan", "rail" })
            {
                string kind = mover;
                yield return new Scenario("regression.continuous_" + kind, () => ContinuousMover(kind));
            }
            // W2-08: event metadata, fault isolation, validation split and non-inserting route searches.
            yield return new Scenario("core.event_metadata_from_entities", EventMetadata);
            yield return new Scenario("core.routine_events_never_evict_notable", RoutineEventsNeverEvict);
            yield return new Scenario("core.legacy_log_evicts_routine_first", LegacyLogEvictsRoutineFirst);
            yield return new Scenario("core.milestone_helpers", MilestoneHelpers);
            yield return new Scenario("core.fault_in_village_cancels_job_and_keeps_stepping", FaultInVillage);
            yield return new Scenario("core.fault_without_actor_logs_and_keeps_stepping", FaultWithoutActor);
            yield return new Scenario("core.validate_split_and_repair", ValidateSplitAndRepair);
            yield return new Scenario("core.path_search_inserts_no_tiles", PathSearchInsertsNoTiles);
            yield return new Scenario("core.path_search_matches_inserted_terrain", PathSearchMatchesInsertedTerrain);
            yield return new Scenario("core.path_search_sees_new_claims", PathSearchSeesNewClaims);
        }

        // ---- W2-08 ------------------------------------------------------------------------------

        /// <summary>Calls World's private Note(village, kind, text, entityId) the way the simulation does.</summary>
        static void NoteVia(World w, Village v, string kind, string text, string id = "")
        {
            var note = typeof(World).GetMethods(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).Single(m => m.Name == "Note" && m.GetParameters().Length == 4);
            note.Invoke(w, new object[] { v, kind, text, id });
        }
        static void EventMetadata()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            job.Position = new Int2(4, 2);
            var den = v.Buildings.First(b => b.Kind == "den");
            NoteVia(w, v, EventKind.Death, c.Name + " died from needs", c.Id);
            var death = v.Events.Last();
            Check(death.Kind == EventKind.Death && death.Severity == EventSeverity.Alert && death.Seq > 0, "A death was not an alert with a sequence number");
            Check(death.Position.HasValue && death.Position.Value.Equals(c.Position) && death.ActorName == c.Name, "A cat event did not carry the cat's position and name");
            NoteVia(w, v, EventKind.Completed, c.Name + " completed hunt", job.Id);
            Check(v.Events.Last() == death, "A job completion entered the notable log");
            var completed = v.RoutineEvents.Last();
            Check(completed.Severity == EventSeverity.Info && completed.ActorName == c.Name && completed.Position.Value.Equals(job.Position), "A completion did not carry the worker and the job position");
            Check(completed.Args != null && completed.Args["job"] == JobKind.Hunt, "A completion did not name its job kind");
            NoteVia(w, v, EventKind.Upgrade, "den reached level 2", den.Id);
            var upgrade = v.Events.Last();
            Check(upgrade.Severity == EventSeverity.Notable && upgrade.Position.Value.Equals(den.Position) && upgrade.ActorName == "", "A building event did not carry the building position");
            NoteVia(w, v, EventKind.Expansion, "Settlement grew", "");
            Check(!v.Events.Last().Position.HasValue && v.Events.Last().Args == null, "An event without an entity invented a position or arguments");
            var since = w.EventsSince(v, death.Seq);
            Check(since.Select(e => e.Kind).SequenceEqual(new[] { EventKind.Completed, EventKind.Upgrade, EventKind.Expansion }), "EventsSince did not merge both logs in sequence order: " + string.Join(",", since.Select(e => e.Kind)));
            Check(w.EventsSince(v, 0).Count == v.Events.Count + v.RoutineEvents.Count, "EventsSince(0) did not return the whole log");
            var legacy = new Event { Kind = EventKind.Raid, Text = "old raid" };
            Check(legacy.EffectiveSeverity() == EventSeverity.Alert && new Event { Kind = EventKind.Completed }.EffectiveSeverity() == EventSeverity.Info, "Events saved without a severity did not fall back to their kind's");
            Valid(w);
        }
        static void RoutineEventsNeverEvict()
        {
            var w = Fixture(out var v, out var c);
            NoteVia(w, v, "raid", "Raiders approach the south gate");
            for (int i = 0; i < 150; i++)
                NoteVia(w, v, "completed", c.Name + " completed hunt " + i);
            Check(v.Events.Count(e => e.Kind == "raid") == 1, "Routine completions evicted the raid warning");
            Check(v.Events.All(e => e.Kind != "completed"), "Routine completions entered the notable log");
            Check(v.RoutineEvents.Count == World.RoutineEventCap && v.RoutineEvents.Last().Text.EndsWith(" 149", StringComparison.Ordinal) && v.RoutineEvents.First().Text.EndsWith(" 50", StringComparison.Ordinal), "The routine ring did not keep the newest entries");
            for (int i = 0; i < 120; i++)
                NoteVia(w, v, "migration", "Wanderer " + i);
            Check(v.Events.Count == World.NotableEventCap && v.Events.Last().Text == "Wanderer 119", "The notable log lost its cap or its newest entry");
        }
        static void LegacyLogEvictsRoutineFirst()
        {
            var w = Fixture(out var v, out var c);
            // A log saved before the split: no severities, completions mixed with a raid.
            v.Events.Clear();
            v.Events.Add(new Event { Kind = EventKind.Raid, Text = "old raid", Seq = 1 });
            for (int i = 0; i < 99; i++)
                v.Events.Add(new Event { Kind = EventKind.Completed, Text = "old completion " + i, Seq = 2 + i });
            w.NextEventSeq = 101;
            NoteVia(w, v, EventKind.Migration, "A wanderer approaches");
            NoteVia(w, v, EventKind.Birth, "A kitten was born");
            Check(v.Events.Count == World.NotableEventCap && v.Events[0].Text == "old raid", "A legacy raid was evicted before legacy completions");
            Check(v.Events.Count(e => e.Kind == EventKind.Completed) == 97 && v.Events.First(e => e.Kind == EventKind.Completed).Text == "old completion 2", "Legacy completions were not evicted oldest first");
        }
        static void MilestoneHelpers()
        {
            var w = Fixture(out var v, out var c);
            var den = v.Buildings.First(b => b.Kind == "den");
            var pile = v.Stockpiles[0];
            var built = w.NoteBuildingCompleted(v, den);
            Check(built.Kind == EventKind.BuildingCompleted && built.Severity == EventSeverity.Notable && built.Args["building"] == "den" && built.Position.Value.Equals(den.Position) && v.Events.Last() == built, "Building completion milestone is wrong");
            var found = w.NoteResourceDiscovered(v, c, "stone", new Int2(9, 9));
            Check(found.Kind == EventKind.Discovery && found.Severity == EventSeverity.Notable && v.Events.Last() == found && found.Position.Value.Equals(new Int2(9, 9)) && found.ActorName == c.Name && found.Args["resource"] == "stone", "A discovery milestone was not notable with its own position");
            var full = w.NoteStorageFull(v, pile, "food");
            Check(full != null && full.Kind == EventKind.StorageFull && full.Position.Value.Equals(pile.Position) && full.Args["resource"] == "food", "Storage-full milestone is wrong");
            Check(w.NoteStorageFull(v, pile, "food") == null && w.NoteStorageFull(v, pile, "water") != null, "Storage-full notices were not throttled per resource");
            var empty = w.NoteStorageEmpty(v, "water");
            Check(empty.Kind == EventKind.StorageEmpty && empty.Severity == EventSeverity.Alert && w.NoteStorageEmpty(v, "water") == null, "Storage-empty notice is not a throttled alert");
            w.Step(World.StorageNoticeSeconds);
            Check(w.NoteStorageEmpty(v, "water") != null, "Storage-empty notice never repeated after its throttle");
            var rank = w.NoteRankUp(v, c, "hunting", 2);
            Check(rank.Kind == EventKind.RankUp && rank.Args["skill"] == "hunting" && rank.Args["rank"] == "2" && rank.ActorName == c.Name, "Rank-up milestone is wrong");
            var raid = new Raid { Id = w.Id("helper-raid"), Position = new Int2(0, 5) };
            v.Raids.Add(raid);
            var repelled = w.NoteRaidRepelled(v, raid);
            Check(repelled.Kind == EventKind.RaidRepelled && repelled.Severity == EventSeverity.Notable && repelled.Position.Value.Equals(raid.Position), "Raid-repelled milestone is wrong");
            var loot = w.NoteExpeditionLoot(v, c, new[] { new Stack("food", 3), new Stack("bone", 1.5) });
            Check(loot.Kind == EventKind.ExpeditionLoot && loot.Args["food"] == "3" && loot.Args["bone"] == "1.5" && loot.Text.Contains("1.5 bone"), "Expedition loot milestone is wrong: " + loot.Text);
            Check(v.RoutineEvents.Count == 0, "A milestone entered the routine log");
        }
        /// <summary>A water job whose saved local cargo is corrupt: completing it throws inside the village tick.</summary>
        static Job PoisonJob(World w, Village v, Cat c)
        {
            var job = new Job { Id = w.Id("poison-work"), Kind = "water", Resource = "water", Amount = 5, Phase = "working", CatId = c.Id, Position = c.Position, RequiredWork = 0.01 };
            job.Local.Add(new Stack("water", -100));
            v.Jobs.Add(job);
            c.JobId = job.Id;
            c.BuildingId = "";
            c.ControlledBy = "";
            w.Reservations.Add(new Reservation { OwnerId = job.Id, VillageId = v.Id, PileId = v.Stockpiles[0].Id, Resource = "food", Amount = 3 });
            return job;
        }
        static void FaultInVillage()
        {
            var w = Fixture(out var home, out var c);
            var personal = w.Village(Act(w, home, new GameAction { Kind = "FoundVillage", Name = "Fault neighbour" }).EntityId);
            Check(w.Villages.IndexOf(personal) > w.Villages.IndexOf(home), "Fault fixture must tick the faulting village first");
            var job = PoisonJob(w, home, c);
            var other = home.Cats.First(cat => cat != c);
            var neighbour = personal.Cats.First(cat => cat.Alive);
            double neighbourHunger = neighbour.Hunger, start = w.TimeSeconds;
            w.Step(World.SimulationStepSeconds);
            Near(w.TimeSeconds - start, World.SimulationStepSeconds, "A village fault stopped the clock");
            Check(neighbour.Hunger < neighbourHunger, "A fault in one village stopped the next village's tick");
            Check(job.Completed && job.CatId == "" && c.JobId == "" && !w.Reservations.Any(r => r.OwnerId == job.Id), "The faulting job was not cancelled with its claims released");
            var error = home.Events.Last();
            Check(error.Kind == EventKind.Error && error.Severity == EventSeverity.Alert && error.EntityId == c.Id && error.Args["cancelled"] == "true" && error.Args["phase"] == "village", "The fault was not logged as an alert on the cat");
            Check(w.IsolatedFaults().Count == 1 && w.IsolatedFaults()[0].Contains("Negative water"), "The fault was not recorded for diagnostics");
            double otherHunger = other.Hunger;
            w.Step(5);
            Check(other.Hunger < otherHunger && w.IsolatedFaults().Count == 1, "The faulting village did not resume cleanly");
            Check(home.Events.Count(e => e.Kind == EventKind.Error) == 1, "A single fault logged more than one error");
            // Not Valid(w): it may assert that no fault was isolated. Its invariant checks still hold here.
            Check(w.Validate().Count == 0, string.Join("; ", w.Validate()));
            Check(w.Reservations.All(r => w.Villages.Any(x => x.Jobs.Any(j => !j.Completed && (j.Id == r.OwnerId || j.Id + ":resume" == r.OwnerId)) || x.Buildings.Any(b => !b.Completed && b.Id == r.OwnerId))), "Fault recovery left a claim without a live owner");
            Check(home.Cats.Concat(personal.Cats).All(cat => cat.Cargo.All(s => World.Finite(s.Amount) && s.Amount >= 0)), "Fault recovery kept corrupt cargo");
        }
        static void FaultWithoutActor()
        {
            var w = Fixture(out var v, out var c);
            // An unloading caravan with no requested-goods record throws in the trade phase, where no cat is acting.
            w.TradeOffers.Add(new TradeOffer { Id = w.Id("broken-caravan"), Status = "unloading", X = 3, Z = 2, FromVillageId = v.Id, ToVillageId = v.Id, Offered = new Stack("logs", 1), Requested = null });
            double hunger = v.Cats[1].Hunger;
            for (int i = 0; i < 3; i++)
                w.Step(1);
            Near(w.TimeSeconds, 3, "A trade fault stopped the clock");
            Check(v.Cats[1].Hunger < hunger, "A trade fault stopped the villages");
            Check(w.IsolatedFaults().Count > 0 && w.IsolatedFaults().All(f => f.Contains("trades")), "The trade fault was not isolated in its phase");
            var errors = v.Events.Where(e => e.Kind == EventKind.Error).ToList();
            Check(errors.Count == 1 && errors[0].EntityId == "" && errors[0].Args["phase"] == "trades", "A repeating fault without an actor was not logged once");
        }
        static void ValidateSplitAndRepair()
        {
            var w = Fixture(out var v, out var c);
            var pile = v.Stockpiles[0];
            var owner = new Job { Id = w.Id("claim-owner"), Kind = "haul", Phase = "fetch", Position = pile.Position };
            v.Jobs.Add(owner);
            w.Reservations.Add(new Reservation { OwnerId = owner.Id, VillageId = v.Id, PileId = pile.Id, Resource = "food", Amount = 600 });
            w.Reservations.Add(new Reservation { OwnerId = owner.Id, VillageId = v.Id, PileId = pile.Id, Resource = "food", Amount = 600 });
            var den = v.Buildings.First(b => b.Kind == "den" && b.Completed);
            int capacity = w.DenCapacity(v);
            var crowd = v.Cats.Where(cat => cat.Alive).Take(capacity + 2).ToList();
            foreach (var cat in crowd)
                cat.BedId = den.Id;
            var kept = new Job { Id = w.Id("kept-work"), Kind = "hunt", CatId = c.Id, Position = c.Position };
            var extra = new Job { Id = w.Id("extra-work"), Kind = "logs", CatId = c.Id, Position = c.Position };
            v.Jobs.Add(extra);
            v.Jobs.Add(kept);
            c.JobId = kept.Id;
            var repairable = w.ValidateRepairable();
            Check(repairable.Contains("Overclaimed " + pile.Id) && repairable.Contains("Overbooked bed " + den.Id) && repairable.Contains("Double work owner " + c.Id), "Repairable breaches were not reported: " + string.Join("; ", repairable));
            Check(w.ValidateFatal().Count == 0 && w.Validate().Count == repairable.Count, "Validate is not the union of fatal and repairable breaches");
            var repaired = w.RepairInvariants();
            Check(repaired.Count == 3 && w.ValidateRepairable().Count == 0 && w.Validate().Count == 0, "Repair left breaches: " + string.Join("; ", w.Validate()));
            Near(w.Reservations.Where(r => r.PileId == pile.Id && r.Resource == "food").Sum(r => r.Amount), 1000, "Over-claims were not trimmed to the stock");
            Check(crowd.Count(cat => cat.BedId == den.Id) == capacity && crowd.Take(capacity).All(cat => cat.BedId == den.Id), "The den did not keep its first sleepers");
            Check(c.JobId == kept.Id && kept.CatId == c.Id && extra.CatId == "" && !extra.Completed && extra.BlockedReason == Blocked.WorkerRequired, "The double owner did not keep its current job and release the other");
            Check(v.RoutineEvents.Count(e => e.Kind == EventKind.Repaired) == 3 && v.Events.All(e => e.Kind != EventKind.Repaired), "Repairs were not logged as routine events");
            Check(new[] { "overclaimed", "overbooked_bed", "double_work_owner" }.All(r => v.RoutineEvents.Any(e => e.Args["repair"] == r)), "A repair event did not name its repair");
            w.TimeSeconds = double.NaN;
            v.Cats.Add(new Cat { Id = c.Id, VillageId = v.Id, Name = "Twin" });
            var fatal = w.ValidateFatal();
            Check(fatal.Contains("Invalid clock") && fatal.Contains("Duplicate entity identity") && w.ValidateRepairable().Count == 0, "Fatal breaches were not reported as fatal: " + string.Join("; ", fatal));
            Check(w.RepairInvariants().Count == 0 && w.ValidateFatal().Count == fatal.Count, "Repair touched a fatal breach");
        }
        static Int2 FarTarget(World w, Village v, int distance)
        {
            // A passable generated tile walled in on every side: the search explores until its budget ends.
            for (int x = distance; x < distance + 40; x++)
            {
                var p = new Int2(v.Center.X + x, v.Center.Z);
                var t = w.GenerateSurfaceTile(p);
                if (!t.Water && !t.Mountain)
                    return p;
            }
            throw new InvalidOperationException("No dry target tile in the fixture row");
        }
        static void PathSearchInsertsNoTiles()
        {
            var w = Fixture(out var v, out var c);
            var target = FarTarget(w, v, 30);
            var cage = new[] { new Int2(target.X + 1, target.Z), new Int2(target.X - 1, target.Z), new Int2(target.X, target.Z + 1), new Int2(target.X, target.Z - 1) };
            foreach (var p in cage)
                w.TileAt(p).Wall = true;
            int tiles = w.Tiles.Count;
            uint random = w.RandomState;
            Check(w.Path(v.Center, target, v) == null && w.Path(c.Position, target) == null, "The caged target was reachable");
            Check(w.Tiles.Count == tiles && w.GetTile(target) == null, "A failed route search inserted " + (w.Tiles.Count - tiles) + " tiles");
            Check(w.RandomState == random, "A route search consumed entropy");
            Check(w.Walkable(target) && w.GetTile(target) != null && w.Tiles.Count == tiles + 1, "Walkable no longer explores the tile a mover steps on");
        }
        static void PathSearchMatchesInsertedTerrain()
        {
            var lazy = Fixture(out var a, out var ca);
            var eager = Fixture(out var b, out var cb);
            var target = FarTarget(lazy, a, 18);
            for (int x = -6; x <= target.X + 6; x++)
                for (int z = -12; z <= 12; z++)
                    eager.TileAt(new Int2(x, z));
            var routeLazy = lazy.Path(a.Center, target, a);
            var routeEager = eager.Path(b.Center, target, b);
            Check(routeLazy != null && routeEager != null, "Fixture target is unreachable");
            Check(routeLazy.SequenceEqual(routeEager), "A search over generated tiles chose a different route than over inserted ones");
            Check(lazy.GetTile(target) == null, "A successful route search inserted its target");
        }
        static void PathSearchSeesNewClaims()
        {
            var w = Fixture(out var v, out var c);
            Int2? blocked = null, beside = null;
            // Beyond the founding plateau (radius + 6 plus a 20-tile shoulder), where water and rock survive.
            for (int x = v.Radius + 27; x < v.Radius + 160 && blocked == null; x++)
                for (int z = -40; z <= 40 && blocked == null; z++)
                {
                    var p = new Int2(v.Center.X + x, v.Center.Z + z);
                    var q = new Int2(p.X - 1, p.Z);
                    var tp = w.GenerateSurfaceTile(p);
                    var tq = w.GenerateSurfaceTile(q);
                    bool pBlocked = tp.Mountain || tp.Water && tp.WaterDepth > .45;
                    bool qOpen = !tq.Mountain && !(tq.Water && tq.WaterDepth > .45);
                    if (pBlocked && qOpen && w.GetTile(p) == null && w.GetTile(q) == null)
                    {
                        blocked = p;
                        beside = q;
                    }
                }
            Check(blocked.HasValue, "No blocked generated tile near the fixture");
            Check(w.Path(beside.Value, blocked.Value) == null, "A blocked generated tile was passable");
            int tiles = w.Tiles.Count;
            v.Radius = Math.Max(Math.Abs(blocked.Value.X - v.Center.X), Math.Abs(blocked.Value.Z - v.Center.Z)) + 1;
            var route = w.Path(beside.Value, blocked.Value);
            Check(route != null && route.Count == 1 && route[0].Equals(blocked.Value), "A route search read stale generated terrain after a claim changed");
            Check(w.Tiles.Count == tiles, "The claimed search inserted tiles");
        }
        static void ContinuousNpc()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            double x = c.X, hunger = c.Hunger;
            Check(v.Cats.All(cat => cat.ControlledBy == ""), "NPC fixture must not rely on a directly controlled cat");
            w.Step(0.1);
            Near(c.X - x, 0.16, "NPC movement did not advance at its physical speed before one second");
            Near(hunger - c.Hunger, 0.1 * 20 / 3600, "Needs did not advance continuously");
            w.Step(0.9);
            Near(c.X - x, 1.6, "Fixed steps changed one-second movement speed");
            Near(hunger - c.Hunger, 20.0 / 3600, "Fixed steps accelerated needs depletion");
            Valid(w);
        }
        static void ContinuousFailedPath(string mode)
        {
            var w = FailedPathFixture(out var v, out var c);
            var job = v.Jobs.Single();
            long before = PathSearches(w);
            w.Step(0.1);
            Check(c.BlockedReason == "blocked_route" && c.Path.Count == 0 && c.X == 1 && job.Progress == 0, "Failed path advanced travel or work");
            Check(PathSearches(w) - before == 1, "Unchanged failed movement searched again before the one-second planning boundary");
            if (mode == "cadence")
            {
                w.Step(0.85);
                Check(PathSearches(w) - before == 1, "Failed search repeated within the planning interval");
                before = PathSearches(w);
                w.Step(0.05);
                Check(PathSearches(w) - before == 1, "Failed movement did not retry at the next planning boundary");
                before = PathSearches(w);
                w.Step(0.95);
                Check(PathSearches(w) == before, "Second failed search was not cached");
            }
            else if (mode == "recovery")
            {
                v.BoundaryEdges.Clear();
                w.Step(0.85);
                Near(c.X, 1, "Blocked route retried before its planning boundary");
                w.Step(0.05);
                Near(c.X, 1.08, "Reopened route did not resume at the next planning boundary");
                w.Step(2);
                Check(c.Position.Equals(job.Position) && job.Progress > 0, "Recovered route did not resume the same physical job");
            }
            else if (mode == "destination")
            {
                job.Position = new Int2(2, 2);
                w.Step(0.05);
                Near(c.X, 1.08, "A changed destination waited for the previous failure deadline");
                Check(PathSearches(w) - before == 2 && c.BlockedReason == "", "Changed destination did not get an immediate successful search");
            }
            else if (mode == "start")
            {
                // A restored/reassigned actor can have a different reached anchor with the same destination.
                c.Position = new Int2(2, 2);
                c.X = 2;
                w.Step(0.05);
                Check(PathSearches(w) - before == 2 && c.BlockedReason == "blocked_route", "A changed start reused the old anchor's failed search");
            }
            else
            {
                var split = FailedPathFixture(out var otherVillage, out var otherCat);
                w.Step(0.93);
                for (int i = 0; i < 20; i++)
                {
                    split.Step(0.03);
                    split.Step(0.02);
                }
                split.Step(0.03);
                Check(Fingerprint(w) == Fingerprint(split), "Fractional partitions changed failed-route timing or authoritative state");
                v.BoundaryEdges.Clear();
                otherVillage.BoundaryEdges.Clear();
                w.Step(1);
                split.Step(0.97);
                split.Step(0.03);
                Check(Fingerprint(w) == Fingerprint(split) && c.X > 1 && otherCat.X > 1, "Fractional partitions changed blocked-route recovery");
            }
            Valid(w);
        }
        static void ContinuousWork()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            w.Step(0.1);
            Near(job.Progress, 0.1, "Stationary work did not advance before one second");
            w.Step(0.9);
            Near(job.Progress, 1, "Fixed steps accelerated work");
            Valid(w);
        }
        static void ContinuousBudget()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            job.Position = new Int2(2, 2);
            c.X = 1.96;
            c.Path = new List<Int2> { job.Position };
            w.Step(0.05);
            Near(c.X, 2, "Cat did not finish its partial edge");
            Near(job.Progress, 0.025, "Travel and work each received a full actor timestep");
            Valid(w);
        }
        static void ContinuousPartition()
        {
            var whole = ContinuousFixture(out var a, out var ca, true);
            var split = ContinuousFixture(out var b, out var cb, true);
            string paused = Fingerprint(whole);
            whole.Step(0);
            Check(Fingerprint(whole) == paused, "Zero elapsed time changed paused authoritative state");
            whole.Step(1.03);
            for (int i = 0; i < 20; i++)
            {
                split.Step(0.03);
                split.Step(0.02);
            }
            split.Step(0.03);
            Check(Fingerprint(whole) == Fingerprint(split), "Fractional partitions changed authoritative state or random order");
            Near(a.Jobs.Single().Progress, 1, "Only complete fixed quanta may advance behavior");
            Near(whole.TimeSeconds, 1.03, "Requested elapsed time was discarded");
            Near(whole.PendingSeconds, 0.03, "Legacy fractional-second save meaning changed");
        }
        static void ContinuousDirectTurn()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            var tile = w.TileAt(new Int2(1, 3));
            tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false;
            tile.Biome = "grass";
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            w.Step(0.05);
            Near(c.X, 1.08, "Direct turn fixture did not enter its first edge");
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Mid-edge turn did not retrace the old segment");
            Near(c.Z, 2, "Mid-edge turn cut diagonally across grid edges");
            w.Step(0.05);
            Near(c.X, 1, "Second segment left the cardinal route");
            Near(c.Z, 2.08, "Second segment did not continue after physical retreat");
            Valid(w);
        }
        static void ContinuousTurnRate(string terrain)
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            var oldSegment = w.TileAt(new Int2(2, 2));
            oldSegment.Road = terrain == "road";
            oldSegment.Biome = terrain == "forest" ? "forest" : "grass";
            var newSegment = w.TileAt(new Int2(1, 3));
            newSegment.Wall = newSegment.Water = newSegment.Mountain = newSegment.Road = newSegment.Dirt = false;
            newSegment.Biome = "grass";
            double speed = 1.6 * 1.3 * (terrain == "road" ? 1.75 : terrain == "forest" ? 0.85 : 1);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            w.Step(0.05);
            Near(c.X, 1 + speed * 0.05, "Forward segment did not use research and " + terrain + " movement rates");
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Retreat changed the researched " + terrain + " segment speed");
            Near(c.Z, 2, "Retreat granted unearned time on the next segment");
            w.Step(0.05);
            Near(c.X, 1, "Turn cut across the previous segment");
            Near(c.Z, 2 + 1.6 * 1.3 * 0.05, "Turn did not resume at the new segment's own speed");
            Valid(w);
        }
        static void ContinuousTurnRemainingBudget()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            c.X = 1 + 1.6 * 1.3 * 0.025;
            var tile = w.TileAt(new Int2(1, 3));
            tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false;
            tile.Biome = "grass";
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Partial retreat did not return to its anchor");
            Near(c.Z, 2 + 1.6 * 1.3 * 0.025, "Partial retreat did not leave its actual unused movement time");
            Valid(w);
        }
        static void ContinuousReboardRate()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            c.Path.Clear();
            var start = c.Position;
            var next = new Int2(2, 2);
            var end = new Int2(3, 2);
            w.TileAt(next).Road = true;
            var vehicle = new Vehicle { Id = w.Id("rate-cart"), Mode = "rail", Position = start, HasContinuousPosition = true, X = 1.5, Z = 2, Progress = 0.5, Cargo = new List<Stack> { new Stack("logs", 8) } };
            v.Vehicles.Add(vehicle);
            var route = new TransportRoute { Id = w.Id("rate-route"), Mode = "rail", CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = v.Stockpiles[0].Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, next, end } };
            v.Routes.Add(route);
            vehicle.RouteId = c.BuildingId = route.Id;
            w.Step(0.05);
            Near(c.X, 1 + 1.6 * 1.3 * 1.75 * 0.05, "Fractional reboarding ignored the researched road movement speed");
            Near(vehicle.X, 1.5, "Vehicle moved before physical reboarding");
            Near(World.Amount(vehicle.Cargo, "logs"), 8, "Reboarding changed finite cargo");
            Valid(w);
        }
        static void ContinuousMissingSupply()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water");
            // Thirsty cats drink at known open water when stores are dry; this case needs no water anywhere.
            foreach (var tile in w.Tiles.Where(t => t.Water))
                tile.Water = false;
            c.Thirst = 1;
            job.Position = new Int2(4, 2);
            job.Phase = "travel";
            c.Path = w.Path(c.Position, job.Position, v);
            c.X = 1.95;
            for (int tick = 0; tick < 40; tick++)
            {
                double x = c.X;
                w.Step(World.SimulationStepSeconds);
                Check(c.X + 1e-9 >= x, "A missing need supply repeatedly reversed the active physical work route");
                Check(c.JobId == job.Id && job.CatId == c.Id, "Missing-supply retry changed active work ownership");
            }
            Check(c.Position.Equals(job.Position) && job.Progress > 0, "Missing-supply retries prevented physical arrival and work");
            Near(Goods(w, v, "water"), 0, "Missing-supply retry created water");
            Check(c.Thirst < 1, "Missing-supply retry satisfied thirst without water");
            Valid(w);
        }
        static void ContinuousRate(string activity)
        {
            var w = Fixture(out var v, out var c);
            c.Stats.Clear();
            c.Stats.Add(new Stack("medicine", 50));
            c.Stats.Add(new Stack("building", 50));
            Farm farm = null;
            double before;
            if (activity == "research")
            {
                var station = Station(w, v, "research_hut", c.Position);
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
                v.ResearchPoints = 0;
                before = 0;
            }
            else if (activity == "farm")
            {
                farm = new Farm { Id = w.Id("continuous-farm"), WorkerId = c.Id, Position = c.Position, CycleSeconds = 600 };
                v.Farms.Add(farm);
                c.BuildingId = farm.Id;
                before = 0;
            }
            else
            {
                var den = v.Buildings.First(b => b.Id == c.BedId);
                // Cats sleep on the den's doorstep, never inside the den.
                c.Position = den.Entrance;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                c.Goal = "need_sleep";
                c.Rest = 50;
                before = c.Rest;
            }
            double Rate() => activity == "research" ? 1.0 / 600 : activity == "farm" ? 1 : 0.15 - 12.0 / 3600;
            double Value() => activity == "research" ? v.ResearchPoints : activity == "farm" ? farm.Growth : c.Rest;
            w.Step(0.2);
            Near(Value() - before, Rate() * 0.2, activity + " did not advance continuously at the original rate");
            w.Step(0.8);
            Near(Value() - before, Rate(), activity + " accelerated with fixed steps");
            Valid(w);
        }
        static void ContinuousCombatBudget()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            foreach (var cat in v.Cats)
                cat.Skills.Clear();
            c.Skills.Add(new Stack("fight", 1));
            var job = v.Jobs.Single();
            var first = new Raid { Id = w.Id("first-raid"), Position = c.Position };
            var second = new Raid { Id = w.Id("second-raid"), Position = c.Position };
            v.Raids.Add(first);
            v.Raids.Add(second);
            w.Step(0.05);
            Near(job.Progress, 0.05, "Existing work lost the first actor quantum");
            Near(first.Health + second.Health, 60, "Work and combat each received the same cat's full quantum");
            w.Step(0.05);
            Check(first.Health < 30 && second.Health == 30, "One cat attacked two raids with the same quantum: first=" + first.Health + " second=" + second.Health + " goal=" + c.Goal + " path=" + c.Path.Count);
            Valid(w);
        }
        static void ContinuousTransportInterruption(bool shipping)
        {
            var w = Fixture(out var v, out var c);
            string mode = shipping ? "shipping" : "rail";
            var start = new Int2(4, 2);
            var middle = new Int2(5, 2);
            var end = new Int2(6, 2);
            foreach (var p in new[] { start, middle, end })
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Mountain = false;
                tile.Rail = !shipping;
                tile.Water = shipping && p.Equals(middle);
                tile.Dock = shipping && !p.Equals(middle);
            }
            c.Position = start;
            c.X = start.X;
            c.Z = start.Z;
            var station = Station(w, v, "research_hut", new Int2(2, 4));
            var destination = new Stockpile { Id = w.Id("interruption-destination"), Position = end };
            v.Stockpiles.Add(destination);
            v.Stockpiles[0].Position = start;
            var vehicle = new Vehicle { Id = w.Id("interruption-vehicle"), Mode = mode, Position = start, Cargo = new List<Stack> { new Stack("logs", 8) } };
            v.Vehicles.Add(vehicle);
            var route = new TransportRoute { Id = w.Id("interruption-route"), Mode = mode, CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = destination.Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, middle, end } };
            vehicle.RouteId = c.BuildingId = route.Id;
            v.Routes.Add(route);
            w.Step(0.2);
            Near(vehicle.X, start.X + 0.2, "Interruption fixture did not reach a partial edge");
            Check(!w.Apply(Context(v), new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }).Success, "Fractional vehicle driver was reassigned before stopping");
            if (shipping)
            {
                c.Thirst = 0;
                w.Step(0.05);
                Near(c.X, vehicle.X, "Driver disembarked over water while its anchor was still the dock");
                Check(!c.Goal.StartsWith("need_", StringComparison.Ordinal), "Shipping needs interrupted an undocked fractional segment");
            }
            Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(v.Routes.Contains(route) && route.CancelRequested, "Cancellation discarded a vehicle mid-edge");
            double x = vehicle.X;
            w.Step(0.05);
            Check(vehicle.X < x && vehicle.X >= x - 0.05 - 1e-9, "Cancellation snapped the vehicle to its anchor");
            for (int tick = 0; tick < 400 && v.Routes.Contains(route); tick++)
                w.Step(0.05);
            Check(!v.Routes.Contains(route) && vehicle.RouteId == "" && vehicle.Position.Equals(start), "Vehicle did not complete physical cancellation after the docked driver's need trip: phase=" + route.Phase + " goal=" + c.Goal + " x=" + vehicle.X + " catX=" + c.X);
            Near(vehicle.X, start.X, "Cancelled vehicle retained a fractional position");
            Near(Goods(w, v, "logs"), 8, "Cancellation lost or duplicated finite cargo");
            Valid(w);
        }
        static void ContinuousMover(string kind)
        {
            var w = Fixture(out var v, out var c);
            var start = new Int2(4, 2);
            var next = new Int2(5, 2);
            var end = new Int2(6, 2);
            foreach (var p in new[] { start, next, end })
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Water = tile.Mountain = false;
                tile.Rail = true;
            }
            object mover;
            double speed;
            if (kind == "merchant")
            {
                var trader = v.Trader;
                trader.Phase = "arriving";
                trader.Position = start;
                trader.Progress = 0.8;
                trader.Path = new List<Int2> { next, end };
                trader.VisitDestination = end;
                trader.Goods.Add(new Stack("logs", 8));
                mover = trader;
                speed = 0.8;
            }
            else if (kind == "raid")
            {
                var raid = new Raid { Id = w.Id("continuous-raid"), Position = start, Progress = 12, Path = new List<Int2> { next, end }, Loot = new List<Stack> { new Stack("logs", 8) } };
                v.Raids.Add(raid);
                mover = raid;
                speed = 0.5;
            }
            else if (kind == "caravan")
            {
                var trade = new TradeOffer { Id = w.Id("continuous-trade"), Status = "outbound", X = start.X, Z = start.Z, FromVillageId = v.Id, ToVillageId = v.Id, Offered = new Stack("logs", 8), Path = new List<Int2> { next, end } };
                w.TradeOffers.Add(trade);
                mover = trade;
                speed = 0.8;
            }
            else
            {
                c.Position = start;
                c.X = start.X;
                c.Z = start.Z;
                var destination = new Stockpile { Id = w.Id("continuous-destination"), Position = end };
                v.Stockpiles.Add(destination);
                var vehicle = new Vehicle { Id = w.Id("continuous-cart"), Mode = "rail", Position = start, Cargo = new List<Stack> { new Stack("logs", 8) } };
                v.Vehicles.Add(vehicle);
                var route = new TransportRoute { Id = w.Id("continuous-route"), Mode = "rail", CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = destination.Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, next, end } };
                vehicle.RouteId = route.Id;
                v.Routes.Add(route);
                mover = vehicle;
                speed = 1;
            }
            w.Step(0.2);
            var xField = mover.GetType().GetField("X");
            double x = xField == null ? start.X : (double)xField.GetValue(mover);
            Near(x, start.X + speed * 0.2, kind + " did not advance its authoritative fractional position");
            var position = mover.GetType().GetField("Position");
            if (position != null)
                Check(((Int2)position.GetValue(mover)).Equals(start), kind + " grid anchor moved before reaching the next tile");
        }
        static void ContinuousLegacyCaravan(int direction)
        {
            var w = Fixture(out var v, out var c);
            var next = new Int2(direction * 5, 2);
            var tile = w.TileAt(next);
            tile.Wall = tile.Water = tile.Mountain = false;
            var trade = new TradeOffer { Id = w.Id("legacy-fractional-caravan"), Status = "outbound", X = direction * 4.6, Z = 2, Progress = 0.8, FromVillageId = v.Id, ToVillageId = v.Id, Offered = new Stack("logs", 8), Path = new List<Int2> { next, new Int2(direction * 6, 2) } };
            w.TradeOffers.Add(trade);
            w.Step(0.2);
            Near(trade.X, direction * 4.76, "Legacy caravan fractional position was rounded or received old cadence credit twice");
            Check(trade.Position.Equals(new Int2(direction * 4, 2)), "Legacy caravan did not retain the last reached grid anchor");
            Near(trade.Offered.Amount, 8, "Legacy continuous initialization changed finite cargo");
        }
    }
}
