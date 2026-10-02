using System.Linq;
using IdleCatForest.Simulation;
using NUnit.Framework;

namespace IdleCatForest.Presentation.Tests
{
    /// <summary>
    /// A real founded colony, stepped for a few minutes in both pacing profiles: every line the card,
    /// census and log would show is readable player text.
    /// </summary>
    public sealed class WorldTextTests
    {
        [TestCase("classic")]
        [TestCase("demo")]
        public void AFoundedColonyReadsCleanly(string profile)
        {
            var world = World.Create(7);
            world.PacingProfile = profile;
            var village = world.Villages[0];
            int checkedLines = 0;
            for (int minute = 0; minute < 4; minute++)
            {
                world.Step(60);
                foreach (var cat in village.Cats)
                {
                    TextAssert.Clean(UiText.NowLine(world, village, cat), cat.Goal + " / " + cat.JobId);
                    TextAssert.CleanOrEmpty(UiText.CauseLine(world, village, cat), cat.BlockedReason);
                    TextAssert.Clean(UiText.Pretty(cat.Goal), cat.Goal);
                    TextAssert.Clean(UiText.Skills(cat.Skills));
                    TextAssert.Clean(UiText.Goods(cat.Cargo));
                    checkedLines++;
                }
                foreach (var building in village.Buildings)
                    TextAssert.Clean(UiText.Name(village, building), building.Kind);
                foreach (var pile in village.Stockpiles)
                    TextAssert.Clean(UiText.Name(village, pile), pile.Kind);
                foreach (var e in village.Events)
                {
                    TextAssert.Clean(UiText.EventLine(village, e), e.Kind + ": " + e.Text);
                    TextAssert.Clean(UiText.EventStamp(world, e));
                }
                foreach (var group in EventLines.Collapse(village.Events, village))
                    TextAssert.Clean(group.Line, group.Kind);
                foreach (string resource in new[] { "food", "water", "logs" })
                {
                    TextAssert.Clean(UiText.Reported(world, village, resource, true), resource);
                    TextAssert.Clean(UiText.Reported(world, village, resource), resource);
                }
            }
            TextAssert.Clean(UiText.Clock(world.TimeSeconds, world.CurrentPacing().DayScale));
            // The copy itself, for review in the test log (dotnet test -v n or --logger "console;verbosity=detailed").
            TestContext.Out.WriteLine(UiText.Clock(world.TimeSeconds, world.CurrentPacing().DayScale));
            foreach (var cat in village.Cats.Take(30))
                TestContext.Out.WriteLine(cat.Name + ": " + UiText.NowLine(world, village, cat) + " | " + UiText.CauseLine(world, village, cat));
            foreach (var group in EventLines.Collapse(village.Events, village).Reverse().Take(12))
                TestContext.Out.WriteLine(UiText.EventStamp(world, group.Events[0]) + "  " + group.Line);
            Assert.That(checkedLines, Is.GreaterThan(100));
            Assert.That(village.Cats.Select(c => UiText.NowLine(world, village, c)).Distinct().Count(), Is.GreaterThan(1), "The colony shows more than one activity.");
        }
    }
}
