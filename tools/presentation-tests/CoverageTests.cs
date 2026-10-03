using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using IdleCatForest.Simulation;
using NUnit.Framework;

namespace IdleCatForest.Presentation.Tests
{
    /// <summary>
    /// Every code the simulation can hand the UI has authored text. Reflection over the Ids classes, so a
    /// code added later (by Wave 2 packages or after) fails here until its copy is written. W2-INT-b runs
    /// this category after merging Wave 2.
    /// </summary>
    [Category("Coverage")]
    public sealed class CoverageTests
    {
        private static IEnumerable<(string name, string value)> Constants(Type type) =>
            type.GetFields(BindingFlags.Public | BindingFlags.Static)
                .Where(f => f.IsLiteral && f.FieldType == typeof(string))
                .Select(f => (f.Name, (string)f.GetRawConstantValue()));

        private static IEnumerable<TestCaseData> Cases(Type type) =>
            Constants(type).Select(c => new TestCaseData(c.value).SetName(type.Name + "." + c.name));

        public static IEnumerable<TestCaseData> BlockedCodes() => Cases(typeof(Blocked));
        public static IEnumerable<TestCaseData> ResultCodes() => Cases(typeof(ResultCode));
        public static IEnumerable<TestCaseData> EventKinds() => Cases(typeof(EventKind));
        public static IEnumerable<TestCaseData> GoalKeys() => Cases(typeof(GoalKey));
        public static IEnumerable<TestCaseData> JobKinds() => Cases(typeof(JobKind));
        public static IEnumerable<TestCaseData> JobPhases() => Cases(typeof(JobPhase));

        [Test]
        public void TheFrozenListsAreAllPresent()
        {
            // docs/DEMO.md 3.8: members may be added, never removed.
            Assert.That(Constants(typeof(Blocked)).Count(), Is.GreaterThanOrEqualTo(19));
            Assert.That(Constants(typeof(ResultCode)).Count(), Is.GreaterThanOrEqualTo(16));
            Assert.That(Constants(typeof(EventKind)).Count(), Is.GreaterThanOrEqualTo(31));
            Assert.That(Constants(typeof(GoalKey)).Count(), Is.GreaterThanOrEqualTo(33));
        }

        [TestCaseSource(nameof(BlockedCodes))]
        public void EveryBlockedReasonHasASentenceAndACause(string code)
        {
            Assert.That(Reasons.IsAuthored(code), Is.True, "No authored reason for " + code);
            TextAssert.Clean(UiText.Reason(code), code);
            TextAssert.Clean(UiText.HumanReason(code), code);
            TextAssert.Clean(UiText.Cause(code), code);
            Assert.That(UiText.Reason(code), Does.EndWith("."), "A reason is a full sentence: " + code);
            Assert.That(UiText.Reason(code), Is.Not.EqualTo(Names.Humanize(code) + "."), "Fallback text for " + code);
        }

        [TestCaseSource(nameof(ResultCodes))]
        public void EveryResultCodeHasText(string code)
        {
            Assert.That(Reasons.IsAuthoredResult(code), Is.True, "No authored text for result " + code);
            TextAssert.Clean(UiText.Result(code), code);
            if (!Reasons.IsSuccess(code))
            {
                TextAssert.Clean(UiText.Reason(code), code);
                TextAssert.Clean(UiText.ActionError(ActionResult.Fail(code, "engine message " + code)), code);
            }
        }

        [TestCaseSource(nameof(EventKinds))]
        public void EveryEventKindHasALine(string kind)
        {
            Assert.That(EventLines.IsAuthored(kind), Is.True, "No authored line for event kind " + kind);
            // An event with no usable text still reads well.
            TextAssert.Clean(UiText.EventLine(new Event { Kind = kind, Text = "", EntityId = "cat-25" }), kind);
            // An event whose text leaks ids and keys is cleaned.
            TextAssert.Clean(UiText.EventLine(new Event { Kind = kind, Text = "store-17 reached some_state at -8,-8", EntityId = "store-17" }), kind);
        }

        [TestCaseSource(nameof(GoalKeys))]
        public void EveryGoalKeyHasANowLine(string goal)
        {
            // The separator only appears between a job kind and its phase.
            string composed = goal;
            if (goal == GoalKey.Separator)
                composed = JobKind.Hunt + GoalKey.Separator + JobPhase.Working;
            // A prefix is followed by a detail such as a farm phase or a pile id.
            else if (goal.EndsWith(" ", StringComparison.Ordinal) || goal.EndsWith("_", StringComparison.Ordinal))
                composed = goal + "store-17";
            Assert.That(NowLines.IsAuthoredGoal(composed), Is.True, "No authored goal text for " + goal);
            TextAssert.Clean(NowLines.GoalText(composed), goal);
            TextAssert.Clean(UiText.Pretty(composed), goal);

            var village = new Village { Id = "v", Name = "Mosslight" };
            var cat = new Cat { Id = "cat-1", Name = "Hazel", Goal = composed };
            village.Cats.Add(cat);
            TextAssert.Clean(UiText.NowLine(new World(), village, cat), goal);
        }

