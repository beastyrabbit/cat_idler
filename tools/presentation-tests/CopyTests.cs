using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using NUnit.Framework;

namespace IdleCatForest.Presentation.Tests
{
    public sealed class NameTests
    {
        [Test]
        public void DuplicateBuildingsAreNumberedInListOrder()
        {
            var village = new Village { Id = "v" };
            var hut = new Building { Id = "research_hut-4", Kind = "research_hut" };
            village.Buildings.AddRange(new[]
            {
                new Building { Id = "den-1", Kind = "den" }, hut, new Building { Id = "den-2", Kind = "den" }, new Building { Id = "den-3", Kind = "den" },
            });
            Assert.That(Names.Building(village, village.Buildings[3]), Is.EqualTo("Den 3"));
            Assert.That(Names.Building(village, "den-1"), Is.EqualTo("Den 1"));
            Assert.That(UiText.Name(village, hut), Is.EqualTo("Research Hut"), "A single building of its kind is not numbered.");
            Assert.That(Names.Building(village, "den-99"), Is.EqualTo(""), "A missing building has no name, never an id.");
        }

        [Test]
        public void PilesAreNamedByWhatTheyBelongTo()
        {
            var village = new Village { Id = "v" };
            var store = new Building { Id = "food_storage-5", Kind = "food_storage" };
            var field = new Farm { Id = "farm-6", Crop = "grain" };
            village.Buildings.Add(store);
            village.Farms.Add(field);
            var container = new Stockpile { Id = "container-7", ManagedBy = store.Id };
            var harvest = new Stockpile { Id = "harvest-8", ManagedBy = field.Id, Kind = "farm_output" };
            var logs = new Stockpile { Id = "pile-9", Accepts = new List<string> { "logs" } };
            var first = new Stockpile { Id = "store-10" };
            var second = new Stockpile { Id = "pile-11" };
            var spill = new Stockpile { Id = "spill-12", Kind = "spill" };
            village.Stockpiles.AddRange(new[] { container, harvest, logs, first, second, spill });

            Assert.That(Names.Pile(village, container), Is.EqualTo("Food Store"));
            Assert.That(Names.Pile(village, harvest), Is.EqualTo("Grain field"));
            Assert.That(Names.Pile(village, logs), Is.EqualTo("Log pile"));
            Assert.That(Names.Pile(village, first), Is.EqualTo("Stockpile 1"));
            Assert.That(Names.Pile(village, second), Is.EqualTo("Stockpile 2"));
            Assert.That(Names.Pile(village, spill), Is.EqualTo("Dropped goods"));
            Assert.That(Names.Pile(village, "store-404"), Is.EqualTo(""));
        }

        [Test]
        public void ResourcesReadAsLabelsAndInRunningText()
        {
            Assert.That(Names.ResourceLabel("blocks"), Is.EqualTo("Stone blocks"));
            Assert.That(Names.Resource("blocks"), Is.EqualTo("stone blocks"));
            Assert.That(Names.Amount(3, "logs"), Is.EqualTo("3 logs"));
            Assert.That(Names.Amount(1, "planks"), Is.EqualTo("1 plank"));
            Assert.That(Names.Amount(5, "food"), Is.EqualTo("5 food"));
            Assert.That(Names.Amount(0.5, "water"), Is.EqualTo("0.5 water"));
        }

        [Test]
        public void OfficersAndStudiesAreProperNouns()
        {
            var village = new Village { Id = "v", LeaderId = "cat-1" };
            var hazel = new Cat { Id = "cat-2", Name = "Hazel", OfficerRole = "steward" };
            Assert.That(Names.Titled(village, hazel), Is.EqualTo("Steward Hazel"));
            Assert.That(Names.Titled(village, new Cat { Id = "cat-1", Name = "Pip" }), Is.EqualTo("Leader Pip"));
            Assert.That(Names.Studies(new[] { "sawmill", "sawmill" }), Is.EqualTo(Catalog.Study("sawmill").Name), "Requirements never repeat a name.");
            Assert.That(Names.Item("tool", "wood"), Is.EqualTo("Wooden tool"));
            Assert.That(UiText.Name(NameKind.Category, "recipe_resource"), Is.EqualTo("Crafts"));
        }

        [Test]
        public void UnknownIdsBecomeWordsWithoutNumbers()
        {
            Assert.That(Names.Humanize("expedition_storage_required"), Is.EqualTo("Expedition storage required"));
            Assert.That(Names.Humanize("spoilageResistance"), Is.EqualTo("Spoilage resistance"));
            Assert.That(Names.Humanize("cat-25"), Is.EqualTo("Cat"));
            Assert.That(Names.Humanize("42"), Is.EqualTo(""));
        }
    }

