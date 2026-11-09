using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class PrimaryQueueContinuityScenarios
    {
        public static void Run(Action<string, Action> test, Action<bool, string> check)
        {
            var coverage = new Coverage(check);
            foreach (bool later in new[] { false, true })
            {
                string position = later ? "later" : "primary";
                foreach (bool stale in new[] { false, true })
                    test("regression.primary_queue_" + position + "_officer_" + (stale ? "replacement" : "append"), () => coverage.OfficerWriter(later, stale));
                foreach (bool repeat in new[] { false, true })
                    test("acceptance.primary_queue_" + position + "_physical_" + (repeat ? "repeat" : "finite"), () => coverage.ProductionWriter(later, repeat));
                foreach (string edit in new[] { "add", "remove", "move", "repeat", "pause", "resume" })
                    test("regression.primary_queue_" + position + "_manual_" + edit, () => coverage.ManualWriter(later, edit));
            }
            foreach (string refusal in new[] { "locked", "wrong_station", "unknown_recipe", "limit", "negative_index", "missing_index", "edge", "unknown_edit", "missing_worker", "active_remove", "active_move" })
                test("regression.primary_queue_refused_" + refusal, () => coverage.RefusedEdit(refusal));
            test("regression.primary_queue_mixed_officer_cleanup", () => coverage.OfficerCleanup(true));
            test("regression.primary_queue_untouched_officer_cleanup", () => coverage.OfficerCleanup(false));
            test("regression.primary_queue_preorders_vacancy_and_capacity", coverage.Preorders);
            test("regression.primary_queue_delegate_preserves_historical_mirror", coverage.Delegate);
            test("regression.primary_queue_barracks_station_pause_is_separate", coverage.BarracksPause);
            foreach (bool repeat in new[] { false, true })
                test("regression.primary_queue_zero_slot_legacy_" + (repeat ? "repeat" : "finite"), () => coverage.LegacyCompletion(repeat));
            test("regression.primary_queue_historical_domain_boundary", () => RunHistoricalContinuity());
            test("acceptance.primary_queue_natural_farmer_chain", () => RunChain());
        }

        static void Require(bool condition, string message)
        {
            if (!condition) throw new InvalidOperationException("Primary queue: " + message);
        }
        static ActionResult Apply(World world, GameAction action) => world.Apply(new PlayerContext
        {
            PlayerId = "primary-queue-continuity",
            VillageId = world.Villages[0].Id
        }, action);
        static void Act(World world, GameAction action)
        {
            var result = Apply(world, action);
            Require(result.Success, action.Kind + "/" + action.Edit + ": " + result.Error);
        }
        static GameAction Edit(Building station, Cat worker, string edit) => new GameAction
        {
            Kind = worker == null ? "EditProductionQueue" : "EditProductionWorkSlot",
            BuildingId = station.Id,
            CatId = worker?.Id ?? "",
            Edit = edit
        };
        static List<QueueEntry> Separate(List<QueueEntry> queue) => queue.Select(q => new QueueEntry
        {
            RecipeId = q.RecipeId,
            Repeat = q.Repeat,
            AutomatedBy = q.AutomatedBy
        }).ToList();
        static bool At(Cat cat, Int2 at) => cat.Position.Equals(at) && Math.Abs(cat.X - at.X) + Math.Abs(cat.Z - at.Z) < 1e-8;
        static double Physical(Village village, string resource) => village.Stockpiles.Sum(p => World.Amount(p.Goods, resource))
            + village.Cats.Sum(c => World.Amount(c.Cargo, resource)) + village.Jobs.Sum(j => World.Amount(j.Local, resource))
            + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource))
            + village.Buildings.SelectMany(b => b.Slots).Sum(s => World.Amount(s.Inputs, resource) + World.Amount(s.Outputs, resource));
        static void Valid(World world) => Require(world.Validate().Count == 0, string.Join("; ", world.Validate()));

        sealed class Coverage
        {
            readonly Action<bool, string> check;
            public Coverage(Action<bool, string> check) { this.check = check; }

            World Fixture(out Village village, out Building station, out Cat worker, bool later = false)
            {
                var world = World.Create(41);
                village = world.Villages[0];
                village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
                village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
                var store = village.Stockpiles[0];
                store.Position = new Int2(3, 2);
                store.Capacity = 100000;
                store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
                village.LastLeaderResearch = 0;
                // Writer fixtures isolate one real worker; RunChain below retains
                // all thirty uncontrolled cats, ordinary skills and normal needs.
                foreach (var cat in village.Cats)
                {
                    cat.ControlledBy = "fixture-held";
                    cat.ControlLeaseUntil = 10000000;
                    cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3;
                }
                village.Research.Add("textiles");
                station = new Building { Id = world.Id("queue-clothier"), Kind = "clothier", Completed = true, Position = new Int2(1, 5) };
                village.Buildings.Add(station);
                Act(world, new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.Cats[0].Id });
                worker = village.Cats[1];
                worker.ControlledBy = "";
                Act(world, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
                if (later)
                {
                    village.Research.Add("clothier_crews");
                    var add = Edit(station, worker, "add"); add.RecipeId = "fibre_to_cloth"; add.Repeat = true; Act(world, add);
                    var pause = Edit(station, worker, "pause"); pause.Enabled = true; Act(world, pause);
                    var primaryWorker = worker;
                    worker = village.Cats[2]; worker.ControlledBy = "";
                    Act(world, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
                    Act(world, new GameAction { Kind = "AssignWorker", CatId = primaryWorker.Id });
                    primaryWorker.ControlledBy = "fixture-held";
                    // Existing positions survive a reduced capacity. WorkerId now
                    // names position one while the vacant paused zero stays primary.
                    village.Research.Remove("clothier_crews");
                    check(station.Slots.Count == 2 && station.WorkerId == worker.Id && station.Slots[0].CatId == ""
                        && station.Slots[0].Paused && Catalog.BuildingEffect(village, "clothier", "worker_slots", 1) == 1,
                        "later-position fixture lost retained positions or reduced capacity");
                }
                return world;
            }

            public void OfficerWriter(bool later, bool stale)
            {
                var world = Fixture(out var village, out var station, out var worker, later);
                var slot = station.Slots.Single(s => s.CatId == worker.Id);
                var slots = station.Slots; var queue = slot.Queue;
                QueueEntry staleOrder = null;
                if (stale)
                {
                    var add = Edit(station, worker, "add"); add.RecipeId = "fibre_to_cloth"; add.Repeat = true; Act(world, add);
                    staleOrder = slot.Queue[0]; staleOrder.AutomatedBy = "cloth_leader";
                }
                Act(world, Edit(station, worker, "delegate"));
                station.Queue = Separate(station.Slots[0].Queue);
                var historical = station.Queue; var primary = station.Slots[0]; var primaryOrders = primary.Queue.ToArray();
                World.Add(village.Stockpiles[0].Goods, "fibre", 5);
                world.TimeSeconds = 9;
                world.Step(1);
                check(ReferenceEquals(station.Slots, slots) && ReferenceEquals(slot.Queue, queue) && slot.Queue.Count == 1
                    && slot.Queue[0].RecipeId == "fibre_to_thread" && slot.Queue[0].AutomatedBy == "cloth_leader" && !slot.Queue[0].Repeat
                    && !ReferenceEquals(slot.Queue[0], staleOrder), "officer failed to append/replace the exact funded automatic order");
                check(worker.JobId == "" && world.Reservations.Count == 0 && World.Amount(village.Stockpiles[0].Goods, "fibre") == 5,
                    "officer choice started work or consumed physical inputs early");
                check(later ? ReferenceEquals(station.Queue, historical) && primary.Queue.SequenceEqual(primaryOrders) && primary.Paused && primary.CatId == ""
                    : ReferenceEquals(station.Queue, slot.Queue), "officer writer mirrored a later position or left the primary mirror stale");
                Valid(world);
            }

            public void ManualWriter(bool later, string edit)
            {
                var world = Fixture(out _, out var station, out var worker, later);
                var slot = station.Slots.Single(s => s.CatId == worker.Id);
                foreach (string recipe in new[] { "fibre_to_thread", "fibre_to_cloth" })
                {
                    var add = Edit(station, worker, "add"); add.RecipeId = recipe; Act(world, add);
                }
                slot.Paused = edit == "resume";
                foreach (var q in slot.Queue) q.AutomatedBy = "cloth_leader";
                Act(world, Edit(station, worker, "delegate"));
                station.Queue = Separate(station.Slots[0].Queue);
                var mirror = station.Queue; var primaryOrders = station.Slots[0].Queue.ToArray();
                var queue = slot.Queue; var slots = station.Slots; var first = queue[0]; var second = queue[1];
                var action = Edit(station, worker, edit == "resume" ? "pause" : edit);
                action.RecipeId = "fibre_to_thread"; action.Enabled = edit != "resume"; action.Direction = 1;
                Act(world, action);
                check(ReferenceEquals(station.Slots, slots) && ReferenceEquals(slot.Queue, queue) && slot.AutomatedBy == ""
                    && slot.Queue.All(q => q.AutomatedBy == ""), "successful manual writer replaced the slot/list or retained officer control");
                check(edit == "add" ? queue.Count == 3 && ReferenceEquals(queue[0], first) && ReferenceEquals(queue[1], second)
                    : edit == "remove" ? queue.Count == 1 && ReferenceEquals(queue[0], second)
                    : edit == "move" ? queue.Count == 2 && ReferenceEquals(queue[0], second) && ReferenceEquals(queue[1], first)
                    : queue.Count == 2 && ReferenceEquals(queue[0], first) && ReferenceEquals(queue[1], second), "manual writer changed the wrong ordered identities");
                check(edit != "repeat" || first.Repeat, "repeat edit did not change the exact retained entry");
                check(edit != "pause" && edit != "resume" || slot.Paused == (edit == "pause"), "manual pause/resume changed the wrong position");
                check(later ? ReferenceEquals(station.Queue, mirror) && station.Slots[0].Queue.SequenceEqual(primaryOrders)
                    : ReferenceEquals(station.Queue, queue), "manual writer mirrored a later position or failed to bind the primary");
                Valid(world);
            }

            public void RefusedEdit(string refusal)
            {
                var world = Fixture(out var village, out var station, out var worker);
                var add = Edit(station, worker, "add"); add.RecipeId = "fibre_to_thread"; add.Repeat = true; Act(world, add);
                add.RecipeId = "fibre_to_cloth"; Act(world, add);
                var slot = station.Slots[0];
                if (refusal == "limit") while (slot.Queue.Count < 64) slot.Queue.Add(new QueueEntry { RecipeId = "fibre_to_thread", Repeat = true });
                slot.Paused = true;
                foreach (var q in slot.Queue) q.AutomatedBy = "cloth_leader";
                Act(world, Edit(station, worker, "delegate"));
                if (refusal.StartsWith("active_", StringComparison.Ordinal))
                {
                    var job = new Job
                    {
                        Id = world.Id("queue-active"),
                        Kind = "production",
                        CatId = worker.Id,
                        TargetId = station.Id,
                        RecipeId = "fibre_to_thread",
                        Position = station.Position,
                        Phase = "fetch"
                    };
                    village.Jobs.Add(job); worker.JobId = slot.JobId = job.Id;
                }
                station.Queue = new List<QueueEntry> { new QueueEntry { RecipeId = "fibre_to_cloth", Repeat = false, AutomatedBy = "historical" } };
                var mirror = station.Queue; var mirrorOrder = mirror[0]; var slots = station.Slots; var queue = slot.Queue; var orders = queue.ToArray();
                long id = world.NextId; uint entropy = world.RandomState; double time = world.TimeSeconds;
                string jobId = worker.JobId; var jobs = village.Jobs.ToArray();
                var action = Edit(station, worker, "remove");
                if (refusal == "locked" || refusal == "wrong_station" || refusal == "unknown_recipe" || refusal == "limit")
                {
                    action.Edit = "add";
                    action.RecipeId = refusal == "locked" ? "foraging_preparation" : refusal == "wrong_station" ? "logs_to_planks"
                        : refusal == "unknown_recipe" ? "missing-recipe" : "fibre_to_thread";
                }
                else if (refusal == "negative_index") action.Index = -1;
                else if (refusal == "missing_index") action.Index = queue.Count;
                else if (refusal == "edge") { action.Edit = "move"; action.Direction = -1; }
                else if (refusal == "unknown_edit") action.Edit = "unknown";
                else if (refusal == "missing_worker") action.CatId = village.Cats[2].Id;
                else if (refusal == "active_move") { action.Edit = "move"; action.Direction = 1; }
                var result = Apply(world, action);
                check(!result.Success, "invalid edit was accepted: " + refusal);
                check(ReferenceEquals(station.Queue, mirror) && ReferenceEquals(mirror.Single(), mirrorOrder) && mirrorOrder.AutomatedBy == "historical"
                    && ReferenceEquals(station.Slots, slots) && ReferenceEquals(slot.Queue, queue) && queue.SequenceEqual(orders)
                    && orders.All(q => q.Repeat && q.AutomatedBy == "cloth_leader") && slot.Paused && slot.AutomatedBy == "cloth_leader",
                    "refused edit repaired history, changed identity, repeat, pause or officer ownership");
                check(world.NextId == id && world.RandomState == entropy && world.TimeSeconds == time && worker.JobId == jobId
                    && village.Jobs.SequenceEqual(jobs) && world.Reservations.Count == 0 && worker.Cargo.Count == 0
                    && worker.BuildingId == station.Id && slot.CatId == worker.Id, "refused edit changed work, goods, time, entropy or identity allocation");
                Valid(world);
            }

            public void OfficerCleanup(bool removePrimary)
            {
                var world = Fixture(out var village, out var station, out var worker);
                var primary = station.Slots[0]; var manual = new QueueEntry { RecipeId = "fibre_to_thread", Repeat = true };
                var foreign = new QueueEntry { RecipeId = "fibre_to_cloth", AutomatedBy = "forester" };
                primary.Queue.AddRange(new[] { manual, foreign });
                if (removePrimary) primary.Queue.Insert(1, new QueueEntry { RecipeId = "fibre_to_thread", AutomatedBy = "cloth_leader" });
                // Already-vacant primary still owns its queue. A later occupied
                // position and an untouched building exercise the non-sweep rule.
                Act(world, new GameAction { Kind = "AssignWorker", CatId = worker.Id });
                primary.AutomatedBy = "cloth_leader";
                var laterManual = new QueueEntry { RecipeId = "fibre_to_cloth", Repeat = true };
                var later = new WorkSlot
                {
                    CatId = worker.Id,
                    AutomatedBy = "cloth_leader",
                    Queue = new List<QueueEntry>
                    { new QueueEntry { RecipeId = "fibre_to_thread", AutomatedBy = "cloth_leader" }, laterManual }
                };
                station.Slots.Add(later); station.WorkerId = worker.Id; worker.BuildingId = station.Id;
                station.Queue = Separate(primary.Queue); var mirror = station.Queue; var queue = primary.Queue; var slots = station.Slots;
                var untouched = new Building
                {
                    Id = world.Id("untouched-queue"),
                    Kind = "clothier",
                    Completed = true,
                    Queue = new List<QueueEntry> { new QueueEntry { RecipeId = "fibre_to_thread", AutomatedBy = "historical" } }
                };
                untouched.Slots.Add(new WorkSlot { Queue = new List<QueueEntry> { new QueueEntry { RecipeId = "fibre_to_cloth" } } });
                village.Buildings.Add(untouched); var untouchedMirror = untouched.Queue;
                Act(world, new GameAction { Kind = "UnassignOfficer", Role = "cloth_leader" });
                check(ReferenceEquals(station.Slots, slots) && ReferenceEquals(primary.Queue, queue) && primary.Queue.SequenceEqual(new[] { manual, foreign })
                    && later.Queue.Count == 1 && ReferenceEquals(later.Queue[0], laterManual), "cleanup replaced positions or removed manual/other-role orders");
                check(primary.CatId == "" && later.CatId == "" && worker.BuildingId == "" && primary.AutomatedBy == "" && later.AutomatedBy == ""
                    && manual.Repeat && foreign.AutomatedBy == "forester" && laterManual.Repeat, "cleanup changed retained entry ownership, repeats or vacancy behavior");
                check(removePrimary ? ReferenceEquals(station.Queue, primary.Queue) : ReferenceEquals(station.Queue, mirror),
                    "cleanup failed to mirror an actual primary removal or repaired an untouched historical primary");
                check(ReferenceEquals(untouched.Queue, untouchedMirror) && untouched.Queue[0].AutomatedBy == "historical", "cleanup swept an unrelated historical mirror");
                Valid(world);
            }

            public void Preorders()
            {
                var world = Fixture(out _, out var station, out var worker);
                // An authored pre-staffing station has no position yet.
                var preorder = new Building { Id = world.Id("preorder"), Kind = "clothier", Completed = true, Position = new Int2(-3, 5) };
                world.Villages[0].Buildings.Add(preorder);
                foreach (bool repeat in new[] { false, true })
                {
                    var add = Edit(preorder, null, "add"); add.RecipeId = "fibre_to_thread"; add.Repeat = repeat; Act(world, add);
                }
                var pause = Edit(preorder, null, "pause"); pause.Enabled = true; Act(world, pause);
                var queue = preorder.Queue; var orders = queue.ToArray();
                Act(world, new GameAction { Kind = "AssignWorker", BuildingId = preorder.Id, CatId = worker.Id });
                var slot = preorder.Slots.Single();
                check(ReferenceEquals(slot.Queue, queue) && queue.SequenceEqual(orders) && !orders[0].Repeat && orders[1].Repeat && preorder.Paused,
                    "first position failed to adopt exact finite/repeating preorders and station pause");
                Act(world, new GameAction { Kind = "AssignWorker", CatId = worker.Id });
                Act(world, new GameAction { Kind = "AssignWorker", BuildingId = preorder.Id, CatId = worker.Id });
                check(preorder.Slots.Count == 1 && ReferenceEquals(preorder.Slots[0], slot) && ReferenceEquals(slot.Queue, queue)
                    && queue.SequenceEqual(orders) && preorder.Paused && station.Slots.Single().CatId == "", "release/refill replaced a vacancy or its retained preorder identities");
                Valid(world);
            }

            public void Delegate()
            {
                var world = Fixture(out var village, out var station, out var worker);
                var add = Edit(station, worker, "add"); add.RecipeId = "fibre_to_thread"; add.Repeat = true; Act(world, add);
                var slot = station.Slots[0]; slot.Paused = true;
                var queue = slot.Queue; var order = queue[0];
                var job = new Job
                {
                    Id = world.Id("delegated-batch"),
                    Kind = "production",
                    CatId = worker.Id,
                    TargetId = station.Id,
                    RecipeId = order.RecipeId,
                    Phase = "working",
                    Position = station.Position,
                    Progress = 2
                };
                village.Jobs.Add(job); worker.JobId = slot.JobId = job.Id;
                station.Queue = Separate(queue); var mirror = station.Queue;
                Act(world, Edit(station, worker, "delegate"));
                check(ReferenceEquals(station.Queue, mirror) && ReferenceEquals(slot.Queue, queue) && ReferenceEquals(queue.Single(), order)
                    && order.AutomatedBy == "" && order.Repeat && slot.AutomatedBy == "cloth_leader" && slot.Paused
                    && slot.CatId == worker.Id && worker.JobId == job.Id && slot.JobId == job.Id && job.Progress == 2 && job.Phase == "working",
                    "handback repaired history, relabeled manual orders or changed the exact paused worker/batch");
                Valid(world);
            }

            public void BarracksPause()
            {
                var world = Fixture(out _, out var station, out _);
                // Station training Pause is deliberately separate from production
                // position Pause, including compatibility mirror maintenance.
                var barracks = new Building { Id = world.Id("queue-barracks"), Kind = "barracks", Completed = true };
                var primary = new WorkSlot { AutomatedBy = "captain" };
                barracks.Slots.Add(primary); world.Villages[0].Buildings.Add(barracks);
                var mirror = barracks.Queue; var queue = primary.Queue;
                foreach (bool enabled in new[] { true, false })
                {
                    var pause = Edit(barracks, null, "pause"); pause.Enabled = enabled; Act(world, pause);
                    check(barracks.Paused == enabled && !primary.Paused && primary.AutomatedBy == "captain"
                        && ReferenceEquals(barracks.Queue, mirror) && ReferenceEquals(primary.Queue, queue)
                        && !ReferenceEquals(barracks.Queue, queue) && station.Slots.Count == 1,
                        "Barracks training pause changed production ownership, positions or repaired a separate mirror");
                }
                Valid(world);
            }

            public void ProductionWriter(bool later, bool repeat)
            {
                var world = Fixture(out var village, out var station, out var worker, later);
                var add = Edit(station, worker, "add"); add.RecipeId = "fibre_to_thread"; add.Repeat = repeat; Act(world, add);
                add.RecipeId = "fibre_to_cloth"; add.Repeat = false; Act(world, add);
                var slot = station.Slots.Single(s => s.CatId == worker.Id); var queue = slot.Queue; var head = queue[0]; var tail = queue[1];
                station.Queue = Separate(station.Slots[0].Queue); var historical = station.Queue; var primaryOrders = station.Slots[0].Queue.ToArray();
                World.Add(village.Stockpiles[0].Goods, "fibre", 5);
                bool claimed = false, carried = false, worked = false, committed = false, delivered = false;
                Job batch = null;
                for (int tick = 0; tick < 18000 && !delivered; tick++)
                {
                    world.Step(World.SimulationStepSeconds);
                    batch = batch ?? village.Jobs.SingleOrDefault(j => j.CatId == worker.Id && j.Kind == "production");
                    if (batch == null) continue;
                    claimed |= world.Reservations.Any(r => r.OwnerId == batch.Id && r.Resource == "fibre");
                    if (World.Amount(worker.Cargo, "fibre") > 0)
                    {
                        check(claimed, "production picked up input without its prior physical claim"); carried = true;
                    }
                    if (batch.Progress > 0 && batch.Phase == "working")
                    {
                        check(carried && At(worker, station.Position) && World.Amount(batch.Local, "fibre") == 5,
                            "work skipped physical pickup/delivery of the exact bill"); worked = true;
                    }
                    if (!committed && batch.Phase == "output_delivery")
                    {
                        check(worked && ReferenceEquals(slot.Queue, queue) && queue.Count == (repeat ? 2 : 1)
                            && ReferenceEquals(queue[0], tail) && (!repeat || ReferenceEquals(queue[1], head) && head.Repeat),
                            "production did not commit/rotate the exact finite or repeating head once");
                        check(later ? ReferenceEquals(station.Queue, historical) && station.Slots[0].Queue.SequenceEqual(primaryOrders)
                            : ReferenceEquals(station.Queue, queue), "production mirrored a later queue or left the primary mirror stale");
                        check(Physical(village, "fibre") == 0 && Physical(village, "thread") == 5, "output commit failed finite physical conservation");
                        var pause = Edit(station, worker, "pause"); pause.Enabled = true; Act(world, pause);
                        committed = true;
                    }
                    delivered = batch.Completed;
                }
                check(claimed && carried && worked && committed && delivered && worker.JobId == "" && worker.Cargo.Count == 0
                    && World.Amount(village.Stockpiles[0].Goods, "thread") == 5 && world.Reservations.Count == 0 && station.Inputs.Count == 0,
                    "bounded production failed to physically deliver its single finite output without leftover claims/cargo");
                Valid(world);
            }

            public void LegacyCompletion(bool repeat)
            {
                var world = Fixture(out var village, out var station, out var worker);
                // Accepted legacy snapshot: a no-position active job has already
                // staged its exact bill and earned all but one quantum of work.
                station.Slots.Clear();
                var head = new QueueEntry { RecipeId = "fibre_to_thread", Repeat = repeat };
                var tail = new QueueEntry { RecipeId = "fibre_to_cloth" };
                station.Queue.AddRange(new[] { head, tail }); var queue = station.Queue;
                worker.Position = station.Position; worker.X = worker.Position.X; worker.Z = worker.Position.Z;
                var recipe = Catalog.Recipe(head.RecipeId);
                var job = new Job
                {
                    Id = world.Id("zero-slot-legacy"),
                    Kind = "production",
                    TargetId = station.Id,
                    CatId = worker.Id,
                    RecipeId = recipe.Id,
                    Position = station.Position,
                    Phase = "working",
                    RequiredWork = recipe.Seconds,
                    Progress = recipe.Seconds - .01,
                    Reserved = new List<Stack> { new Stack("fibre", 5) },
                    Local = new List<Stack> { new Stack("fibre", 5) }
                };
                village.Jobs.Add(job); worker.JobId = job.Id;
                Valid(world);
                world.Step(World.SimulationStepSeconds);
                check(station.Slots.Count == 0 && ReferenceEquals(station.Queue, queue) && queue.Count == (repeat ? 2 : 1)
                    && ReferenceEquals(queue[0], tail) && (!repeat || ReferenceEquals(queue[1], head))
                    && job.Phase == "output_delivery" && Physical(village, "fibre") == 0 && Physical(village, "thread") == 5,
                    "zero-slot legacy completion changed fallback semantics or duplicated/lost its finite physical bill/output");
                Valid(world);
            }
        }

        /// <summary>Root may strictly save/reload at each label; no serialization belongs in this shared suite.</summary>
        public static void RunHistoricalContinuity(Func<World, string, World> checkpoint = null)
        {
            var world = World.Create(41); var village = world.Villages[0];
            village.Research.Add("textiles");
            var station = new Building { Id = world.Id("historical-primary"), Kind = "clothier", Completed = true, Position = new Int2(1, 5) };
            village.Buildings.Add(station);
            string stationId = station.Id, workerId = village.Cats[1].Id;
            Act(world, new GameAction { Kind = "AssignWorker", BuildingId = stationId, CatId = workerId });
            var add = Edit(station, world.Cat(workerId), "add"); add.RecipeId = "fibre_to_thread"; add.Repeat = true; Act(world, add);
            station.Queue = new List<QueueEntry> { new QueueEntry { RecipeId = "fibre_to_cloth", AutomatedBy = "historical", Repeat = false } };
            void Boundary(string label)
            {
                if (checkpoint != null) world = checkpoint(world, label);
                Require(world != null, "historical checkpoint returned no world"); Valid(world);
                village = world.Villages[0]; station = village.Buildings.Single(b => b.Id == stationId);
            }
            void Divergent() => Require(station.Queue.Count == 1 && station.Queue[0].RecipeId == "fibre_to_cloth"
                && station.Queue[0].AutomatedBy == "historical" && !station.Queue[0].Repeat && station.Slots[0].Queue.Count == 1
                && station.Slots[0].Queue[0].RecipeId == "fibre_to_thread" && station.Slots[0].Queue[0].Repeat,
                "strict historical continuation repaired/merged divergent serialized queues before a relevant mutation");
            Boundary("historical: divergent input"); Divergent();
            var queue = station.Slots[0].Queue; var order = queue[0]; var mirror = station.Queue;
            var refusal = Edit(station, world.Cat(workerId), "move"); refusal.Direction = -1;
            Require(!Apply(world, refusal).Success && ReferenceEquals(station.Queue, mirror) && ReferenceEquals(station.Slots[0].Queue, queue)
                && ReferenceEquals(queue[0], order), "historical refusal changed list/order identity");
            Boundary("historical: refused edit"); Divergent();
            Act(world, new GameAction { Kind = "BoostCat", CatId = workerId, Enabled = false });
            Boundary("historical: unrelated action"); Divergent();
            // No production or officer work is funded. An unrelated Step must
            // retain the old mirror rather than perform an end-of-Step sweep.
            world.Step(World.SimulationStepSeconds);
            Boundary("historical: unrelated step"); Divergent();
            queue = station.Slots[0].Queue; order = queue[0];
            var pause = Edit(station, world.Cat(workerId), "pause"); pause.Enabled = true; Act(world, pause);
            Require(ReferenceEquals(station.Queue, queue) && ReferenceEquals(station.Slots[0].Queue[0], order) && order.Repeat
                && station.Queue.Count == 1 && station.Slots[0].Paused, "relevant primary edit failed to use only operative historical orders");
            Boundary("historical: relevant primary edit");
        }

        /// <summary>Two ordinary Farmer batches. A checkpoint may return only an exact world round trip.</summary>
        public static void RunChain(Func<World, string, World> checkpoint = null)
        {
            var world = World.Create(41); var village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine"); village.Stockpiles.Clear(); village.LastLeaderResearch = 0;
            var provisions = new Stockpile
            {
                Id = world.Id("queue-provisions"),
                Position = new Int2(3, 2),
                Width = 1,
                Depth = 1,
                Capacity = 1500,
                Accepts = new List<string> { "preserves", "water", "fibre" },
                Goods = new List<Stack> { new Stack("preserves", 600), new Stack("water", 600), new Stack("fibre", 10) }
            };
            var grain = new Stockpile
            {
                Id = world.Id("queue-grain"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Capacity = 100,
                Accepts = new List<string> { "grain" },
                Goods = new List<Stack> { new Stack("grain", 8.02) }
            };
            var flour = new Stockpile { Id = world.Id("queue-flour"), Position = new Int2(3, -3), Width = 1, Depth = 1, Capacity = 64, Accepts = new List<string> { "flour" } };
            village.Stockpiles.AddRange(new[] { provisions, grain, flour });
            string[] studies = { "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation" };
            village.ResearchPoints = studies.Sum(id => Catalog.Study(id).Cost);
            foreach (string study in studies) Act(world, new GameAction { Kind = "ResearchNode", NodeId = study });
            village.Buildings.Add(new Building { Id = world.Id("queue-field"), Kind = "field", Position = new Int2(6, 4), Completed = true });
            var mill = new Building { Id = world.Id("queue-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true }; village.Buildings.Add(mill);
            string villageId = village.Id, millId = mill.Id, workerId = village.Cats[3].Id, grainId = grain.Id, flourId = flour.Id;
            Village V() => world.Village(villageId);
            Building Mill() => V().Buildings.Single(b => b.Id == millId);
            Cat Worker() => world.Cat(workerId);
            WorkSlot Slot() => Mill().Slots.Single(s => s.CatId == workerId);
            Stockpile Pile(string id) => V().Stockpiles.Single(p => p.Id == id);
            Act(world, new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[1].Id });
            Act(world, new GameAction { Kind = "AssignWorker", BuildingId = millId, CatId = workerId });
            var pause = Edit(mill, Worker(), "pause"); pause.Enabled = true; Act(world, pause);
            Act(world, Edit(mill, Worker(), "delegate")); pause.Enabled = false; Act(world, pause); Act(world, Edit(mill, Worker(), "delegate"));
            Require(village.Cats.Count == 30 && village.Cats.All(c => c.Alive && c.ControlledBy == "" && c.Hunger == 100 && c.Thirst == 100 && c.Rest == 100)
                && village.Jobs.Count == 0 && world.Reservations.Count == 0 && Math.Abs(village.ResearchPoints) < 1e-8
                && Slot().AutomatedBy == "farmer" && !Slot().Paused && ReferenceEquals(mill.Queue, Slot().Queue), "natural public-action setup lost normal colony, exact budget or handback");
            foreach (var leg in new[] { (Worker().Position, grain.Position), (grain.Position, mill.Position), (mill.Position, flour.Position) })
                Require(world.Path(leg.Item1, leg.Item2, village) != null, "natural chain missing physical route");
            var recipe = Catalog.Recipe("grain_to_flour");
            double bill = world.RecipeInput(village, recipe, recipe.Inputs.Single().Amount), output = world.RecipeOutput(village, recipe, recipe.Outputs.Single().Amount);
            Require(bill == 4 && Math.Abs(output - 2.2) < 1e-8 && world.RecipeDuration(village, recipe) == 600, "natural chain changed normal researched recipe");
            int committed = 0; double grainLoss = 0, flourLoss = 0;
            var claims = new HashSet<string>(); var picked = new HashSet<string>(); var worked = new HashSet<string>(); var commits = new HashSet<string>(); var deliveries = new HashSet<string>();
            void Conserved()
            {
                Require(Math.Abs(Physical(V(), "grain") + committed * bill + grainLoss - 8.02) < 1e-7
                    && Math.Abs(Physical(V(), "flour") + flourLoss - committed * output) < 1e-7, "natural chain lost physical Grain/Flour conservation");
                Require(Slot().AutomatedBy == "farmer" && Slot().CatId == workerId && Worker().BuildingId == millId && !Slot().Paused
                    && V().Cats.All(c => c.Alive && c.ControlledBy == ""), "natural continuation changed officer/worker ownership, pause or ordinary cats");
            }
            void Boundary(string label)
            {
                Conserved(); Valid(world);
                if (checkpoint != null) world = checkpoint(world, label);
                Require(world != null, "checkpoint returned no world at " + label); Conserved(); Valid(world);
            }
            // Starting at second nine matches the recorded RED without a warmup.
            world.TimeSeconds = 9;
            Boundary("chain: public Resume and Farmer handback");
            bool appended = false, nextChoice = false;
            for (int tick = 0; tick < 36000 && deliveries.Count < 2; tick++)
            {
                world.Step(World.SimulationStepSeconds);
                bool minute = Math.Abs(world.TimeSeconds / 60 - Math.Round(world.TimeSeconds / 60)) < 1e-9;
                double Loss(string resource) => minute ? V().Stockpiles.Sum(p => Math.Max(0, World.Amount(p.Goods, resource)
                    - world.Reservations.Where(r => r.PileId == p.Id && r.Resource == resource).Sum(r => r.Amount)) * .0002 / (1 - .0002)) : 0;
                grainLoss += Loss("grain"); flourLoss += Loss("flour");
                var labels = new List<string>();
                if (!appended && Slot().Queue.Count == 1)
                {
                    Require(Slot().Queue[0].RecipeId == recipe.Id && Slot().Queue[0].AutomatedBy == "farmer" && !Slot().Queue[0].Repeat
                        && ReferenceEquals(Mill().Queue, Slot().Queue) && Worker().JobId == "", "first automatic append did not mirror the operative queue before work");
                    appended = true; labels.Add("chain: first automatic append");
                }
                foreach (var job in V().Jobs.Where(j => j.CatId == workerId && j.Kind == "production").ToArray())
                {
                    if (world.Reservations.Any(r => r.OwnerId == job.Id && r.Resource == "grain") && claims.Add(job.Id))
                        labels.Add("chain: batch " + claims.Count + " input claim");
                    if (World.Amount(Worker().Cargo, "grain") > 0 && picked.Add(job.Id))
                    {
                        Require(claims.Contains(job.Id) && At(Worker(), Pile(grainId).Position), "input pickup skipped claim or physical source arrival");
                        labels.Add("chain: batch " + picked.Count + " input pickup");
                    }
                    if (job.Phase == "working" && job.Progress > 0 && worked.Add(job.Id))
                    {
                        Require(picked.Contains(job.Id) && At(Worker(), Mill().Position) && World.Amount(job.Local, "grain") == bill,
                            "positive work skipped physical input delivery"); labels.Add("chain: batch " + worked.Count + " positive work");
                    }
                    if (job.Phase == "output_delivery" && commits.Add(job.Id))
                    {
                        Require(worked.Contains(job.Id) && World.Amount(job.Local, "flour") + World.Amount(Worker().Cargo, "flour") == output && Slot().Queue.Count == 0
                            && ReferenceEquals(Mill().Queue, Slot().Queue), "finite automatic output did not commit its primary head once");
                        committed++; labels.Add("chain: batch " + committed + " output commit");
                    }
                    if (job.Completed && deliveries.Add(job.Id))
                    {
                        Require(commits.Contains(job.Id) && Worker().Cargo.Count == 0 && job.Local.Count == 0 && At(Worker(), Pile(flourId).Position),
                            "completed batch skipped physical output delivery"); labels.Add("chain: batch " + deliveries.Count + " output delivery");
                    }
                }
                if (committed == 1 && !nextChoice && Slot().Queue.Count == 1)
                {
                    Require(Slot().Queue[0].RecipeId == recipe.Id && Slot().Queue[0].AutomatedBy == "farmer"
                        && ReferenceEquals(Mill().Queue, Slot().Queue), "next funded officer choice lost its primary mirror");
                    nextChoice = true; labels.Add("chain: next officer choice");
                }
                Conserved();
                foreach (string label in labels) Boundary(label);
            }
            Require(appended && claims.Count == 2 && nextChoice && picked.Count == 2 && worked.Count == 2 && committed == 2 && deliveries.Count == 2,
                "bounded natural chain failed two real input/work/output/delivery batches");
            // Cross several subsequent planning boundaries with no third bill.
            for (int tick = 0; tick < 620; tick++)
            {
                world.Step(World.SimulationStepSeconds);
                if (Math.Abs(world.TimeSeconds / 60 - Math.Round(world.TimeSeconds / 60)) < 1e-9)
                {
                    grainLoss += Math.Max(0, World.Amount(Pile(grainId).Goods, "grain")) * .0002 / (1 - .0002);
                    flourLoss += Math.Max(0, World.Amount(Pile(flourId).Goods, "flour")) * .0002 / (1 - .0002);
                }
            }
            Require(Worker().JobId == "" && Slot().Queue.Count == 0 && Mill().Queue.Count == 0 && world.Reservations.All(r => r.Resource != "grain")
                && Worker().Cargo.Count == 0 && Mill().Inputs.Count == 0 && Mill().Outputs.Count == 0
                && World.Amount(Pile(flourId).Goods, "flour") > 4.3 && World.Amount(Pile(grainId).Goods, "grain") < .02,
                "settled continuation duplicated a third bill or retained claims/cargo/output");
            Boundary("chain: settled continuation");
        }
    }
}
