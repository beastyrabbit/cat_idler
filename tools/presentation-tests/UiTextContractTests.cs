using System.Collections.Generic;
using IdleCatForest.Simulation;
using NUnit.Framework;

namespace IdleCatForest.Presentation.Tests
{
    /// <summary>
    /// The frozen UiText API (W1-05) returns authored copy. These tests use only that API, so they also
    /// ran red against the Wave 1 stubs.
    /// </summary>
    public sealed class UiTextContractTests
    {
        /// <summary>The no-break space between a number and its unit (Numbers.UnitSpace).</summary>
        private const string U = " ";

        [Test]
        public void ClockStartsOnDayOneAtSixInTheMorning()
        {
            Assert.That(UiText.Clock(0, 60), Is.EqualTo("Day 1 · 06:00"));
            Assert.That(UiText.Clock(60, 60), Is.EqualTo("Day 1 · 07:00"));
            Assert.That(UiText.Clock(18 * 60, 60), Is.EqualTo("Day 2 · 00:00"));
            Assert.That(UiText.Clock(18 * 3600, 1), Is.EqualTo("Day 2 · 00:00"));
            Assert.That(UiText.Clock(21, 60), Is.EqualTo("Day 1 · 06:21"));
        }

        [Test]
        public void NumbersIgnoreTheMachineLocale()
        {
            Assert.That(UiText.Num(0.4, 1), Is.EqualTo("0.4"));
            Assert.That(UiText.Num(24.0, 1), Is.EqualTo("24"));
            Assert.That(UiText.Whole(19995), Is.EqualTo("19,995"));
            Assert.That(UiText.Whole(4.6), Is.EqualTo("5"));
        }

        [Test]
        public void PluralsAgreeWithTheCount()
        {
            Assert.That(UiText.Plural(1, "study", "studies"), Is.EqualTo("1 study"));
            Assert.That(UiText.Plural(2, "study", "studies"), Is.EqualTo("2 studies"));
            Assert.That(UiText.Plural(0, "recipe", "recipes"), Is.EqualTo("0 recipes"));
        }

        [Test]
        public void AgesAreShortDurations()
        {
            Assert.That(UiText.Age(30), Is.EqualTo("30" + U + "s"));
            Assert.That(UiText.Age(12 * 60), Is.EqualTo("12" + U + "min"));
            Assert.That(UiText.Age(6 * 3600), Is.EqualTo("6" + U + "h"));
        }

        [Test]
        public void NamesComeFromTheCatalog()
        {
            Assert.That(UiText.Name(NameKind.Building, "research_hut"), Is.EqualTo("Research Hut"));
            Assert.That(UiText.Name(NameKind.Building, "food_storage"), Is.EqualTo("Food Store"));
            Assert.That(UiText.Name(NameKind.Recipe, "herbal_tonic"), Is.EqualTo("Herbal Tonic"));
            Assert.That(UiText.Name(NameKind.Study, "basic_tools"), Is.EqualTo(Catalog.Study("basic_tools").Name));
            Assert.That(UiText.Name(NameKind.Resource, "logs"), Is.EqualTo("Logs"));
            Assert.That(UiText.Name(NameKind.Role, "steward"), Is.EqualTo("Steward"));
            Assert.That(UiText.Name(NameKind.Upgrade, "click_power"), Does.Not.Contain("click"));
        }

        [Test]
        public void PrettyNeverShowsSnakeCaseOrEntityIds()
        {
            Assert.That(UiText.Pretty("woodworking"), Is.EqualTo("Woodworker"));
            Assert.That(UiText.Pretty("planks_and_blocks_to_tools"), Does.Not.Contain("_"));
            Assert.That(UiText.Pretty("hunt · working"), Is.EqualTo("Hunting"));
            Assert.That(UiText.Pretty("station_store-18"), Is.EqualTo("Station store"));
        }