    public sealed class NumberAndClockTests
    {
        private const string U = Numbers.UnitSpace;

        [Test]
        public void NumbersFormatWithoutTheLocale()
        {
            Assert.That(Numbers.Num(1234567.891, 2), Is.EqualTo("1,234,567.89"));
            Assert.That(Numbers.Num(-0.04, 1), Is.EqualTo("0"));
            Assert.That(Numbers.Floor(4.6), Is.EqualTo("4"), "A cost comparison never rounds up.");
            Assert.That(Numbers.Floor(0.46, 1), Is.EqualTo("0.4"));
            Assert.That(Numbers.Amount(2.25), Is.EqualTo("2.3"));
            Assert.That(Numbers.Amount(12.4), Is.EqualTo("12"));
            Assert.That(Numbers.Signed(-2), Is.EqualTo("−2"));
            Assert.That(Numbers.SignedPercent(0.125, 1), Is.EqualTo("+12.5" + U + "%"));
            Assert.That(Numbers.Num(double.NaN), Is.EqualTo("—"));
            Assert.That(Numbers.Tiles(1), Is.EqualTo("1" + U + "tile"));
            Assert.That(Numbers.Ordinal(22), Is.EqualTo("22nd"));
        }

        [Test]
        public void ClockAndDurations()
        {
            Assert.That(GameClock.TimeOfDay(0, 60), Is.EqualTo("06:00"));
            Assert.That(GameClock.Clock(-5, 60), Is.EqualTo("Day 1 · 06:00"), "Never before the founding.");
            Assert.That(GameClock.Clock(25 * 60 * 2, 60), Is.EqualTo("Day 3 · 08:00"));
            Assert.That(GameClock.Stamp(60, 120, 60), Is.EqualTo("07:00"));
            Assert.That(GameClock.Stamp(60, 24 * 60, 60), Is.EqualTo("Day 1 · 07:00"));
            Assert.That(GameClock.Duration(0.2), Is.EqualTo("1" + U + "s"));
            Assert.That(GameClock.Duration(90 * 60), Is.EqualTo("1" + U + "h 30" + U + "min"));
            Assert.That(GameClock.Duration(3 * 86400), Is.EqualTo("3 days"));
            Assert.That(GameClock.Ago(3), Is.EqualTo("just now"));
            Assert.That(GameClock.Ago(720), Is.EqualTo("12" + U + "min ago"));
        }
    }

    public sealed class NowLineTests
    {
        private World world;
        private Village village;
        private Cat hazel;

        [SetUp]
        public void Colony()
        {
            world = new World();
            village = new Village { Id = "v", Name = "Mosslight", LeaderId = "cat-9" };
            world.Villages.Add(village);
            hazel = new Cat { Id = "cat-1", Name = "Hazel" };
            village.Cats.Add(hazel);
        }

        private static List<Int2> Path(int tiles) => Enumerable.Range(1, tiles).Select(i => new Int2(i, 0)).ToList();

        [Test]
        public void CarryingWaterNamesTheStoreAndDistance()
        {
            var store = new Building { Id = "food_storage-2", Kind = "food_storage" };
            village.Buildings.Add(store);
            village.Stockpiles.Add(new Stockpile { Id = "container-3", ManagedBy = store.Id });
            village.Jobs.Add(new Job { Id = "job-55", Kind = JobKind.Water, Phase = JobPhase.OutputDelivery, DeliveryPileId = "container-3", Resource = "water" });
            hazel.JobId = "job-55";
            hazel.Goal = JobKind.Water + GoalKey.Separator + JobPhase.OutputDelivery;
            hazel.Cargo.Add(new Stack("water", 3));
            hazel.Path = Path(9);
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Bringing water to the Food Store · 9" + Numbers.UnitSpace + "tiles"));
        }

        [Test]
        public void CarryingPiecesCountsThem()
        {
            var sawmill = new Building { Id = "sawmill-2", Kind = "sawmill" };
            village.Buildings.Add(sawmill);
            village.Jobs.Add(new Job { Id = "job-5", Kind = JobKind.Haul, Phase = JobPhase.OutputDelivery, TargetId = "sawmill-2", Resource = "logs" });
            hazel.JobId = "job-5";
            hazel.Cargo.Add(new Stack("logs", 3));
            hazel.Path = Path(31);
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Bringing 3 logs to the Sawmill · 31" + Numbers.UnitSpace + "tiles"));
        }

