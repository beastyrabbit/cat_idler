using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Event = IdleCatForest.Simulation.Event;

namespace IdleCatForest.Tests
{
    /// <summary>
    /// The W2-16 widgets in a running HUD: keyed tables that never rebuild unchanged rows, dropdowns that
    /// survive refreshes, action buttons that report in place, the attention seal and popover, toasts and
    /// the runtime tooltip.
    /// </summary>
    public class WidgetTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => UiTestKit.Root(game);

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-widgets-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated widget test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            AttentionFeed.Invalidate();
            yield return null;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
            GameSettings.ReducedMotion = false;
        }

        private sealed class Census
        {
            public string Name;
            public int Count;
        }

        // ---- Table -------------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator TableRowsAreNotRebuiltWhenNothingChanged()
        {
            var data = new List<(string key, int value)> { ("a", 1), ("b", 2), ("c", 3) };
            int binds = 0;
            var table = new Table<(string key, int value)>(() => data.Select(d => (d.key, d)), _ => new Label(), (e, row) =>
            {
                binds++;
                ((Label)e).SetText(row.value.ToString());
            });
            Root.Add(table);
            yield return null;
            var rows = table.Children().ToList();
            int bound = binds;
            Assert.That(rows, Has.Count.EqualTo(3));
            Assert.That(table.MadeCount, Is.EqualTo(3));

            table.Refresh();
            table.Refresh();
            Assert.That(table.Children().ToList(), Is.EqualTo(rows), "An unchanged refresh replaced or reordered rows.");
            Assert.That(table.MadeCount, Is.EqualTo(3), "An unchanged refresh made new rows.");
            Assert.That(binds, Is.EqualTo(bound), "Unchanged value rows were bound again.");

            data[1] = ("b", 20);
            table.Refresh();
            Assert.That(binds, Is.EqualTo(bound + 1), "Only the changed row is bound.");
            Assert.That(table.RowOf("b"), Is.SameAs(rows[1]));
            Assert.That(((Label)rows[1]).text, Is.EqualTo("20"));

            data.Reverse();
            table.Refresh();
            Assert.That(table.Children().ToList(), Is.EqualTo(new[] { rows[2], rows[1], rows[0] }), "Reordering must move, not rebuild.");
            Assert.That(table.MadeCount, Is.EqualTo(3));

            data.RemoveAt(0);
            table.Refresh();
            Assert.That(table.childCount, Is.EqualTo(2));
            Assert.That(rows[2].parent, Is.Null, "A row whose key disappeared stays in the table.");

            // Reference rows are bound each refresh, but a careful bind leaves their elements untouched.
            var cats = new List<Census> { new Census { Name = "Hazel", Count = 1 }, new Census { Name = "Moss", Count = 2 } };
            var census = new Table<Census>(() => cats.Select(c => (c.Name, c)), _ => new Label(), (e, c) => ((Label)e).SetText(c.Name + " " + c.Count));
            Root.Add(census);
            yield return null;
            var hazel = census.RowOf("Hazel");
            cats[0].Count = 5;
            census.Refresh();
            Assert.That(census.RowOf("Hazel"), Is.SameAs(hazel));
            Assert.That(((Label)hazel).text, Is.EqualTo("Hazel 5"), "A changed reference row was not rebound.");
            Assert.That(census.MadeCount, Is.EqualTo(2));
        }

        [UnityTest]
        public IEnumerator FadingRowsLeaveWithoutDisturbingTheRest()
        {
            var keys = new List<string> { "a", "b", "c" };
            var finish = new List<Action>();
            var table = new Table<string>(() => keys.Select(k => (k, k)), k => new Label(k), null) { Leave = (_, done) => finish.Add(done) };
            Root.Add(table);
            yield return null;
            var b = table.RowOf("b");
            keys.Remove("b");
            table.Refresh();
            Assert.That(b.parent, Is.SameAs(table), "A leaving row is removed before its fade.");
            Assert.That(table.Count, Is.EqualTo(2));
            keys.Add("d");
            table.Refresh();
            Assert.That(table.Keys, Is.EqualTo(new[] { "a", "c", "d" }));
            finish.ForEach(done => done());
            Assert.That(b.parent, Is.Null);
            Assert.That(table.Children().Select(e => ((Label)e).text), Is.EqualTo(new[] { "a", "c", "d" }));
        }

        // ---- BoundDropdown -----------------------------------------------------------------------

        [UnityTest]
        public IEnumerator DropdownSurvivesRefresh()
        {
            var cats = new List<string> { "Hazel", "Moss", "Fern" };
            var picks = new List<string>();
            BoundDropdown<string> dropdown = null;
            // A reference row, so the table binds it on every refresh like a live model would.
            var table = new Table<Census>(() => new[] { ("assign", new Census { Name = "assign" }) }, _ =>
            {
                var row = new VisualElement();
                dropdown = new BoundDropdown<string>(cats, c => c, "Hazel", picks.Add);
                row.Add(dropdown);
                return row;
            }, (row, _) => row.Q<BoundDropdown<string>>().SetItems(cats));
            Root.Add(table);
            yield return null;
            Assert.That(picks, Is.Empty, "Building or binding a dropdown must not report a pick.");
            Assert.That(dropdown.Selected, Is.EqualTo("Hazel"));

            dropdown.value = "Moss";
            Assert.That(picks, Is.EqualTo(new[] { "Moss" }));
            var instance = dropdown;
            for (int i = 0; i < 3; i++)
            {
                table.Refresh();
                yield return null;
            }
            Assert.That(table.Q<BoundDropdown<string>>(), Is.SameAs(instance), "A refresh rebuilt the dropdown.");
            Assert.That(instance.Selected, Is.EqualTo("Moss"), "A refresh reset the player's pick.");

            cats = new List<string> { "Moss", "Fern", "Wren" };
            table.Refresh();
            Assert.That(instance.Selected, Is.EqualTo("Moss"), "A pick that is still listed must survive new items.");
            Assert.That(instance.choices, Is.EqualTo(cats));
            cats = new List<string> { "Fern", "Wren" };
            table.Refresh();
            Assert.That(instance.Selected, Is.EqualTo("Fern"));
            Assert.That(picks, Is.EqualTo(new[] { "Moss" }), "Refreshing items must never report a pick.");
        }

        // ---- ActionButton ------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator ActionButtonShowsPendingThenItsResultInPlace()
        {
            int clicked = 0, confirmed = 0, failed = 0, runs = 0;
            Action onClick = () => clicked++, onConfirm = () => confirmed++, onFail = () => failed++;
            UiEvents.Clicked += onClick;
            UiEvents.Confirmed += onConfirm;
            UiEvents.Failed += onFail;
            try
            {
                var gate = new TaskCompletionSource<ActionResult>();
                var button = new ActionButton("Assign Hazel", () =>
                {
                    runs++;
                    return gate.Task;
                }) { ConfirmedLabel = "Assigned" };
                Root.Add(button);
                yield return null;
                button.Press();
                Assert.That(button.Pending, Is.True);
                Assert.That(button.enabledSelf, Is.False, "A pending button must not take a second click.");
                Assert.That(button.ClassListContains(ActionButton.PendingClass), Is.True);
                button.Press();
                Assert.That(runs, Is.EqualTo(1), "A double click sent the action twice.");

                gate.SetResult(ActionResult.Ok());
                yield return null;
                yield return null;
                Assert.That(button.State, Is.EqualTo(ActionButtonState.Confirmed));
                Assert.That(button.text, Is.EqualTo("Assigned"));
                Assert.That(button.enabledSelf, Is.True);
                Assert.That((clicked, confirmed, failed), Is.EqualTo((1, 1, 0)));
                yield return new WaitForSecondsRealtime(ActionButton.ResultMilliseconds / 1000f + .3f);
                Assert.That(button.State, Is.EqualTo(ActionButtonState.Idle));
                Assert.That(button.text, Is.EqualTo("Assign Hazel"), "The label did not return after its result.");

                var refused = new ActionButton("Study now", () => Task.FromResult(ActionResult.Fail("Insufficient research points")));
                Root.Add(refused);
                refused.Press();
                yield return null;
                Assert.That(refused.State, Is.EqualTo(ActionButtonState.Failed));
                Assert.That(refused.text, Is.EqualTo("Insufficient research points"));
                Assert.That(refused.ClassListContains(ActionButton.FailedClass), Is.True);
                Assert.That(failed, Is.EqualTo(1));

                var coded = new ActionButton("Assign", () => Task.FromResult(ActionResult.Fail(Blocked.WorkerRequired, "raw engine text")));
                Root.Add(coded);
                coded.Press();
                yield return null;
                Assert.That(coded.text, Is.EqualTo(UiText.Reason(Blocked.WorkerRequired)), "A coded refusal must read the humanised reason, not the raw message.");

                var throwing = new ActionButton("Boom", () => throw new InvalidOperationException("test failure"));
                Root.Add(throwing);
                LogAssert.Expect(LogType.Exception, new System.Text.RegularExpressions.Regex("test failure"));
                throwing.Press();
                yield return null;
                Assert.That(throwing.State, Is.EqualTo(ActionButtonState.Failed));
                Assert.That(throwing.text, Is.EqualTo(HudCopy.Failed), "An exception must never reach the player as text.");
            }
            finally
            {
                UiEvents.Clicked -= onClick;
                UiEvents.Confirmed -= onConfirm;
                UiEvents.Failed -= onFail;
            }
        }

        [UnityTest]
        public IEnumerator DestructiveButtonAsksInlineFirst()
        {
            int runs = 0;
            var button = new ActionButton("Remove pile", () =>
            {
                runs++;
                return Task.FromResult(ActionResult.Ok());
            }, "Remove?", destructive: true);
            Root.Add(button);
            yield return null;

            button.Press();
            Assert.That(button.State, Is.EqualTo(ActionButtonState.Confirming));
            Assert.That(runs, Is.Zero, "A destructive action ran without asking.");
            var yes = button.Q<Button>("act-confirm-yes");
            var no = button.Q<Button>("act-confirm-no");
            Assert.That(yes, Is.Not.Null);
            Assert.That(no, Is.Not.Null);
            Assert.That(button.Q<Label>(className: "act-confirm__question").text, Is.EqualTo("Remove?"));

            UiTestKit.Click(no);
            Assert.That(button.State, Is.EqualTo(ActionButtonState.Idle));
            Assert.That(button.text, Is.EqualTo("Remove pile"));
            Assert.That(button.Q<Button>("act-confirm-yes"), Is.Null);
            Assert.That(runs, Is.Zero);
            yield return null;

            button.Press();
            UiTestKit.Click(button.Q<Button>("act-confirm-yes"));
            yield return null;
            Assert.That(runs, Is.EqualTo(1));
            Assert.That(button.State, Is.EqualTo(ActionButtonState.Confirmed), "Yes must run the action, not ask again.");
        }

        // ---- Attention seal and popover ----------------------------------------------------------

        private AttentionSeal Seal => Root.Q<AttentionSeal>();
        private AttentionPopover Popover => Root.Q<AttentionPopover>();

        [UnityTest]
        public IEnumerator SealCountsAttentionAndThePopoverOpensAndClosesWithEscape()
        {
            var expected = Attention.Compute(game.CurrentWorld, game.Selected);
            Assert.That(expected, Is.Not.Empty, "A new colony should have first steps to show.");
            UiTestKit.Refresh(game);
            yield return null;
            Assert.That(Seal.Count, Is.EqualTo(expected.Count));
            Assert.That(Seal.Q<Label>("seal-count").text, Is.EqualTo(expected.Count.ToString()));
            Assert.That(Seal.Button.name, Is.EqualTo(UiNames.Seal));

            int opened = 0, closed = 0;
            Action onOpen = () => opened++, onClose = () => closed++;
            UiEvents.Opened += onOpen;
            UiEvents.Closed += onClose;
            try
            {
                UiTestKit.Click(Seal.Button);
                yield return null;
                Assert.That(game.UI.State.PopoverOpen, Is.True);
                Assert.That(Popover.IsOpen, Is.True);
                Assert.That(Popover.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
                Assert.That(Popover.Rows.Count, Is.EqualTo(Math.Min(AttentionPopover.MaxRows, expected.Count)));
                Assert.That(Popover.Shown.Select(i => i.Id), Is.EqualTo(expected.Take(AttentionPopover.MaxRows).Select(i => i.Id)), "Rows are not most urgent first.");
                var actions = Popover.Rows.Keys.Select(k => Popover.Rows.RowOf(k).Q<ActionButton>()).ToList();
                Assert.That(actions.Count(a => a.Primary), Is.EqualTo(1), "Exactly one row action is the primary.");
                Assert.That(actions[0].Primary, Is.True, "The primary action belongs to the first row.");
                foreach (var key in Popover.Rows.Keys)
                {
                    var row = Popover.Rows.RowOf(key);
                    Assert.That(row.Q<Label>(className: "pop-row__title").text, Is.Not.Empty.And.Not.Contains("_"), "Row title is not player text: " + key);
                    Assert.That(row.Q<Button>(className: "pop-row__pin"), Is.Not.Null, "Row lacks its Show pin: " + key);
                }

                Assert.That(InputRouter.Current.Dispatch(InputIntent.Escape), Is.True, "Escape did not reach the open popover.");
                Assert.That(game.UI.State.PopoverOpen, Is.False);
                Assert.That(Popover.IsOpen, Is.False);
                Assert.That((opened, closed), Is.EqualTo((1, 1)));

                game.UI.OpenPanel(AttentionPopover.PanelId);
                yield return null;
                yield return null;
                Assert.That(Popover.IsOpen, Is.True, "OpenPanel(\"Attention\") must open the popover.");
                Assert.That(game.UI.OpenSection, Is.EqualTo(""), "OpenPanel(\"Attention\") left an empty drawer open.");
                UiTestKit.Click(Seal.Button);
                Assert.That(Popover.IsOpen, Is.False, "The seal toggles the popover closed.");
            }
            finally
            {
                UiEvents.Opened -= onOpen;
                UiEvents.Closed -= onClose;
            }
        }

        [UnityTest]
        public IEnumerator PopoverActionResolvesItsItemAndThePinSelectsTheSubject()
        {
            UiTestKit.Click(Seal.Button);
            UiTestKit.Refresh(game);
            yield return null;
            var hut = Popover.Shown.FirstOrDefault(i => i.Id.StartsWith(AttentionRule.ResearchUnstaffed + ":", StringComparison.Ordinal));
            Assert.That(hut, Is.Not.Null, "The new colony's research hut should need a scholar.");
            var row = Popover.Rows.RowOf(hut.Id);
            var action = row.Q<ActionButton>();
            Assert.That(action.text, Does.StartWith("Let ").And.EndWith(" study"));
            action.Press();
            for (int i = 0; i < 3; i++) yield return null;
            var building = game.Selected.Buildings.Single(b => b.Id == hut.Anchor);
            Assert.That(building.Slots.Any(s => s.CatId != ""), Is.True, "The row action did not staff the hut.");
            AttentionFeed.Invalidate();
            UiTestKit.Refresh(game);
            Assert.That(Popover.Shown.Any(i => i.Id == hut.Id), Is.False, "A solved item stayed in the popover.");

            if (!Popover.IsOpen) UiTestKit.Click(Seal.Button);
            UiTestKit.Refresh(game);
            yield return null;
            var anchored = Popover.Shown.First(i => i.Anchor != "");
            UiTestKit.Click(Popover.Rows.RowOf(anchored.Id).Q<Button>(className: "pop-row__pin"));
            yield return null;
            Assert.That(Popover.IsOpen, Is.False, "The pin closes the popover.");
            Assert.That(game.UI.State.Selected.Id, Is.EqualTo(anchored.Anchor), "The pin did not select the item's subject.");
        }

        // ---- Toasts ------------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator ToastsShowNewNotableEventsAndKeepAlerts()
        {
            // A stack of our own, refreshed only here; the mounted one leaves the panel.
            Root.Q<Toasts>().RemoveFromHierarchy();
            var village = game.Selected;
            village.Events.Add(new Event { Kind = EventKind.Birth, Text = "Old news", Seq = NextSeq() });
            var toasts = new Toasts(game.UI.Context);
            game.UI.Slot(UiSlots.Toasts).Add(toasts);
            double now = 100;
            toasts.Clock = () => now;
            yield return null;
            toasts.Refresh();
            Assert.That(toasts.Visible, Is.Empty, "History present when the HUD opened was replayed as toasts.");

            Add(EventKind.Completed, "Moss completed hunt");
            Add(EventKind.Research, "Basic Tools");
            Add(EventKind.Death, "Fern died from old age");
            toasts.Refresh();
            yield return null;
            var shown = toasts.Visible.ToList();
            Assert.That(shown, Has.Count.EqualTo(2), "Routine events must stay in the log.");
            var titles = shown.Select(t => t.Q<Label>(className: "toast__title").text).ToList();
            Assert.That(titles, Is.EquivalentTo(new[] { HudCopy.ToastTitle(EventKind.Research), HudCopy.ToastTitle(EventKind.Death) }));
            Assert.That(shown[0].ClassListContains("toast--alert"), Is.True, "The newest toast goes on top.");

            for (int i = 0; i < 6; i++) Add(EventKind.Birth, "Kitten " + i);
            toasts.Refresh();
            Assert.That(toasts.Visible.Count, Is.LessThanOrEqualTo(Toasts.MaxVisible));

            now += Toasts.LifeSeconds + 1;
            for (int i = 0; i < 4; i++)
            {
                toasts.Refresh();
                yield return new WaitForSecondsRealtime(.3f);
                now += Toasts.LifeSeconds + 1;
            }
            var left = toasts.Visible.ToList();
            Assert.That(left, Has.Count.EqualTo(1), "Notable toasts must expire; the alert must stay.");
            Assert.That(left[0].ClassListContains("toast--alert"), Is.True);
            UiTestKit.Click(left[0].Q<Button>(className: "toast__close"));
            yield return new WaitForSecondsRealtime(.3f);
            Assert.That(toasts.Visible, Is.Empty, "Dismissing an alert did not remove it.");

            long NextSeq() => ++game.CurrentWorld.NextEventSeq;
            void Add(string kind, string text) => village.Events.Add(new Event { Kind = kind, Text = text, Time = game.CurrentWorld.TimeSeconds, Seq = NextSeq() });
        }

        // ---- Tooltip -----------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator TooltipStaysInsideTheRoot()
        {
            var button = new Button { text = "Food" };
            Root.Add(button);
            Tooltip.Attach(button, "Last physical count of food in storage. Open Stores to see how fresh each report is.");
            yield return null;
            var bounds = Root.layout;
            Tooltip.Show(button, "Last physical count of food in storage. Open Stores to see how fresh each report is.", new Vector2(bounds.width - 2, bounds.height - 2));
            yield return null;
            yield return null;
            var label = Root.Q<Label>(Tooltip.ElementName);
            Assert.That(label, Is.Not.Null);
            Assert.That(label.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(label.layout.xMax, Is.LessThanOrEqualTo(bounds.width + .5f), "The tooltip runs off the right edge.");
            Assert.That(label.layout.yMax, Is.LessThanOrEqualTo(bounds.height + .5f), "The tooltip runs off the bottom edge.");
            Assert.That(label.layout.xMin, Is.GreaterThanOrEqualTo(0));
            Assert.That(button.tooltip, Is.Empty, "The native tooltip property would double up in the Editor.");
            Tooltip.Hide(button);
            Assert.That(label.resolvedStyle.display == DisplayStyle.None || label.style.display == DisplayStyle.None, Is.True);
        }
    }
}