        [Test]
        public void EffectsAreSentences()
        {
            Assert.That(UiText.Effect("modify", "huntYieldMult", 0.1), Is.EqualTo("+10" + U + "% hunt yield"));
            Assert.That(UiText.Effect("modify", "spoilageResistance", 0.16), Is.EqualTo("+16" + U + "% spoilage resistance"));
            Assert.That(UiText.Effect("unlock_building", "clothier", 0), Is.EqualTo("Unlocks the Clothier"));
        }

        [Test]
        public void ConfirmationsReuseTheVerb()
        {
            Assert.That(UiText.Confirm("PlanBuilding"), Is.EqualTo("Planned"));
            Assert.That(UiText.Confirm("ResearchNode"), Is.EqualTo("Studied"));
            Assert.That(UiText.Confirm("MoveCat"), Is.EqualTo(""), "Steering a cat confirms with nothing.");
            Assert.That(UiText.Confirm("KeepCatControl"), Is.EqualTo(""));
        }

        [Test]
        public void ReasonsAreSentencesWithANextStep()
        {
            Assert.That(UiText.Reason("worker_required"), Is.EqualTo("Nobody works here yet. Assign a free cat."));
            Assert.That(UiText.HumanReason("blocked_route"), Is.EqualTo(UiText.Reason("route_blocked")));
            Assert.That(UiText.HumanReason("blocked_route"), Does.Not.Contain("_").And.Not.EqualTo(UiText.Pretty("blocked_route")));
        }

        [Test]
        public void ActionErrorsSpeakInTheColonysVoice()
        {
            Assert.That(UiText.ActionError(ActionResult.Fail("Insufficient research points")),
                Is.EqualTo("Not enough research yet. Cats study at the Research Hut."));
            Assert.That(UiText.ActionError(ActionResult.Fail("Unknown action: SetTestAcceleration")), Is.EqualTo("That can't be done here."));
            var shortfall = ActionResult.Fail(ResultCode.MissingMaterial, "Insufficient research points",
                ("resource", "research"), ("need", "5"), ("have", "0.4"), ("eta", "240"), ("workers", "1"));
            Assert.That(UiText.ActionError(shortfall), Is.EqualTo("Needs 5 research · you have 0.4 · about 4" + U + "min with 1 scholar"));
            Assert.That(UiText.ActionError(ActionResult.Ok()), Is.EqualTo(""));
        }

        [Test]
        public void EventLinesTellTheColonysStory()
        {
            Assert.That(UiText.EventLine(new Event { Kind = EventKind.Completed, Text = "Fennel completed water" }), Is.EqualTo("Fennel brought back water"));
            Assert.That(UiText.EventLine(new Event { Kind = EventKind.Upgrade, Text = "click_power reached level 2" }), Is.EqualTo("Helping paws reached level 2"));
            Assert.That(UiText.EventLine(new Event { Kind = EventKind.Raid, Text = "Raiders approach the south gate" }), Is.EqualTo("Raiders spotted to the south!"));
        }

        [Test]
        public void GoodsListAmountsWithNouns()
        {
            Assert.That(UiText.Goods(new List<Stack> { new Stack("logs", 3), new Stack("food", 5) }), Is.EqualTo("3 logs, 5 food"));
            Assert.That(UiText.Goods(new List<Stack> { new Stack("logs", 1) }), Is.EqualTo("1 log"));
            Assert.That(UiText.Goods(new List<Stack>()), Is.EqualTo("nothing"));
        }

        [Test]
        public void NowLineNamesTheSteeredCat()
        {
            var village = new Village { Id = "v", Name = "Mosslight" };
            var hazel = new Cat { Id = "cat-1", Name = "Hazel", ControlledBy = "player", Goal = GoalKey.PlayerControl };
            village.Cats.Add(hazel);
            Assert.That(UiText.NowLine(new World(), village, hazel), Is.EqualTo("You're steering Hazel"));
        }
    }
}