        [Test]
        public void NappingNamesTheDen()
        {
            village.Buildings.Add(new Building { Id = "den-1", Kind = "den" });
            village.Buildings.Add(new Building { Id = "den-2", Kind = "den" });
            hazel.BedId = "den-2";
            hazel.Goal = GoalKey.IdleNap;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Napping in Den 2"));
        }

        [Test]
        public void SteeringAndSleeping()
        {
            hazel.ControlledBy = "player";
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("You're steering Hazel"));
            hazel.ControlledBy = "";
            village.Buildings.Add(new Building { Id = "den-1", Kind = "den", Completed = true });
            hazel.BedId = "den-1";
            hazel.Goal = GoalKey.NeedSleep;
            hazel.Rest = 81.5; // 13.5 rest at 0.15 per second: 90 s of sleep left on the classic clock
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Sleeping · Den · wakes at 06:01"));
        }

        [Test]
        public void WorkingNamesTheRecipeAndStation()
        {
            var workshop = new Building { Id = "workshop-2", Kind = "workshop" };
            village.Buildings.Add(workshop);
            village.Jobs.Add(new Job { Id = "job-7", Kind = JobKind.Production, Phase = JobPhase.Working, TargetId = workshop.Id, RecipeId = "herbal_tonic" });
            hazel.JobId = "job-7";
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Preparing Herbal Tonic at the Workshop"));
        }

        [Test]
        public void WaitingSaysWhyAndCauseLineNamesTheOwner()
        {
            var sawmill = new Building { Id = "sawmill-2", Kind = "sawmill" };
            village.Buildings.Add(sawmill);
            var job = new Job { Id = "job-7", Kind = JobKind.Production, Phase = JobPhase.Working, TargetId = sawmill.Id, RecipeId = "materials_to_refined", BlockedReason = Blocked.MissingRecipeInput };
            village.Jobs.Add(job);
            hazel.JobId = job.Id;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Waiting at the Sawmill — ingredients are missing"));
            Assert.That(UiText.CauseLine(world, village, hazel), Is.EqualTo("Because ingredients are missing"));

            job.BlockedReason = "";
            job.AutomatedBy = "steward";
            village.Officers.Add(new Officer { Role = "steward", CatId = "cat-3" });
            village.Cats.Add(new Cat { Id = "cat-3", Name = "Pip" });
            Assert.That(UiText.CauseLine(world, village, hazel), Is.EqualTo("Because Steward Pip ordered a workshop shift"));
            job.AutomatedBy = "";
            job.Manual = true;
            Assert.That(UiText.CauseLine(world, village, hazel), Is.EqualTo("Because you asked for a workshop shift"));
        }

        [Test]
        public void StationWorkersBetweenJobsSayWhatTheyWaitFor()
        {
            var sawmill = new Building { Id = "sawmill-2", Kind = "sawmill", BlockedReason = "queue_empty" };
            sawmill.Slots.Add(new WorkSlot { CatId = hazel.Id });
            village.Buildings.Add(sawmill);
            hazel.BuildingId = sawmill.Id;
            hazel.Goal = GoalKey.Idle;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Waiting at the Sawmill — nothing is queued"));
            sawmill.Slots[0].BlockedReason = Blocked.MissingRecipeInput;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Waiting at the Sawmill — ingredients are missing"));
        }

        [Test]
        public void IdleCatsSayWhy()
        {
            hazel.Goal = GoalKey.Idle;
            hazel.AgeHours = 30;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Idle — waiting for a job"));
            hazel.AgeHours = 2;
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Idle — too young to work yet"));
            hazel.Path = Path(4);
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Wandering · 4" + Numbers.UnitSpace + "tiles"));
        }

