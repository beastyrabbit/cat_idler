using System;
using System.Collections;
using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class ForestStockpileAccessTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-stockpile-access-" + Guid.NewGuid().ToString("N"));
            Assert.That(Directory.Exists(directory), Is.False, "Refuse an existing fixture directory.");
            game = new GameObject("Isolated stockpile access test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            game.SetSpeed(0);
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            Assert.That(game.IsRemote, Is.False);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.Zero, "Pause before the first automatic simulation update.");
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        // Freeze this primary and its helpers after authoring; root owns RED execution.
        [UnityTest]
        public IEnumerator GeneralStockpileActionStaysAboveNormalInventoryAtNativePhysicalScale()
        {
            game.View.enabled = false;
            var panel = game.GetComponent<UIDocument>().panelSettings;
            Assert.That(panel.scaleMode, Is.EqualTo(PanelScaleMode.ConstantPhysicalSize));
            float dpi = Screen.dpi > 0 ? Screen.dpi : panel.fallbackDpi;
            float pixelsPerUnit = dpi / panel.referenceDpi * panel.scale;
            Assert.That(pixelsPerUnit, Is.GreaterThan(0));
            // Match the first FarmWorkControls case: physical pixels are not panel units.
            Root.style.flexGrow = 0;
            Root.style.width = Root.style.maxWidth = 2400f / pixelsPerUnit;
            Root.style.height = Root.style.maxHeight = 1500f / pixelsPerUnit;

            string authority = JsonUtility.ToJson(game.CurrentWorld), lastAction = game.LastAction;
            int revision = game.Revision;
            void AssertNavigationUnchanged()
            {
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(authority),
                    "Inventory navigation must preserve full authority, time, RNG, goods, reports, claims and ownership.");
                Assert.That(game.Revision, Is.EqualTo(revision));
                Assert.That(game.LastAction, Is.EqualTo(lastAction), "Navigation must issue no gameplay command.");
                Assert.That(game.Speed, Is.Zero);
                Assert.That(game.CurrentWorld.TimeSeconds, Is.Zero);
            }

            game.UI.OpenPanel("Stores");
            Submit(Root.Q<Button>("stores-inventory-tab"));
            var carrier = game.Selected.Cats.First(c => c.Alive);
            string carrierChoice = carrier.Name + " · " + carrier.Id;
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = carrierChoice;
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Resource").value = "water";
            yield return null; yield return null;

            var drawer = Root.Q<VisualElement>(className: "drawer");
            var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
            Assert.That(drawer.panel, Is.SameAs(Root.panel));
            Assert.That(scroll.panel, Is.SameAs(Root.panel));
            AssertInventory(carrierChoice);
            Assert.That(scroll.verticalScroller.highValue, Is.GreaterThan(40),
                "Use the ordinary long inventory, with its current resource rows and real piles, without fabricated filler.");
            AssertNavigationUnchanged();

            // Navigate the actual inventory before the intended RED, ending back at its top.
            foreach (float fraction in new[] { .5f, 1f, 0f })
            {
                scroll.scrollOffset = new Vector2(0, scroll.verticalScroller.highValue * fraction);
                yield return null; yield return null;
                if (fraction > 0) Assert.That(scroll.scrollOffset.y, Is.GreaterThan(0));
                else Assert.That(scroll.scrollOffset.y, Is.EqualTo(0).Within(.5f));
                AssertInventory(carrierChoice);
                AssertNavigationUnchanged();
            }

            var general = GeneralButton();
            TestContext.WriteLine($"Native physical equivalent 2400x1500: screen {Screen.width}x{Screen.height}, DPI {dpi}, reference {panel.referenceDpi}, scale {panel.scale}, pixels/unit {pixelsPerUnit}; root {Root.worldBound}, drawer {drawer.worldBound}, scroll {scroll.worldBound}, viewport {scroll.contentViewport.worldBound}, general {general.worldBound}");
            // Intended first RED: this existing exact-text action is currently buried in scroll.
            Assert.That(scroll.Contains(general), Is.False,
                "The existing Designate general stockpile action must stay outside scrolling Inventory content in the fixed Stores header.");
            AssertFixedGeneral(drawer, scroll, general);

            // After correction, ordinary scrolling must keep the same action fully reachable.
            foreach (float fraction in new[] { .5f, 1f, 0f })
            {
                scroll.scrollOffset = new Vector2(0, scroll.verticalScroller.highValue * fraction);
                yield return null; yield return null;
                AssertInventory(carrierChoice);
                AssertNavigationUnchanged();
                AssertFixedGeneral(drawer, scroll, GeneralButton());
            }

            Submit(GeneralButton());
            Assert.That(game.UI.HasPlacement, Is.True, "The real button submit must begin the existing two-corner placement tool.");
            Assert.That(game.UI.OpenSection, Is.Empty);
            AssertNavigationUnchanged();
        }

        // Phase 2 composed additions. The phase 1 primary and helpers below remain frozen.
        [UnityTest]
        public IEnumerator FixedGeneralStockpileRetainsInventoryChoicesAcrossRefreshScrollAndNavigation()
        {
            game.View.enabled = false; SetAccessSize(1000, 720);
            game.UI.OpenPanel("Stores");
            string named = PrepareAccessChoices();
            var general = GeneralButton();
            Assert.That(general.name, Is.EqualTo("stores-designate-general-stockpile"));
            var unchanged = AccessAuthorityUnchanged();
            yield return null; yield return null;
            var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
            scroll.scrollOffset = new Vector2(0, scroll.verticalScroller.highValue);
            yield return null; yield return null;
            var offset = scroll.scrollOffset;
            for (int i = 0; i < 3; i++) RefreshAccessUi();
            yield return null; yield return null;
            Assert.That(GeneralButton(), Is.SameAs(general), "Refresh must retain the one fixed control and handler.");
            Assert.That(scroll.scrollOffset.y, Is.EqualTo(offset.y).Within(.5f));
            AssertInventory(named); unchanged();
            AssertFixedGeneral(Root.Q<VisualElement>(className: "drawer"), scroll, general);
            var carrierField = AccessChoice("Cat"); var resourceField = AccessChoice("Resource");
            Submit(general);
            Assert.That(game.UI.HasPlacement, Is.True);
            Assert.That(AccessChoice("Cat"), Is.SameAs(carrierField), "Starting placement must not reconstruct the form.");
            Assert.That(AccessChoice("Resource"), Is.SameAs(resourceField));
            unchanged();
            game.UI.ClosePanel(); game.UI.OpenPanel("Stores"); AssertInventory(named); unchanged();
            Submit(Root.Q<Button>("stores-equipment-tab"));
            Assert.That(general.style.display.value, Is.EqualTo(DisplayStyle.None));
            Assert.That(AccessChoice("Cat").value, Is.EqualTo(named), "Equipment retains the exact named carrier's existing selection behavior.");
            Assert.That(Root.Query<Label>(className: "stock-row").ToList(), Is.Empty);
            Submit(Root.Q<Button>("stores-inventory-tab")); AssertInventory(named); unchanged();
            AccessChoice("Cat").value = "Choose available cat";
            var automatic = AccessChoice("Cat"); resourceField = AccessChoice("Resource");
            Submit(general);
            Assert.That(game.UI.HasPlacement, Is.True);
            Assert.That(AccessChoice("Cat"), Is.SameAs(automatic));
            Assert.That(AccessChoice("Resource"), Is.SameAs(resourceField));
            Assert.That(AccessField("selectedCat"), Is.EqualTo("")); unchanged();
            game.UI.ClosePanel(); game.UI.OpenPanel("Stores");
            AssertInventory("Choose available cat"); unchanged();
        }

        [UnityTest]
        public IEnumerator FixedGeneralStockpileRejectsDeliveredCallbacksOutsideCurrentContext()
        {
            game.View.enabled = false; SetAccessSize(1000, 720);
            game.UI.OpenPanel("Stores"); PrepareAccessChoices();
            yield return null; yield return null;
            var general = GeneralButton();
            Submit(Root.Q<Button>("stores-equipment-tab"));
            Assert.That(general.style.display.value, Is.EqualTo(DisplayStyle.None)); AssertAccessCallbackIgnored(general);
            game.UI.OpenPanel("Work"); AssertAccessCallbackIgnored(general);
            game.UI.ClosePanel(); AssertAccessCallbackIgnored(general);
            game.UI.OpenPanel("Stores"); Submit(Root.Q<Button>("stores-inventory-tab"));
            var strip = general.parent;
            general.RemoveFromHierarchy();
            Assert.That(general.panel, Is.Null); AssertAccessCallbackIgnored(general);
            strip.Add(general);
            Assert.That(general.panel, Is.SameAs(Root.panel));
            var free = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "EnterCatControl", CatId = free.Id }).Result.Success, Is.True);
            Assert.That(game.View.DirectControl, Is.True); AssertAccessCallbackIgnored(general);
            Assert.That(game.Send(new GameAction { Kind = "LeaveCatControl", CatId = free.Id }).Result.Success, Is.True);
            typeof(ForestUI).GetField("renderedRemote", AccessFlags).SetValue(game.UI, !game.IsRemote);
            AssertAccessCallbackIgnored(general);
            game.UI.OpenPanel("Stores");
            string formerVillage = game.SelectedVillageId;
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "Stockpile callback grove" }).Result.Success, Is.True);
            Assert.That(game.SelectedVillageId, Is.Not.EqualTo(formerVillage));
            AssertAccessCallbackIgnored(general); // Before any UI render for the newly selected village.
            game.UI.OpenPanel("Stores");
            var former = game.Selected;
            var replacement = JsonUtility.FromJson<Village>(JsonUtility.ToJson(former));
            game.CurrentWorld.Villages[game.CurrentWorld.Villages.IndexOf(former)] = replacement;
            Assert.That(game.Selected, Is.SameAs(replacement));
            var unchanged = AccessAuthorityUnchanged();
            Submit(general);
            Assert.That(game.UI.HasPlacement, Is.True, "A same-ID snapshot is still the current context without a reference-identity fence.");
            unchanged();
            game.UI.OpenPanel("Stores");
            AssertAccessCallbackIgnored(general); // An already active tool must not be replaced even after reopening Inventory.
            game.UI.ClosePanel(); game.UI.OpenPanel("Stores");
            Submit(AccessButton("Designate selected-resource pile"));
            game.UI.OpenPanel("Stores"); AssertAccessCallbackIgnored(general);
        }

        [UnityTest]
        public IEnumerator FixedGeneralStockpileFitsSingleStoresRowAtBothCompactSizesWithFeedback()
        {
            game.View.enabled = false; game.UI.OpenPanel("Stores");
            var unchanged = AccessAuthorityUnchanged();
            foreach (bool withFeedback in new[] { false, true })
            {
                if (withFeedback)
                {
                    Assert.That(game.Send(new GameAction { Kind = "UnknownStockpileAccessAction" }).Result.Success, Is.False);
                    unchanged = AccessAuthorityUnchanged();
                }
                foreach (var size in new[] { new Vector2(600, 360), new Vector2(550, 336) })
                {
                    SetAccessSize(size.x, size.y); RefreshAccessUi();
                    yield return null; yield return null;
                    var drawer = Root.Q<VisualElement>(className: "drawer");
                    var scroll = Root.Q<ScrollView>(className: "drawer-scroll");
                    var heading = Root.Q<VisualElement>(className: "drawer-heading");
                    var inventory = Root.Q<Button>("stores-inventory-tab");
                    var equipment = Root.Q<Button>("stores-equipment-tab");
                    var general = GeneralButton();
                    Assert.That(Root.ClassListContains("has-feedback"), Is.EqualTo(withFeedback));
                    Assert.That(general.parent.resolvedStyle.flexWrap, Is.EqualTo(Wrap.NoWrap));
                    AssertFixedGeneral(drawer, scroll, general);
                    foreach (var button in new[] { inventory, equipment, general })
                    {
                        AssertInside(drawer.worldBound, button.worldBound); AssertCompleteAccessText(button);
                        Assert.That(button.resolvedStyle.fontSize, Is.EqualTo(14));
                        Assert.That(button.worldBound.yMin, Is.GreaterThanOrEqualTo(heading.worldBound.yMax - .5f));
                        Assert.That(button.worldBound.center.y, Is.EqualTo(general.worldBound.center.y).Within(.5f), "All three controls must share one row.");
                    }
                    Assert.That(inventory.worldBound.xMax, Is.LessThanOrEqualTo(equipment.worldBound.xMin + .5f));
                    Assert.That(equipment.worldBound.xMax, Is.LessThanOrEqualTo(general.worldBound.xMin + .5f));
                    var close = heading.Query<Button>().ToList().Single(b => b.text == "Close");
                    var title = heading.Q<Label>(className: "title");
                    AssertInside(drawer.worldBound, close.worldBound); AssertInside(drawer.worldBound, title.worldBound);
                    Assert.That(title.worldBound.xMax, Is.LessThanOrEqualTo(close.worldBound.xMin + .5f));
                    AssertInside(Root.worldBound, drawer.worldBound);
                    Assert.That(drawer.worldBound.yMax, Is.LessThanOrEqualTo(Root.Q<VisualElement>(className: "bottom").worldBound.yMin + .5f));
                    Assert.That(scroll.worldBound.height, Is.GreaterThan(40));
                    Assert.That(scroll.contentViewport.worldBound.height, Is.GreaterThan(40));
                    float inventoryHeight = scroll.worldBound.height;
                    scroll.scrollOffset = new Vector2(0, scroll.verticalScroller.highValue);
                    yield return null; yield return null;
                    AssertFixedGeneral(drawer, scroll, general); unchanged();
                    Submit(equipment); yield return null; yield return null;
                    Assert.That(general.resolvedStyle.display, Is.EqualTo(DisplayStyle.None));
                    Assert.That(scroll.worldBound.height, Is.GreaterThanOrEqualTo(inventoryHeight - .5f));
                    float equipmentHeight = scroll.worldBound.height;
                    game.UI.OpenPanel("Cats"); yield return null; yield return null;
                    Assert.That(general.parent.resolvedStyle.display, Is.EqualTo(DisplayStyle.None));
                    Assert.That(scroll.worldBound.height, Is.GreaterThan(equipmentHeight));
                    game.UI.OpenPanel("Stores"); Submit(inventory); unchanged();
                }
            }
        }

        [UnityTest]
        public IEnumerator GeneralAndFilteredStockpileRealInputCancelsResetsAndReplaysExactPublicActions()
        {
            game.View.enabled = false; SetAccessSize(1000, 720);
            game.UI.OpenPanel("Stores"); string named = PrepareAccessChoices();
            yield return null; yield return null;
            var shrine = game.Selected.Buildings.Single(b => b.Kind == "shrine");
            var first = FindAccessRectangle();
            using (var input = new StockpileInput(game, Root))
            {
                var unchanged = AccessAuthorityUnchanged();
                Submit(GeneralButton()); AssertAccessGeneralCommand(first); RefreshAccessUi();
                Assert.That(AccessField("firstPoint"), Is.Null, "Button submit must not also choose a map corner.");
                AssertAccessHint("Stockpile: choose opposite corners inside the village"); unchanged();
                input.Click(shrine.Position); RefreshAccessUi(); AssertAccessHint("Choose opposite corner"); unchanged();
                input.Escape(); RefreshAccessUi(); Assert.That(game.UI.HasPlacement, Is.False);
                Assert.That(AccessField("firstPoint"), Is.Null); unchanged();
                Assert.That(Root.Q<Label>(className: "hint").text, Does.Not.Contain("Esc cancels"));
                yield return null;
                game.UI.OpenPanel("Stores"); AssertInventory(named);
                Submit(GeneralButton()); Assert.That(AccessField("firstPoint"), Is.Null);
                var upper = new Int2(first.X + 1, first.Z + 1);
                input.Click(upper); RefreshAccessUi(); AssertAccessHint("Choose opposite corner"); unchanged();
                // A delivered callback cannot reset the already chosen opposite corner.
                game.UI.OpenPanel("Stores"); AssertAccessCallbackIgnored(GeneralButton());
                game.UI.OpenPanel("Build"); Submit(AccessButton("Build stone road"));
                input.Click(first); RefreshAccessUi(); AssertAccessHint("Choose end of route"); unchanged();
                input.Escape(); RefreshAccessUi(); unchanged(); yield return null;
                game.UI.OpenPanel("Stores"); Submit(GeneralButton()); AssertAccessGeneralCommand(upper);
                input.Click(upper); RefreshAccessUi(); AssertAccessHint("Choose opposite corner"); unchanged();
                CompleteAccessRectangle(input, upper, first, false);
                game.UI.OpenPanel("Stores"); AssertInventory(named);
                var second = FindAccessRectangle(); var secondUpper = new Int2(second.X + 1, second.Z + 1);
                Submit(GeneralButton()); input.Click(second); RefreshAccessUi(); AssertAccessHint("Choose opposite corner");
                CompleteAccessRectangle(input, second, secondUpper, false);
                game.UI.OpenPanel("Stores"); AssertInventory(named);
                var filtered = FindAccessRectangle(); var filteredUpper = new Int2(filtered.X + 1, filtered.Z + 1);
                Submit(GeneralButton()); input.Click(shrine.Position);
                game.UI.OpenPanel("Stores"); Submit(AccessButton("Designate selected-resource pile"));
                Assert.That(AccessField("firstPoint"), Is.Null, "Replacing a draft with the existing filtered tool resets its first point.");
                RefreshAccessUi(); AssertAccessHint("Filtered stockpile: choose opposite corners inside the village");
                input.Click(filtered); RefreshAccessUi(); AssertAccessHint("Choose opposite corner");
                CompleteAccessRectangle(input, filtered, filteredUpper, true);
                game.UI.OpenPanel("Stores"); AssertInventory(named); Submit(GeneralButton());
                unchanged = AccessAuthorityUnchanged();
                input.Click(shrine.Position); RefreshAccessUi(); AssertAccessHint("Choose opposite corner"); unchanged();
                string refusalWorld = JsonUtility.ToJson(game.CurrentWorld); int refusalRevision = game.Revision;
                input.Click(shrine.Position);
                Assert.That(game.LastAction, Is.EqualTo("Invalid stockpile area"));
                Assert.That(game.UI.HasPlacement, Is.False); Assert.That(AccessField("firstPoint"), Is.Null);
                // Refusal changes feedback only. The complete authority and revision remain unchanged.
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(refusalWorld)); Assert.That(game.Revision, Is.EqualTo(refusalRevision));
                game.UI.OpenPanel("Stores"); AssertInventory(named);
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(refusalWorld)); Assert.That(game.Revision, Is.EqualTo(refusalRevision));
            }
        }

        private const System.Reflection.BindingFlags AccessFlags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
        private object AccessField(string name) => typeof(ForestUI).GetField(name, AccessFlags).GetValue(game.UI);
        private DropdownField AccessChoice(string label) => Root.Query<DropdownField>().ToList().Single(f => f.label == label);
        private Button AccessButton(string text) => Root.Query<Button>().ToList().Single(b => b.text == text);
        private void SetAccessSize(float width, float height)
        {
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = width; Root.style.height = Root.style.maxHeight = height;
        }
        private string PrepareAccessChoices()
        {
            var carrier = game.Selected.Cats.First(c => c.Alive);
            string named = carrier.Name + " · " + carrier.Id;
            AccessChoice("Cat").value = named; AccessChoice("Resource").value = "water";
            return named;
        }
        private void RefreshAccessUi()
        {
            typeof(ForestUI).GetField("nextRefresh", AccessFlags).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", AccessFlags).Invoke(game.UI, null);
        }
        private Action AccessAuthorityUnchanged()
        {
            string world = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction; int revision = game.Revision;
            return () =>
            {
                Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(world));
                Assert.That(game.Revision, Is.EqualTo(revision)); Assert.That(game.LastAction, Is.EqualTo(action));
                Assert.That(game.Speed, Is.Zero); Assert.That(game.CurrentWorld.TimeSeconds, Is.Zero);
            };
        }
        private void AssertAccessCallbackIgnored(Button general)
        {
            var unchanged = AccessAuthorityUnchanged();
            var fields = new[] { "placement", "firstPoint", "placementLabel", "placementSecondPointPrompt", "selectedCat", "selectedResource" };
            var values = fields.Select(AccessField).ToArray();
            var feedback = Root.Q<Label>(className: "feedback");
            var copy = (feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback"), game.UI.OpenSection);
            var invoke = typeof(Clickable).GetMethod("Invoke", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic);
            Assert.That(invoke, Is.Not.Null);
            int observed = 0; void Observer() => observed++;
            general.clicked += Observer;
            try { invoke.Invoke(general.clickable, new object[] { null }); }
            finally { general.clicked -= Observer; }
            Assert.That(observed, Is.EqualTo(1), "The real Clickable dispatcher delivered the callback, including while detached or hidden.");
            unchanged();
            for (int i = 0; i < fields.Length; i++) Assert.That(AccessField(fields[i]), Is.EqualTo(values[i]), fields[i]);
            Assert.That((feedback.text, feedback.style.display.value, feedback.ClassListContains("failure"), Root.ClassListContains("has-feedback"), game.UI.OpenSection), Is.EqualTo(copy));
        }
        private static void AssertCompleteAccessText(Button button)
        {
            var style = button.resolvedStyle;
            float width = button.worldBound.width - style.paddingLeft - style.paddingRight - style.borderLeftWidth - style.borderRightWidth;
            float height = button.worldBound.height - style.paddingTop - style.paddingBottom - style.borderTopWidth - style.borderBottomWidth;
            Assert.That(width, Is.GreaterThan(0)); Assert.That(height, Is.GreaterThan(0));
            var measured = button.MeasureTextSize(button.text, width, VisualElement.MeasureMode.Exactly, 0, VisualElement.MeasureMode.Undefined);
            Assert.That(measured.y, Is.LessThanOrEqualTo(height + .5f), button.text + " must fit in full without clipping.");
            foreach (string word in button.text.Split(' '))
                Assert.That(button.MeasureTextSize(word, 0, VisualElement.MeasureMode.Undefined, 0, VisualElement.MeasureMode.Undefined).x, Is.LessThanOrEqualTo(width + .5f));
            Assert.That(style.textOverflow, Is.Not.EqualTo(TextOverflow.Ellipsis));
            Assert.That(button.text, Does.Not.Contain("…"));
        }
        private void AssertAccessHint(string text)
        {
            Assert.That(game.UI.HasPlacement, Is.True);
            var hint = Root.Q<Label>(className: "hint");
            Assert.That(hint.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            Assert.That(hint.text, Is.EqualTo(text + " · Esc cancels"));
        }
        private void AssertAccessGeneralCommand(Int2 point)
        {
            var action = ((Func<Int2, GameAction>)AccessField("placement"))(point);
            Assert.That(action.Kind, Is.EqualTo("DesignateStockpile")); Assert.That(action.Name, Is.EqualTo("General"));
            Assert.That(action.Position, Is.EqualTo(point)); Assert.That(action.Accepts, Is.Empty);
            Assert.That(action.Resource, Is.Empty); Assert.That(action.CatId, Is.Empty);
        }
        private Int2 FindAccessRectangle()
        {
            var probe = JsonUtility.FromJson<World>(JsonUtility.ToJson(game.CurrentWorld));
            var context = new PlayerContext { PlayerId = game.PlayerId, VillageId = game.SelectedVillageId };
            foreach (var tile in probe.Tiles.Where(t => t.Position.Level == 0 && t.ClaimId == game.SelectedVillageId).OrderBy(t => Int2.Distance(t.Position, game.Selected.Center)))
            {
                var p = tile.Position;
                if (probe.Apply(context, new GameAction { Kind = "DesignateStockpile", Position = p, End = new Int2(p.X + 1, p.Z + 1), Name = "General" }).Success) return p;
            }
            Assert.Fail("The unchanged isolated fixture must contain a legal public 2x2 stockpile rectangle.");
            return default;
        }
        private void CompleteAccessRectangle(StockpileInput input, Int2 first, Int2 second, bool filtered)
        {
            var expected = IdleCatForest.Authority.WireJson.Clone(game.CurrentWorld);
            var context = new PlayerContext { PlayerId = game.PlayerId, VillageId = game.SelectedVillageId };
            var command = new GameAction { Kind = "DesignateStockpile", Position = first, End = second, Name = filtered ? "" : "General" };
            command.Path.Add(first); command.Path.Add(new Int2(second.X, first.Z)); command.Path.Add(second);
            if (filtered) command.Accepts.Add("water");
            var accepted = expected.Apply(context, command); Assert.That(accepted.Success, Is.True, accepted.Error);
            int revision = game.Revision, count = game.Selected.Stockpiles.Count;
            input.Click(second);
            Assert.That(game.LastAction, Does.Contain("accepted")); Assert.That(game.Revision, Is.EqualTo(revision + 1));
            Assert.That(game.Selected.Stockpiles.Count, Is.EqualTo(count + 1));
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(JsonUtility.ToJson(expected)), "The entire authority must equal exactly one replayed public action, including RNG, IDs, time, goods, claims and all other places.");
            var pile = game.Selected.Stockpiles.Single(p => p.Id == accepted.EntityId);
            Assert.That(pile.Position, Is.EqualTo(new Int2(Math.Min(first.X, second.X), Math.Min(first.Z, second.Z))));
            Assert.That(pile.Width, Is.EqualTo(2)); Assert.That(pile.Depth, Is.EqualTo(2)); Assert.That(pile.Capacity, Is.EqualTo(400));
            Assert.That(pile.Accepts, Is.EqualTo(filtered ? new[] { "water" } : Array.Empty<string>()));
            Assert.That(pile.Goods, Is.Empty); Assert.That(pile.Report, Is.Empty); Assert.That(pile.CountedAt, Is.EqualTo(-1)); Assert.That(pile.ManagedBy, Is.Empty);
            Assert.That(game.UI.HasPlacement, Is.False); Assert.That(AccessField("firstPoint"), Is.Null);
            Assert.That(game.CurrentWorld.Validate(), Is.Empty);
        }
        private sealed class StockpileInput : IDisposable
        {
            private readonly ForestGame game;
            private readonly UnityEngine.InputSystem.Mouse mouse;
            private readonly UnityEngine.InputSystem.Keyboard keyboard;
            private readonly VisualElement[] interactive;
            public StockpileInput(ForestGame game, VisualElement root)
            {
                this.game = game;
                // Match FarmInspectionInput: isolate world ray input from batch panel/screen mismatch.
                interactive = root.Query<VisualElement>(className: "interactive").ToList().ToArray();
                foreach (var element in interactive) element.RemoveFromClassList("interactive");
                mouse = UnityEngine.InputSystem.InputSystem.AddDevice<UnityEngine.InputSystem.Mouse>();
                keyboard = UnityEngine.InputSystem.InputSystem.AddDevice<UnityEngine.InputSystem.Keyboard>();
                mouse.MakeCurrent(); keyboard.MakeCurrent();
            }
            public void Click(Int2 point)
            {
                typeof(ForestView).GetMethod("UpdateCamera", AccessFlags).Invoke(game.View, null); Physics.SyncTransforms();
                var pixel = game.View.Camera.WorldToScreenPoint(new Vector3(point.X, (float)game.CurrentWorld.WalkHeight(point), point.Z));
                Assert.That(pixel.z, Is.GreaterThan(0)); Assert.That(pixel.x, Is.InRange(1f, Screen.width - 1f)); Assert.That(pixel.y, Is.InRange(1f, Screen.height - 1f));
                var state = new UnityEngine.InputSystem.LowLevel.MouseState { position = pixel };
                UnityEngine.InputSystem.LowLevel.InputState.Change(mouse, state, UnityEngine.InputSystem.LowLevel.InputUpdateType.Dynamic);
                Assert.That(game.UI.PointerOverPanel, Is.False, "World input uses explicit picking isolation.");
                UnityEngine.InputSystem.LowLevel.InputState.Change(mouse, state.WithButton(UnityEngine.InputSystem.LowLevel.MouseButton.Left), UnityEngine.InputSystem.LowLevel.InputUpdateType.Dynamic);
                Assert.That(mouse.leftButton.wasPressedThisFrame, Is.True);
                typeof(ForestView).GetMethod("ReadInput", AccessFlags).Invoke(game.View, null);
                Assert.That(game.View.CursorTile, Is.EqualTo(point));
                UnityEngine.InputSystem.LowLevel.InputState.Change(mouse, state, UnityEngine.InputSystem.LowLevel.InputUpdateType.Dynamic);
            }
            public void Escape()
            {
                UnityEngine.InputSystem.LowLevel.InputState.Change(keyboard, new UnityEngine.InputSystem.LowLevel.KeyboardState(UnityEngine.InputSystem.Key.Escape), UnityEngine.InputSystem.LowLevel.InputUpdateType.Dynamic);
                Assert.That(keyboard.escapeKey.wasPressedThisFrame, Is.True);
                typeof(ForestView).GetMethod("ReadInput", AccessFlags).Invoke(game.View, null);
                UnityEngine.InputSystem.LowLevel.InputState.Change(keyboard, new UnityEngine.InputSystem.LowLevel.KeyboardState(), UnityEngine.InputSystem.LowLevel.InputUpdateType.Dynamic);
            }
            public void Dispose()
            {
                UnityEngine.InputSystem.InputSystem.RemoveDevice(mouse); UnityEngine.InputSystem.InputSystem.RemoveDevice(keyboard);
                foreach (var element in interactive) element.AddToClassList("interactive");
            }
        }
        // End phase 2 composed additions.

        private Button GeneralButton()
        {
            var buttons = Root.Query<Button>().ToList().Where(b => b.text == "Designate general stockpile").ToArray();
            Assert.That(buttons.Length, Is.EqualTo(1), "Find the existing action by exact text, without requiring a new element name or duplicate action.");
            return buttons[0];
        }

        private void AssertInventory(string carrierChoice)
        {
            Assert.That(game.UI.OpenSection, Is.EqualTo("Stores"));
            Assert.That(Root.Q<Button>("stores-inventory-tab").ClassListContains("active"), Is.True);
            Assert.That(Root.Query<Label>(className: "stock-row").ToList().Count,
                Is.EqualTo(Catalog.Resources.Count(r => r != "blessings")), "Keep every current stock row in normal Inventory.");
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Is.EqualTo(carrierChoice));
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Resource").value, Is.EqualTo("water"));
        }

        private void AssertFixedGeneral(VisualElement drawer, ScrollView scroll, Button general)
        {
            Assert.That(general.panel, Is.SameAs(Root.panel));
            Assert.That(general.enabledInHierarchy, Is.True);
            Assert.That(drawer.Contains(general), Is.True);
            Assert.That(scroll.Contains(general), Is.False);
            for (VisualElement element = general; element != null; element = element.parent)
            {
                Assert.That(element.resolvedStyle.display, Is.Not.EqualTo(DisplayStyle.None));
                Assert.That(element.resolvedStyle.visibility, Is.EqualTo(Visibility.Visible));
            }
            Assert.That(general.worldBound.width, Is.GreaterThan(0));
            Assert.That(general.worldBound.height, Is.GreaterThan(0));
            AssertInside(Root.worldBound, general.worldBound);
            AssertInside(drawer.worldBound, general.worldBound);
            Assert.That(general.worldBound.yMax, Is.LessThanOrEqualTo(scroll.worldBound.yMin + .5f),
                "The complete action must fit above the drawer scroll at the observed native physical equivalent.");
        }

        private static void AssertInside(Rect outer, Rect inner)
        {
            Assert.That(inner.xMin, Is.GreaterThanOrEqualTo(outer.xMin - .5f));
            Assert.That(inner.xMax, Is.LessThanOrEqualTo(outer.xMax + .5f));
            Assert.That(inner.yMin, Is.GreaterThanOrEqualTo(outer.yMin - .5f));
            Assert.That(inner.yMax, Is.LessThanOrEqualTo(outer.yMax + .5f));
        }

        private static void Submit(Button button)
        {
            Assert.That(button, Is.Not.Null);
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