        [TestCaseSource(nameof(JobKinds))]
        public void EveryJobKindHasANameAndVerbs(string kind)
        {
            Assert.That(Names.IsAuthored(NameKind.Job, kind), Is.True, "No job name for " + kind);
            Assert.That(NowLines.IsAuthoredJob(kind), Is.True, "No now-line verbs for " + kind);
            TextAssert.Clean(UiText.Name(NameKind.Job, kind), kind);
            foreach (string phase in Constants(typeof(JobPhase)).Select(p => p.value))
                TextAssert.Clean(NowLines.GoalText(kind + GoalKey.Separator + phase), kind + " " + phase);
        }

        [TestCaseSource(nameof(JobPhases))]
        public void EveryJobPhaseHasAName(string phase)
        {
            Assert.That(Names.IsAuthored(NameKind.Phase, phase), Is.True, "No phase name for " + phase);
            TextAssert.Clean(UiText.Name(NameKind.Phase, phase), phase);
        }

        [Test]
        public void EveryCatalogIdHasADisplayName()
        {
            var missing = new List<string>();
            void Check(NameKind kind, IEnumerable<string> ids)
            {
                foreach (string id in ids)
                {
                    if (!Names.IsAuthored(kind, id))
                        missing.Add(kind + " " + id);
                    TextAssert.Clean(UiText.Name(kind, id), kind + " " + id);
                }
            }
            Check(NameKind.Building, Catalog.Buildings);
            Check(NameKind.Resource, Catalog.Resources);
            Check(NameKind.Labor, Catalog.Labors);
            Check(NameKind.Role, Catalog.Roles);
            Check(NameKind.Upgrade, Catalog.Upgrades);
            Check(NameKind.Recipe, Catalog.Recipes.Select(r => r.Id));
            Check(NameKind.Study, Catalog.Research.Select(s => s.Id));
            Check(NameKind.Category, Catalog.Research.Select(s => s.Category).Distinct());
            Check(NameKind.Item, Catalog.Recipes.Where(r => r.ItemKind != "").Select(r => r.ItemKind).Distinct());
            Check(NameKind.Material, Catalog.Recipes.Where(r => r.Material != "").Select(r => r.Material).Distinct());
            Check(NameKind.Labor, Catalog.Recipes.Select(r => r.Labor).Distinct());
            Assert.That(missing, Is.Empty, "Ids without authored names: " + string.Join(", ", missing));
        }

        [Test]
        public void EveryResearchPayloadReadsAsASentence()
        {
            var missing = new List<string>();
            foreach (var study in Catalog.Research)
                foreach (var payload in study.Payloads)
                {
                    if (!Effects.IsAuthoredPayload(payload.Kind))
                        missing.Add(payload.Kind);
                    if (payload.Kind == "modify" && !Effects.IsAuthoredStat(payload.Id))
                        missing.Add("stat " + payload.Id);
                    if (payload.Kind == "unlock_resource" && UiText.Effect(payload).StartsWith("Unlocks", StringComparison.Ordinal))
                        missing.Add("resource family " + payload.Id);
                    TextAssert.Clean(UiText.Effect(payload), study.Id + " " + payload.Kind + " " + payload.Id);
                }
            Assert.That(missing.Distinct(), Is.Empty, "Payloads without authored effect text: " + string.Join(", ", missing.Distinct()));
        }

        [Test]
        public void EveryActionKindHasAConfirmation()
        {
            // The router's own list of kinds: every case label of ActionRouter.Apply.
            var kinds = new[]
            {
                "acceptvillagetrade", "assignofficer", "assignworker", "attackcreature", "boost", "boostcat", "buildbridge", "builddock",
                "buildroad", "buildtransportvehicle", "buyresource", "canceltransportroute", "cancelvillagetrade", "castvote", "clearfarm",
                "createtransportroute", "createzone", "defendraid", "designatefarm", "designatefishingspot", "designategatherspot",
                "designaterail", "designatestockpile", "dispatchscout", "editproductionqueue", "editproductionworkslot", "entercat",
                "entercatcontrol", "equipitem", "exploredungeon", "haulgatherspot", "interactcat", "keepcatcontrol", "leavecat",
                "leavecatcontrol", "movecat", "offermaterials", "offerresource", "offertithe", "offervillagetrade", "planbuilding",
                "purchaseupgrade", "recallexplorer", "removegatherspot", "removestockpile", "removezone", "repairitem", "requestjob",
                "requestvotekick", "researchnode", "sellgoods", "setcatlaborpreference", "trainwarrior", "unassignofficer", "unequipitem",
                "unlocknode",
            };
            foreach (string kind in kinds)
            {
                Assert.That(Confirmations.IsAuthored(kind), Is.True, "No confirmation for " + kind);
                TextAssert.CleanOrEmpty(UiText.Confirm(kind), kind);
                TextAssert.CleanOrEmpty(UiText.Confirm(new GameAction { Kind = kind, Name = "den", NodeId = "basic_tools", Role = "steward", Resource = "food", Labor = "hunt" }), kind);
            }
        }
    }
}