        [Test]
        public void TheWorstCaseStillReadsWell()
        {
            hazel.Goal = "accounting · review-store-56";
            Assert.That(UiText.NowLine(world, village, hazel), Is.EqualTo("Counting the stores"));
            hazel.Goal = "mystery_goal · strange_phase";
            TextAssert.Clean(UiText.NowLine(world, village, hazel));
            hazel.Goal = "";
            TextAssert.Clean(UiText.NowLine(world, village, hazel));
            Assert.That(UiText.NowLine(world, village, null), Is.EqualTo(""));
        }
    }

    public sealed class ReasonAndEffectTests
    {
        [Test]
        public void BothRouteCodesReadTheSame()
        {
            Assert.That(UiText.Reason(Blocked.BlockedRoute), Is.EqualTo(UiText.Reason(Blocked.RouteBlocked)));
            Assert.That(UiText.Cause(Blocked.BlockedRoute), Is.EqualTo("there is no path"));
        }

        [Test]
        public void ReasonsWithDetailsAndUnknownCodesStayReadable()
        {
            TextAssert.Clean(UiText.Reason("driver_required · build bridge access to salvage wreck"));
            Assert.That(UiText.Reason("driver_required · build bridge access to salvage wreck"), Does.StartWith("This route needs a driver"));
            Assert.That(UiText.Reason("some_new_code"), Is.EqualTo("Some new code."));
            Assert.That(UiText.Reason(""), Is.EqualTo(""));
            Assert.That(UiText.Reason("resident"), Is.EqualTo(""), "A migration state is not a reason.");
        }

        [Test]
        public void ShortfallsGiveNumbersAndNextSteps()
        {
            var args = new Dictionary<string, string> { ["resource"] = "blessings", ["need"] = "5", ["have"] = "2.75" };
            Assert.That(UiText.Reason(ResultCode.MissingMaterial, args), Is.EqualTo("Needs 5 blessings · you have 2.7. Offer food at the Shrine to earn more."));
            args = new Dictionary<string, string> { ["resource"] = "planks", ["need"] = "1", ["have"] = "0" };
            Assert.That(UiText.Reason(ResultCode.MissingMaterial, args), Is.EqualTo("Needs 1 plank · you have 0"));
        }

        [Test]
        public void EngineMessagesNeverLeakIds()
        {
            Assert.That(UiText.ActionError(ActionResult.Fail("No known reachable food; scout first")), Is.EqualTo("Nobody knows where to find food yet. Send a scout first."));
            TextAssert.Clean(UiText.ActionError(ActionResult.Fail("Remove zone_gather -3,4 from store-17")));
            Assert.That(Reasons.Clean("Remove zone_gather -3,4 from store-17"), Is.EqualTo("Remove zone gather from"));
            Assert.That(UiText.ActionError(ActionResult.Fail(ResultCode.FullGameOnly, "Deepstone Vault is outside the demo")), Is.EqualTo("That comes with the full game."));
        }

        [Test]
        public void ConfirmationsNameTheirSubject()
        {
            var village = new Village { Id = "v" };
            village.Cats.Add(new Cat { Id = "cat-1", Name = "Hazel" });
            Assert.That(UiText.Confirm(new GameAction { Kind = "PlanBuilding", Name = "den" }), Is.EqualTo("Den planned"));
            Assert.That(UiText.Confirm(new GameAction { Kind = "ResearchNode", NodeId = "basic_tools" }), Is.EqualTo("Studied " + Catalog.Study("basic_tools").Name));
            Assert.That(UiText.Confirm(new GameAction { Kind = "EnterCatControl", CatId = "cat-1" }, village), Is.EqualTo("Hazel is yours · Tab to hand back"));
            Assert.That(UiText.Confirm(new GameAction { Kind = "AssignOfficer", CatId = "cat-1", Role = "steward" }, village), Is.EqualTo("Hazel is now Steward"));
            Assert.That(UiText.Confirm(new GameAction { Kind = "MoveCat", CatId = "cat-1" }, village), Is.EqualTo(""));
        }

        [Test]
        public void BuildingAndUnlockEffects()
        {
            string u = Numbers.UnitSpace;
            Assert.That(UiText.Effect(new ResearchPayload { Kind = "modify_building", Id = "sawmill", Attribute = "cycle_time", Operation = "multiply", Value = 0.9 }), Is.EqualTo("Sawmill works 10" + u + "% faster"));
            Assert.That(UiText.Effect(new ResearchPayload { Kind = "modify_building", Id = "mill", Attribute = "worker_slots", Operation = "add", Value = 1 }), Is.EqualTo("+1 worker at the Mill"));
            Assert.That(UiText.Effect(new ResearchPayload { Kind = "modify", Id = "housingPerDen", Operation = "add", Value = 2 }), Is.EqualTo("+2 beds per Den"));
            Assert.That(UiText.Effect("unlock_recipe", "herbal_tonic", 0), Is.EqualTo("Unlocks Herbal Tonic at the Workshop"));
            Assert.That(UiText.Effect("unlock_resource", "hunting_reserves", 0), Is.EqualTo("More storage for hunting goods"));
            Assert.That(UiText.Effect("unlock_capability", "mountain_travel", 0), Is.EqualTo("Opens mountain paths"));
            Assert.That(Effects.Of(Catalog.Study("basic_tools")), Is.Not.Empty);
        }
    }

    public sealed class EventLineTests
    {
        private static Event E(string kind, string text, double time, string entity = "", long seq = 0) =>
            new Event { Kind = kind, Text = text, Time = time, EntityId = entity, Seq = seq };

        [Test]
        public void KindsGetTheirOwnTemplates()
        {
            var village = new Village { Id = "v" };
            village.Cats.Add(new Cat { Id = "cat-4", Name = "Wren" });
            village.Buildings.Add(new Building { Id = "den-1", Kind = "den" });
            Assert.That(UiText.EventLine(E(EventKind.Death, "Birch died from needs", 0)), Is.EqualTo("Birch has died. Cats need food, water and rest to stay well."));
            Assert.That(UiText.EventLine(E(EventKind.Emergency, "Fennel preempted work for physical water", 0)), Is.EqualTo("Fennel dropped their work to fetch water"));
            Assert.That(UiText.EventLine(village, E(EventKind.Discovery, "289 notes delivered to shrine", 0, "cat-4")), Is.EqualTo("Wren mapped 289 new places"));
            Assert.That(UiText.EventLine(E(EventKind.Migration, "Thistle approaches the gate", 0)), Is.EqualTo("Thistle is at the gate, hoping to join"));
            Assert.That(UiText.EventLine(village, E(EventKind.BuildingCompleted, "", 0, "den-1")), Is.EqualTo("The Den is finished"));
            Assert.That(UiText.EventLine(E(EventKind.Research, "Textiles", 0, "textiles")), Does.StartWith("Studied Textiles"));
            Assert.That(UiText.EventLine(E(EventKind.Dungeon, "Wren defeated a cave beetle", 0)), Is.EqualTo("Wren defeated a cave beetle"));
        }

        [Test]
        public void RoutineEventsWithinAMinuteCollapse()
        {
            var events = new List<Event>
            {
                E(EventKind.Completed, "Fennel completed water", 1, seq: 1),
                E(EventKind.Completed, "Birch completed hunt", 2, seq: 2),
                E(EventKind.Completed, "Fennel completed water", 10, seq: 3),
                E(EventKind.Completed, "Moss completed water", 20, seq: 4),
                E(EventKind.Birth, "Pebble was born", 30, seq: 5),
                E(EventKind.Completed, "Ash completed hunt", 31, seq: 6),
                E(EventKind.Completed, "Ash completed hunt", 200, seq: 7),
            };
            var groups = EventLines.Collapse(events);
            Assert.That(groups.Select(g => g.Line), Is.EqualTo(new[]
            {
                "Fennel, Birch and Moss finished 4 jobs · 3 water runs, 1 hunt",
                "Pebble was born!",
                "Ash came back from a hunt",
                "Ash came back from a hunt",
            }));
            Assert.That(groups[0].Count, Is.EqualTo(4));
            Assert.That(groups[0].LastSeq, Is.EqualTo(4));
            Assert.That(groups[0].FirstTime, Is.EqualTo(1));
            Assert.That(groups[0].LastTime, Is.EqualTo(20));
        }

        [Test]
        public void AScoutsReturnIsOneLine()
        {
            var village = new Village { Id = "v" };
            village.Cats.Add(new Cat { Id = "cat-4", Name = "Hazel" });
            var events = new List<Event>
            {
                E(EventKind.Discovery, "169 notes delivered to shrine", 50, "cat-4", 1),
                E(EventKind.Completed, "Hazel completed scout", 50, "job-9", 2),
            };
            Assert.That(EventLines.Collapse(events, village).Select(g => g.Line), Is.EqualTo(new[] { "Hazel mapped 169 new places" }));
        }

        [Test]
        public void IdenticalNotableLinesFoldWithACount()
        {
            var events = Enumerable.Range(0, 3).Select(i => E(EventKind.Expansion, "Restored wall project claimed one tile", i, "4,5")).ToList();
            var groups = EventLines.Collapse(events);
            Assert.That(groups, Has.Count.EqualTo(1));
            Assert.That(groups[0].Line, Is.EqualTo("The fence repair took in more ground (×3)"));
            Assert.That(EventLines.Collapse(null), Is.Empty);
        }
    }
}
