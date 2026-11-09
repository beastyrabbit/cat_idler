using System;
using System.Collections;
using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class ForestUiRevisionTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-ui-revision-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated UI revision test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator FarmWorkControlsWorldClickOffersLocalNamedWorkerAndExactActionsAtNativePhysicalScale()
        {
            var plots = PrepareFarmInspectionPlots(24); var deep = plots.Last();
            var world = game.CurrentWorld; var village = game.Selected;
            var panel = game.GetComponent<UIDocument>().panelSettings;
            float dpi = Screen.dpi > 0 ? Screen.dpi : panel.fallbackDpi;
            float pixelsPerUnit = dpi / panel.referenceDpi * panel.scale;
            // Reproduce the recorded 2400 by 1500 physical window in logical
            // panel units. ConstantPhysicalSize does not use pixels as UI units.
            SetFarmInspectionSize(2400f / pixelsPerUnit, 1500f / pixelsPerUnit);
            world.Step(.1);
            Assert.That(deep.BlockedReason, Is.EqualTo("worker_required"));
            var adults = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.OfficerRole == ""
                && c.ControlledBy == "" && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "" && c.Position.Level == 0
                && c.Migration != "arriving" && c.Migration != "departing" && !village.Routes.Any(r => r.CatId == c.Id)).Take(2).ToArray();
            Assert.That(adults.Length, Is.EqualTo(2));
            var previousWorker = adults[0]; var worker = adults[1];
            var assigned = game.Send(new GameAction { Kind = "AssignWorker", CatId = previousWorker.Id, TargetId = plots[0].Id }).Result;
            Assert.That(assigned.Success, Is.True, assigned.Error);
            string prepared = previousWorker.Name + " · " + previousWorker.Id;
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = prepared;
            game.UI.ClosePanel();
            RefreshFarmInspectionSetupFeedback();
            yield return null; yield return null;
            var otherFarms = village.Farms.Where(f => f.Id != deep.Id).ToDictionary(f => f.Id, f => JsonUtility.ToJson(f));
            double pausedAt = world.TimeSeconds;
            using (var input = new FarmInspectionInput(game, Root))
            {
                var unchanged = FarmInspectionAuthorityUnchanged();
                input.Click(new Int2(deep.Position.X + 1, deep.Position.Z + 1));
                yield return null; yield return null; yield return null;
                unchanged();
                var card = AssertFarmInspectionVisible(deep, false);
                var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
                TestContext.WriteLine($"Native physical equivalent: screen {Screen.width}x{Screen.height}, DPI {dpi}, reference {panel.referenceDpi}, scale {panel.scale}, pixels/unit {pixelsPerUnit}; root {Root.worldBound}, drawer {Root.Q<VisualElement>(className: "drawer").worldBound}, viewport {scroll.contentViewport.worldBound}, card {card.worldBound}");
                var choice = card.Q<DropdownField>("farm-worker-choice-" + deep.Id);
                Assert.That(choice, Is.Not.Null, "An exact farm map click must offer a local named worker choice beside that live plot's Assign and Clear controls.");
                Assert.That(choice.value, Is.EqualTo(prepared), "Map navigation must retain the prepared shared Work choice.");

                string named = worker.Name + " · " + worker.Id;
                Assert.That(choice.choices, Does.Contain(named), "The local chooser must offer this current eligible adult by name and stable ID.");
                var offset = scroll.scrollOffset;
                choice.value = named;
                yield return null;
                unchanged();
                Assert.That(choice.value, Is.EqualTo(named));
                Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Is.EqualTo(named));
                Assert.That(Root.Q<VisualElement>("farm-card-" + plots[0].Id).Q<DropdownField>("farm-worker-choice-" + plots[0].Id).value, Is.EqualTo(named), "The top and other live farm fields must agree with the deliberate local choice.");
                Assert.That(scroll.scrollOffset, Is.EqualTo(offset), "Choosing a worker must leave the exact plot in place without scrolling.");
                var assign = card.Query<Button>().ToList().Single(b => b.text == "Assign selected cat");
                var clear = card.Query<Button>().ToList().Single(b => b.text == "Clear farm");
                var identity = card.Query<Label>().ToList().Single(l => l.text == "Plot · " + deep.Id + " · " + deep.Position + " · 2 × 2");
                void AssertControlsVisible()
                {
                    AssertInside(scroll.contentViewport.worldBound, card.worldBound, "The native-size card must fit with its crop, assigned worker, ownership and any current warning.");
                    foreach (var element in new VisualElement[] { identity, choice, assign, clear })
                    {
                        Assert.That(element.panel, Is.SameAs(Root.panel));
                        Assert.That(element.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
                        Assert.That(element.resolvedStyle.visibility, Is.EqualTo(Visibility.Visible));
                        Assert.That(element.worldBound.height, Is.GreaterThan(0));
                        AssertInside(scroll.contentViewport.worldBound, element.worldBound, "Exact plot identity, the closed named choice, Assign and Clear must be visible together at the native physical equivalent without scrolling.");
                    }
                }
                AssertControlsVisible();
                int revision = game.Revision;
                Submit(assign);
                Assert.That(game.Revision, Is.EqualTo(revision + 1)); Assert.That(game.LastAction, Does.Contain("accepted"));
                Assert.That(deep.WorkerId, Is.EqualTo(worker.Id)); Assert.That(worker.BuildingId, Is.EqualTo(deep.Id)); Assert.That(deep.AutomatedBy, Is.Empty);
                RefreshFarmInspectionSetupFeedback();
                yield return null; yield return null;
                card = WorkFarmCard(deep.Id); choice = card.Q<DropdownField>("farm-worker-choice-" + deep.Id);
                Assert.That(choice, Is.Not.Null); Assert.That(choice.value, Is.EqualTo(named));
                assign = card.Query<Button>().ToList().Single(b => b.text == "Assign selected cat");
                clear = card.Query<Button>().ToList().Single(b => b.text == "Clear farm");
                identity = card.Query<Label>().ToList().Single(l => l.text == "Plot · " + deep.Id + " · " + deep.Position + " · 2 × 2");
                Assert.That(FarmCardCopy(card), Does.Contain("Worker · " + named).And.Contain("Assignment · Manual").And.Not.Contain("Waiting for a worker"));
                AssertControlsVisible();
                Assert.That(village.Farms.Count, Is.EqualTo(otherFarms.Count + 1));
                foreach (var farm in village.Farms.Where(f => f.Id != deep.Id)) Assert.That(JsonUtility.ToJson(farm), Is.EqualTo(otherFarms[farm.Id]));
                Assert.That(plots[0].WorkerId, Is.EqualTo(previousWorker.Id)); Assert.That(previousWorker.BuildingId, Is.EqualTo(plots[0].Id));
                Assert.That(game.Speed, Is.Zero); Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.Validate(), Is.Empty);
            }
        }

        [UnityTest]
        public IEnumerator FarmWorkControlsShareSelectionAcrossPeersNavigationAndNewPlots()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false; SetFarmInspectionSize(1000, 720);
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            var top = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
            var first = FarmWorkChoice(farms.Manual.Id); var second = FarmWorkChoice(farms.Automatic.Id);
            AssertFarmWorkChoiceValues("Choose available cat");
            int peerEvents = 0;
            top.RegisterValueChangedCallback(_ => peerEvents++); second.RegisterValueChangedCallback(_ => peerEvents++);
            var unchanged = FarmInspectionAuthorityUnchanged();
            string prepared = farms.Replacement.Name + " · " + farms.Replacement.Id;
            first.value = prepared;
            AssertFarmWorkChoiceValues(prepared); Assert.That(peerEvents, Is.Zero, "Synchronizing peers must not publish a second selection event."); unchanged();
            string busy = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            top.value = busy; AssertFarmWorkChoiceValues(busy);
            second.value = prepared; AssertFarmWorkChoiceValues(prepared); unchanged();
            foreach (var farm in new[] { farms.Manual, farms.Automatic, farms.Manual })
            {
                game.UI.InspectFarm(farm.Id); yield return null; yield return null;
                AssertFarmInspectionVisible(farm); AssertFarmWorkChoiceValues(prepared); unchanged();
            }
            var retained = Root.Query<DropdownField>().ToList().ToDictionary(f => f, f => f.value);
            var designated = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 5), End = new Int2(10, 5) }).Result;
            Assert.That(designated.Success, Is.True, designated.Error);
            RefreshFarmInspectionSetupFeedback();
            unchanged = FarmInspectionAuthorityUnchanged();
            Assert.That(FarmWorkChoice(designated.EntityId).value, Is.EqualTo(prepared), "Constructing a new farm chooser must retain the shared choice.");
            foreach (var entry in retained) { Assert.That(entry.Key.panel, Is.SameAs(Root.panel)); Assert.That(entry.Key.value, Is.EqualTo(entry.Value)); }
            AssertFarmWorkChoiceValues(prepared); unchanged();
            game.UI.ClosePanel(); game.UI.OpenPanel("Work"); AssertFarmWorkChoiceValues(prepared); unchanged();
            game.UI.OpenPanel("Stores");
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Is.EqualTo(prepared), "Inventory must keep the same prepared carrier."); unchanged();
        }

        [Test]
        public void FarmWorkControlsResolveSameIdSnapshotsAndCurrentRosterWithoutLabelOrIndexRetargeting()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            var first = FarmWorkChoice(farms.Manual.Id); var second = FarmWorkChoice(farms.Automatic.Id);
            string chosen = farms.Replacement.Name + " · " + farms.Replacement.Id;
            first.value = chosen;
            var original = game.Selected; string originalBytes = JsonUtility.ToJson(original);
            var current = JsonUtility.FromJson<Village>(originalBytes);
            game.CurrentWorld.Villages[game.CurrentWorld.Villages.IndexOf(original)] = current;
            var unchanged = FarmInspectionAuthorityUnchanged();
            string named = farms.FarmerWorker.Name + " · " + farms.FarmerWorker.Id;
            EmitFarmWorkChoice(second, named); AssertFarmWorkChoiceValues(named); unchanged();
            Assert.That(FarmWorkChoice(farms.Manual.Id), Is.SameAs(first), "Equivalent same-ID snapshots keep the fields and accept current IDs.");
            first.value = chosen;
            var replacement = current.Cats.Single(c => c.Id == farms.Replacement.Id);
            var renamed = current.Cats.Single(c => c.Id == farms.FarmerWorker.Id);
            renamed.Name = "Current Roster Farmer"; replacement.Name = "Current Prepared Worker";
            current.Cats.Reverse();
            var added = new Cat { Id = game.CurrentWorld.Id("ui-roster-cat"), Name = "New Current Cat", VillageId = current.Id, Position = new Int2(1, 3), X = 1, Z = 3 };
            current.Cats.Add(added);
            unchanged = FarmInspectionAuthorityUnchanged();
            // The old menu is still attached, before any live roster refresh.
            EmitFarmWorkChoice(second, named);
            chosen = replacement.Name + " · " + replacement.Id;
            AssertFarmWorkChoiceValues(chosen); unchanged();
            Assert.That(second.choices, Does.Not.Contain(named).And.Contain(renamed.Name + " · " + renamed.Id).And.Contain(added.Name + " · " + added.Id));
            CollectionAssert.AreEqual(new[] { "Choose available cat" }.Concat(current.Cats.Where(c => c.Alive).Select(c => c.Name + " · " + c.Id)), second.choices);
            EmitFarmWorkChoice(second, named); AssertFarmWorkChoiceValues(chosen); unchanged();
            second.value = added.Name + " · " + added.Id; AssertFarmWorkChoiceValues(second.value); unchanged();
            added.Alive = false;
            unchanged = FarmInspectionAuthorityUnchanged();
            EmitFarmWorkChoice(first, added.Name + " · " + added.Id);
            AssertFarmWorkChoiceValues("Choose available cat"); unchanged();
            Assert.That(first.choices, Does.Not.Contain(added.Name + " · " + added.Id), "A dead selected cat becomes empty, never the first living cat.");
            second.value = chosen;
            current.Cats.Remove(replacement);
            unchanged = FarmInspectionAuthorityUnchanged();
            RefreshFarmWorkControls(); AssertFarmWorkChoiceValues("Choose available cat"); unchanged();
            Assert.That(first.choices, Does.Not.Contain(chosen)); Assert.That(FarmWorkChoice(farms.Manual.Id), Is.SameAs(first));
            Assert.That(JsonUtility.ToJson(original), Is.EqualTo(originalBytes), "Roster reconciliation must not mutate obsolete Cat or Village objects.");
        }

        [Test]
        public void FarmWorkControlsIgnoreClosedDetachedAndRebuiltChooserCallbacks()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            RefreshFarmInspectionSetupFeedback();
            string prepared = farms.Replacement.Name + " · " + farms.Replacement.Id;
            string other = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            foreach (string scope in new[] { "closed", "other section", "rebuilt" })
            {
                game.UI.OpenPanel("Work"); FarmWorkChoice(farms.Automatic.Id).value = prepared;
                var oldTop = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
                var oldLocal = FarmWorkChoice(farms.Automatic.Id);
                if (scope == "closed") game.UI.ClosePanel();
                else game.UI.OpenPanel(scope == "rebuilt" ? "Work" : "Build");
                if (scope == "closed") Assert.That(oldLocal.panel, Is.SameAs(Root.panel), scope);
                else Assert.That(oldLocal.panel, Is.Null, scope);
                var unchanged = FarmInspectionAuthorityUnchanged();
                EmitFarmWorkChoice(oldTop, other); EmitFarmWorkChoice(oldLocal, other); unchanged();
                game.UI.OpenPanel("Work"); AssertFarmWorkChoiceValues(prepared); unchanged();
            }
        }

        [Test]
        public void FarmWorkControlsIgnoreRemovedFarmEventsBeforeAndAfterRefreshAndKeepSurvivingPeers()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            var stale = FarmWorkChoice(farms.Automatic.Id); var surviving = FarmWorkChoice(farms.Manual.Id);
            string prepared = farms.Replacement.Name + " · " + farms.Replacement.Id;
            surviving.value = prepared;
            var old = game.Selected; var current = JsonUtility.FromJson<Village>(JsonUtility.ToJson(old));
            current.Farms.RemoveAll(f => f.Id == farms.Automatic.Id);
            game.CurrentWorld.Villages[game.CurrentWorld.Villages.IndexOf(old)] = current;
            Assert.That(stale.panel, Is.SameAs(Root.panel), "The removed ID must be rejected while its old field is still attached.");
            var unchanged = FarmInspectionAuthorityUnchanged();
            EmitFarmWorkChoice(stale, farms.ManualWorker.Name + " · " + farms.ManualWorker.Id);
            AssertFarmWorkChoiceValues(prepared); unchanged();
            RefreshFarmWorkControls(); Assert.That(stale.panel, Is.Null);
            Assert.That(FarmWorkChoice(farms.Manual.Id), Is.SameAs(surviving));
            surviving.value = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            string next = surviving.value;
            EmitFarmWorkChoice(stale, prepared); AssertFarmWorkChoiceValues(next); unchanged();
            game.UI.OpenPanel("Work"); AssertFarmWorkChoiceValues(next); unchanged();
            Assert.That(old.Farms.Count, Is.EqualTo(2)); Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [Test]
        public void FarmWorkControlsIgnoreVillageSwitchChooserEventsBeforeAndAfterRebuild()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            RefreshFarmInspectionSetupFeedback();
            var oldVillage = game.Selected;
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "Farm chooser isolation" }).Result.Success, Is.True);
            string personalId = game.Selected.Id;
            Assert.That(game.Send(new GameAction { Kind = "JoinVillage", TargetId = oldVillage.Id }).Result.Success, Is.True);
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            string prepared = farms.Replacement.Name + " · " + farms.Replacement.Id;
            FarmWorkChoice(farms.Automatic.Id).value = prepared;
            var stale = new[] { Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat"), FarmWorkChoice(farms.Automatic.Id) };
            Assert.That(game.Send(new GameAction { Kind = "JoinVillage", TargetId = personalId }).Result.Success, Is.True);
            Assert.That(stale.All(f => f.panel == Root.panel), Is.True, "Switch village before refreshing old Work content.");
            // No yield or refresh follows this setup command until both old callbacks are dispatched.
            var unchanged = FarmInspectionAuthorityUnchanged();
            foreach (var field in stale) EmitFarmWorkChoice(field, farms.ManualWorker.Name + " · " + farms.ManualWorker.Id);
            unchanged();
            RefreshFarmInspectionSetupFeedback();
            unchanged = FarmInspectionAuthorityUnchanged();
            AssertFarmWorkChoiceValues("Choose available cat");
            var personal = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
            personal.value = personal.choices.Last(); string current = personal.value;
            foreach (var field in stale) { Assert.That(field.panel, Is.Null); EmitFarmWorkChoice(field, prepared); }
            AssertFarmWorkChoiceValues(current); unchanged();
            game.UI.OpenPanel("Work"); AssertFarmWorkChoiceValues(current); unchanged();
            Assert.That(farms.Manual.WorkerId, Is.EqualTo(farms.ManualWorker.Id)); Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [Test]
        public void FarmWorkControlsEmptyRefusesAndBusyChoiceReassignsThenClearsOnlyItsExactPlot()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            Assert.That(FarmWorkChoice(farms.Automatic.Id).value, Is.EqualTo("Choose available cat"));
            var world = game.CurrentWorld; string before = JsonUtility.ToJson(world); int revision = game.Revision;
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(game.LastAction, Is.EqualTo("Available living cat required")); Assert.That(game.Revision, Is.EqualTo(revision));
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(before), "Empty farm choice must preserve the existing command refusal and every work owner.");
            string busy = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            Assert.That(FarmWorkChoice(farms.Automatic.Id).choices, Does.Contain(busy), "An ordinary assigned worker remains a named manual choice.");
            FarmWorkChoice(farms.Automatic.Id).value = busy;
            double pausedAt = world.TimeSeconds; var rng = world.RandomState;
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(game.Revision, Is.EqualTo(revision + 1)); Assert.That(game.LastAction, Does.Contain("accepted"));
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.ManualWorker.Id)); Assert.That(farms.ManualWorker.BuildingId, Is.EqualTo(farms.Automatic.Id));
            Assert.That(farms.Manual.WorkerId, Is.Empty); Assert.That(farms.Automatic.AutomatedBy, Is.Empty);
            Assert.That(FarmCardCopy(WorkFarmCard(farms.Automatic.Id)), Does.Contain("Worker · " + busy).And.Contain("Assignment · Manual"));
            string other = JsonUtility.ToJson(farms.Manual);
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Clear farm"));
            Assert.That(game.Revision, Is.EqualTo(revision + 2)); Assert.That(game.Selected.Farms.Select(f => f.Id), Is.EqualTo(new[] { farms.Manual.Id }));
            Assert.That(JsonUtility.ToJson(farms.Manual), Is.EqualTo(other)); Assert.That(farms.ManualWorker.BuildingId, Is.Empty);
            Assert.That(farms.ManualWorker.JobId, Is.Empty); Assert.That(farms.ManualWorker.ResumeJobId, Is.Empty);
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.RandomState, Is.EqualTo(rng)); Assert.That(world.Validate(), Is.Empty);
        }

        [TestCase("controlled", "Available living cat required")]
        [TestCase("explorer", "Recall the explorer and wait for its return before assigning work")]
        [TestCase("driver", "Return the vehicle to a safe stopping point before reassigning its driver")]
        public void FarmWorkControlsPreserveProtectedCatRefusalsAndExistingOwnership(string protection, string reason)
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            var cat = farms.ManualWorker; var village = game.Selected;
            // Explicit protected-state snapshots exercise the unchanged authority fence.
            // They do not claim a native expedition or an underway shipping journey.
            if (protection == "controlled") cat.ControlledBy = "ui-protected-owner";
            else if (protection == "explorer") cat.DungeonId = "ui-protected-expedition";
            else village.Routes.Add(new TransportRoute { Id = game.CurrentWorld.Id("ui-protected-route"), Mode = "shipping", CatId = cat.Id, PathIndex = 1 });
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            string named = cat.Name + " · " + cat.Id;
            Assert.That(FarmWorkChoice(farms.Automatic.Id).choices, Does.Contain(named));
            var unchanged = FarmInspectionAuthorityUnchanged();
            FarmWorkChoice(farms.Automatic.Id).value = named; AssertFarmWorkChoiceValues(named); unchanged();
            string before = JsonUtility.ToJson(game.CurrentWorld); int revision = game.Revision;
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(game.LastAction, Is.EqualTo(reason)); Assert.That(game.Revision, Is.EqualTo(revision));
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before));
            Assert.That(farms.Manual.WorkerId, Is.EqualTo(cat.Id)); Assert.That(cat.BuildingId, Is.EqualTo(farms.Manual.Id)); Assert.That(farms.Automatic.WorkerId, Is.Empty);
        }

        [UnityTest]
        public IEnumerator FarmWorkControlsPausedFarmerTakeoverKeepsLiveFeedbackAndSurvivesVacancy()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false;
            Assert.That(game.Send(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = game.Selected.Cats[0].Id }).Result.Success, Is.True);
            game.CurrentWorld.Step(10);
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.FarmerWorker.Id)); Assert.That(farms.Automatic.AutomatedBy, Is.EqualTo("farmer"));
            RefreshFarmInspectionSetupFeedback(); game.UI.OpenPanel("Work");
            var card = WorkFarmCard(farms.Automatic.Id);
            Assert.That(FarmCardCopy(card), Does.Contain("Worker · " + farms.FarmerWorker.Name + " · " + farms.FarmerWorker.Id).And.Contain("Assignment · Farmer"));
            double pausedAt = game.CurrentWorld.TimeSeconds;
            var unchanged = FarmInspectionAuthorityUnchanged();
            FarmWorkChoice(farms.Automatic.Id).value = farms.Replacement.Name + " · " + farms.Replacement.Id; unchanged();
            Submit(card.Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.Replacement.Id)); Assert.That(farms.FarmerWorker.BuildingId, Is.Empty); Assert.That(farms.Automatic.AutomatedBy, Is.Empty);
            RefreshFarmInspectionSetupFeedback();
            unchanged = FarmInspectionAuthorityUnchanged();
            yield return null;
            RefreshFarmWorkControls(); unchanged();
            Assert.That(FarmCardCopy(WorkFarmCard(farms.Automatic.Id)), Does.Contain("Worker · " + farms.Replacement.Name + " · " + farms.Replacement.Id).And.Contain("Assignment · Manual").And.Not.Contain("Waiting for a worker"));
            Assert.That(game.Send(new GameAction { Kind = "UnassignOfficer", Role = "farmer" }).Result.Success, Is.True);
            RefreshFarmInspectionSetupFeedback();
            unchanged = FarmInspectionAuthorityUnchanged(); RefreshFarmWorkControls(); unchanged();
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.Replacement.Id)); Assert.That(farms.Replacement.BuildingId, Is.EqualTo(farms.Automatic.Id));
            Assert.That(farms.Manual.WorkerId, Is.EqualTo(farms.ManualWorker.Id));
            Assert.That(game.Speed, Is.Zero); Assert.That(game.CurrentWorld.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [UnityTest]
        public IEnumerator FarmWorkControlsCompactWarningAndPopupKeepObjectsFocusAndOrdinaryScroll()
        {
            var plots = PrepareFarmInspectionPlots(4); var farm = plots.Last();
            // This is the stricter inferred logical size, not a new native measurement.
            SetFarmInspectionSize(880, 550); game.CurrentWorld.Step(.1);
            Assert.That(farm.BlockedReason, Is.EqualTo("worker_required"));
            RefreshFarmInspectionSetupFeedback();
            using (var input = new FarmInspectionInput(game, Root))
            {
                var unchanged = FarmInspectionAuthorityUnchanged();
                input.Click(new Int2(farm.Position.X + 1, farm.Position.Z + 1));
                yield return null; yield return null; yield return null;
                var card = AssertFarmInspectionVisible(farm); var choice = FarmWorkChoice(farm.Id);
                var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
                Assert.That(card.Q<Label>(className: "warning").text, Does.Contain("Waiting for a worker"));
                foreach (var element in card.Children())
                    if (element.resolvedStyle.display != DisplayStyle.None) AssertInside(scroll.contentViewport.worldBound, element.worldBound, "Compact warning, identity, ownership, chooser and complete actions must fit together.");
                choice.Focus(); var focused = Root.focusController.focusedElement; var offset = scroll.scrollOffset;
                Assert.That(focused, Is.Not.Null);
                var choices = Root.Query<DropdownField>().ToList(); var labels = choice.choices;
                using (var open = NavigationSubmitEvent.GetPooled()) { open.target = choice; choice.SendEvent(open); }
                yield return null; yield return null;
                var popup = Root.panel.visualTree.Q<VisualElement>(className: GenericDropdownMenu.ussClassName);
                Assert.That(popup, Is.Not.Null);
                var popupFocus = Root.focusController.focusedElement;
                RefreshFarmWorkControls(); yield return null; RefreshFarmWorkControls();
                Assert.That(Root.panel.visualTree.Q<VisualElement>(className: GenericDropdownMenu.ussClassName), Is.SameAs(popup));
                Assert.That(choice.choices, Is.SameAs(labels), "An unchanged roster must not reset the menu's choices list.");
                CollectionAssert.AreEqual(choices, Root.Query<DropdownField>().ToList());
                Assert.That(Root.focusController.focusedElement, Is.SameAs(popupFocus)); Assert.That(scroll.scrollOffset, Is.EqualTo(offset)); unchanged();
                var menuScroll = popup.Q<ScrollView>(); Assert.That(menuScroll, Is.Not.Null);
                var point = menuScroll.contentViewport.worldBound.center;
                var panelOrigin = RuntimePanelUtils.ScreenToPanel(Root.panel, Vector2.zero);
                var panelExtent = RuntimePanelUtils.ScreenToPanel(Root.panel, new Vector2(Screen.width, Screen.height)) - panelOrigin;
                var cursor = new Vector2((point.x - panelOrigin.x) / panelExtent.x * Screen.width, Screen.height - (point.y - panelOrigin.y) / panelExtent.y * Screen.height);
                InputState.Change(Mouse.current, new MouseState { position = cursor, scroll = new Vector2(0, 1) }, InputUpdateType.Dynamic);
                Assert.That(game.UI.PointerOverPanel, Is.True, "The real farm popup owns the wheel pointer even outside management chrome.");
                float zoom = game.View.ManagementZoom;
                typeof(ForestView).GetMethod("ReadInput", FarmInspectionFlags).Invoke(game.View, null);
                Assert.That(game.View.ManagementZoom, Is.EqualTo(zoom)); Assert.That(scroll.scrollOffset, Is.EqualTo(offset)); unchanged();
                InputState.Change(Mouse.current, new MouseState { position = cursor }, InputUpdateType.Dynamic);
                // A menu retained across a rename cannot choose its obsolete label.
                var worker = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
                string obsolete = worker.Name + " · " + worker.Id; worker.Name = "Renamed During Popup";
                unchanged = FarmInspectionAuthorityUnchanged();
                EmitFarmWorkChoice(choice, obsolete); AssertFarmWorkChoiceValues("Choose available cat"); unchanged();
                popup.RemoveFromHierarchy(); choice.Focus();
                scroll.scrollOffset = Vector2.zero;
                yield return null; yield return null;
                RefreshFarmWorkControls();
                Assert.That(scroll.scrollOffset.y, Is.Zero.Within(.5f), "After the exact reveal and popup close, ordinary player scrolling stays free."); unchanged();
                Assert.That(game.CurrentWorld.Validate(), Is.Empty);
            }
        }

        private DropdownField FarmWorkChoice(string farmId)
        {
            var field = Root.Q<DropdownField>("farm-worker-choice-" + farmId);
            Assert.That(field, Is.Not.Null, "Every current farm needs its own named worker choice: " + farmId);
            return field;
        }

        private void AssertFarmWorkChoiceValues(string expected)
        {
            var fields = Root.Query<DropdownField>().ToList().Where(f => f.label == "Cat" || f.name.StartsWith("farm-worker-choice-", StringComparison.Ordinal)).ToList();
            Assert.That(fields.Count, Is.GreaterThan(0));
            foreach (var field in fields) Assert.That(field.value, Is.EqualTo(expected), field.name);
        }

        private void RefreshFarmWorkControls()
        {
            var unchanged = FarmInspectionAuthorityUnchanged();
            typeof(ForestUI).GetField("nextRefresh", FarmInspectionFlags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", FarmInspectionFlags).Invoke(game.UI, null);
            unchanged();
        }

        private static void EmitFarmWorkChoice(DropdownField field, string value)
        {
            int observed = 0;
            void Observe(ChangeEvent<string> _) => observed++;
            field.RegisterValueChangedCallback(Observe);
            try
            {
                using (var change = ChangeEvent<string>.GetPooled(field.value, value))
                {
                    change.target = field;
                    if (field.panel != null) field.SendEvent(change);
                    else
                    {
                        // Unity 6000.6's EventCallbackRegistry.InvokeCallbacks is
                        // marked "For unit tests only". Invoke the retained registry
                        // without attaching the field. Value-change callbacks use
                        // BubbleUp even on the target; AtTarget is unsupported here.
                        var registryField = typeof(CallbackEventHandler).GetField("m_CallbackRegistry", FarmInspectionFlags);
                        Assert.That(registryField, Is.Not.Null);
                        var registry = registryField.GetValue(field);
                        Assert.That(registry, Is.Not.Null);
                        var invoke = registry.GetType().GetMethod("InvokeCallbacks", FarmInspectionFlags, null,
                            new[] { typeof(EventBase), typeof(PropagationPhase) }, null);
                        var currentTarget = typeof(EventBase).GetProperty(nameof(EventBase.currentTarget)).GetSetMethod(true);
                        var phase = typeof(EventBase).GetProperty(nameof(EventBase.propagationPhase)).GetSetMethod(true);
                        Assert.That(invoke, Is.Not.Null); Assert.That(currentTarget, Is.Not.Null); Assert.That(phase, Is.Not.Null);
                        currentTarget.Invoke(change, new object[] { field });
                        phase.Invoke(change, new object[] { PropagationPhase.BubbleUp });
                        try { invoke.Invoke(registry, new object[] { change, PropagationPhase.BubbleUp }); }
                        finally
                        {
                            currentTarget.Invoke(change, new object[] { null });
                            phase.Invoke(change, new object[] { PropagationPhase.None });
                        }
                    }
                }
            }
            finally { field.UnregisterValueChangedCallback(Observe); }
            Assert.That(observed, Is.EqualTo(1), "Dispatch the retained field's real event callbacks, including on detached controls.");
        }

        [UnityTest]
        public IEnumerator FarmInspectionWorldClickOpensExactLiveWorkCardWithoutChangingAuthority()
        {
            game.View.enabled = false;
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000; Root.style.height = Root.style.maxHeight = 620;
            game.UI.ClosePanel();
            var world = game.CurrentWorld; var village = game.Selected;
            var origin = new Int2(village.Center.X + village.Radius + 6, village.Center.Z);
            // Finite support only. Designation itself must pass the public authority rules.
            for (int x = village.Radius + 1; x <= village.Radius + 10; x++)
                for (int z = -4; z <= 3; z++)
                {
                    var at = new Int2(village.Center.X + x, village.Center.Z + z);
                    var tile = world.TileAt(at); World.ClearSurface(tile, 0);
                    tile.Wall = tile.Bridge = tile.Road = tile.Rail = false; tile.ClaimId = "";
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
            village.Buildings.Add(new Building { Id = world.Id("ui-inspection-field"), Kind = "field", Position = new Int2(origin.X, origin.Z - 4), Completed = true });
            var clicked = new Int2(origin.X + 1, origin.Z + 1);
            var result = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = origin, End = clicked }).Result;
            Assert.That(result.Success, Is.True, result.Error);
            var farm = village.Farms.Single(f => f.Id == result.EntityId);
            Assert.That(farm.Position, Is.EqualTo(origin)); Assert.That(farm.Width, Is.EqualTo(2)); Assert.That(farm.Depth, Is.EqualTo(2));
            Assert.That(village.Cats.Any(c => c.Alive && c.Position.Level == clicked.Level && Vector2.Distance(new Vector2((float)c.X, (float)c.Z), new Vector2(clicked.X, clicked.Z)) < .8f), Is.False, "No cat may intercept this farm click.");
            Assert.That(village.Buildings.Any(b => b.Position.Level == clicked.Level && clicked.X >= b.Position.X && clicked.X < b.Position.X + b.Width && clicked.Z >= b.Position.Z && clicked.Z < b.Position.Z + b.Depth), Is.False, "No building may intercept this farm click.");
            Assert.That(world.Validate(), Is.Empty);
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestUI).GetField("nextRefresh", flags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", flags).Invoke(game.UI, null);
            yield return null;
            game.View.FocusVillage();
            typeof(ForestView).GetMethod("Reconcile", flags).Invoke(game.View, null);
            typeof(ForestView).GetMethod("UpdateCamera", flags).Invoke(game.View, null);
            Physics.SyncTransforms();
            Assert.That(GameObject.Find("farm:" + farm.Id + ":1:1"), Is.Not.Null, "Click an existing rendered footprint cell, away from the anchor.");
            Assert.That(game.View.RenderedLevel, Is.EqualTo(clicked.Level));
            var pixel = game.View.Camera.WorldToScreenPoint(new Vector3(clicked.X, (float)world.WalkHeight(clicked), clicked.Z));
            Assert.That(pixel.z, Is.GreaterThan(0)); Assert.That(pixel.x, Is.InRange(1f, Screen.width - 1f)); Assert.That(pixel.y, Is.InRange(1f, Screen.height - 1f));
            // Batch Editor panel coordinates can overlap the synthetic world ray.
            // Isolate picking as ForestCameraFenceTests does; composed cases retain
            // the interactive classes to verify real UI exclusion separately.
            foreach (var element in Root.Query<VisualElement>(className: "interactive").ToList()) element.RemoveFromClassList("interactive");

            Mouse mouse = null; Keyboard keyboard = null;
            try
            {
                mouse = InputSystem.AddDevice<Mouse>(); keyboard = InputSystem.AddDevice<Keyboard>();
                mouse.MakeCurrent(); keyboard.MakeCurrent();
                // Match ForestCameraFenceTests' Dynamic-buffer input, without a global input update.
                var state = new MouseState { position = new Vector2(pixel.x, pixel.y) };
                InputState.Change(mouse, state, InputUpdateType.Dynamic);
                var readInput = typeof(ForestView).GetMethod("ReadInput", flags);
                readInput.Invoke(game.View, null);
                Assert.That(game.View.CursorTile, Is.EqualTo(clicked), "The current camera ray must resolve the intended farm cell before pressing.");
                Assert.That(mouse.leftButton.wasPressedThisFrame, Is.False);
                Assert.That(game.UI.PointerOverPanel || game.UI.TextInputFocused || game.UI.HasPlacement || game.View.DirectControl, Is.False, "The real pointer must reach management selection outside UI and placement.");
                Assert.That(game.Speed, Is.Zero); Assert.That(game.UI.OpenSection, Is.Empty);

                string before = JsonUtility.ToJson(world), action = game.LastAction; int revision = game.Revision;
                var feedback = Root.Q<Label>(className: "feedback");
                string copy = feedback.text; var display = feedback.style.display.value;
                bool failed = feedback.ClassListContains("failure"), shown = Root.ClassListContains("has-feedback");
                void AssertInspectionChangesOnlyUi()
                {
                    Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before), "Inspection must preserve the full authority, including clocks, RNG, goods and ownership.");
                    Assert.That(game.Revision, Is.EqualTo(revision)); Assert.That(game.LastAction, Is.EqualTo(action), "Inspection must issue no command, including a rejected one.");
                    Assert.That(feedback.text, Is.EqualTo(copy)); Assert.That(feedback.style.display.value, Is.EqualTo(display));
                    Assert.That(feedback.ClassListContains("failure"), Is.EqualTo(failed)); Assert.That(Root.ClassListContains("has-feedback"), Is.EqualTo(shown));
                }
                InputState.Change(mouse, state.WithButton(UnityEngine.InputSystem.LowLevel.MouseButton.Left), InputUpdateType.Dynamic);
                Assert.That(mouse.leftButton.wasPressedThisFrame, Is.True, "Deliver one fresh synthetic left press.");
                readInput.Invoke(game.View, null);
                Assert.That(game.View.CursorTile, Is.EqualTo(clicked));
                AssertInspectionChangesOnlyUi();
                Assert.That(game.UI.OpenSection, Is.EqualTo("Work"), "Clicking a rendered owned farm tile must open Work at that plot's existing live card.");

                yield return null; yield return null;
                Assert.That(Root.Q<VisualElement>(className: "drawer").resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
                var card = WorkFarmCard(farm.Id);
                Assert.That(FarmCardCopy(card), Does.Contain("Plot · " + farm.Id + " · " + origin + " · 2 × 2").And.Contain("Worker · unassigned").And.Contain("Assignment · Unstaffed"));
                Assert.That(card.Query<Button>().ToList().Count(b => b.text == "Assign selected cat"), Is.EqualTo(1));
                Assert.That(card.Query<Button>().ToList().Count(b => b.text == "Clear farm"), Is.EqualTo(1));
                var viewport = Root.Q<ScrollView>(className: "drawer-scroll").contentViewport;
                Assert.That(viewport.worldBound.height, Is.GreaterThan(0)); Assert.That(card.worldBound.height, Is.GreaterThan(0));
                AssertInside(viewport.worldBound, card.worldBound, "The exact live farm card and its existing controls must be fully visible without manual scrolling.");
                AssertInspectionChangesOnlyUi(); Assert.That(world.Validate(), Is.Empty);
            }
            finally
            {
                if (mouse != null) InputSystem.RemoveDevice(mouse);
                if (keyboard != null) InputSystem.RemoveDevice(keyboard);
            }
        }

        [UnityTest]
        public IEnumerator FarmInspectionDeepCardsSettleFromShortPagesAndKeepPlayerScroll()
        {
            var plots = PrepareFarmInspectionPlots(24); var early = plots[0]; var deep = plots.Last();
            SetFarmInspectionSize(1000, 720);
            yield return null;
            using (var input = new FarmInspectionInput(game, Root))
            {
                var unchanged = FarmInspectionAuthorityUnchanged();
                input.Click(deep.Position); yield return null; yield return null;
                AssertFarmInspectionVisible(deep); unchanged();
                game.View.InspectBuilding(game.Selected.Buildings.First(b => b.Kind == "field").Id); game.UI.OpenPanel("Inspect");
                yield return null; yield return null;
                game.View.InspectBuilding(game.Selected.Buildings.First(b => b.Kind == "field").Id); game.View.InspectBuilding(""); game.View.ZoomBy(-20);
                input.Click(deep.Position); yield return null; yield return null; yield return null;
                AssertFarmInspectionVisible(deep);
                var choice = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
                choice.value = choice.choices.Last(); string selected = choice.value;
                foreach (var farm in new[] { early, deep, early, deep })
                {
                    input.Click(farm.Position); yield return null; yield return null;
                    AssertFarmInspectionVisible(farm);
                    Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Is.EqualTo(selected));
                }
                RefreshFarmFeedbackWithoutAuthorityChanges(); yield return null; yield return null;
                AssertFarmInspectionVisible(deep);
                var scroll = Root.Q<ScrollView>(className: "drawer-scroll"); scroll.scrollOffset = Vector2.zero;
                yield return null; yield return null; yield return null;
                RefreshFarmFeedbackWithoutAuthorityChanges(); yield return null;
                Assert.That(scroll.scrollOffset.y, Is.Zero.Within(.5f), "Completed inspection must leave later player scrolling alone.");
                input.Click(deep.Position); yield return null; yield return null;
                AssertFarmInspectionVisible(deep); unchanged();
            }
        }

        [UnityTest]
        public IEnumerator FarmInspectionCompactCardRevealsIdentityAndLeavesControlsScrollable()
        {
            var deep = PrepareFarmInspectionPlots(24).Last(); SetFarmInspectionSize(600, 480);
            yield return null; yield return null;
            using (var input = new FarmInspectionInput(game, Root))
            {
                var unchanged = FarmInspectionAuthorityUnchanged();
                input.Click(deep.Position); yield return null; yield return null; yield return null;
                Assert.That(Root.ClassListContains("compact"), Is.True);
                var card = AssertFarmInspectionVisible(deep, false);
                var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
                var clear = card.Query<Button>().ToList().Single(b => b.text == "Clear farm");
                scroll.ScrollTo(clear); yield return null; yield return null;
                AssertInside(scroll.contentViewport.worldBound, clear.worldBound, "Compact inspection must leave the existing controls reachable by ordinary scrolling.");
                var offset = scroll.scrollOffset;
                RefreshFarmFeedbackWithoutAuthorityChanges(); yield return null; yield return null;
                Assert.That(scroll.scrollOffset, Is.EqualTo(offset)); unchanged();
            }
        }

        [UnityTest]
        public IEnumerator FarmInspectionWorldPickingUsesEveryCellAndKeepsCatBuildingLevelAndVillagePriority()
        {
            var farm = PrepareFarmInspectionPlots(0).Single(); SetFarmInspectionSize(1000, 720);
            yield return null;
            using (var input = new FarmInspectionInput(game, Root))
            {
                var unchanged = FarmInspectionAuthorityUnchanged();
                foreach (var delta in new[] { new Int2(0, 0), new Int2(1, 0), new Int2(0, 1), new Int2(1, 1) })
                {
                    game.UI.ClosePanel(); input.Click(new Int2(farm.Position.X + delta.X, farm.Position.Z + delta.Z));
                    yield return null; yield return null; AssertFarmInspectionVisible(farm);
                }
                game.UI.ClosePanel(); input.Click(new Int2(farm.Position.X + 2, farm.Position.Z));
                Assert.That(game.UI.OpenSection, Is.Empty); unchanged();

                // Controlled overlap snapshots test picking, not legal farm placement.
                var cat = game.Selected.Cats.First(c => c.Alive); var oldPosition = cat.Position; double oldX = cat.X, oldZ = cat.Z;
                PlaceFarmCat(cat, farm.Position);
                var building = new Building { Id = "inspection-overlap-building", Kind = "field", Position = farm.Position, Width = 2, Depth = 2, Completed = true };
                game.Selected.Buildings.Add(building);
                input.Click(farm.Position);
                Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect")); Assert.That(game.View.SelectedCatId, Is.EqualTo(cat.Id));
                cat.Position = oldPosition; cat.X = oldX; cat.Z = oldZ;
                input.Click(new Int2(farm.Position.X + 1, farm.Position.Z + 1));
                Assert.That(game.View.SelectedBuildingId, Is.EqualTo(building.Id)); Assert.That(game.View.SelectedCatId, Is.Empty);
                game.Selected.Buildings.Remove(building);
                input.Click(farm.Position); yield return null; yield return null;
                Assert.That(game.View.SelectedBuildingId, Is.Empty, "The existing building miss must clear the old tether subject."); AssertFarmInspectionVisible(farm);

                game.UI.ClosePanel(); var pixel = input.Project(farm.Position);
                typeof(ForestView).GetField("viewedLevel", FarmInspectionFlags).SetValue(game.View, -1);
                input.Click(new Int2(farm.Position.X, farm.Position.Z, -1), false, new Vector2(pixel.x, pixel.y));
                Assert.That(game.UI.OpenSection, Is.Empty, "A surface farm must not match a click at another rendered level.");
                typeof(ForestView).GetField("viewedLevel", FarmInspectionFlags).SetValue(game.View, 0);
                game.Selected.Farms.Remove(farm);
                var foreign = new Village { Id = "inspection-foreign-village", Center = new Int2(200, 200) }; foreign.Farms.Add(farm); game.CurrentWorld.Villages.Add(foreign);
                var foreignUnchanged = FarmInspectionAuthorityUnchanged();
                input.Click(farm.Position); Assert.That(game.UI.OpenSection, Is.Empty, "Only the current village's plots can be picked."); foreignUnchanged();
            }
        }

        [UnityTest]
        public IEnumerator FarmInspectionInputHonorsRealUiTypingPlacementAndDirectControl()
        {
            var farm = PrepareFarmInspectionPlots(0).Single(); SetFarmInspectionSize(1000, 720);
            game.UI.OpenPanel("Work"); yield return null; yield return null;
            using (var input = new FarmInspectionInput(game, Root, false))
            {
                // Keep real interactive classes. Position the actual drawer over the
                // projected ray because batch panel/screen dimensions need not agree.
                var pixel = input.Project(farm.Position);
                var panelPoint = RuntimePanelUtils.ScreenToPanel(Root.panel, new Vector2(pixel.x, Screen.height - pixel.y));
                var drawer = Root.Q<VisualElement>(className: "drawer");
                drawer.style.left = panelPoint.x - Root.worldBound.xMin - 20; drawer.style.top = panelPoint.y - Root.worldBound.yMin - 20;
                drawer.style.width = 100; drawer.style.height = 100;
                yield return null; yield return null;
                var unchanged = FarmInspectionAuthorityUnchanged();
                input.Click(farm.Position, true, new Vector2(pixel.x, pixel.y)); Assert.That(game.UI.OpenSection, Is.EqualTo("Work"));
                Assert.That(Root.Q<ScrollView>(className: "drawer-scroll").scrollOffset.y, Is.Zero.Within(.5f)); unchanged();
                drawer.style.left = drawer.style.top = drawer.style.width = drawer.style.height = StyleKeyword.Null;
                foreach (var element in Root.Query<VisualElement>(className: "interactive").ToList()) element.RemoveFromClassList("interactive");
                game.UI.OpenPanel("Research"); yield return null;
                var search = Root.Query<TextField>().ToList().Single(f => f.label == "Search studies"); search.Focus();
                Assert.That(game.UI.TextInputFocused, Is.True);
                input.Click(farm.Position); Assert.That(game.UI.OpenSection, Is.EqualTo("Research")); unchanged(); search.Blur();
                game.UI.OpenPanel("Work"); Submit("Designate grain farm");
                input.Click(farm.Position); RefreshFarmFeedbackWithoutAuthorityChanges(); AssertPlacementHint("Choose opposite corner");
                Assert.That(game.UI.OpenSection, Is.Empty); unchanged();
                // The second point is intercepted even though this overlap is rejected.
                int revision = game.Revision;
                input.Click(new Int2(farm.Position.X + 1, farm.Position.Z + 1));
                Assert.That(game.UI.OpenSection, Is.Empty); Assert.That(game.Revision, Is.EqualTo(revision)); Assert.That(game.Selected.Farms.Count, Is.EqualTo(1));
                game.UI.ClosePanel();
                var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId && c.JobId == "" && c.BuildingId == "");
                pixel = input.Project(farm.Position);
                Assert.That(game.Send(new GameAction { Kind = "EnterCatControl", CatId = cat.Id }).Result.Success, Is.True);
                var controlledUnchanged = FarmInspectionAuthorityUnchanged();
                // Keep the isolated management ray over the plot while checking the
                // input-mode guard. This is not close-camera verification.
                input.Click(farm.Position, false, new Vector2(pixel.x, pixel.y)); Assert.That(game.View.DirectControl, Is.True); Assert.That(game.View.ControlledCat.Id, Is.EqualTo(cat.Id));
                Assert.That(game.UI.OpenSection, Is.Empty); controlledUnchanged();
                Assert.That(game.Send(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id }).Result.Success, Is.True);
                input.Click(farm.Position); yield return null; yield return null; AssertFarmInspectionVisible(farm);
            }
        }

        [UnityTest]
        public IEnumerator FarmInspectionPendingRevealCancelsOnNavigationModeAndMissingIdentity()
        {
            var plots = PrepareFarmInspectionPlots(24); var deep = plots.Last(); SetFarmInspectionSize(1000, 720);
            yield return null;
            var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
            foreach (string change in new[] { "close", "category", "rebuild", "ground", "level", "control", "village", "delete" })
            {
                game.UI.ClosePanel(); game.UI.InspectFarm(deep.Id);
                var village = game.Selected; var originalPosition = deep.Position;
                Cat controlled = null;
                if (change == "close") game.UI.ClosePanel();
                if (change == "category") game.UI.OpenPanel("Village");
                if (change == "rebuild") game.UI.OpenPanel("Work");
                if (change == "ground") using (var input = new FarmInspectionInput(game, Root)) input.Click(new Int2(deep.Position.X + 2, deep.Position.Z));
                if (change == "level") typeof(ForestView).GetField("viewedLevel", FarmInspectionFlags).SetValue(game.View, -1);
                if (change == "control") { controlled = village.Cats.First(c => c.Alive); controlled.ControlledBy = game.PlayerId; }
                if (change == "village") village.Id = "inspection-temporary-other-id";
                if (change == "delete") { village.Farms.Remove(deep); village.Farms.Add(new Farm { Id = "inspection-new-id", Position = originalPosition, Width = 2, Depth = 2 }); }
                scroll.scrollOffset = Vector2.zero;
                var unchanged = FarmInspectionAuthorityUnchanged();
                yield return null; yield return null; yield return null;
                Assert.That(scroll.scrollOffset.y, Is.Zero.Within(.5f), "An obsolete reveal must not scroll after " + change); unchanged();
                if (change == "level") typeof(ForestView).GetField("viewedLevel", FarmInspectionFlags).SetValue(game.View, 0);
                if (controlled != null) controlled.ControlledBy = "";
                if (change == "village") village.Id = game.SelectedVillageId;
                if (change == "delete") { village.Farms.RemoveAll(f => f.Id == "inspection-new-id"); village.Farms.Add(deep); }
            }
            game.UI.ClosePanel(); game.UI.InspectFarm(deep.Id); game.UI.InspectFarm(plots[0].Id);
            yield return null; yield return null; yield return null;
            AssertFarmInspectionVisible(plots[0]);
            game.UI.ClosePanel(); game.UI.InspectFarm("inspection-removed-id");
            Assert.That(game.UI.OpenSection, Is.Empty, "A missing ID cannot fall back to the same coordinates or another plot.");
        }

        [UnityTest]
        public IEnumerator FarmInspectionSameIdSnapshotRevealsCurrentCardAndUsesExistingExactCommands()
        {
            var farms = PrepareFarmCards(); game.View.enabled = false; SetFarmInspectionSize(1000, 720);
            RefreshFarmInspectionSetupFeedback();
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = farms.Replacement.Name + " · " + farms.Replacement.Id;
            game.UI.InspectFarm(farms.Automatic.Id);
            var old = game.Selected; var current = JsonUtility.FromJson<Village>(JsonUtility.ToJson(old));
            current.Farms.Reverse();
            var target = current.Farms.Single(f => f.Id == farms.Automatic.Id); target.Phase = "mature"; target.BlockedReason = "worker_required";
            target.Position = new Int2(target.Position.X, target.Position.Z + 1);
            target.Handoff = new Int2(target.Position.X - 1, target.Position.Z);
            game.CurrentWorld.Villages[game.CurrentWorld.Villages.IndexOf(old)] = current;
            var unchanged = FarmInspectionAuthorityUnchanged();
            RefreshFarmFeedbackWithoutAuthorityChanges(); yield return null; yield return null;
            var card = AssertFarmInspectionVisible(target); unchanged();
            Assert.That(FarmCardCopy(card), Does.Contain("grain · mature").And.Contain("Waiting for a worker"));
            Submit(card.Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            var worker = current.Cats.Single(c => c.Id == farms.Replacement.Id);
            Assert.That(target.WorkerId, Is.EqualTo(worker.Id)); Assert.That(worker.BuildingId, Is.EqualTo(target.Id));
            Assert.That(current.Farms.Single(f => f.Id == farms.Manual.Id).WorkerId, Is.EqualTo(farms.ManualWorker.Id));
            RefreshFarmInspectionSetupFeedback();
            card = WorkFarmCard(target.Id);
            Assert.That(FarmCardCopy(card), Does.Contain(worker.Name).And.Contain(worker.Id).And.Contain("Assignment · Manual").And.Not.Contain("Waiting for a worker"));
            Submit(card.Query<Button>().ToList().Single(b => b.text == "Clear farm"));
            Assert.That(current.Farms.Select(f => f.Id), Is.EqualTo(new[] { farms.Manual.Id })); Assert.That(worker.BuildingId, Is.Empty);
            Assert.That(old.Farms.Count, Is.EqualTo(2)); Assert.That(farms.Automatic.WorkerId, Is.Empty, "The obsolete snapshot must remain untouched.");
            Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [Test]
        public void AreaPlacementGrainFirstCornerShowsOppositeCornerWithoutChangingAuthority()
        {
            game.UI.OpenPanel("Work"); Submit("hunt");
            Assert.That(game.LastAction, Does.Contain("accepted"), "Keep real command feedback and an existing order while choosing the farm corner.");
            AssertFirstPlacementPoint("Work", "Designate grain farm", "Choose opposite corner");
        }

        [TestCase("Work", "Designate catnip farm")]
        [TestCase("Work", "Designate herbs farm")]
        [TestCase("Work", "Designate gathering")]
        [TestCase("Build", "Mark gathering zone")]
        [TestCase("Build", "Mark avoid zone")]
        [TestCase("Stores", "Designate general stockpile")]
        [TestCase("Stores", "Designate selected-resource pile")]
        public void AreaPlacementOtherFirstCornersShowOppositeCornerWithoutChangingAuthority(string section, string tool)
        {
            AssertFirstPlacementPoint(section, tool, "Choose opposite corner");
        }

        [TestCase("Build", "Build stone road")]
        [TestCase("Routes", "Lay rail")]
        public void AreaPlacementRoadAndRailKeepEndOfRouteHint(string section, string tool)
        {
            AssertFirstPlacementPoint(section, tool, "Choose end of route");
        }

        [TestCase("Build", "Build stone road", "Road: choose start, then end")]
        [TestCase("Routes", "Lay rail", "Rail: choose start, then end")]
        public void AreaPlacementCancellationAndSwitchingCannotLeakHints(string section, string tool, string initialHint)
        {
            AssertFirstPlacementPoint("Work", "Designate grain farm", "Choose opposite corner");
            // Management Esc calls this same public cancellation method.
            AssertPlacementUiOnly(() => game.UI.ClosePanel());
            AssertNoPlacementHint();

            AssertPlacementUiOnly(() => { game.UI.OpenPanel(section); Submit(tool); });
            AssertPlacementHint(initialHint);
            var point = new Int2(game.Selected.Center.X + game.Selected.Radius + 1, game.Selected.Center.Z + 1);
            AssertPlacementUiOnly(() => PlaceUiTile(point));
            AssertPlacementHint("Choose end of route");

            // Selecting another tool must replace the route's first point and prompt.
            AssertPlacementUiOnly(() => { game.UI.OpenPanel("Work"); Submit("Designate herbs farm"); });
            AssertPlacementHint("Exterior herbs farm: choose opposite corners");
            AssertPlacementUiOnly(() => PlaceUiTile(new Int2(point.X + 1, point.Z + 1)));
            AssertPlacementHint("Choose opposite corner");
            AssertPlacementUiOnly(() => game.UI.ClosePanel());
            AssertNoPlacementHint();
        }

        [Test]
        public void AreaPlacementSingleClickFishingSubmitsOnceAfterCancellation()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            game.UI.OpenPanel("Work"); Submit("Designate grain farm");
            AssertPlacementUiOnly(() => PlaceUiTile(new Int2(village.Center.X + village.Radius + 1, village.Center.Z + 1)));
            Assert.That(game.UI.HasPlacement, Is.True);
            AssertPlacementUiOnly(() => game.UI.ClosePanel());
            AssertNoPlacementHint();
            AssertPlacementUiOnly(() => { game.UI.OpenPanel("Work"); Submit("Designate fishing"); });
            AssertPlacementHint("Choose a known dry shore tile beside water");

            // Reuse the founding world's mapped dry shore beside its starter water.
            var shore = new Int2(village.Center.X + 2, village.Center.Z + village.Radius + 1);
            Assert.That(village.Known.Contains(shore) && world.Walkable(shore), Is.True);
            Assert.That(world.TileAt(new Int2(shore.X, shore.Z + 1)).Water, Is.True);
            var oldPiles = village.Stockpiles.Select(p => p.Id).ToArray();
            int revision = game.Revision; double pausedAt = world.TimeSeconds;
            PlaceUiTile(shore);
            Assert.That(game.LastAction, Does.Contain("accepted"));
            Assert.That(game.Revision, Is.EqualTo(revision + 1), "A single click must dispatch exactly one accepted command.");
            Assert.That(village.Stockpiles.Count, Is.EqualTo(oldPiles.Length + 1));
            CollectionAssert.IsSubsetOf(oldPiles, village.Stockpiles.Select(p => p.Id).ToArray());
            var fishing = village.Stockpiles.Single(p => !oldPiles.Contains(p.Id));
            Assert.That(fishing.Kind, Is.EqualTo("fishing")); Assert.That(fishing.Position, Is.EqualTo(shore));
            Assert.That(fishing.Width, Is.EqualTo(1)); Assert.That(fishing.Depth, Is.EqualTo(1));
            RefreshFarmFeedbackWithoutAuthorityChanges();
            AssertNoPlacementHint();
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.Validate(), Is.Empty);
        }

        [Test]
        public void AreaPlacementReversedFarmCornersSubmitExactInclusivePlot()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            var origin = new Int2(village.Center.X + village.Radius + 6, village.Center.Z);
            // Reuse the exterior support-terrain pattern from the farm rendering test.
            for (int x = village.Radius + 1; x <= village.Radius + 10; x++)
                for (int z = -4; z <= 3; z++)
                {
                    var at = new Int2(village.Center.X + x, village.Center.Z + z);
                    var tile = world.TileAt(at); World.ClearSurface(tile, 0);
                    tile.Wall = tile.Bridge = tile.Road = tile.Rail = false; tile.ClaimId = "";
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
            village.Buildings.Add(new Building { Id = world.Id("ui-placement-field"), Kind = "field", Position = new Int2(origin.X, origin.Z - 4), Completed = true });
            Assert.That(world.Path(village.Center, origin), Is.Not.Null, "The UI must submit a reachable exterior plot.");
            Assert.That(world.Validate(), Is.Empty);
            var oldFarms = village.Farms.Select(f => f.Id).ToArray();
            double pausedAt = world.TimeSeconds;
            game.UI.OpenPanel("Work"); Submit("Designate grain farm");
            AssertPlacementUiOnly(() => PlaceUiTile(new Int2(origin.X + 3, origin.Z + 2)));
            AssertPlacementHint("Choose opposite corner");
            int revision = game.Revision;
            PlaceUiTile(origin);
            Assert.That(game.LastAction, Does.Contain("accepted"));
            Assert.That(game.Revision, Is.EqualTo(revision + 1));
            Assert.That(village.Farms.Count, Is.EqualTo(oldFarms.Length + 1));
            CollectionAssert.IsSubsetOf(oldFarms, village.Farms.Select(f => f.Id).ToArray());
            var farm = village.Farms.Single(f => !oldFarms.Contains(f.Id));
            Assert.That(farm.Crop, Is.EqualTo("grain")); Assert.That(farm.Position, Is.EqualTo(origin));
            Assert.That(farm.Width, Is.EqualTo(4)); Assert.That(farm.Depth, Is.EqualTo(3));
            RefreshFarmFeedbackWithoutAuthorityChanges();
            AssertNoPlacementHint();
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.Validate(), Is.Empty);
        }

        [UnityTest]
        public IEnumerator AssignedWorkFollowsPublicFarmApproachGrowthAndPausedRelease()
        {
            var farms = PrepareFarmCards(); var world = game.CurrentWorld;
            var worker = farms.ManualWorker; var farm = farms.Manual;
            Assert.That(game.Speed, Is.Zero);
            Assert.That(worker.BuildingId, Is.EqualTo(farm.Id)); Assert.That(worker.JobId, Is.Empty);
            Assert.That(farm.Growth, Is.Zero); AssertAssignedWorkHeader(1);

            double startX = worker.X, startZ = worker.Z;
            world.Step(.1);
            Assert.That(Math.Abs(worker.X - startX) + Math.Abs(worker.Z - startZ), Is.GreaterThan(0), "The public farm assignment must actually approach its plot before testing its retained count.");
            Assert.That(worker.Position, Is.Not.EqualTo(farm.Position)); Assert.That(farm.Growth, Is.Zero);
            AssertAssignedWorkHeader(1);
            for (int step = 0; step < 120 && farm.Growth == 0; step++) world.Step(.25);
            Assert.That(worker.Position, Is.EqualTo(farm.Position), "The bounded route must reach the real plot.");
            Assert.That(farm.Growth, Is.GreaterThan(0), "The assigned cat must perform real work without growth credit.");
            Assert.That(farm.Harvest, Is.Zero); Assert.That(worker.JobId, Is.Empty); AssertAssignedWorkHeader(1);

            double pausedAt = world.TimeSeconds;
            var released = game.Send(new GameAction { Kind = "AssignWorker", CatId = worker.Id }).Result;
            Assert.That(released.Success, Is.True, released.Error);
            Assert.That(farm.WorkerId, Is.Empty); Assert.That(worker.BuildingId, Is.Empty);
            AssertAssignedWorkHeader(0);
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt), "Public release and its summary refresh must work while simulation time stays paused.");
            Assert.That(world.Validate(), Is.Empty);
            yield return null;
        }

        [UnityTest]
        public IEnumerator AssignedWorkCountsDistinctRetainedSnapshotCommitments()
        {
            var cats = game.Selected.Cats;
            // These explicit presentation snapshots cover retained markers, not
            // execution of research, accounting, equipment or transport systems.
            foreach (var cat in cats)
            {
                cat.JobId = cat.ResumeJobId = cat.BuildingId = cat.OfficerRole = cat.ControlledBy = "";
                cat.Goal = "idle"; cat.Cargo.Clear();
            }
            cats[0].OfficerRole = "leader"; // An office alone is not a work assignment.
            cats[1].JobId = "snapshot-order";
            cats[2].BuildingId = "snapshot-farm";
            cats[3].JobId = "snapshot-batch"; cats[3].BuildingId = "snapshot-mill";
            cats[4].BuildingId = "snapshot-research-hut";
            cats[5].BuildingId = "snapshot-accounting-tent"; cats[5].Goal = "accounting travel";
            cats[6].BuildingId = "snapshot-route"; cats[6].BlockedReason = "blocked_route";
            cats[7].ResumeJobId = "snapshot-equipment"; cats[7].Goal = "need_drink";
            cats[8].JobId = cats[8].ResumeJobId = "snapshot-current-and-resume"; cats[8].Goal = "need_eat";
            cats[9].BuildingId = "snapshot-paused-empty-station";
            game.Selected.Buildings.Add(new Building
            {
                Id = cats[9].BuildingId, Kind = "mill", Completed = true, Paused = true, WorkerId = cats[9].Id,
                Slots = new System.Collections.Generic.List<WorkSlot> { new WorkSlot { CatId = cats[9].Id, Paused = true } }
            });
            cats[10].Alive = false; cats[10].JobId = cats[10].ResumeJobId = cats[10].BuildingId = "snapshot-dead-work";
            cats[11].ControlledBy = "snapshot-direct"; cats[11].Goal = "player_control"; cats[11].Cargo.Add(new Simulation.Stack("logs", 2));
            cats[12].Goal = "need_drink";
            cats[13].ControlledBy = "snapshot-direct-return"; cats[13].BuildingId = "snapshot-return-route";
            AssertAssignedWorkHeader(10, 29);

            cats[3].JobId = ""; cats[8].ResumeJobId = "";
            AssertAssignedWorkHeader(10, 29); // Remaining workplace/current orders still commit the same cats.
            cats[7].ResumeJobId = ""; cats[8].JobId = ""; cats[9].BuildingId = "";
            AssertAssignedWorkHeader(7, 29);
            yield return null;
        }

        [UnityTest]
        public IEnumerator AssignedWorkFollowsPublicTakeoverRetainedReturnAndReplacementSnapshot()
        {
            var farms = PrepareFarmCards(); var world = game.CurrentWorld; var village = game.Selected;
            var worker = farms.ManualWorker;
            AssertAssignedWorkHeader(1);
            double pausedAt = world.TimeSeconds;
            var takeover = game.Send(new GameAction { Kind = "EnterCatControl", CatId = worker.Id }).Result;
            Assert.That(takeover.Success, Is.True, takeover.Error);
            Assert.That(worker.ControlledBy, Is.EqualTo(game.PlayerId));
            Assert.That(farms.Manual.WorkerId, Is.Empty); Assert.That(worker.BuildingId, Is.Empty);
            AssertAssignedWorkHeader(0);
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.Validate(), Is.Empty);

            // A supported retained-return marker is synthetic here. This does
            // not claim that a loaded vessel has physically returned in play.
            worker.BuildingId = "snapshot-retained-return";
            AssertAssignedWorkHeader(1);
            worker.BuildingId = "";
            AssertAssignedWorkHeader(0);

            var replacement = JsonUtility.FromJson<Village>(JsonUtility.ToJson(village));
            replacement.Cats[4].JobId = "snapshot-new-order";
            replacement.Cats[5].BuildingId = "snapshot-new-position";
            replacement.Cats[6].ResumeJobId = "snapshot-new-resume";
            world.Villages[world.Villages.IndexOf(village)] = replacement;
            Assert.That(game.Selected, Is.SameAs(replacement));
            AssertAssignedWorkHeader(3);
            Assert.That(village.Cats[4].JobId, Is.Empty, "The former village objects must remain separate from the replacement snapshot.");
            yield return null;
        }

        [UnityTest]
        public IEnumerator StaleFarmCardMissingPlotSendsNoCommandOrFeedback()
        {
            var farms = PrepareFarmCards(); var village = game.Selected;
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            var stale = WorkFarmCard(farms.Manual.Id).Query<Button>().ToList();
            Assert.That(game.Send(new GameAction { Kind = "ClearFarm", TargetId = farms.Manual.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = farms.ManualWorker.Id, TargetId = farms.Automatic.Id }).Result.Success, Is.True);
            Assert.That(stale.All(b => b.panel == Root.panel), Is.True, "No UI refresh has removed these controls. The missing-ID check must protect an attached stale card.");
            Assert.That(village.Farms.Any(f => f.Id == farms.Manual.Id), Is.False);
            AssertFarmCardCallbacksIgnored(stale);
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.ManualWorker.Id));
            Assert.That(farms.ManualWorker.BuildingId, Is.EqualTo(farms.Automatic.Id));

            RefreshPausedUiWithoutAuthorityChanges();
            Assert.That(stale.All(b => b.panel == null), Is.True);
            AssertFarmCardCallbacksIgnored(stale);
            Assert.That(game.CurrentWorld.Validate(), Is.Empty);
            yield return null;
        }

        [UnityTest]
        public IEnumerator StaleFarmCardClosedRebuiltOrSwitchedVillageSendsNoCommand()
        {
            var farms = PrepareFarmCards(); var village = game.Selected;
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = farms.Replacement.Name + " · " + farms.Replacement.Id;
            var closed = WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList();
            game.UI.ClosePanel();
            Assert.That(closed.All(b => b.panel == Root.panel), Is.True, "Closing Work hides its existing content; its callbacks must still reject activation.");
            AssertFarmCardCallbacksIgnored(closed);

            game.UI.OpenPanel("Work");
            var rebuilt = WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList();
            game.UI.OpenPanel("Work");
            Assert.That(rebuilt.All(b => b.panel == null), Is.True);
            AssertFarmCardCallbacksIgnored(rebuilt);
            Assert.That(village.Farms.Any(f => f.Id == farms.Automatic.Id), Is.True);

            var switched = WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList();
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "Stale farm card isolation" }).Result.Success, Is.True);
            Assert.That(game.Selected.Id, Is.Not.EqualTo(village.Id));
            Assert.That(switched.All(b => b.panel == Root.panel), Is.True, "The old village's card remains attached until the next refresh.");
            AssertFarmCardCallbacksIgnored(switched);
            RefreshPausedUiWithoutAuthorityChanges();
            Assert.That(switched.All(b => b.panel == null), Is.True);
            AssertFarmCardCallbacksIgnored(switched);
            Assert.That(village.Farms.Any(f => f.Id == farms.Automatic.Id), Is.True);
            Assert.That(farms.Manual.WorkerId, Is.EqualTo(farms.ManualWorker.Id));
            Assert.That(game.CurrentWorld.Validate(), Is.Empty);
            yield return null;
        }

        [UnityTest]
        public IEnumerator StaleFarmCardSameIdSnapshotUsesCurrentSelectionAndActions()
        {
            var farms = PrepareFarmCards(); var world = game.CurrentWorld; var original = game.Selected;
            double pausedAt = world.TimeSeconds;
            game.UI.OpenPanel("Work");
            var choice = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
            choice.value = farms.ManualWorker.Name + " · " + farms.ManualWorker.Id;
            var assign = WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat");
            choice.value = farms.Replacement.Name + " · " + farms.Replacement.Id;
            // Full same-ID object replacement models an authority snapshot.
            // It is presentation evidence, not shared-server or native play.
            var current = JsonUtility.FromJson<Village>(JsonUtility.ToJson(original));
            world.Villages[world.Villages.IndexOf(original)] = current;
            var currentFarm = current.Farms.Single(f => f.Id == farms.Automatic.Id);
            var currentCat = current.Cats.Single(c => c.Id == farms.Replacement.Id);
            Assert.That(currentFarm, Is.Not.SameAs(farms.Automatic));
            InvokeFarmCardCallback(assign);
            Assert.That(currentFarm.WorkerId, Is.EqualTo(currentCat.Id), "A retained valid control must use the current chooser, not the cat selected when the card was built.");
            Assert.That(currentCat.BuildingId, Is.EqualTo(currentFarm.Id));
            Assert.That(farms.Automatic.WorkerId, Is.Empty); Assert.That(farms.Replacement.BuildingId, Is.Empty);
            Assert.That(current.Farms.Single(f => f.Id == farms.Manual.Id).WorkerId, Is.EqualTo(farms.ManualWorker.Id));

            var clear = WorkFarmCard(currentFarm.Id).Query<Button>().ToList().Single(b => b.text == "Clear farm");
            var next = JsonUtility.FromJson<Village>(JsonUtility.ToJson(current));
            world.Villages[world.Villages.IndexOf(current)] = next;
            InvokeFarmCardCallback(clear);
            Assert.That(next.Farms.Any(f => f.Id == currentFarm.Id), Is.False, "Clear must resolve the current same-ID snapshot too.");
            Assert.That(next.Cats.Single(c => c.Id == currentCat.Id).BuildingId, Is.Empty);
            Assert.That(current.Farms.Any(f => f.Id == currentFarm.Id), Is.True);
            Assert.That(next.Farms.Single(f => f.Id == farms.Manual.Id).WorkerId, Is.EqualTo(farms.ManualWorker.Id));
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(world.Validate(), Is.Empty);
            yield return null;
        }

        [Test]
        public void FarmsIdentifyExactPlotWorkerAndOwnership()
        {
            var farms = PrepareFarmCards(); var village = game.Selected;
            Assert.That(game.Send(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[0].Id }).Result.Success, Is.True);
            game.CurrentWorld.Step(10);
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.FarmerWorker.Id), "The fixture must obtain a real public Farmer assignment before checking its presentation.");
            Assert.That(farms.Automatic.AutomatedBy, Is.EqualTo("farmer"));
            Assert.That(farms.Manual.Crop, Is.EqualTo(farms.Automatic.Crop)); Assert.That(farms.Manual.Phase, Is.EqualTo(farms.Automatic.Phase));
            game.UI.OpenPanel("Work");
            string manualCopy = FarmCardCopy(WorkFarmCard(farms.Manual.Id)), automaticCopy = FarmCardCopy(WorkFarmCard(farms.Automatic.Id));
            Assert.That(manualCopy, Does.Contain(farms.Manual.Position.ToString()).And.Contain(farms.ManualWorker.Name).And.Contain(farms.ManualWorker.Id).And.Contain("Manual"), "A farm's exact plot and manual worker must be readable beside its own controls.");
            Assert.That(automaticCopy, Does.Contain(farms.Automatic.Position.ToString()).And.Contain(farms.FarmerWorker.Name).And.Contain(farms.FarmerWorker.Id).And.Contain("Farmer"));
            Assert.That(manualCopy, Is.Not.EqualTo(automaticCopy), "Same-crop, same-phase plots must have distinguishable cards.");
            foreach (var farm in new[] { farms.Manual, farms.Automatic })
            {
                var buttons = WorkFarmCard(farm.Id).Query<Button>().ToList();
                Assert.That(buttons.Count(b => b.text == "Assign selected cat"), Is.EqualTo(1)); Assert.That(buttons.Count(b => b.text == "Clear farm"), Is.EqualTo(1));
            }
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = farms.Replacement.Name + " · " + farms.Replacement.Id;
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.Replacement.Id)); Assert.That(farms.Replacement.BuildingId, Is.EqualTo(farms.Automatic.Id));
            Assert.That(farms.FarmerWorker.BuildingId, Is.Empty);
            Assert.That(farms.Automatic.AutomatedBy, Is.Empty, "A real manual reassignment must clear the former Farmer ownership before the card calls it Manual.");
            automaticCopy = FarmCardCopy(WorkFarmCard(farms.Automatic.Id));
            Assert.That(automaticCopy, Does.Contain(farms.Replacement.Name).And.Contain(farms.Replacement.Id).And.Contain("Manual").And.Not.Contain("Farmer"));
            Assert.That(farms.Manual.WorkerId, Is.EqualTo(farms.ManualWorker.Id)); Assert.That(farms.ManualWorker.BuildingId, Is.EqualTo(farms.Manual.Id)); Assert.That(farms.Manual.AutomatedBy, Is.Empty);
            Submit(WorkFarmCard(farms.Automatic.Id).Query<Button>().ToList().Single(b => b.text == "Clear farm"));
            Assert.That(village.Farms.Select(f => f.Id), Is.EqualTo(new[] { farms.Manual.Id }), "Clear must remove only the exact plot identified beside that button.");
            Assert.That(farms.Replacement.BuildingId, Is.Empty); Assert.That(farms.Manual.WorkerId, Is.EqualTo(farms.ManualWorker.Id)); Assert.That(farms.ManualWorker.BuildingId, Is.EqualTo(farms.Manual.Id));
            Assert.That(WorkFarmCard(farms.Manual.Id), Is.Not.Null); Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [UnityTest]
        public IEnumerator FarmFeedbackPausedPublicAssignmentShowsCurrentStaffingWithoutAdvancingAuthority()
        {
            var farms = PrepareFarmCards(); var world = game.CurrentWorld;
            var farm = farms.Automatic; var worker = farms.Replacement;
            world.Step(.1);
            Assert.That(farm.WorkerId, Is.Empty); Assert.That(farm.BlockedReason, Is.EqualTo("worker_required"));
            Assert.That(farm.Growth, Is.Zero); Assert.That(game.Speed, Is.Zero);
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = worker.Name + " · " + worker.Id;
            yield return null;
            var warning = WorkFarmCard(farm.Id).Q<Label>(className: "warning");
            Assert.That(warning.text, Does.Contain("Waiting for a worker"));
            Assert.That(warning.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            double pausedAt = world.TimeSeconds, growth = farm.Growth;
            Submit(WorkFarmCard(farm.Id).Query<Button>().ToList().Single(b => b.text == "Assign selected cat"));
            Assert.That(farm.WorkerId, Is.EqualTo(worker.Id)); Assert.That(worker.BuildingId, Is.EqualTo(farm.Id));
            Assert.That(farm.AutomatedBy, Is.Empty); Assert.That(farm.BlockedReason, Is.EqualTo("worker_required"), "The presentation must handle a real stored staffing warning without rewriting authority.");
            string assigned = JsonUtility.ToJson(world), action = game.LastAction;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            yield return null;
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(assigned)); Assert.That(game.LastAction, Is.EqualTo(action));
            Assert.That(world.TimeSeconds, Is.EqualTo(pausedAt)); Assert.That(farm.Growth, Is.EqualTo(growth));
            var card = WorkFarmCard(farm.Id); warning = card.Q<Label>(className: "warning");
            Assert.That(FarmCardCopy(card), Does.Contain("Worker · " + worker.Name + " · " + worker.Id).And.Contain("Assignment · Manual"));
            Assert.That(warning.text, Is.Empty, "A completed paused Assign must not still ask for a worker.");
            Assert.That(warning.resolvedStyle.display, Is.EqualTo(DisplayStyle.None));
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(assigned));
        }

        [Test]
        public void FarmFeedbackSnapshotGuardsKeepGenuineBlockersAndUnavailableStaffingWarnings()
        {
            var farms = PrepareFarmCards(); var farm = farms.Manual; var worker = farms.ManualWorker;
            game.UI.OpenPanel("Work"); RefreshPausedUiWithoutAuthorityChanges();
            // These authored feedback snapshots guard presentation only. They
            // do not establish physical route failure, harvest or cat revival.
            var blockers = new[]
            {
                (Reason: "blocked_route", Copy: "No walkable route"),
                (Reason: "blocked_harvest_route", Copy: "The harvest cannot reach storage"),
                (Reason: "blocked_handoff_route", Copy: "The field's collection point is unreachable"),
                (Reason: "handoff_full", Copy: "The field's collection point is full")
            };
            var card = WorkFarmCard(farm.Id); var warning = card.Q<Label>(className: "warning");
            foreach (var blocker in blockers)
            {
                farm.BlockedReason = blocker.Reason; RefreshFarmFeedbackWithoutAuthorityChanges();
                Assert.That(WorkFarmCard(farm.Id), Is.SameAs(card));
                Assert.That(warning.text, Does.Contain(blocker.Copy)); Assert.That(warning.style.display.value, Is.EqualTo(DisplayStyle.Flex));
            }
            farm.BlockedReason = "worker_required"; farm.WorkerId = "";
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Does.Contain("Waiting for a worker"));
            farm.WorkerId = "snapshot-missing-farm-worker";
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Does.Contain("Waiting for a worker"));
            Assert.That(FarmCardCopy(card), Does.Contain("snapshot-missing-farm-worker · unavailable"));
            farm.WorkerId = worker.Id; worker.Alive = false;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Does.Contain("Waiting for a worker"));
            worker.Alive = true;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Is.Empty, "A current living worker makes only worker_required obsolete.");
            Assert.That(warning.style.display.value, Is.EqualTo(DisplayStyle.None));
            worker.Alive = false;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Does.Contain("Waiting for a worker"), "The retained card must notice living-to-dead changes with the same worker ID, name and stored reason.");
            Assert.That(warning.style.display.value, Is.EqualTo(DisplayStyle.Flex));
            worker.Alive = true;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Is.Empty); Assert.That(warning.style.display.value, Is.EqualTo(DisplayStyle.None));
            farm.BlockedReason = "blocked_handoff_route";
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Does.Contain("The field's collection point is unreachable"));
            farm.BlockedReason = "";
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(warning.text, Is.Empty); Assert.That(warning.style.display.value, Is.EqualTo(DisplayStyle.None));
        }

        [UnityTest]
        public IEnumerator FarmsRefreshStatusWithoutReplacingWorkChoices()
        {
            var farms = PrepareFarmCards(); var world = game.CurrentWorld; var village = game.Selected;
            // A bounded growth credit exercises a real phase boundary. It does
            // not shorten the ordinary cycle or claim a full crop playthrough.
            farms.Automatic.Growth = farms.Automatic.CycleSeconds / 12 - .5;
            double nextPlanning = (Math.Floor(world.TimeSeconds / 10) + 1) * 10;
            world.Step(nextPlanning - world.TimeSeconds - .05);
            PlaceFarmCat(farms.FarmerWorker, farms.Automatic.Position);
            Assert.That(farms.Automatic.WorkerId, Is.Empty); Assert.That(farms.Automatic.BlockedReason, Is.EqualTo("worker_required"));
            game.UI.OpenPanel("Work");
            var choices = Root.Query<DropdownField>().ToList(); var catChoice = choices.Single(f => f.label == "Cat");
            var farmChoices = village.Farms.ToDictionary(f => f.Id, f => WorkFarmCard(f.Id).Q<DropdownField>("farm-worker-choice-" + f.Id));
            Assert.That(farmChoices.Values, Has.None.Null);
            var nonFarmChoices = choices.Where(f => !farmChoices.Values.Contains(f)).ToArray();
            void ExpectFarmMembership(string addedId, string removedId)
            {
                RefreshPausedUiWithoutAuthorityChanges();
                if (removedId != "")
                {
                    Assert.That(farmChoices[removedId].panel, Is.Null, "A removed farm must detach its chooser rather than retain a ghost field.");
                    farmChoices.Remove(removedId);
                }
                if (addedId != "")
                {
                    var added = WorkFarmCard(addedId).Q<DropdownField>("farm-worker-choice-" + addedId);
                    Assert.That(added, Is.Not.Null);
                    Assert.That(choices.Contains(added), Is.False, "Only the designated farm may introduce a new chooser.");
                    Assert.That(added.value, Is.EqualTo(catChoice.value));
                    farmChoices.Add(addedId, added);
                }
                CollectionAssert.AreEquivalent(village.Farms.Select(f => f.Id), farmChoices.Keys);
                foreach (var farm in village.Farms)
                    Assert.That(WorkFarmCard(farm.Id).Q<DropdownField>("farm-worker-choice-" + farm.Id), Is.SameAs(farmChoices[farm.Id]), "Surviving farms must retain their exact prepared chooser objects.");
                var expected = nonFarmChoices.Concat(village.Farms.Select(f => farmChoices[f.Id])).ToList();
                CollectionAssert.AreEqual(expected, Root.Query<DropdownField>().ToList(), "Farm membership may change only its exact local fields, preserving all other chooser objects and their order.");
                choices.Clear(); choices.AddRange(expected);
            }
            catChoice.value = farms.Replacement.Name + " · " + farms.Replacement.Id;
            yield return null;
            catChoice.Focus(); var focused = Root.focusController.focusedElement;
            Assert.That(focused, Is.Not.Null, "The regression needs actual keyboard focus in the prepared Work form.");
            var scroll = Root.Q<ScrollView>(className: "drawer-scroll"); scroll.scrollOffset = new Vector2(0, 120);
            yield return null;
            Vector2 offset = scroll.scrollOffset;
            Assert.That(offset.y, Is.GreaterThan(50), "The fixture must actually scroll the open Work panel.");
            string Copy() => string.Join("\n", Root.Query<Button>().ToList().Where(b => b.text == "Clear farm").Select(b => FarmCardCopy(b.parent)));
            Assert.That(Copy(), Does.Contain("Waiting for a worker"), "The starting open panel must show the genuine vacant-plot warning.");
            void RefreshUnchanged()
            {
                RefreshPausedUiWithoutAuthorityChanges();
                CollectionAssert.AreEqual(choices, Root.Query<DropdownField>().ToList(), "Live farm feedback must preserve every prepared Work chooser object.");
                Assert.That(catChoice.value, Is.EqualTo(farms.Replacement.Name + " · " + farms.Replacement.Id));
                Assert.That(Root.focusController.focusedElement, Is.SameAs(focused), "Farm status must retain keyboard focus in the prepared form.");
                Assert.That(scroll.scrollOffset, Is.EqualTo(offset), "Farm feedback must retain the open drawer's scroll position.");
                Assert.That(game.UI.OpenSection, Is.EqualTo("Work"));
            }
            Assert.That(game.Send(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[0].Id }).Result.Success, Is.True);
            world.Step(.1);
            Assert.That(farms.Automatic.WorkerId, Is.EqualTo(farms.FarmerWorker.Id)); Assert.That(farms.Automatic.BlockedReason, Is.Empty);
            RefreshUnchanged();
            Assert.That(Copy(), Does.Not.Contain("Waiting for a worker"), "A real Farmer assignment must clear the obsolete warning in the same open panel.");
            Assert.That(FarmCardCopy(WorkFarmCard(farms.Automatic.Id)), Does.Contain(farms.FarmerWorker.Name).And.Contain(farms.FarmerWorker.Id).And.Contain("Farmer"));
            world.Step(1);
            Assert.That(farms.Automatic.Phase, Is.EqualTo("sprout"), "The assigned cat must perform real remaining work at the plot before its displayed phase changes.");
            RefreshUnchanged();
            Assert.That(FarmCardCopy(WorkFarmCard(farms.Automatic.Id)), Does.Contain("sprout"));

            // This replacement and rename model a fresh authority snapshot;
            // they are presentation evidence, not physical farm relocation.
            var replacement = JsonUtility.FromJson<Farm>(JsonUtility.ToJson(farms.Automatic));
            replacement.Position = new Int2(11, 3); replacement.Handoff = new Int2(11, 2); replacement.Phase = "mature"; replacement.BlockedReason = "blocked_route";
            village.Farms[village.Farms.IndexOf(farms.Automatic)] = replacement; farms.FarmerWorker.Name = "Renamed Farmer Worker";
            RefreshUnchanged();
            Assert.That(FarmCardCopy(WorkFarmCard(replacement.Id)), Does.Contain(replacement.Position.ToString()).And.Contain(farms.FarmerWorker.Name).And.Contain("mature").And.Contain("No walkable route"), "Cards must resolve the current same-ID farm and cat from the current village.");
            var designated = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 5), End = new Int2(10, 5) }).Result;
            Assert.That(designated.Success, Is.True, designated.Error);
            ExpectFarmMembership(designated.EntityId, "");
            RefreshUnchanged(); Assert.That(WorkFarmCard(designated.EntityId), Is.Not.Null, "New plot membership must appear while Work remains open.");
            Assert.That(game.Send(new GameAction { Kind = "ClearFarm", TargetId = replacement.Id }).Result.Success, Is.True);
            ExpectFarmMembership("", replacement.Id);
            RefreshUnchanged();
            Assert.That(Root.Query<Button>().ToList().Count(b => b.text == "Clear farm"), Is.EqualTo(2));
            Assert.That(Copy(), Does.Not.Contain(replacement.Id), "Removed plots must disappear from the still-open panel.");
            Assert.That(WorkFarmCard(farms.Manual.Id), Is.Not.Null); Assert.That(WorkFarmCard(designated.EntityId), Is.Not.Null);
        }

        [Test]
        public void RoutesDestinationSurvivesNavigationActionsAndRemovedPileOrVillageChanges()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            var drivers = village.Cats.Where(c => c.Alive && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(drivers.Length, Is.EqualTo(2));
            var draft = PrepareDispatchLane("draft", 25);
            var other = PrepareDispatchLane("other", 28);
            var otherCreated = game.Send(new GameAction { Kind = "CreateTransportRoute", CatId = drivers[1].Id, TargetId = other.Source.Id, BuildingId = other.Destination.Id, Mode = "shipping", Resource = "logs", Amount = 4, Path = other.Path, Repeat = false }).Result;
            Assert.That(otherCreated.Success, Is.True, otherCreated.Error);
            game.UI.OpenPanel("Routes");
            DropdownField Choice(string label) => Root.Query<DropdownField>().ToList().Single(f => f.label == label);
            Choice("Mode").value = "shipping"; Choice("Cat").value = drivers[0].Name + " · " + drivers[0].Id;
            Choice("Resource").value = "logs"; Choice("Source pile").value = draft.Source.Id; Choice("Destination pile").value = draft.Destination.Id;
            Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount").value = 4;
            void AssertDraft()
            {
                Assert.That(Choice("Destination pile").value, Is.EqualTo(draft.Destination.Id), "A valid prepared destination must survive navigation and action-triggered full Routes rebuilds.");
                Assert.That(Choice("Source pile").value, Is.EqualTo(draft.Source.Id));
                Assert.That(Choice("Cat").value, Is.EqualTo(drivers[0].Name + " · " + drivers[0].Id));
                Assert.That(Choice("Mode").value, Is.EqualTo("shipping")); Assert.That(Choice("Resource").value, Is.EqualTo("logs"));
                Assert.That(Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount").value, Is.EqualTo(4));
            }
            string authority = JsonUtility.ToJson(world), lastAction = game.LastAction;
            game.UI.OpenPanel("Stores"); game.UI.OpenPanel("Routes"); AssertDraft();
            game.UI.OpenPanel("Routes"); AssertDraft();
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(authority), "Preparing and reopening a dispatch form must not rewrite authority.");
            Assert.That(game.LastAction, Is.EqualTo(lastAction));
            Submit("Cancel route");
            Assert.That(village.Routes, Is.Empty, "The existing route's normal UI Cancel must complete before dispatching the draft.");
            AssertDraft();
            Submit("Create transport route");
            var created = village.Routes.Single();
            Assert.That(created.CatId, Is.EqualTo(drivers[0].Id)); Assert.That(created.SourceId, Is.EqualTo(draft.Source.Id)); Assert.That(created.DestinationId, Is.EqualTo(draft.Destination.Id));
            Assert.That(created.Amount, Is.EqualTo(4)); Assert.That(created.Mode, Is.EqualTo("shipping")); Assert.That(created.Resource, Is.EqualTo("logs"));
            CollectionAssert.AreEqual(draft.Path, created.Path);
            Assert.That(draft.Source.Goods.Single().Amount, Is.EqualTo(8), "Dispatch must not instantly load or deliver its finite cargo.");
            Assert.That(draft.Destination.Goods, Is.Empty); Assert.That(draft.Vehicle.Cargo, Is.Empty); Assert.That(created.Phase, Is.EqualTo("boarding"));
            AssertDraft(); Submit("Cancel route"); AssertDraft();

            var destinationChoice = Choice("Destination pile"); destinationChoice.Focus(); var focused = Root.focusController.focusedElement;
            var removed = game.Send(new GameAction { Kind = "RemoveStockpile", TargetId = draft.Destination.Id }).Result;
            Assert.That(removed.Success, Is.True, removed.Error);
            RefreshPausedUiWithoutAuthorityChanges();
            Assert.That(Choice("Destination pile"), Is.SameAs(destinationChoice), "Removing a destination must update that chooser without replacing the prepared form.");
            Assert.That(Root.focusController.focusedElement, Is.SameAs(focused));
            Assert.That(destinationChoice.choices, Does.Not.Contain(draft.Destination.Id));
            Assert.That(destinationChoice.value, Is.EqualTo(village.Stockpiles[0].Id), "A missing destination falls back visibly to the first current pile.");
            Submit("Create transport route");
            Assert.That(village.Routes.Any(r => r.DestinationId == draft.Destination.Id), Is.False, "A removed hidden destination must never receive a dispatch.");
            Assert.That(draft.Source.Goods.Single().Amount, Is.EqualTo(8));

            var oldCreate = FindButton("Create transport route");
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "Route draft isolation" }).Result.Success, Is.True);
            var personal = game.Selected;
            Assert.That(personal.Id, Is.Not.EqualTo(village.Id));
            lastAction = game.LastAction; Submit(oldCreate);
            Assert.That(game.LastAction, Is.EqualTo(lastAction), "A stale form from another village must rebuild before sending any command.");
            RefreshPausedUiWithoutAuthorityChanges();
            Assert.That(Choice("Destination pile").value, Is.EqualTo(personal.Stockpiles[0].Id), "Changing village starts a destination choice from that village's own piles.");
            CollectionAssert.AreEqual(personal.Stockpiles.Select(p => p.Id), Choice("Destination pile").choices);
            Assert.That(Choice("Destination pile").choices, Does.Not.Contain(other.Destination.Id));
            lastAction = game.LastAction; authority = JsonUtility.ToJson(world); Submit(oldCreate);
            Assert.That(game.LastAction, Is.EqualTo(lastAction), "Rendering the new village must not reactivate a retained Create control from the old village.");
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(authority), "An old village's detached dispatch control must never submit the current village's draft.");
            Assert.That(village.Routes, Is.Empty); Assert.That(personal.Routes, Is.Empty); Assert.That(world.Validate(), Is.Empty);
        }

        [Test]
        public void RoutesIdentifyCurrentAssignmentsAndCancelOnlyTheChosenRoute()
        {
            var village = game.Selected;
            var drivers = village.Cats.Where(c => c.Alive && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(drivers.Length, Is.EqualTo(2)); drivers[0].Name = "North Sailor"; drivers[1].Name = "South Sailor";
            var north = PrepareDispatchLane("north", 25); var south = PrepareDispatchLane("south", 28);
            TransportRoute Dispatch(Cat driver, Stockpile source, Stockpile destination, System.Collections.Generic.List<Int2> path)
            {
                var result = game.Send(new GameAction { Kind = "CreateTransportRoute", CatId = driver.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "shipping", Resource = "logs", Amount = 4, Path = path, Repeat = false }).Result;
                Assert.That(result.Success, Is.True, result.Error); return village.Routes.Single(r => r.Id == result.EntityId);
            }
            var northRoute = Dispatch(drivers[0], north.Source, north.Destination, north.Path);
            var southRoute = Dispatch(drivers[1], south.Source, south.Destination, south.Path);
            Assert.That(northRoute.Phase, Is.EqualTo(southRoute.Phase)); Assert.That(northRoute.Resource, Is.EqualTo(southRoute.Resource));
            game.UI.OpenPanel("Routes");
            VisualElement Card(string pileId)
            {
                var cancel = Root.Query<Button>().ToList().SingleOrDefault(b => b.text == "Cancel route" && b.parent.Query<Label>().ToList().Any(l => l.text.Contains(pileId)));
                Assert.That(cancel, Is.Not.Null, "Each route must visibly identify its endpoint beside its own Cancel control: " + pileId);
                return cancel.parent;
            }
            // Identify the assignment through visible endpoint copy beside its
            // own Cancel control, without depending on a new hidden card name.
            VisualElement AssignmentCard(Stockpile source) => Card(source.Id);
            string Copy(VisualElement card) => string.Join("\n", card.Query<Label>().ToList().Select(l => l.text));
            string northCopy = Copy(AssignmentCard(north.Source)), southCopy = Copy(AssignmentCard(south.Source));
            Assert.That(northCopy, Does.Contain(drivers[0].Name).And.Contain(drivers[0].Id).And.Contain(north.Source.Id).And.Contain(north.Source.Position.ToString()).And.Contain(north.Destination.Id).And.Contain(north.Destination.Position.ToString()), "Each route must identify its driver and exact endpoint locations beside Cancel.");
            Assert.That(southCopy, Does.Contain(drivers[1].Name).And.Contain(drivers[1].Id).And.Contain(south.Source.Id).And.Contain(south.Source.Position.ToString()).And.Contain(south.Destination.Id).And.Contain(south.Destination.Position.ToString()));
            Assert.That(northCopy, Is.Not.EqualTo(southCopy), "Same-mode, phase and resource routes must remain distinguishable.");
            var choices = Root.Query<DropdownField>().ToList(); var amount = Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount");
            amount.value = 7; amount.Focus(); var focused = Root.focusController.focusedElement;
            // Authored rename/location snapshots test presentation invalidation;
            // they are not evidence of a physical move or sailing delivery.
            drivers[0].Name = "Renamed North Sailor";
            north.Source.Position = new Int2(-33, 25); north.Destination.Position = new Int2(-27, 25);
            PrepareDispatchTile(north.Source.Position, true, north.Source); PrepareDispatchTile(north.Destination.Position, true, north.Destination);
            RefreshPausedUiWithoutAuthorityChanges();
            northCopy = Copy(AssignmentCard(north.Source));
            Assert.That(northCopy, Does.Contain(drivers[0].Name).And.Contain(drivers[0].Id).And.Contain(north.Source.Position.ToString()).And.Contain(north.Destination.Position.ToString()), "Open cards must read current resolved names and endpoint positions even when route fields are unchanged.");
            CollectionAssert.AreEqual(choices, Root.Query<DropdownField>().ToList(), "Assignment feedback must preserve every dispatch chooser object.");
            Assert.That(Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount"), Is.SameAs(amount)); Assert.That(amount.value, Is.EqualTo(7));
            Assert.That(Root.focusController.focusedElement, Is.SameAs(focused));
            Submit(AssignmentCard(south.Source).Query<Button>().ToList().Single(b => b.text == "Cancel route"));
            Assert.That(village.Routes.Select(r => r.Id), Is.EqualTo(new[] { northRoute.Id }), "The Cancel beside South Sailor's assignment must cancel only that exact route.");
            Assert.That(drivers[1].BuildingId, Is.Empty); Assert.That(south.Vehicle.RouteId, Is.Empty);
            Assert.That(drivers[0].BuildingId, Is.EqualTo(northRoute.Id)); Assert.That(north.Vehicle.RouteId, Is.EqualTo(northRoute.Id));
            Assert.That(north.Source.Goods.Single().Amount, Is.EqualTo(8)); Assert.That(south.Source.Goods.Single().Amount, Is.EqualTo(8));
            Assert.That(north.Vehicle.Cargo, Is.Empty); Assert.That(south.Vehicle.Cargo, Is.Empty); Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }

        [Test]
        public void RoutesRefreshStatusAndRemovalWithoutReplacingFocusedDispatchChoices()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            var driver = village.Cats.First(c => c.Alive && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "");
            var source = new Stockpile { Id = "ui-route-source", Position = new Int2(1, 5), Width = 1, Depth = 1, Capacity = 8 };
            var destination = new Stockpile { Id = "ui-route-destination", Position = new Int2(5, 5), Width = 1, Depth = 1, Capacity = 8 };
            source.Goods.Add(new Simulation.Stack("logs", 8)); village.Stockpiles.Add(source); village.Stockpiles.Add(destination);
            var path = Enumerable.Range(1, 5).Select(x => new Int2(x, 5)).ToList();
            foreach (var at in path)
            {
                Assert.That(village.Buildings.Any(b => b.Position.Level == at.Level && at.X >= b.Position.X && at.X < b.Position.X + b.Width && at.Z >= b.Position.Z && at.Z < b.Position.Z + b.Depth), Is.False, "Synthetic channel must not overlap retained building footprints.");
                Assert.That(village.Stockpiles.Any(p => p != source && p != destination && p.Position.Level == at.Level && at.X >= p.Position.X && at.X < p.Position.X + p.Width && at.Z >= p.Position.Z && at.Z < p.Position.Z + p.Depth), Is.False, "Synthetic channel must not overlap another retained pile.");
                var tile = world.TileAt(at); World.ClearSurface(tile, 0); tile.Wall = tile.Bridge = tile.Rail = tile.Road = tile.Dirt = false;
                tile.Dock = at.Equals(source.Position) || at.Equals(destination.Position);
                if (!tile.Dock) World.SetWaterSurface(tile, 0, .8);
            }
            var vessel = new Vehicle { Id = "ui-route-vessel", Mode = "shipping", Position = source.Position, HasContinuousPosition = true, X = source.Position.X, Z = source.Position.Z };
            village.Vehicles.Add(vessel);
            var created = game.Send(new GameAction { Kind = "CreateTransportRoute", CatId = driver.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "shipping", Resource = "logs", Amount = 8, Path = path, Repeat = true }).Result;
            Assert.That(created.Success, Is.True, created.Error);
            var route = village.Routes.Single(r => r.Id == created.EntityId);
            // Prepare valid presentation snapshots after public dispatch. These
            // authored phase/position changes are not physical sailing evidence.
            source.Goods.Clear(); vessel.Cargo.Add(new Simulation.Stack("logs", 8));
            route.Phase = "outbound"; route.PathIndex = 1; route.BlockedReason = "route_blocked";
            vessel.Position = driver.Position = path[1]; vessel.X = driver.X = 2.25; vessel.Z = driver.Z = 5;
            game.UI.OpenPanel("Routes");
            DropdownField Choice(string label) => Root.Query<DropdownField>().ToList().Single(f => f.label == label);
            var mode = Choice("Mode"); mode.value = "shipping";
            var cat = Choice("Cat"); cat.value = driver.Name + " · " + driver.Id;
            var resource = Choice("Resource"); resource.value = "logs";
            var from = Choice("Source pile"); from.value = source.Id;
            var to = Choice("Destination pile"); to.value = destination.Id;
            var amount = Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount"); amount.value = 8;
            amount.Focus(); var focused = Root.focusController.focusedElement;
            Assert.That(focused, Is.Not.Null, "The dispatch form must have actual keyboard focus for this regression.");
            string Copy() => string.Join("\n", Root.Query<Label>().ToList().Select(l => l.text));
            void RefreshUnchanged()
            {
                string before = JsonUtility.ToJson(world), action = game.LastAction;
                var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
                typeof(ForestUI).GetField("nextRefresh", flags).SetValue(game.UI, 0f);
                typeof(ForestUI).GetMethod("Update", flags).Invoke(game.UI, null);
                Assert.That(JsonUtility.ToJson(world), Is.EqualTo(before), "Paused route feedback refresh must not advance or rewrite authority.");
                Assert.That(game.LastAction, Is.EqualTo(action), "Reading route feedback must issue no command.");
                Assert.That(Choice("Mode"), Is.SameAs(mode)); Assert.That(mode.value, Is.EqualTo("shipping"));
                Assert.That(Choice("Cat"), Is.SameAs(cat)); Assert.That(cat.value, Is.EqualTo(driver.Name + " · " + driver.Id));
                Assert.That(Choice("Resource"), Is.SameAs(resource)); Assert.That(resource.value, Is.EqualTo("logs"));
                Assert.That(Choice("Source pile"), Is.SameAs(from)); Assert.That(from.value, Is.EqualTo(source.Id));
                Assert.That(Choice("Destination pile"), Is.SameAs(to)); Assert.That(to.value, Is.EqualTo(destination.Id));
                Assert.That(Root.Query<DoubleField>().ToList().Single(f => f.label == "Cargo amount"), Is.SameAs(amount)); Assert.That(amount.value, Is.EqualTo(8));
                Assert.That(Root.focusController.focusedElement, Is.SameAs(focused), "Route feedback must keep the player's focused dispatch field.");
                Assert.That(game.UI.OpenSection, Is.EqualTo("Routes"));
                Assert.That(world.Validate(), Is.Empty);
            }
            Assert.That(Copy(), Does.Contain("shipping · outbound").And.Contain("No walkable route"));
            // Replacing the same ID models a fresh authority snapshot. The UI
            // must read the current route rather than its captured old instance.
            village.Routes.Remove(route);
            route = new TransportRoute { Id = route.Id, Mode = route.Mode, CatId = route.CatId, VehicleId = route.VehicleId, SourceId = route.SourceId, DestinationId = route.DestinationId, Resource = route.Resource, Amount = route.Amount, Path = route.Path.ToList(), PathIndex = route.PathIndex, Phase = "returning_to_dock", CancelRequested = true, Repeat = false, BlockedReason = "return_route_blocked" };
            village.Routes.Add(route);
            RefreshUnchanged();
            Assert.That(Copy(), Does.Contain("shipping · returning to dock").And.Contain("The return route is blocked"), "An open Routes panel must show the current snapshot's return phase and actual blocker.");
            route.BlockedReason = ""; vessel.X = driver.X = 2.2;
            RefreshUnchanged();
            Assert.That(Copy(), Does.Not.Contain("The return route is blocked"), "Recovered sailing must remove its obsolete visible warning without closing Routes.");
            Assert.That(FindButton("Cancel route"), Is.Not.Null);
            village.Routes.Remove(route); vessel.RouteId = driver.BuildingId = "";
            vessel.Cargo.Clear(); source.Goods.Add(new Simulation.Stack("logs", 8));
            vessel.Position = driver.Position = source.Position; vessel.X = driver.X = source.Position.X; vessel.Z = driver.Z = source.Position.Z;
            RefreshUnchanged();
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "Cancel route"), Is.False, "An ended route must disappear from the still-open panel.");
            Assert.That(Copy(), Does.Not.Contain("shipping · returning to dock"));
        }

        [Test]
        public void WorkshopRecipeChooserOnlyOffersUnlockedRecipes()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            Assert.That(Catalog.RecipeAvailable(village, Catalog.Recipe("carpentry_specialty")), Is.False);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var choice = Root.Query<DropdownField>().ToList().Single(d => d.label == "Recipe");
            CollectionAssert.AreEquivalent(Catalog.Recipes.Where(r => r.Building == station.Kind && Catalog.RecipeAvailable(village, r)).Select(r => r.Id), choice.choices,
                "The recipe chooser must offer orders the colony can actually place.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text.Contains("require research")), Is.True,
                "Explain where the remaining recipes become available.");
            village.Research.Add("carpentry_specialty");
            game.UI.OpenPanel("Inspect");
            Assert.That(Root.Query<DropdownField>().ToList().Single(d => d.label == "Recipe").choices, Does.Contain("carpentry_specialty"),
                "Unlocking research must make its recipe available without changing the workplace.");
        }

        [Test]
        public void WorkshopRecipePreviewShowsAdjustedBillsBeforeStaffing()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "woodworking");
            village.Research.Add("woodworking_workflow");
            foreach (var pile in village.Stockpiles) { pile.CountedAt = -1; pile.Report.Clear(); }
            var counted = village.Stockpiles.First(p => p.Kind == "storage");
            counted.CountedAt = 120; counted.Report.Add(new Simulation.Stack("planks", .5));
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Assert.That(station.Slots.Any(s => s.CatId != ""), Is.False);
            var preview = Root.Q<VisualElement>("recipe-preview");
            Assert.That(preview, Is.Not.Null, "Players need the selected recipe's bill before committing a worker or an order.");
            string Copy() => string.Join("\n", Root.Q<VisualElement>("recipe-preview").Query<Label>().ToList().Select(l => l.text));
            string adjusted = game.CurrentWorld.RecipeInput(village, Catalog.Recipe("planks_and_blocks_to_tools"), 2).ToString("0.##");
            string copy = Copy();
            Assert.That(copy, Does.Contain("needs " + adjusted).And.Contain("reported " + .5.ToString("0.##")).And.Contain("uncounted"));
            Assert.That(preview.Query<Button>().ToList().Any(b => b.text == "Inspect supplier · wood cutter"), Is.True);
            foreach (var pile in village.Stockpiles) { pile.Goods.Clear(); pile.Goods.Add(new Simulation.Stack("planks", 999)); }
            game.UI.OpenPanel("Inspect");
            Assert.That(Copy(), Is.EqualTo(copy), "Recipe planning must retain historical reports rather than expose hidden live goods.");
            var ingredients = Root.Q<Label>("recipe-ingredients");
            Assert.That(ingredients, Is.Not.Null);
            Assert.That(ingredients.text, Does.Contain(adjusted), "The selected recipe summary must agree with the authoritative adjusted bill.");
        }

        [Test]
        public void WorkshopQueuePinsCurrentBatchWhileFutureOrdersRemainEditable()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int batch = 0; batch < 3; batch++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var job = new Job { Id = "ui-finite-active-batch", Kind = "production", TargetId = station.Id, CatId = cat.Id, RecipeId = "logs_to_planks", Phase = "working", RequiredWork = 600 };
            village.Jobs.Add(job); cat.JobId = job.Id;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var cards = Root.Query<VisualElement>(className: "queue-entry").ToList();
            Button Control(int index, string text) => cards[index].Query<Button>().ToList().Single(b => b.text == text);
            Assert.That(Control(0, "Remove").enabledSelf, Is.False, "Removing an in-flight finite order must not consume its identical successor at completion.");
            Assert.That(Control(0, "↓").enabledSelf, Is.False);
            Assert.That(Control(1, "↑").enabledSelf, Is.False, "A future order cannot displace the batch currently collecting or consuming material.");
            Assert.That(Control(1, "Remove").enabledSelf, Is.True);
            Assert.That(Control(2, "↑").enabledSelf, Is.True);
            Assert.That(Control(0, "Repeat off").enabledSelf, Is.True);
            cat.ResumeJobId = job.Id; cat.JobId = ""; cat.Goal = "need_drink";
            game.UI.OpenPanel("Inspect");
            Assert.That(Root.Query<VisualElement>(className: "queue-entry").ToList()[0].Query<Button>().ToList().Single(b => b.text == "Remove").enabledSelf, Is.False,
                "A needs interruption preserves the same current batch.");
            job.Phase = "output_delivery";
            game.UI.OpenPanel("Inspect");
            Assert.That(Root.Query<VisualElement>(className: "queue-entry").ToList()[0].Query<Button>().ToList().Single(b => b.text == "Remove").enabledSelf, Is.True,
                "Once the batch has committed its outputs, only future orders remain in the queue.");
        }

        [Test]
        public void WorkshopRecipeStudyShortcutClearsUnrelatedSearch()
        {
            game.UI.OpenPanel("Research");
            Root.Query<DropdownField>().ToList().Single(d => d.label == "Category").value = "building";
            Root.Query<TextField>().ToList().Single(f => f.label == "Search studies").value = "unrelated previous search";
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Submit("Study recipe unlocks");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Research"));
            Assert.That(Root.Query<TextField>().ToList().Single(f => f.label == "Search studies").value, Is.Empty,
                "The recipe shortcut must not inherit an unrelated search.");
            Assert.That(Root.Query<DropdownField>().ToList().Single(d => d.label == "Category").value, Is.EqualTo("recipe_resource"));
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == "Carpentry Specialties"), Is.True);
        }

        [Test]
        public void WorkplaceListDistinguishesDuplicateWorkshopsByLocation()
        {
            var stations = game.Selected.Buildings.Where(b => b.Kind == "wood_cutter").ToArray();
            Assert.That(stations.Length, Is.GreaterThanOrEqualTo(2));
            foreach (var station in stations)
            {
                game.UI.OpenPanel("Work");
                var button = Root.Query<Button>().ToList().SingleOrDefault(b => b.text.StartsWith("wood cutter · ", StringComparison.Ordinal) && b.text.EndsWith(" · " + station.Position, StringComparison.Ordinal));
                Assert.That(button, Is.Not.Null, "Duplicate workshops need a visible location so players can choose the intended supplier without trial selection.");
                Submit(button);
                Assert.That(game.View.SelectedBuilding.Id, Is.EqualTo(station.Id), "The chosen location must inspect that exact workshop.");
            }
        }

        [Test]
        public void EquipmentOrdersExposePhysicalSourceAndExactCancellation()
        {
            var village = game.Selected;
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            var source = village.Stockpiles.First(p => p.Kind == "storage");
            var item = new Item { Id = "ui-retrieval-tool", Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = "ui-retrieval-job", Condition = 72 };
            var job = new Job { Id = "ui-retrieval-job", Kind = "equip", Phase = "item_fetch", CatId = cat.Id, SourceId = source.Id, TargetId = item.Id, Position = source.Position, Manual = true, ItemIds = new System.Collections.Generic.List<string> { item.Id } };
            village.Items.Add(item); village.Jobs.Add(job); cat.JobId = job.Id; cat.Goal = "retrieve equipment";
            game.UI.OpenPanel("Stores");
            Submit("Equipment");
            var order = Root.Q<VisualElement>("equipment-order-" + job.Id);
            Assert.That(order, Is.Not.Null, "Stores must explain a claimed item instead of offering a second Equip action.");
            string copy = string.Join("\n", order.Query<Label>().ToList().Select(l => l.text));
            Assert.That(copy, Does.Contain(cat.Name).And.Contain(source.Id).And.Contain("after arrival"));
            var card = Root.Q<VisualElement>("equipment-item-" + item.Id);
            Assert.That(card, Is.Not.Null);
            Assert.That(card.Query<Button>().ToList().Single(b => b.text == "Retrieve for selected cat").enabledSelf, Is.False);
            cat.JobId = ""; cat.ResumeJobId = job.Id; cat.Goal = "need_drink";
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Q<VisualElement>("equipment-order-" + job.Id), Is.Not.Null, "Inspect must retain cancellation during a needs trip as well as travel.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == "Meeting needs; this equipment order will resume afterward."), Is.True);
            Submit("Cancel equipment order");
            Assert.That(job.Completed, Is.True);
            Assert.That(item.LocationId, Is.EqualTo(source.Id));
            Assert.That(cat.Equipment, Does.Not.Contain(item.Id));
        }

        [UnityTest]
        public IEnumerator EquipmentNavigationKeepsStoresDestinationsVisibleAboveLongInventory()
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000; Root.style.height = Root.style.maxHeight = 620;
            var village = game.Selected;
            var pile = village.Stockpiles.First(p => p.Kind == "storage");
            village.Items.Add(new Item { Id = "navigation-stored-tool", Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = pile.Id, Condition = 17, MaxCondition = 42 });
            game.UI.OpenPanel("Stores");
            yield return null; yield return null;
            var equipment = Root.Q<Button>("stores-equipment-tab");
            Assert.That(equipment, Is.Not.Null, "Equipment needs an explicit visible destination before the long inventory and pile lists.");
            var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
            scroll.scrollOffset = new Vector2(0, 10000);
            yield return null; yield return null;
            Assert.That(equipment.worldBound.yMax, Is.LessThanOrEqualTo(scroll.worldBound.yMin + .5f), "Stores destinations must remain above the scrolling contents.");
            Submit("Equipment");
            yield return null; yield return null;
            Assert.That(scroll.scrollOffset.y, Is.EqualTo(0).Within(.5f));
            Assert.That(Root.Q<VisualElement>("equipment-item-navigation-stored-tool"), Is.Not.Null);
            Assert.That(Root.Query<Label>(className: "stock-row").ToList(), Is.Empty);
            game.UI.OpenPanel("Stores");
            Assert.That(Root.Q<VisualElement>("equipment-item-navigation-stored-tool"), Is.Not.Null, "Ordinary refresh must retain the chosen destination.");
            Submit("Inventory");
            yield return null; yield return null;
            Assert.That(Root.Query<Label>(className: "stock-row").ToList().Count, Is.GreaterThan(0));
            Assert.That(Root.Q<VisualElement>("equipment-item-navigation-stored-tool"), Is.Null);
            Assert.That(scroll.scrollOffset.y, Is.EqualTo(0).Within(.5f));
            foreach (var size in new[] { new Vector2(600, 360), new Vector2(550, 336) })
            {
                Root.style.width = Root.style.maxWidth = size.x; Root.style.height = Root.style.maxHeight = size.y;
                yield return null; yield return null;
                var drawer = Root.Q<VisualElement>(className: "drawer");
                AssertInside(drawer.worldBound, equipment.worldBound, "Equipment must remain reachable in compact Stores.");
                AssertInside(drawer.worldBound, Root.Q<Button>("stores-inventory-tab").worldBound, "Inventory must remain reachable in compact Stores.");
                Assert.That(scroll.worldBound.height, Is.GreaterThan(40), "The fixed destinations must leave usable scrolling content.");
                float storesHeight = scroll.worldBound.height;
                game.UI.OpenPanel("Cats");
                yield return null; yield return null;
                Assert.That(equipment.parent.resolvedStyle.display, Is.EqualTo(DisplayStyle.None));
                Assert.That(scroll.worldBound.height, Is.GreaterThan(storesHeight), "Leaving Stores must restore its fixed destination space.");
                game.UI.OpenPanel("Stores");
                yield return null; yield return null;
            }
        }

        [Test]
        public void WorkshopHoverUsesPositionStatusInsteadOfAssumingStaffedMeansWorking()
        {
            var village = game.Selected; village.GrandfatheredRecipes = true;
            var station = village.Buildings.First(b => b.Kind == "woodworking"); station.Kind = "clothier";
            village.Research.Add("clothier_crews");
            var cats = village.Cats.Where(c => c.Alive && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            foreach (var cat in cats) Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id); game.UI.ClosePanel(); station.BlockedReason = "queue_empty";
            string Hover()
            {
                var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
                typeof(ForestUI).GetMethod("UpdateTetheredInspector", flags).Invoke(game.UI, null);
                return ((Label)typeof(ForestUI).GetField("tetheredInspectorBody", flags).GetValue(game.UI)).text;
            }
            Assert.That(Hover(), Does.Contain(cats[0].Name + " · Waiting for a recipe").And.Contain(cats[1].Name + " · Waiting for a recipe").And.Not.Contain("Working"), "Staffing alone must not claim that a finished finite queue is working.");
            cats[0].Goal = "need_drink";
            Assert.That(Hover(), Does.Contain(cats[0].Name + " · Getting water").And.Contain(cats[1].Name + " · Waiting for a recipe"));
            cats[0].Goal = "idle"; station.Slots[0].Paused = true;
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[1].Id, Edit = "add", RecipeId = "fibre_to_thread" }).Result.Success, Is.True);
            var job = new Job { Id = "hover-textile-production", Kind = "production", RecipeId = "fibre_to_thread", CatId = cats[1].Id, TargetId = station.Id, Position = station.Position, Phase = "working", RequiredWork = 600 };
            village.Jobs.Add(job); cats[1].JobId = job.Id; station.Slots[1].JobId = job.Id;
            Assert.That(Hover(), Does.Contain(cats[0].Name + " · Paused").And.Contain(cats[1].Name + " · fibre to thread · Working").And.Not.Contain("Ready for work"), "A paused first position must not hide the later position's actual work or repeat a stale building-wide reason.");
            job.Phase = "fetch"; job.BlockedReason = "input_unreachable";
            Assert.That(Hover(), Does.Contain(cats[1].Name + " · fibre to thread · Collecting ingredients").And.Contain("Recipe ingredients cannot reach this workshop"));
            game.Send(new GameAction { Kind = "AssignWorker", CatId = cats[0].Id });
            game.Send(new GameAction { Kind = "AssignWorker", CatId = cats[1].Id });
            Assert.That(Hover(), Does.Contain("Waiting for a worker").And.Not.Contain("Working"));
        }

        [UnityTest]
        public IEnumerator EquipmentNavigationInspectorUsesTheExactCatAndShowsReturnsBeforePreferences()
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000; Root.style.height = Root.style.maxHeight = 620;
            var village = game.Selected;
            var cat = village.Cats.Where(c => c.Alive && c.JobId == "" && c.BuildingId == "").Skip(1).First();
            var pile = village.Stockpiles.First(p => p.Kind == "storage");
            var armor = new Item { Id = "navigation-worn-armor", Kind = "armor", Material = "leather", VillageId = village.Id, LocationId = cat.Id, Condition = 23, MaxCondition = 50 };
            var tool = new Item { Id = "navigation-exact-tool", Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = pile.Id, Condition = 17, MaxCondition = 42 };
            village.Items.Add(armor); village.Items.Add(tool); cat.Equipment.Add(armor.Id);
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var preferences = Root.Query<Label>().ToList().Single(l => l.text == "Work preferences");
            var returns = FindButton("Return armor to storage");
            Assert.That(returns.worldBound.yMin, Is.LessThan(preferences.worldBound.yMin), "Equipment controls must precede the full labor preference grid.");
            Submit("Choose equipment for this cat");
            yield return null; yield return null;
            Assert.That(game.UI.OpenSection, Is.EqualTo("Stores"));
            Assert.That(Root.Q<Button>("stores-equipment-tab").ClassListContains("active"), Is.True);
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Does.EndWith(" · " + cat.Id));
            var retrieve = Root.Q<VisualElement>("equipment-item-" + tool.Id).Query<Button>().ToList().Single(b => b.text == "Retrieve for selected cat");
            Submit(retrieve);
            var order = village.Jobs.Single(j => !j.Completed && j.Kind == "equip");
            Assert.That(order.CatId, Is.EqualTo(cat.Id), "The shortcut must not equip the unrelated globally selected cat.");
            Assert.That(order.ItemIds, Is.EqualTo(new[] { tool.Id }));
            Assert.That(tool.LocationId, Is.EqualTo(order.Id));
            Assert.That(cat.Equipment, Does.Not.Contain(tool.Id));
        }

        [UnityTest]
        public IEnumerator EquipmentNavigationRoutesReportsAndHaulingToInventoryAndDefenseToEquipment()
        {
            game.UI.OpenPanel("Defense"); Submit("Choose equipment in Stores");
            yield return null; yield return null;
            var equipment = Root.Q<Button>("stores-equipment-tab");
            Assert.That(equipment, Is.Not.Null.And.Property("text").EqualTo("Equipment"));
            Assert.That(equipment.ClassListContains("active"), Is.True, "Defense must explicitly open equipment even after earlier Inventory use.");
            game.UI.OpenPanel("Work"); Submit("Haul goods · choose source");
            yield return null; yield return null;
            Assert.That(Root.Q<Button>("stores-inventory-tab").ClassListContains("active"), Is.True);
            Assert.That(Root.Query<Label>(className: "stock-row").ToList().Count, Is.GreaterThan(0));
            Submit("Equipment");
            var village = game.Selected; village.GrandfatheredRecipes = true;
            var station = village.Buildings.First(b => b.Kind == "woodworking");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "planks_and_blocks_to_tools" }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect"); Submit("View inventory & hauling");
            yield return null; yield return null;
            Assert.That(Root.Q<Button>("stores-inventory-tab").ClassListContains("active"), Is.True, "Reported-stock links must not inherit the previous Equipment destination.");
            Assert.That(Root.Q<ScrollView>(className: "drawer-scroll").scrollOffset.y, Is.EqualTo(0).Within(.5f));
        }

        [Test]
        public void ReturningEquipmentNamesItsDestinationAndStaysWornUntilDelivery()
        {
            var village = game.Selected;
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            var destination = village.Stockpiles.First(p => p.Kind == "storage");
            var item = new Item { Id = "ui-return-armor", Kind = "armor", Material = "leather", VillageId = village.Id, LocationId = cat.Id, Condition = 48 };
            var job = new Job { Id = "ui-return-job", Kind = "unequip", Phase = "item_return", CatId = cat.Id, SourceId = cat.Id, TargetId = destination.Id, Position = destination.Position, BlockedReason = "storage_full", Manual = true, ItemIds = new System.Collections.Generic.List<string> { item.Id } };
            village.Items.Add(item); village.Jobs.Add(job); cat.JobId = job.Id; cat.Equipment.Add(item.Id);
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            var order = Root.Q<VisualElement>("equipment-order-" + job.Id);
            Assert.That(order, Is.Not.Null);
            Assert.That(string.Join("\n", order.Query<Label>().ToList().Select(l => l.text)), Does.Contain(destination.Id).And.Contain("Still equipped"));
            Assert.That(string.Join("\n", order.Query<Label>().ToList().Select(l => l.text)), Does.Contain("cancel and issue a new return"), "A return keeps its exact destination, so adding another pile alone cannot unblock it.");
            Assert.That(Root.Query<Button>().ToList().Single(b => b.text == "Return armor to storage").enabledSelf, Is.False, "A pending return must not offer a duplicate order.");
            Assert.That(Root.Q<ProgressBar>("live-work-progress"), Is.Null, "Travel is a physical order rather than a fake crafting countdown.");
            Submit("Cancel equipment order");
            Assert.That(job.Completed, Is.True);
            Assert.That(item.LocationId, Is.EqualTo(cat.Id));
            Assert.That(cat.Equipment, Does.Contain(item.Id));
        }

        [Test]
        public void WindowResizingPreservesPhysicalTextSize()
        {
            var panel = game.GetComponent<UIDocument>().panelSettings;
            Assert.That(panel.scaleMode, Is.EqualTo(PanelScaleMode.ConstantPhysicalSize), "Small windows must reflow rather than shrink the entire management UI.");
            Assert.That(panel.referenceDpi, Is.InRange(96f, 120f));
        }

        [UnityTest]
        public IEnumerator WorkshopSupplyNamesTheQueuedBillUsingReportedStocksAndFindsASupplier()
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000; Root.style.height = Root.style.maxHeight = 620;
            var village = game.Selected;
            village.GrandfatheredRecipes = true;
            var station = village.Buildings.First(b => b.Kind == "woodworking");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "planks_and_blocks_to_tools" }).Result.Success, Is.True);
            for (int index = 0; index < 8; index++) Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "planks_and_blocks_to_tools" }).Result.Success, Is.True);
            foreach (var pile in village.Stockpiles) { pile.CountedAt = -1; pile.Report.Clear(); }
            var counted = village.Stockpiles.First(p => p.Kind == "storage");
            counted.CountedAt = 120;
            counted.Report.Add(new Simulation.Stack("planks", 1));
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var supply = Root.Q<VisualElement>(className: "recipe-supply");
            Assert.That(supply, Is.Not.Null, "A queued workshop order must show its named bill rather than only a generic ingredient warning.");
            string Copy() => string.Join("\n", Root.Q<VisualElement>(className: "recipe-supply").Query<Label>().ToList().Select(l => l.text));
            var copy = Copy();
            Assert.That(copy, Does.Contain("Planks").And.Contain("Blocks").And.Contain("reported").And.Contain("uncounted"));
            Assert.That(copy, Does.Contain((120d / 3600).ToString("0.0") + "h"), "The report must disclose when the counted stock was observed, using the UI's numeric locale.");
            foreach (var pile in village.Stockpiles) { pile.Goods.Clear(); pile.Goods.Add(new Simulation.Stack("planks", 999)); }
            game.UI.OpenPanel("Inspect");
            Assert.That(Copy(), Is.EqualTo(copy), "Hidden live goods must not change the displayed Accountant bill.");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 900);
            yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(600), "The supplier return must exceed the shorter supplier page's scroll range.");
            var supplier = Root.Query<Button>().ToList().FirstOrDefault(b => b.text == "Inspect supplier · wood cutter");
            Assert.That(supplier, Is.Not.Null, "Planks should lead to an existing supplying workshop.");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = supplier; supplier.SendEvent(submit); }
            Assert.That(game.View.SelectedBuilding.Kind, Is.EqualTo("wood_cutter"));
            yield return null; yield return null;
            Submit("Back");
            yield return null; yield return null;
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"), "Back from a supplier must retain the original workshop context.");
            Assert.That(game.View.SelectedBuilding.Id, Is.EqualTo(station.Id));
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.5f));
            Submit("Inspect supplier · wood cutter");
            game.UI.OpenPanel("Stores"); game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect"); Submit("Back");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Stores"), "Deliberate category navigation must discard the old supplier return.");
        }

        [Test]
        public void WorkshopDelegationReturnsTheExactWorkerAndPreservesManualOrders()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cats = village.Cats.Where(c => c.Alive && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            village.Research.Add("sawmill");
            village.Buildings.Add(new Building { Id = "ui-delegation-sawmill", Kind = "sawmill", Completed = true });
            Assert.That(game.Send(new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = cats[1].Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cats[0].Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[0].Id, Edit = "add", RecipeId = "logs_to_planks", Repeat = true }).Result.Success, Is.True);
            var slot = station.Slots.Single(s => s.CatId == cats[0].Id);
            var order = slot.Queue.Single();
            slot.Paused = true;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var handback = Root.Query<Button>().ToList().SingleOrDefault(b => b.text == "Return to Forester");
            Assert.That(handback, Is.Not.Null, "A manual work position needs an in-place return to its officer without releasing the cat.");
            Assert.That(handback.enabledSelf, Is.False, "Resume first, because normal Resume intentionally takes manual control.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == "Resume work, then return this position to the Forester."), Is.True);
            Submit("Resume");
            Submit("Return to Forester");
            Assert.That(slot.AutomatedBy, Is.EqualTo("forester"));
            Assert.That(slot.CatId, Is.EqualTo(cats[0].Id));
            Assert.That(slot.Queue.Single(), Is.SameAs(order));
            Assert.That(order.Repeat, Is.True);
            Assert.That(order.AutomatedBy, Is.Empty, "Returning staffing must not relabel a deliberate manual recipe.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == "Repeating orders continue until you change them."), Is.True);
        }

        [Test]
        public void OfficerPrerequisitesDistinguishStudiesAndBuildingsAndNavigateToStudy()
        {
            game.Selected.Research.RemoveAll(id => id == "sawmill");
            game.Selected.Buildings.RemoveAll(b => b.Kind == "sawmill");
            game.UI.OpenPanel("Officers");
            var card = Root.Query<VisualElement>(className: "officer-card").ToList().SingleOrDefault(c => c.name == "forester");
            Assert.That(card, Is.Not.Null, "Each office needs a stable card for its own prerequisite feedback.");
            var copy = string.Join("\n", card.Query<Label>().ToList().Select(l => l.text));
            Assert.That(copy, Does.Contain("Study · Sawmill · needed").And.Contain("Building · Sawmill · needs completion"));
            var studyButton = card.Query<Button>().ToList().Single(b => b.text == "Find study · Sawmill");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = studyButton; studyButton.SendEvent(submit); }
            Assert.That(game.UI.OpenSection, Is.EqualTo("Research"));
            Assert.That(Root.Query<TextField>().ToList().Single(f => f.label == "Search studies").value, Is.EqualTo("sawmill"));
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == Catalog.Study("sawmill").Name), Is.True);
            game.Selected.Research.Add("sawmill");
            game.UI.OpenPanel("Officers");
            card = Root.Query<VisualElement>(className: "officer-card").ToList().Single(c => c.name == "forester");
            Assert.That(string.Join("\n", card.Query<Label>().ToList().Select(l => l.text)), Does.Contain("Study · Sawmill · complete").And.Contain("Building · Sawmill · needs completion"));
        }

        [Test]
        public void GlassClearingChromeProvidesAttentionSealAndSubjectInspector()
        {
            Assert.That(Root.Q<Button>(className: "attention-seal"), Is.Not.Null);
            Assert.That(Root.Q<VisualElement>(className: "tethered-inspector"), Is.Not.Null);
            Assert.That(Root.Q<VisualElement>(className: "top").resolvedStyle.backgroundColor.a, Is.LessThan(1f));
        }

        [UnityTest]
        public IEnumerator WorldFocusReservesOnlyTheVisibleSidebarAndDrawer()
        {
            var property = typeof(ForestUI).GetProperty("WorldLeftFraction");
            Assert.That(property, Is.Not.Null, "The camera needs the actual visible management bounds to keep its subject out from under the drawer.");
            float Fraction() => (float)property.GetValue(game.UI);
            yield return null;
            float navigation = Fraction();
            Assert.That(navigation, Is.GreaterThan(0));
            game.UI.OpenPanel("Cats");
            yield return null; yield return null;
            Assert.That(Fraction(), Is.GreaterThan(navigation), "An open cat roster occupies more of the world than navigation alone.");
            Assert.That(Fraction(), Is.InRange(0, .8f));
            game.UI.ClosePanel();
            yield return null; yield return null;
            Assert.That(Fraction(), Is.EqualTo(navigation).Within(.01f));
            game.UI.OpenPanel("Research");
            yield return null; yield return null;
            Assert.That(Fraction(), Is.EqualTo(navigation).Within(.01f), "A full-page research map must not move the camera toward an invisible sliver of forest.");
        }

        [UnityTest]
        public IEnumerator FarmFeedbackDirectControlHidesTetherAndRestoresTheSameSelectedCat()
        {
            var cats = PrepareFarmFeedbackTether(); var cat = cats[0];
            game.View.InspectCat(cat.Id);
            yield return null; yield return null; yield return null;
            var tether = Root.Q<VisualElement>(className: "tethered-inspector");
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(game.View.Camera.orthographic, Is.True);
            game.View.EnterSelectedCat();
            Assert.That(game.View.ControlledCat, Is.SameAs(cat)); Assert.That(cat.ControlledBy, Is.EqualTo(game.PlayerId));
            RefreshPausedUiWithoutAuthorityChanges();
            yield return null; yield return null;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(game.View.DirectControl, Is.True); Assert.That(game.View.Camera.orthographic, Is.False);
            Assert.That(game.View.SelectedCat, Is.SameAs(cat));
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "The selected-cat tether must leave the close camera clear during actual public control.");
            game.View.LeaveCat();
            Assert.That(game.View.DirectControl, Is.False);
            RefreshPausedUiWithoutAuthorityChanges();
            yield return null; yield return null; yield return null;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(game.View.Camera.orthographic, Is.True); Assert.That(game.View.SelectedCat, Is.SameAs(cat));
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(tether.Q<Label>(className: "title").text, Is.EqualTo(cat.Name));
            Assert.That(game.Speed, Is.Zero);
        }

        [UnityTest]
        public IEnumerator FarmFeedbackControlledTetherStaysHiddenWhenStatusAndSelectedSubjectChange()
        {
            var cats = PrepareFarmFeedbackTether(); var controlled = cats[0]; var selected = cats[1];
            game.View.InspectCat(controlled.Id);
            yield return null; yield return null; yield return null;
            var tether = Root.Q<VisualElement>(className: "tethered-inspector");
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            game.View.EnterSelectedCat();
            RefreshPausedUiWithoutAuthorityChanges();
            yield return null; yield return null;
            Assert.That(game.View.ControlledCat, Is.SameAs(controlled)); Assert.That(game.View.Camera.orthographic, Is.False);
            // Synthetic displayed-status changes exercise both changed-content
            // measurement and the later placement refresh while control stays real.
            controlled.BlockedReason = "blocked_route";
            RefreshFarmFeedbackWithoutAuthorityChanges();
            yield return null;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "A changed controlled-cat status must not re-show the tether through its content-measurement branch.");
            selected.Goal = "need_drink"; selected.BlockedReason = "blocked_route";
            game.View.InspectCat(selected.Id);
            RefreshFarmFeedbackWithoutAuthorityChanges();
            yield return null;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(game.View.ControlledCat, Is.SameAs(controlled)); Assert.That(game.View.SelectedCat, Is.SameAs(selected));
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "Changing the selected subject during control must keep both content and placement branches hidden.");
            game.View.LeaveCat();
            RefreshPausedUiWithoutAuthorityChanges();
            yield return null; yield return null; yield return null;
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(game.View.DirectControl, Is.False); Assert.That(game.View.Camera.orthographic, Is.True);
            Assert.That(game.View.SelectedCat, Is.SameAs(selected)); Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(tether.Q<Label>(className: "title").text, Is.EqualTo(selected.Name));
            Assert.That(tether.Q<Label>(className: "body").text, Does.Contain("need drink").And.Contain("No walkable route"));
        }

        [UnityTest]
        public IEnumerator TetheredInspectorFitsTheVisibleForestAndYieldsToTheDrawer()
        {
            Root.style.flexGrow = 0;
            Root.style.width = Root.style.maxWidth = 600;
            Root.style.height = Root.style.maxHeight = 360;
            game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id);
            yield return null; yield return null; yield return null;
            var tether = Root.Q<VisualElement>(className: "tethered-inspector");
            var header = Root.Q<VisualElement>(className: "top");
            var navigation = Root.Q<VisualElement>(className: "navigation");
            var footer = Root.Q<VisualElement>(className: "bottom");
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            AssertInside(Root.worldBound, tether.worldBound, "The world label must use panel bounds at the current display scale.");
            Assert.That(tether.worldBound.xMin, Is.GreaterThanOrEqualTo(navigation.worldBound.xMax), "The world label must not cover navigation.");
            Assert.That(tether.worldBound.yMin, Is.GreaterThanOrEqualTo(header.worldBound.yMax), "The world label must remain below the header.");
            Assert.That(tether.worldBound.yMax, Is.LessThanOrEqualTo(footer.worldBound.yMin), "The whole world label must remain above the footer.");
            game.UI.OpenPanel("Cats");
            yield return null; yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "When the drawer leaves too little forest, its controls take priority over the world label.");
            game.UI.ClosePanel();
            Root.style.width = Root.style.maxWidth = 1000;
            Root.style.height = Root.style.maxHeight = 620;
            yield return null; yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex), "Closing the drawer restores the selected subject's label.");
            AssertInside(Root.worldBound, tether.worldBound, "Resizing must reposition the entire world label.");
            game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "The full subject inspector already provides these details and must keep its controls clear.");
        }

        [UnityTest]
        public IEnumerator TetheredInspectorMeasuresANewSubjectAfterAnOversizedLabelIsHidden()
        {
            Root.style.flexGrow = 0;
            Root.style.width = Root.style.maxWidth = 1000;
            Root.style.height = Root.style.maxHeight = 800;
            var cats = game.Selected.Cats.Where(c => c.Alive).Take(2).ToArray();
            cats[0].BlockedReason = "No materials\nNo walkable route\nWaiting for a carrier\nNo storage space\nInputs reserved\nDestination blocked\nWork paused";
            game.View.InspectCat(cats[0].Id);
            yield return null; yield return null; yield return null;
            var tether = Root.Q<VisualElement>(className: "tethered-inspector");
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex), "The tall window must first lay out the long subject label.");
            float longHeight = tether.worldBound.height;
            Root.style.height = Root.style.maxHeight = 360;
            yield return null; yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "The long label must genuinely be hidden before changing subjects.");
            game.View.InspectCat(cats[1].Id);
            yield return null; yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex), "A shorter subject must be measured even while the previous label is hidden.");
            Assert.That(tether.worldBound.height, Is.LessThan(longHeight));
            AssertInside(Root.worldBound, tether.worldBound, "The restored label must fit the shortened window.");
        }

        [Test]
        public void InspectReturnsToItsRosterAndKeepsTheActiveCategoryVisible()
        {
            game.UI.OpenPanel("Cats");
            Assert.That(FindButton("Cats").ClassListContains("active"), Is.True);
            var cat = game.Selected.Cats.First(c => c.Alive);
            Submit("Inspect " + cat.Name);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"));
            Assert.That(FindButton("Cats").ClassListContains("active"), Is.True);
            Submit("Back");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"));
            Submit("Close");
            Assert.That(Root.Query<Button>().ToList().Any(b => b.ClassListContains("navigation-button") && b.ClassListContains("active")), Is.False);
        }

        [Test]
        public void BlockedCatExplainsTheRouteProblemWithoutInternalCodes()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            cat.BlockedReason = "blocked_route";
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            var labels = Root.Query<Label>().ToList();
            Assert.That(labels.Any(l => l.text.Contains("No walkable route")), Is.True, "A player needs to understand why the cat cannot reach its work.");
            Assert.That(labels.Any(l => l.text == "blocked_route"), Is.False);
        }

        [Test]
        public void FirstWorkerQueueAppearsOnceWithEveryEntryControl()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Query<Label>().ToList().Count(l => l.text == "1. logs to planks"), Is.EqualTo(1), "The first worker's authoritative queue must not be duplicated as a second station queue.");
            foreach (var text in new[] { "Pause", "Resume", "↑", "↓", "Repeat off", "Remove" }) Assert.That(FindButton(text), Is.Not.Null);
            Submit("Repeat off");
            Assert.That(station.Slots[0].Queue[0].Repeat, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = cat.Id }).Result.Success, Is.True);
            game.UI.OpenPanel("Inspect");
            Submit("Remove");
            Assert.That(station.Slots[0].Queue, Is.Empty, "The station queue must remain editable while its first work position is vacant.");
        }

        [Test]
        public void WorkshopCrewCapacityAndNonRecipeWorkersCanBeManagedFromTheirInspector()
        {
            var hut = game.Selected.Buildings.First(b => b.Kind == "research_hut");
            game.Selected.ResearchPoints = 1000;
            void Buy(string id)
            {
                if (game.Selected.Research.Contains(id)) return;
                foreach (var prerequisite in Catalog.Study(id).Prerequisites) Buy(prerequisite);
                Assert.That(game.Send(new GameAction { Kind = "ResearchNode", NodeId = id }).Result.Success, Is.True, id);
            }
            Buy("research_hut_crews");
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = hut.Id, Edit = "pause", Enabled = true }).Result.Success, Is.True);
            var cats = game.Selected.Cats.Where(c => c.Alive && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = hut.Id, CatId = cats[0].Id }).Result.Success, Is.True);
            game.UI.OpenPanel("Work");
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "research hut · 1 / 2 staffed"), Is.True, "A researched vacancy must be visible before opening the workplace.");
            game.View.InspectBuilding(hut.Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Q<Label>(className: "staff-summary")?.text, Is.EqualTo("1 / 2 work positions staffed"));
            Assert.That(Root.Query<VisualElement>(className: "work-position").ToList().Count, Is.EqualTo(2), "Research workers and unlocked vacancies need the same visible work positions as recipe workshops.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text == "Work position 2 · Vacant"), Is.True);
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = cats[1].Name + " · " + cats[1].Id;
            Submit("Assign worker");
            Assert.That(Root.Q<Label>(className: "staff-summary").text, Is.EqualTo("2 / 2 work positions staffed"));
            var position = Root.Query<VisualElement>(className: "work-position").ToList().Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + cats[1].Name));
            Assert.That(position.Query<Button>().ToList().Any(b => b.text == "Resume work"), Is.True, "A pause inherited from the vacant station must offer Resume rather than another Pause.");
            var resume = position.Query<Button>().ToList().Single(b => b.text == "Resume work");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = resume; resume.SendEvent(submit); }
            Assert.That(hut.Paused, Is.False, "Resuming an inherited station pause must reach the authoritative station flag.");
            position = Root.Query<VisualElement>(className: "work-position").ToList().Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + cats[1].Name));
            var pause = position.Query<Button>().ToList().Single(b => b.text == "Pause work");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = pause; pause.SendEvent(submit); }
            Assert.That(hut.Slots.Single(s => s.CatId == cats[1].Id).Paused, Is.True, "A research worker must use the existing authoritative per-slot pause action.");
            Submit("Release worker · " + cats[1].Name);
            Assert.That(cats[1].BuildingId, Is.Empty);
            Assert.That(Root.Q<Label>(className: "staff-summary").text, Is.EqualTo("1 / 2 work positions staffed"));
        }

        [Test]
        public void WorkshopWorkerStatusExplainsTheCurrentJobAndNeedsInsteadOfStaleQueueBlockers()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            var slot = station.Slots.Single(s => s.CatId == cat.Id);
            slot.BlockedReason = "missing_input";
            var job = new Job { Id = "ui-worker-current-job", Kind = "production", RecipeId = "logs_to_planks", CatId = cat.Id, TargetId = station.Id, Phase = "fetch", BlockedReason = "blocked_route", RequiredWork = 60 };
            game.Selected.Jobs.Add(job); cat.JobId = slot.JobId = job.Id;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var position = Root.Q<VisualElement>(className: "work-position");
            Assert.That(position.Q<Label>(className: "worker-status")?.text, Is.EqualTo("logs to planks · Collecting ingredients"));
            Assert.That(position.Query<Label>().ToList().Any(l => l.text.Contains("No walkable route")), Is.True);
            Assert.That(position.Query<Label>().ToList().Any(l => l.text.Contains("Waiting for recipe ingredients")), Is.False, "An old queue check must not hide the actual job's route problem.");
            job.BlockedReason = "input_unreachable";
            game.UI.OpenPanel("Inspect");
            position = Root.Q<VisualElement>(className: "work-position");
            Assert.That(position.Query<Label>().ToList().Any(l => l.text == "Recipe ingredients cannot reach this workshop. Check the stockpile access and workshop route."), Is.True, "A route shortage must explain access rather than blame another worker's reservation.");
            job.Completed = true; cat.JobId = ""; cat.Goal = "need_drink"; cat.BlockedReason = "";
            game.UI.OpenPanel("Inspect");
            position = Root.Q<VisualElement>(className: "work-position");
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Getting water · work position kept"));
            Assert.That(position.Query<Label>().ToList().Any(l => l.text.Contains("Waiting for recipe ingredients")), Is.False, "A worker meeting a personal need has not abandoned its assigned position.");
        }

        [UnityTest]
        public IEnumerator InspectorKeepsItsChooserUntilThePlayerFinishesChoosing()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null;
            var choice = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
            choice.Focus();
            var menu = new GenericDropdownMenu();
            foreach (var name in choice.choices) menu.AddItem(name, false, () => { });
            menu.DropDown(choice.worldBound, choice, true, false);
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat"), Is.SameAs(choice), "The inspector must not replace the cat chooser while its menu is open.");
            var popup = Root.panel.visualTree.Q<VisualElement>(className: GenericDropdownMenu.ussClassName);
            Assert.That(popup, Is.Not.Null, "The inspector's live refresh must leave the player's popup attached.");
            popup.RemoveFromHierarchy();
            FindButton("Close").Focus();
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat"), Is.Not.SameAs(choice), "Ordinary inspector refresh must resume after the chooser closes and focus leaves it.");
        }

        [TestCase("accounting_tent")]
        [TestCase("mouse_farm")]
        public void SpecializedWorkersDoNotOfferPauseControlsTheirActiveWorkDoesNotHonor(string kind)
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "research_hut");
            station.Kind = kind;
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            var slot = station.Slots.Single(s => s.CatId == cat.Id);
            slot.Paused = station.Paused = true;
            if (kind == "mouse_farm")
            {
                var job = new Job { Id = "ui-mouse-farm-active-job", Kind = kind, CatId = cat.Id, TargetId = station.Id, Phase = "working", RequiredWork = 600 };
                game.Selected.Jobs.Add(job); cat.JobId = slot.JobId = job.Id;
            }
            else cat.Goal = "accounting · returning to tent";
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var position = Root.Q<VisualElement>(className: "work-position");
            Assert.That(position.Query<Button>().ToList().Any(b => b.text == "Pause work" || b.text == "Resume work"), Is.False, "The inspector must not offer a pause the active specialist work ignores.");
            Assert.That(position.Q<Label>(className: "worker-status").text.Contains("Paused"), Is.False, "Ignored saved flags must not claim that active work has stopped.");
        }

        [Test]
        public void WorkplacesKeepProductionAndConstructionReachableWithoutThirtyPassiveDenRows()
        {
            int existingDens = game.Selected.Buildings.Count(b => b.Kind == "den" && b.Completed);
            for (int index = existingDens; index < 30; index++)
                game.Selected.Buildings.Insert(0, new Building { Id = "ui-passive-den-" + index, Kind = "den", Completed = true, Position = new Int2(index * 3, 30) });
            foreach (var kind in new[] { "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "walls", "workshop", "field", "school", "accounting_tent", "mouse_farm" })
                if (!game.Selected.Buildings.Any(b => b.Kind == kind)) game.Selected.Buildings.Add(new Building { Id = "ui-work-list-" + kind, Kind = kind, Completed = true });
            var scaffold = new Building { Id = "ui-unfinished-den", Kind = "den", Completed = false };
            game.Selected.Buildings.Add(scaffold);
            game.UI.OpenPanel("Work");
            var buttons = Root.Query<Button>().ToList();
            Assert.That(buttons.Count(b => b.text.StartsWith("den · ", StringComparison.Ordinal)), Is.EqualTo(1), "Thirty completed Dens must not bury productive workplaces; the unfinished Den still needs a builder.");
            foreach (var kind in new[] { "food storage", "water bowl", "beds", "herb garden", "nursery", "elder corner", "walls", "shrine" })
                Assert.That(buttons.Any(b => b.text.StartsWith(kind + " · ", StringComparison.Ordinal)), Is.False, "A completed passive building has no productive worker position: " + kind);
            foreach (var kind in new[] { "wood cutter", "workshop", "research hut", "school", "accounting tent", "mouse farm" })
                Assert.That(buttons.Any(b => b.text.StartsWith(kind + " · ", StringComparison.Ordinal)), Is.True, "A functioning workplace must remain listed: " + kind);
            Assert.That(buttons.Any(b => b.text == "field · farm support"), Is.True);
            Assert.That(buttons.Any(b => b.text == "barracks · training site"), Is.True);
            foreach (var action in new[] { "hunt", "train", "Designate grain farm", "Research work · choose workplace" })
                Assert.That(buttons.Any(b => b.text == action), Is.True, "Filtering workplaces must retain manual work: " + action);
            var woodCutter = buttons.First(b => b.text.StartsWith("wood cutter · ", StringComparison.Ordinal));
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = woodCutter; woodCutter.SendEvent(submit); }
            Assert.That(game.View.SelectedBuilding.Kind, Is.EqualTo("wood_cutter"));
            Assert.That(FindButton("Assign worker"), Is.Not.Null);
            Submit("Back");
            var researchHut = Root.Query<Button>().ToList().First(b => b.text.StartsWith("research hut · 0 / 1 staffed", StringComparison.Ordinal));
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = researchHut; researchHut.SendEvent(submit); }
            Assert.That(game.View.SelectedBuilding.Kind, Is.EqualTo("research_hut"));
            Assert.That(FindButton("Assign worker"), Is.Not.Null);
            game.View.InspectBuilding(game.Selected.Buildings.First(b => b.Kind == "den" && b.Completed).Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Q<Label>(className: "staff-summary"), Is.Null, "Passive buildings must not invent a staffing benefit when inspected from the world.");
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "Assign worker"), Is.False);
            game.UI.OpenPanel("Work");
            var unfinishedDen = Root.Query<Button>().ToList().Single(b => b.text.StartsWith("den · under construction", StringComparison.Ordinal));
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = unfinishedDen; unfinishedDen.SendEvent(submit); }
            Assert.That(game.View.SelectedBuilding.Id, Is.EqualTo(scaffold.Id));
            Assert.That(FindButton("Assign worker"), Is.Not.Null, "A planned Den must retain its real builder assignment.");
        }

        [UnityTest]
        public IEnumerator WorkerInspectorReturnsToItsWorkshopAndScrollWithoutLeakingAcrossNavigation()
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000; Root.style.height = Root.style.maxHeight = 620;
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int index = 0; index < 8; index++) Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.UI.OpenPanel("Work"); game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 220);
            yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(100), "The return flow must preserve a genuinely scrolled workshop.");
            Submit("Inspect worker · " + cat.Name);
            Assert.That(game.View.SelectedCat.Id, Is.EqualTo(cat.Id));
            yield return null; yield return null;
            Submit("Back");
            yield return null; yield return null;
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"), "Back from a worker must return to the originating workshop rather than the Work roster.");
            Assert.That(game.View.SelectedBuilding?.Id, Is.EqualTo(station.Id));
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.5f), "Returning from a worker must restore the workshop's queue position.");
            Submit("Inspect worker · " + cat.Name);
            game.UI.OpenPanel("Cats"); game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect"); Submit("Back");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"), "Explicit category navigation must discard the old workshop return.");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect"); Submit("Inspect worker · " + cat.Name);
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "Workshop return isolation" }).Result.Success, Is.True);
            game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id); game.UI.OpenPanel("Inspect"); Submit("Back");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"), "A different village must never reopen a workshop from the previous village.");
            Assert.That(game.View.SelectedBuilding, Is.Null);
        }

        [Test]
        public void DirectControlOffersOnlyReachableStockpilesAndTransfersTheirGoods()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
            game.View.InspectCat(cat.Id);
            Assert.That(game.Send(new GameAction { Kind = "EnterCatControl", CatId = cat.Id }).Result.Success, Is.True);
            cat.Cargo.Clear();
            var origin = new Int2(4, 0); var destination = new Int2(6, 0);
            foreach (var point in new[] { origin, new Int2(5, 0), destination })
            {
                var tile = game.CurrentWorld.TileAt(point); tile.Wall = tile.Mountain = tile.Water = false;
            }
            var pile = new Stockpile { Id = "ui-control-transfer", Position = destination, Width = 1, Depth = 1, Capacity = 100 };
            pile.Goods.Add(new Simulation.Stack("food", 3)); game.Selected.Stockpiles.Add(pile);
            bool OffersPile() => Root.Query<Button>().ToList().Any(b => b.text == "Take from " + pile.Id || b.text == "Deposit at " + pile.Id);
            cat.Position = origin; cat.X = origin.X; cat.Z = origin.Z;
            game.UI.OpenPanel("Inspect");
            Assert.That(OffersPile(), Is.False, "A pile two tiles away cannot be used yet.");
            cat.Position = new Int2(5, 0); cat.X = 4.75;
            game.UI.OpenPanel("Inspect");
            Assert.That(OffersPile(), Is.False, "The cat must physically reach interaction range, not only change its anchor tile.");
            cat.X = cat.Position.X;
            game.CurrentWorld.TileAt(destination).Wall = true; game.UI.OpenPanel("Inspect");
            Assert.That(OffersPile(), Is.False, "A wall must not expose a storage interaction.");
            game.CurrentWorld.TileAt(destination).Wall = false;
            var fence = new BoundaryEdge { From = cat.Position, To = destination }; game.Selected.BoundaryEdges.Add(fence);
            game.UI.OpenPanel("Inspect");
            Assert.That(OffersPile(), Is.False, "A closed fence edge must not expose storage on its far side.");
            game.Selected.BoundaryEdges.Remove(fence); game.UI.OpenPanel("Inspect");
            Root.Query<DoubleField>().ToList().Single(f => f.label == "Amount").value = 1;
            Submit("Take from " + pile.Id);
            Assert.That(cat.Cargo.Single(s => s.Resource == "food").Amount, Is.EqualTo(1), game.LastAction);
            Assert.That(pile.Goods.Single(s => s.Resource == "food").Amount, Is.EqualTo(2));
            Submit("Deposit at " + pile.Id);
            Assert.That(cat.Cargo, Is.Empty, game.LastAction);
            Assert.That(pile.Goods.Single(s => s.Resource == "food").Amount, Is.EqualTo(3));
        }

        [UnityTest]
        public IEnumerator NavigationStartsAtTheTopWhileLiveRefreshKeepsItsPlace()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            for (int i = 0; i < 8; i++) Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.UI.OpenPanel("Build");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 250);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must actually scroll the building list.");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "A new inspector must show its staffing and status before the queue.");
            yield return null; yield return null;
            drawer.scrollOffset = new Vector2(0, 150);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100));
            game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id); game.UI.OpenPanel("Inspect");
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Changing the inspected subject must also start at the top.");
            yield return null; yield return null;
            drawer.scrollOffset = new Vector2(0, 100);
            yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(25));
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.1f), "The ordinary live refresh must preserve the player's place.");
        }

        [UnityTest]
        public IEnumerator ResearchPrerequisiteNavigationStartsAtTheRequestedStudy()
        {
            game.UI.OpenPanel("Research");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 250);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must actually scroll the research catalog.");
            var prerequisite = Root.Query<Button>().ToList().First(b => b.text == "Requires Research Hut");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = prerequisite; prerequisite.SendEvent(submit); }
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Following a prerequisite must reveal the requested study at the top, not reuse the old catalog offset.");
            Assert.That(Root.Query<TextField>().ToList().Single(f => f.label == "Search studies").value, Is.EqualTo("research_hut"));
            var firstCard = Root.Query<VisualElement>(className: "research-card").ToList().First();
            Assert.That(firstCard.Q<Label>(className: "subtitle").text, Is.EqualTo("Research Hut"));
        }

        [UnityTest]
        public IEnumerator NarrowWindowKeepsHeaderRowsAndScrollablePanelsSeparate()
        {
            game.Selected.ResearchPoints = 19995; game.Selected.Blessings = 200;
            Root.style.flexGrow = 0; Root.style.width = 600; Root.style.maxWidth = 600; Root.style.height = 360; Root.style.maxHeight = 360;
            game.UI.OpenPanel("Build");
            yield return null; yield return null; yield return null;
            Assert.That(Root.worldBound.width, Is.EqualTo(600).Within(.5f), "The regression must exercise the narrow panel width.");
            var header = Root.Q<VisualElement>(className: "top");
            var brand = Root.Q<VisualElement>(className: "brand");
            var clock = Root.Q<VisualElement>(className: "clock");
            var resources = Root.Q<VisualElement>(className: "resources");
            Assert.That(resources.worldBound.yMin, Is.GreaterThanOrEqualTo(Mathf.Max(brand.worldBound.yMax, clock.worldBound.yMax)), "Narrow windows need resources below the village name and time controls.");
            Assert.That(brand.worldBound.xMax, Is.LessThanOrEqualTo(clock.worldBound.xMin), "The village summary must not overlap the clock.");
            var attention = Root.Q<Button>(className: "attention-seal");
            AssertInside(header.worldBound, attention.worldBound, "The attention control must fit inside the compact header.");
            foreach (var neighbor in new[] { brand, clock, resources })
                Assert.That(attention.worldBound.Overlaps(neighbor.worldBound), Is.False, "The attention control must not cover the village title, clock or resources.");
            var chips = resources.Query<VisualElement>(className: "resource").ToList();
            for (int i = 0; i < chips.Count; i++)
            {
                AssertInside(header.worldBound, chips[i].worldBound, "Every resource chip must fit inside the header.");
                foreach (var label in chips[i].Query<Label>().ToList()) AssertInside(chips[i].worldBound, label.worldBound, "Resource values and names must fit their row without inherited label padding.");
                if (i > 0) Assert.That(chips[i - 1].worldBound.xMax, Is.LessThanOrEqualTo(chips[i].worldBound.xMin + .5f), "Resource values must not overlap adjacent chips.");
            }
            var speeds = clock.Query<Button>().ToList();
            foreach (var button in speeds)
            {
                AssertInside(header.worldBound, button.worldBound, "All four time controls must fit inside the header.");
                Assert.That(button.worldBound.yMin, Is.EqualTo(speeds[0].worldBound.yMin).Within(.5f), "Time controls must stay on one row.");
            }
            var nav = Root.Q<ScrollView>(className: "navigation");
            var drawer = Root.Q<VisualElement>(className: "drawer");
            var bottom = Root.Q<VisualElement>(className: "bottom");
            foreach (var panel in new VisualElement[] { nav, drawer })
            {
                Assert.That(panel.worldBound.yMin, Is.GreaterThanOrEqualTo(header.worldBound.yMax), "Management panels must begin below the taller narrow header.");
                Assert.That(panel.worldBound.yMax, Is.LessThanOrEqualTo(bottom.worldBound.yMin), "The scrollable panels must not cover the footer.");
                Assert.That(panel.worldBound.height, Is.GreaterThan(40), "Short windows still need a usable scroll viewport.");
            }
            foreach (var size in new[] { new Vector2(600, 360), new Vector2(550, 336) })
            {
                Root.style.width = Root.style.maxWidth = size.x; Root.style.height = Root.style.maxHeight = size.y;
                foreach (var section in new[] { "Cats", "Inspect" })
                {
                    if (section == "Inspect") game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id);
                    game.UI.OpenPanel(section);
                    yield return null; yield return null; yield return null;
                    Assert.That(Root.Q<ScrollView>(className: "drawer-scroll").contentViewport.worldBound.height, Is.GreaterThanOrEqualTo(70), section + " needs readable content beneath its heading at " + size);
                    AssertInside(drawer.worldBound, FindButton("Close").worldBound, "Close must remain visible in short inspectors.");
                    Assert.That(drawer.worldBound.yMax, Is.LessThanOrEqualTo(bottom.worldBound.yMin), "The narrow drawer must remain above the footer.");
                }
            }
        }

        private static readonly System.Reflection.BindingFlags FarmInspectionFlags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;

        private void SetFarmInspectionSize(float width, float height)
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = width; Root.style.height = Root.style.maxHeight = height;
        }

        private Farm[] PrepareFarmInspectionPlots(int preceding)
        {
            game.View.enabled = false; game.UI.ClosePanel();
            var world = game.CurrentWorld; var village = game.Selected;
            int xStart = village.Center.X + village.Radius + 6, zStart = village.Center.Z;
            // Bounded authored support; every plot is created by public designation.
            for (int x = village.Center.X + village.Radius + 1; x <= xStart + 16; x++)
                for (int z = zStart - 6; z <= zStart + 8; z++)
                {
                    var at = new Int2(x, z); var tile = world.TileAt(at); World.ClearSurface(tile, 0);
                    tile.Wall = tile.Bridge = tile.Road = tile.Rail = false; tile.ClaimId = "";
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
            var field = new Building { Id = world.Id("ui-inspection-field"), Kind = "field", Position = new Int2(xStart, zStart - 6), Completed = true };
            village.Buildings.Add(field);
            var plots = new System.Collections.Generic.List<Farm>();
            for (int index = 0; index <= preceding; index++)
            {
                var at = index == preceding ? new Int2(xStart, zStart + 5) : new Int2(xStart + index % 8 * 2, zStart - 4 + index / 8 * 3);
                var end = index == preceding ? new Int2(at.X + 1, at.Z + 1) : at;
                var result = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = at, End = end }).Result;
                Assert.That(result.Success, Is.True, result.Error); plots.Add(village.Farms.Single(f => f.Id == result.EntityId));
            }
            RefreshFarmInspectionSetupFeedback();
            game.View.FocusVillage(); typeof(ForestView).GetMethod("Reconcile", FarmInspectionFlags).Invoke(game.View, null);
            // Aim the isolated camera at the support patch and leave ample world
            // space beside both ordinary and compact drawers. Navigation itself
            // must not recenter or zoom it.
            game.View.InspectBuilding(field.Id); game.View.InspectBuilding(""); game.View.ZoomBy(-20);
            Physics.SyncTransforms(); Assert.That(world.Validate(), Is.Empty);
            return plots.ToArray();
        }

        private void RefreshFarmInspectionSetupFeedback()
        {
            // Public setup/Assign commands intentionally publish fresh feedback.
            // Render it before capturing the navigation-only conservation fence.
            typeof(ForestUI).GetField("nextRefresh", FarmInspectionFlags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", FarmInspectionFlags).Invoke(game.UI, null);
        }

        private VisualElement AssertFarmInspectionVisible(Farm farm, bool wholeCard = true)
        {
            Assert.That(game.UI.OpenSection, Is.EqualTo("Work"));
            var card = Root.Q<VisualElement>("farm-card-" + farm.Id);
            Assert.That(card, Is.Not.Null); Assert.That(card.panel, Is.SameAs(Root.panel));
            string identity = "Plot · " + farm.Id + " · " + farm.Position + " · " + farm.Width + " × " + farm.Depth;
            var plot = card.Query<Label>().ToList().Single(l => l.text == identity);
            var viewport = Root.Q<ScrollView>(className: "drawer-scroll").contentViewport;
            Assert.That(viewport.worldBound.height, Is.GreaterThan(0)); Assert.That(card.worldBound.height, Is.GreaterThan(0));
            AssertInside(viewport.worldBound, plot.worldBound, "The exact current plot identity must be visible without manual scrolling.");
            var buttons = card.Query<Button>().ToList();
            Assert.That(buttons.Count(b => b.text == "Assign selected cat"), Is.EqualTo(1)); Assert.That(buttons.Count(b => b.text == "Clear farm"), Is.EqualTo(1));
            if (wholeCard) AssertInside(viewport.worldBound, card.worldBound, "The exact card and its existing controls must fit in the settled viewport.");
            return card;
        }

        private Action FarmInspectionAuthorityUnchanged()
        {
            string before = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction; int revision = game.Revision;
            var feedback = Root.Q<Label>(className: "feedback");
            var shown = (feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback"));
            return () =>
            {
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before), "Inspection must preserve full authority, clocks, RNG, goods and ownership.");
                Assert.That(game.Revision, Is.EqualTo(revision)); Assert.That(game.LastAction, Is.EqualTo(action), "Inspection must issue no accepted or rejected command.");
                Assert.That((feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback")), Is.EqualTo(shown));
            };
        }

        private sealed class FarmInspectionInput : IDisposable
        {
            private readonly ForestGame game;
            private readonly Mouse mouse;
            private readonly Keyboard keyboard;
            public FarmInspectionInput(ForestGame game, VisualElement root, bool isolatePicking = true)
            {
                this.game = game;
                if (isolatePicking) foreach (var element in root.Query<VisualElement>(className: "interactive").ToList()) element.RemoveFromClassList("interactive");
                mouse = InputSystem.AddDevice<Mouse>(); keyboard = InputSystem.AddDevice<Keyboard>(); mouse.MakeCurrent(); keyboard.MakeCurrent();
            }
            public Vector3 Project(Int2 at)
            {
                typeof(ForestView).GetMethod("UpdateCamera", FarmInspectionFlags).Invoke(game.View, null); Physics.SyncTransforms();
                var pixel = game.View.Camera.WorldToScreenPoint(new Vector3(at.X, (float)game.CurrentWorld.WalkHeight(at), at.Z));
                Assert.That(pixel.z, Is.GreaterThan(0)); Assert.That(pixel.x, Is.InRange(1f, Screen.width - 1f)); Assert.That(pixel.y, Is.InRange(1f, Screen.height - 1f));
                return pixel;
            }
            public void Click(Int2 at, bool overPanel = false, Vector2? fixedPixel = null)
            {
                var pixel = fixedPixel ?? (Vector2)Project(at);
                var released = new MouseState { position = pixel };
                InputState.Change(mouse, released, InputUpdateType.Dynamic);
                Assert.That(game.UI.PointerOverPanel, Is.EqualTo(overPanel), "The composed input fixture must prove real UI ownership or its explicit picking isolation.");
                InputState.Change(mouse, released.WithButton(UnityEngine.InputSystem.LowLevel.MouseButton.Left), InputUpdateType.Dynamic);
                Assert.That(mouse.leftButton.wasPressedThisFrame, Is.True);
                typeof(ForestView).GetMethod("ReadInput", FarmInspectionFlags).Invoke(game.View, null);
                Assert.That(game.View.CursorTile, Is.EqualTo(at), "Reproject after every drawer/camera change and hit the intended tile.");
                InputState.Change(mouse, released, InputUpdateType.Dynamic);
            }
            public void Dispose()
            {
                InputSystem.RemoveDevice(mouse); InputSystem.RemoveDevice(keyboard);
            }
        }

        private void AssertFirstPlacementPoint(string section, string tool, string expectedHint)
        {
            game.UI.OpenPanel(section); Submit(tool);
            Assert.That(game.UI.HasPlacement, Is.True, "The public tool must enter placement before the first click.");
            var point = new Int2(game.Selected.Center.X + game.Selected.Radius + 1, game.Selected.Center.Z + 1);
            AssertPlacementUiOnly(() => PlaceUiTile(point));
            AssertPlacementHint(expectedHint);
        }

        private void AssertPlacementUiOnly(Action interaction)
        {
            var world = game.CurrentWorld; string before = JsonUtility.ToJson(world), action = game.LastAction;
            int revision = game.Revision;
            var feedback = Root.Q<Label>(className: "feedback");
            var feedbackBefore = (feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback"));
            interaction();
            RefreshFarmFeedbackWithoutAuthorityChanges();
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before), "Choosing or cancelling a point must preserve the full authoritative world.");
            Assert.That(game.Revision, Is.EqualTo(revision), "A draft point or tool switch must not dispatch an accepted command.");
            Assert.That(game.LastAction, Is.EqualTo(action), "A draft point or tool switch must not replace command feedback, even with a rejection.");
            Assert.That((feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback")), Is.EqualTo(feedbackBefore));
        }

        private void AssertPlacementHint(string expectedHint)
        {
            Assert.That(game.UI.HasPlacement, Is.True, "Choosing the first corner must keep placement active.");
            var hint = Root.Q<Label>(className: "hint");
            Assert.That(hint.panel, Is.Not.Null);
            Assert.That(hint.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(hint.resolvedStyle.visibility, Is.EqualTo(Visibility.Visible));
            Assert.That(hint.text, Is.EqualTo(expectedHint + " · Esc cancels"), "The refreshed visible hint must describe this tool's next point.");
        }

        private void AssertNoPlacementHint()
        {
            Assert.That(game.UI.HasPlacement, Is.False);
            Assert.That(Root.Q<Label>(className: "hint").text, Does.Not.Contain("Choose opposite corner").And.Not.Contain("Choose end of route").And.Not.Contain("Esc cancels"));
        }

        private void PlaceUiTile(Int2 point)
        {
            var place = typeof(ForestUI).GetMethod("Place", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            Assert.That(place, Is.Not.Null);
            place.Invoke(game.UI, new object[] { point });
        }

        private (Farm Manual, Farm Automatic, Cat ManualWorker, Cat FarmerWorker, Cat Replacement) PrepareFarmCards()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0]; store.Position = new Int2(3, -4); store.Width = store.Depth = 1; store.Capacity = 100000;
            store.Goods = new System.Collections.Generic.List<Simulation.Stack> { new Simulation.Stack("food", 1000), new Simulation.Stack("water", 1000), new Simulation.Stack("fibre", 10) };
            village.LastLeaderResearch = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            for (int index = 0; index < village.Cats.Count; index++)
            {
                // One manual owner, one Farmer hire and the scaled four-adult reserve.
                var cat = village.Cats[index]; cat.ControlledBy = index >= 1 && index <= 6 ? "" : "fixture-held"; cat.ControlLeaseUntil = 10000000;
                cat.Skills.Clear(); cat.Preferences.Clear(); cat.Boosted = false; cat.Hunger = cat.Thirst = cat.Rest = 100; PlaceFarmCat(cat, new Int2(1, 3));
            }
            var manual = village.Cats[1]; var automatic = village.Cats[2]; var replacement = village.Cats[3];
            manual.Name = "Manual Plot Worker"; automatic.Name = "Farmer Plot Worker"; replacement.Name = "Prepared Manual Worker";
            automatic.Preferences.Add("farm"); World.Add(automatic.Skills, "farm", 1);
            // Reuse the finite public prerequisite chain from the farm scenarios.
            village.ResearchPoints = 23;
            foreach (string study in new[] { "research_hut", "water_carriers", "irrigation" })
            {
                var researched = game.Send(new GameAction { Kind = "ResearchNode", NodeId = study }).Result;
                Assert.That(researched.Success, Is.True, researched.Error);
            }
            Assert.That(village.ResearchPoints, Is.Zero);
            village.Buildings.Add(new Building { Id = world.Id("ui-farm-field"), Kind = "field", Position = new Int2(4, 4), Width = 2, Depth = 2, Completed = true });
            for (int x = 9; x <= 11; x++)
                for (int z = 0; z <= 6; z++)
                {
                    var at = new Int2(x, z); World.ClearSurface(world.TileAt(at), 0);
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
            Farm Designate(int z)
            {
                var result = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, z), End = new Int2(10, z) }).Result;
                Assert.That(result.Success, Is.True, result.Error); return village.Farms.Single(f => f.Id == result.EntityId);
            }
            var manualFarm = Designate(1); var automaticFarm = Designate(3);
            Assert.That(world.Path(automatic.Position, automaticFarm.Position, village), Is.Not.Null);
            var assigned = game.Send(new GameAction { Kind = "AssignWorker", CatId = manual.Id, TargetId = manualFarm.Id }).Result;
            Assert.That(assigned.Success, Is.True, assigned.Error); Assert.That(automaticFarm.WorkerId, Is.Empty); Assert.That(world.Validate(), Is.Empty);
            return (manualFarm, automaticFarm, manual, automatic, replacement);
        }

        private static void PlaceFarmCat(Cat cat, Int2 at)
        {
            cat.Position = at; cat.X = at.X; cat.Z = at.Z; cat.Path.Clear();
        }

        private VisualElement WorkFarmCard(string farmId)
        {
            var clear = Root.Query<Button>().ToList().SingleOrDefault(b => b.text == "Clear farm" && b.parent.Query<Label>().ToList().Any(l => l.text.Contains(farmId)));
            Assert.That(clear, Is.Not.Null, "Each farm must visibly identify its exact plot ID beside its own Assign and Clear controls: " + farmId);
            return clear.parent;
        }

        private static string FarmCardCopy(VisualElement card) => string.Join("\n", card.Query<Label>().ToList().Select(l => l.text));

        private (Stockpile Source, Stockpile Destination, Vehicle Vehicle, System.Collections.Generic.List<Int2> Path) PrepareDispatchLane(string name, int z)
        {
            var village = game.Selected;
            var source = new Stockpile { Id = "ui-dispatch-" + name + "-source", Position = new Int2(-32, z), Width = 1, Depth = 1, Capacity = 8 };
            var destination = new Stockpile { Id = "ui-dispatch-" + name + "-destination", Position = new Int2(-28, z), Width = 1, Depth = 1, Capacity = 8 };
            var path = Enumerable.Range(-32, 5).Select(x => new Int2(x, z)).ToList();
            foreach (var at in path) PrepareDispatchTile(at, at.Equals(source.Position) || at.Equals(destination.Position));
            source.Goods.Add(new Simulation.Stack("logs", 8)); village.Stockpiles.Add(source); village.Stockpiles.Add(destination);
            Assert.That(game.CurrentWorld.TileAt(source.Position).Water || game.CurrentWorld.TileAt(destination.Position).Water, Is.False, "Both one-tile endpoint footprints must remain dry.");
            var vehicle = new Vehicle { Id = "ui-dispatch-" + name + "-vessel", Mode = "shipping", Position = source.Position, HasContinuousPosition = true, X = source.Position.X, Z = source.Position.Z };
            village.Vehicles.Add(vehicle);
            return (source, destination, vehicle, path);
        }

        private void PrepareDispatchTile(Int2 at, bool dock, Stockpile replacing = null)
        {
            var village = game.Selected;
            Assert.That(village.Buildings.Any(b => b.Position.Level == at.Level && at.X >= b.Position.X && at.X < b.Position.X + b.Width && at.Z >= b.Position.Z && at.Z < b.Position.Z + b.Depth), Is.False, "Authored dispatch geometry must not overlap retained building footprints.");
            Assert.That(village.Stockpiles.Any(p => p != replacing && p.Position.Level == at.Level && at.X >= p.Position.X && at.X < p.Position.X + p.Width && at.Z >= p.Position.Z && at.Z < p.Position.Z + p.Depth), Is.False, "Authored dispatch geometry must not overlap another retained pile footprint.");
            var tile = game.CurrentWorld.TileAt(at); World.ClearSurface(tile, 0); tile.Wall = tile.Bridge = tile.Rail = tile.Road = tile.Dirt = false; tile.Dock = dock;
            if (!dock) World.SetWaterSurface(tile, 0, .8);
        }

        private Cat[] PrepareFarmFeedbackTether()
        {
            Root.style.flexGrow = 0;
            Root.style.width = Root.style.maxWidth = 1000;
            Root.style.height = Root.style.maxHeight = 620;
            game.UI.ClosePanel();
            var cats = game.Selected.Cats.Where(c => c.Alive && c.Id != game.Selected.LeaderId && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(cats.Length, Is.EqualTo(2), "Public takeover must use genuinely free living cats.");
            Assert.That(game.Speed, Is.Zero);
            return cats;
        }

        private void RefreshFarmFeedbackWithoutAuthorityChanges()
        {
            int revision = game.Revision;
            var feedback = Root.Q<Label>(className: "feedback");
            string copy = feedback.text; var display = feedback.style.display.value;
            bool failed = feedback.ClassListContains("failure"), shown = Root.ClassListContains("has-feedback");
            RefreshPausedUiWithoutAuthorityChanges();
            Assert.That(game.Revision, Is.EqualTo(revision));
            Assert.That(feedback.text, Is.EqualTo(copy)); Assert.That(feedback.style.display.value, Is.EqualTo(display));
            Assert.That(feedback.ClassListContains("failure"), Is.EqualTo(failed)); Assert.That(Root.ClassListContains("has-feedback"), Is.EqualTo(shown));
        }

        private void RefreshPausedUiWithoutAuthorityChanges()
        {
            var world = game.CurrentWorld; string before = JsonUtility.ToJson(world), action = game.LastAction;
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestUI).GetField("nextRefresh", flags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", flags).Invoke(game.UI, null);
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(before), "Paused feedback refresh must not advance or rewrite authority.");
            Assert.That(game.LastAction, Is.EqualTo(action), "Reading feedback must issue no command.");
            Assert.That(world.Validate(), Is.Empty);
        }

        private void AssertAssignedWorkHeader(int assigned, int alive = 30)
        {
            string before = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction;
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestUI).GetField("nextRefresh", flags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", flags).Invoke(game.UI, null);
            Assert.That(Root.Q<Label>(className: "village-summary").text,
                Is.EqualTo(alive + " cats · " + assigned + " assigned · " + game.Selected.Research.Count + " studies"),
                "The header must count each living cat's retained work commitment once, including jobless positions and resumable orders.");
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before), "Reading the assignment summary must not advance or rewrite authority.");
            Assert.That(game.LastAction, Is.EqualTo(action), "Reading the assignment summary must send no command.");
        }

        private static void AssertInside(Rect outer, Rect inner, string reason)
        {
            Assert.That(inner.xMin, Is.GreaterThanOrEqualTo(outer.xMin - .5f), reason);
            Assert.That(inner.xMax, Is.LessThanOrEqualTo(outer.xMax + .5f), reason);
            Assert.That(inner.yMin, Is.GreaterThanOrEqualTo(outer.yMin - .5f), reason);
            Assert.That(inner.yMax, Is.LessThanOrEqualTo(outer.yMax + .5f), reason);
        }

        private Button FindButton(string text) => Root.Query<Button>().ToList().Single(b => b.text == text);
        private void AssertFarmCardCallbacksIgnored(System.Collections.Generic.IEnumerable<Button> buttons)
        {
            foreach (var button in buttons)
            {
                string before = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction;
                int revision = game.Revision;
                var feedback = Root.Q<Label>(className: "feedback");
                string copy = feedback.text; var display = feedback.style.display.value;
                bool failed = feedback.ClassListContains("failure"), shown = Root.ClassListContains("has-feedback");
                InvokeFarmCardCallback(button);
                Assert.That(game.LastAction, Is.EqualTo(action), "An obsolete " + button.text + " control must send no command, including a rejected one.");
                Assert.That(game.Revision, Is.EqualTo(revision), "An obsolete control must not dispatch a successful command.");
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before), "An obsolete control must leave the full authoritative world unchanged.");
                Assert.That(feedback.text, Is.EqualTo(copy)); Assert.That(feedback.style.display.value, Is.EqualTo(display));
                Assert.That(feedback.ClassListContains("failure"), Is.EqualTo(failed)); Assert.That(Root.ClassListContains("has-feedback"), Is.EqualTo(shown));
            }
        }

        private static void InvokeFarmCardCallback(Button button)
        {
            // Unity 6000.6's UIElements API documents Clickable.Invoke(EventBase)
            // as "Invokes a click action". Invoke the existing manipulator's
            // protected method so detached NavigationSubmit suppression cannot
            // masquerade as callback safety. No product test hook is needed.
            var invoke = typeof(Clickable).GetMethod("Invoke", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic);
            Assert.That(invoke, Is.Not.Null);
            int observed = 0;
            void Observer() => observed++;
            button.clicked += Observer;
            try { invoke.Invoke(button.clickable, new object[] { null }); }
            finally { button.clicked -= Observer; }
            Assert.That(observed, Is.EqualTo(1), "The existing click dispatcher must invoke its registered actions. This observer proves dispatch, not entry inside the product handler.");
        }

        private void Submit(string text)
        {
            Submit(FindButton(text));
        }
        private static void Submit(Button button)
        {
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
