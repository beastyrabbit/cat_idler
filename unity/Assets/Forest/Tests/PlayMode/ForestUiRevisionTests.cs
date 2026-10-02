using System;
using System.Collections;
using System.Collections.Generic;
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
    /// <summary>
    /// The Glass Clearing chrome: panel scale, mount slots, GUIDE §4 geometry at the reference (1920 × 1080),
    /// compact (1280 × 720) and narrow columns, the HUD's Escape order and keys on the input router, focus
    /// handling and the drawer frame.
    /// </summary>
    public class ForestUiRevisionTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => UiTestKit.Root(game);

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

        // ---- Scale and structure ----------------------------------------------------------------------

        [Test]
        public void PanelUsesTheBackingFactorTimesTheInterfaceSize()
        {
            var panel = game.GetComponent<UIDocument>().panelSettings;
            Assert.That(panel.scaleMode, Is.EqualTo(PanelScaleMode.ConstantPixelSize), "Text keeps its physical size; layouts reflow instead of scaling (GUIDE §4).");
            Assert.That(panel.scale, Is.EqualTo(ForestUI.PanelScaleFor(Screen.dpi, GameSettings.UiScale)), "One source of truth for the scale.");
            Assert.That(ForestUI.PanelScaleFor(220, 1), Is.EqualTo(2), "A Retina display draws one point as two pixels.");
            Assert.That(ForestUI.PanelScaleFor(96, 1), Is.EqualTo(1));
            Assert.That(ForestUI.PanelScaleFor(0, 1), Is.EqualTo(1), "An unknown dpi is a standard display.");
            Assert.That(ForestUI.PanelScaleFor(220, 1.25f), Is.EqualTo(2.5f), "The player's interface size multiplies the backing factor.");
            Assert.That(ForestUI.PanelScaleFor(96, 0), Is.EqualTo(1), "An unset interface size never collapses the HUD.");
        }

        [Test]
        public void GlassChromeReplacesTheOpaqueBarsAndMountsInSlots()
        {
            foreach (var slot in UiSlots.All)
            {
                var element = UiTestKit.Find<VisualElement>(game, slot);
                Assert.That(element.parent, Is.SameAs(Root), slot + " mounts directly on the HUD root.");
                Assert.That(element.pickingMode, Is.EqualTo(PickingMode.Ignore), slot + " must not swallow world clicks.");
                Assert.That(game.UI.Slot(slot), Is.SameAs(element));
            }
            Assert.That(UiTestKit.Find<VisualElement>(game, VillageChip.ChipName).parent, Is.SameAs(game.UI.Slot(UiSlots.Chip)));
            Assert.That(UiTestKit.Find<VisualElement>(game, ResourcePills.RowName).parent, Is.SameAs(game.UI.Slot(UiSlots.Pills)));
            Assert.That(UiTestKit.Find<VisualElement>(game, Dock.BarName), Is.Not.Null);
            Assert.That(UiTestKit.Find<VisualElement>(game, HintLine.LineName).parent, Is.SameAs(game.UI.Slot(UiSlots.Hint)));
            Assert.That(UiTestKit.Find<VisualElement>(game, UiNames.Drawer).parent, Is.SameAs(game.UI.Slot(UiSlots.Drawer)));
            foreach (var legacy in new[] { "hud-top", "hud-nav", "hud-bottom", "hud-brand" })
                Assert.That(UiTestKit.FindFirst<VisualElement>(game, legacy), Is.Null, legacy + " was one of the four opaque slabs.");
            foreach (var cls in new[] { "top", "navigation", "bottom", "view-controls" })
                Assert.That(Root.Query<VisualElement>(className: cls).ToList(), Is.Empty, "." + cls + " must be gone.");
            foreach (var resource in new[] { "food", "water", "logs", "stone", "research", "blessings" })
                Assert.That(UiTestKit.Find<Button>(game, "resource-" + resource), Is.Not.Null, resource);
            Assert.That(game.UI.WorldLeftFraction, Is.EqualTo(0), "Glass drawers overlay the map; the camera never shifts for them.");
        }

        [Test]
        public void DockListsEverySectionInFiveGroupsAndTheCardIsTheOnlyInspector()
        {
            Assert.That(game.UI.Registry.Find(DrawerHost.InspectId), Is.Null, "The tethered card is the only inspector.");
            Assert.That(ForestUI.Sections, Does.Not.Contain(DrawerHost.InspectId));
            var bar = UiTestKit.Find<VisualElement>(game, Dock.BarName);
            foreach (var entry in game.UI.Registry.Dock)
                Assert.That(UiTestKit.Button(game, UiNames.Nav(entry.Id)).parent, Is.SameAs(bar), entry.Id + " sits in the dock.");
            Assert.That(ForestUI.Sections.First(), Is.EqualTo("Overview"));
            Assert.That(bar.Query<VisualElement>(className: "dock-sep").ToList().Count(e => e.style.display != DisplayStyle.None), Is.EqualTo(4), "Five groups need four separators.");
            Assert.That(UiTestKit.WithPrefix<Button>(game, "nav-").Where(b => b.parent == bar).Select(b => b.name), Is.EqualTo(game.UI.Registry.Dock.Select(e => UiNames.Nav(e.Id))), "The dock follows the registry order.");

            UiTestKit.Click(game, UiNames.Nav("Cats"));
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"));
            Assert.That(UiTestKit.Button(game, UiNames.Nav("Cats")).ClassListContains("on"), Is.True, "The active section is lit in sun.");
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            game.UI.OpenPanel(DrawerHost.InspectId);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"), "Inspecting from a list keeps the list open beside the card.");
            Assert.That(game.UI.State.Selected, Is.EqualTo(new Subject(SubjectKind.Cat, cat.Id)));
            UiTestKit.Click(game, UiNames.Nav("Cats"));
            Assert.That(game.UI.OpenSection, Is.EqualTo(""), "Clicking the active icon closes its drawer.");
            Assert.That(UiTestKit.WithPrefix<Button>(game, "nav-").Any(b => b.ClassListContains("on")), Is.False);
        }

        // ---- GUIDE §4 geometry --------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator ReferenceColumnMatchesTheGuideAt1920By1080Points()
        {
            yield return Resize(1920, 1080);
            Assert.That(Root.ClassListContains("has-minimap") && !Root.ClassListContains("compact") && !Root.ClassListContains("narrow"), Is.True, "1920 points is the reference column.");
            var chip = Rect(VillageChip.ChipName);
            Assert.That(chip.xMin, Is.EqualTo(20).Within(.5f), "Chip left 20.");
            Assert.That(chip.yMin, Is.EqualTo(18).Within(.5f), "Chip top 18.");
            Assert.That(chip.yMax, Is.LessThanOrEqualTo(82.5f), "The drawer and popover start at 82, below the chip.");
            var pills = Rect(ResourcePills.RowName);
            Assert.That(pills.xMax, Is.EqualTo(1900).Within(.5f), "Pills right 20.");
            Assert.That(Rect("resource-food").yMin, Is.EqualTo(18).Within(.5f), "Pills top 18.");
            var dock = Rect(Dock.BarName);
            Assert.That(dock.center.x, Is.EqualTo(960).Within(1f), "The dock is centred.");
            Assert.That(dock.yMax, Is.EqualTo(1060).Within(.5f), "Dock bottom 20.");
            Assert.That(Rect(UiNames.Nav("Cats")).width, Is.EqualTo(52).Within(.5f), "52 point dock cells.");
            var hint = Rect(HintLine.LineName);
            Assert.That(hint.xMin, Is.EqualTo(20).Within(.5f));
            Assert.That(hint.yMax, Is.EqualTo(1056).Within(.5f), "Hint bottom 24.");
            AssertChromeClear(1920, 1080);

            game.UI.OpenPanel("Cats");
            yield return Frames();
            var sheet = Rect(UiNames.Drawer);
            Assert.That(sheet.xMin, Is.EqualTo(20).Within(.5f));
            Assert.That(sheet.yMin, Is.EqualTo(82).Within(.5f));
            Assert.That(sheet.yMax, Is.EqualTo(1080 - 96).Within(.5f));
            Assert.That(sheet.width, Is.EqualTo(660).Within(.5f));
            Assert.That(sheet.yMax, Is.LessThanOrEqualTo(dock.yMin), "The drawer ends above the dock.");
            AssertInside(sheet, Rect(DrawerHost.TitleName), "The drawer title fits its header.");
            AssertInside(sheet, Rect(UiNames.DrawerClose), "Esc stays inside the drawer.");
            Assert.That(UiTestKit.Find<VisualElement>(game, DrawerHost.DimName).resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "Only the wide drawer dims the map.");

            game.UI.OpenPanel("Research");
            yield return Frames();
            Assert.That(Rect(UiNames.Drawer).width, Is.EqualTo(1220).Within(.5f), "Research opens wide.");
            Assert.That(UiTestKit.Find<VisualElement>(game, DrawerHost.DimName).resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex), "The wide drawer dims the map 35 %.");
            Assert.That(game.UI.WorldLeftFraction, Is.EqualTo(0));
        }

        [UnityTest]
        public IEnumerator CompactColumnMatchesTheGuideAt1280By720Points()
        {
            yield return Resize(1280, 720);
            Assert.That(Root.ClassListContains("compact") && !Root.ClassListContains("narrow") && !Root.ClassListContains("has-minimap"), Is.True, "1280 points is the compact column.");
            var chip = Rect(VillageChip.ChipName);
            Assert.That(chip.xMin, Is.EqualTo(12).Within(.5f), "Chip left 12.");
            Assert.That(chip.yMin, Is.EqualTo(12).Within(.5f), "Chip top 12.");
            Assert.That(Rect(ResourcePills.RowName).xMax, Is.EqualTo(1268).Within(.5f), "Pills right 12.");
            var dock = Rect(Dock.BarName);
            Assert.That(dock.yMax, Is.EqualTo(708).Within(.5f), "Dock bottom 12.");
            Assert.That(Rect(UiNames.Nav("Cats")).width, Is.EqualTo(42).Within(.5f), "42 point dock cells.");
            AssertChromeClear(1280, 720);

            game.UI.OpenPanel("Work");
            yield return Frames();
            var sheet = Rect(UiNames.Drawer);
            Assert.That(sheet.width, Is.EqualTo(560).Within(.5f), "Drawer width 560.");
            Assert.That(sheet.yMin, Is.GreaterThanOrEqualTo(chip.yMax), "The drawer starts below the chip.");
            Assert.That(sheet.yMax, Is.LessThanOrEqualTo(dock.yMin), "The drawer ends above the dock.");
            game.UI.OpenPanel("Research");
            yield return Frames();
            sheet = Rect(UiNames.Drawer);
            Assert.That(sheet.xMin, Is.EqualTo(12).Within(.5f));
            Assert.That(sheet.xMax, Is.EqualTo(1268).Within(.5f), "Research: full width minus 24.");
        }

        [UnityTest]
        public IEnumerator NarrowWindowsReflowWithoutClippingOrScrollingTheDock()
        {
            foreach (var size in new[] { new Vector2(1000, 640), new Vector2(960, 540), new Vector2(640, 360) })
            {
                yield return Resize(size.x, size.y);
                Assert.That(Root.ClassListContains("narrow"), Is.True, size + " is below 1100 points.");
                Assert.That(UiTestKit.Button(game, "resource-research").resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "Research and blessings move to the Overview drawer.");
                Assert.That(UiTestKit.Find<Label>(game, VillageChip.CensusLabel).resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "The census line hides below 1100.");
                AssertChromeClear(size.x, size.y);
                game.UI.OpenPanel("Cats");
                yield return Frames();
                var sheet = Rect(UiNames.Drawer);
                Assert.That(sheet.xMin, Is.EqualTo(12).Within(.5f));
                Assert.That(sheet.xMax, Is.EqualTo(size.x - 12).Within(.5f), "Narrow drawers are full width minus 24.");
                Assert.That(sheet.yMin, Is.GreaterThanOrEqualTo(Rect(VillageChip.ChipName).yMax));
                Assert.That(sheet.yMax, Is.LessThanOrEqualTo(Rect(Dock.BarName).yMin), "The drawer stays above the dock.");
                Assert.That(UiTestKit.Find<ScrollView>(game, UiNames.DrawerScroll).contentViewport.worldBound.height, Is.GreaterThan(60), "A short window still shows drawer content at " + size);
                game.UI.ClosePanel();
            }
        }

        // ---- Input -------------------------------------------------------------------------------------

        [UnityTest]
        public IEnumerator EscapeClosesFieldPopoverPlacementDrawerThenSelectionInOrder()
        {
            var router = new InputRouter();
            game.UI.BindInput(router);
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            game.UI.OpenPanel("Research");
            yield return Frames();
            var search = UiTestKit.Find<TextField>(game, "field-search-studies");
            search.Focus();
            yield return null;
            Assert.That(game.UI.TextInputFocused, Is.True, "Setup: the search field has focus.");
            Assert.That(router.Dispatch(InputIntent.Escape), Is.True);
            Assert.That(game.UI.TextInputFocused, Is.False, "0. Esc first leaves a text field.");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Research"), "Leaving the field closes nothing else.");

            game.UI.OpenPanel("Stores");
            UiTestKit.Click(game, UiNames.Placement("stockpile"));
            game.UI.State.PopoverOpen = true;
            Assert.That(game.UI.HasPlacement, Is.True);
            Assert.That(game.UI.OpenSection, Is.EqualTo(""), "Placement is a mode: the drawer steps aside.");
            Assert.That(router.Dispatch(InputIntent.Escape), Is.True);
            Assert.That(game.UI.State.PopoverOpen, Is.False, "1. The popover closes first.");
            Assert.That(game.UI.HasPlacement, Is.True);

            Assert.That(router.Dispatch(InputIntent.Escape), Is.True);
            Assert.That(game.UI.HasPlacement, Is.False, "2. Then the placement mode ends.");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Stores"), "Leaving the mode brings back the drawer it hid.");

            Assert.That(router.Dispatch(InputIntent.Escape), Is.True);
            Assert.That(game.UI.OpenSection, Is.EqualTo(""), "3. Then the drawer closes.");
            UiTestKit.Tick(game);
            Assert.That(game.UI.State.Selected.Kind, Is.EqualTo(SubjectKind.Cat), "Closing the drawer keeps the selection.");

            Assert.That(router.Dispatch(InputIntent.Escape), Is.True);
            UiTestKit.Tick(game);
            Assert.That(game.UI.State.Selected.Kind, Is.EqualTo(SubjectKind.None), "4. Then the selection clears.");
            Assert.That(game.View.SelectedCatId, Is.EqualTo(""));

            Assert.That(router.Dispatch(InputIntent.Escape), Is.False, "With nothing open the HUD leaves Esc to the pause menu (Fallback).");
        }

        [Test]
        public void EscapePrioritiesFollowTheFrozenOrder()
        {
            var order = new List<string>();
            var router = new InputRouter();
            game.UI.BindInput(router);
            router.Bind(InputIntent.Escape, InputPriority.Modal, () => { order.Add("modal"); return false; });
            router.Bind(InputIntent.Escape, InputPriority.Control, () => { order.Add("control"); return false; });
            router.Bind(InputIntent.Escape, InputPriority.Fallback, () => { order.Add("fallback"); return false; });
            game.UI.State.PopoverOpen = true;
            router.Dispatch(InputIntent.Escape);
            Assert.That(order, Is.EqualTo(new[] { "modal" }), "A modal sees Esc before the popover; the popover then consumes it.");
            Assert.That(game.UI.State.PopoverOpen, Is.False);
            order.Clear();
            game.UI.OpenPanel("Cats");
            router.Dispatch(InputIntent.Escape);
            Assert.That(order, Is.EqualTo(new[] { "modal", "control" }), "Leaving direct control comes before closing a drawer.");
            Assert.That(game.UI.OpenSection, Is.EqualTo(""));
            order.Clear();
            router.Dispatch(InputIntent.Escape);
            Assert.That(order, Is.EqualTo(new[] { "modal", "control", "fallback" }), "With nothing to close, Esc reaches the pause menu.");
        }

        [Test]
        public void SpaceAndNumberKeysPauseAndPickThePlayerSpeeds()
        {
            var router = new InputRouter();
            game.UI.BindInput(router);
            Assert.That(router.Dispatch(InputIntent.Speed3), Is.True);
            Assert.That(game.Speed, Is.EqualTo(4), "3 is 4×.");
            Assert.That(router.Dispatch(InputIntent.TogglePause), Is.True);
            Assert.That(game.Speed, Is.EqualTo(0), "Space pauses.");
            UiTestKit.Refresh(game);
            Assert.That(UiTestKit.Find<Label>(game, VillageChip.PausedLabel).style.display.value, Is.EqualTo(DisplayStyle.Flex), "The chip says paused.");
            Assert.That(UiTestKit.Button(game, UiNames.Act("speed", "0")).ClassListContains("on"), Is.True, "The pause segment is lit.");
            Assert.That(router.Dispatch(InputIntent.TogglePause), Is.True);
            Assert.That(game.Speed, Is.EqualTo(4), "Space resumes at the last speed.");
            Assert.That(router.Dispatch(InputIntent.Speed2), Is.True);
            Assert.That(game.Speed, Is.EqualTo(2));
            Assert.That(router.Dispatch(InputIntent.Speed1), Is.True);
            Assert.That(game.Speed, Is.EqualTo(1));
            foreach (float speed in new float[] { 0, 1, 2, 4 })
                Assert.That(UiTestKit.Button(game, UiNames.Act("speed", speed.ToString(System.Globalization.CultureInfo.InvariantCulture))), Is.Not.Null);
            UiTestKit.Click(game, UiNames.Act("speed", "2"));
            Assert.That(game.Speed, Is.EqualTo(2));
            UiTestKit.Refresh(game);
            Assert.That(UiTestKit.WithPrefix<Button>(game, UiNames.Act("speed") + "-").Where(b => b.ClassListContains("on")).Select(b => b.name), Is.EqualTo(new[] { UiNames.Act("speed", "2") }));
        }

        [UnityTest]
        public IEnumerator TextFieldFocusReachesTheRouterAndButtonsNeverKeepFocus()
        {
            game.UI.OpenPanel("Research");
            yield return Frames();
            var search = UiTestKit.Find<TextField>(game, "field-search-studies");
            search.Focus();
            yield return null;
            Assert.That(game.UI.TextInputFocused, Is.True);
            Assert.That(InputRouter.Current.TextFieldFocused, Is.True, "Typing must not pan the camera or switch speeds.");
            search.Blur();
            yield return null;
            Assert.That(game.UI.TextInputFocused, Is.False);
            Assert.That(InputRouter.Current.TextFieldFocused, Is.False);

            var dockButton = UiTestKit.Button(game, UiNames.Nav("Cats"));
            dockButton.Focus();
            yield return null; yield return null;
            Assert.That(Root.focusController.focusedElement, Is.Not.SameAs(dockButton), "WASD and Enter must not walk or re-fire HUD buttons.");
            Assert.That(dockButton.focusable, Is.False);
        }

        [Test]
        public void EverythingButTheBareMapCountsAsPointerOverTheHud()
        {
            Assert.That(game.UI.IsHudElement(null), Is.False);
            Assert.That(game.UI.IsHudElement(Root), Is.False, "The root never picks.");
            Assert.That(game.UI.IsHudElement(Root.parent), Is.False, "The panel behind the root is the map.");
            Assert.That(game.UI.IsHudElement(UiTestKit.Button(game, UiNames.Nav("Cats"))), Is.True);
            Assert.That(game.UI.IsHudElement(UiTestKit.Find<Label>(game, VillageChip.NameLabel)), Is.True, "Any element inside the chip, not only buttons.");
            var popup = new VisualElement();
            Root.panel.visualTree.Add(popup);
            Assert.That(game.UI.IsHudElement(popup), Is.True, "Dropdown popups live outside the HUD root and still block world clicks.");
            popup.RemoveFromHierarchy();
        }

        // ---- Chrome content ----------------------------------------------------------------------------

        [Test]
        public void PillsShowEstimatesUntilCountedThenFreshness()
        {
            foreach (var pile in game.Selected.Stockpiles) pile.CountedAt = -1;
            UiTestKit.Refresh(game);
            var food = UiTestKit.Button(game, "resource-food");
            Assert.That(food.ClassListContains("estimate"), Is.True);
            Assert.That(food.Q<Label>(className: "pill-count").text, Does.StartWith("≈"), "An uncounted store shows an estimate, never a dash.");
            var storage = game.Selected.Stockpiles.First(p => p.Kind == "storage");
            storage.CountedAt = game.CurrentWorld.TimeSeconds;
            UiTestKit.Refresh(game);
            Assert.That(food.ClassListContains("estimate"), Is.False);
            Assert.That(food.Q<Label>(className: "pill-count").text, Is.EqualTo(ResourcePills.Whole(game.CurrentWorld.ReportedTotal(game.Selected, "food"))));
            Assert.That(food.Q(className: "pill-check").ClassListContains("gone"), Is.False, "A fresh count carries the check mark.");
            Assert.That(UiTestKit.Button(game, "resource-research").Q<Label>(className: "pill-unit").text, Does.StartWith("res"));
            UiTestKit.Click(game, "resource-research");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Research"), "A pill opens its drawer.");
        }

        [Test]
        public void ChipCountsStationWorkersAsAtWorkAndUsesNoLocaleDecimals()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            int before = game.Selected.Cats.Count(VillageChip.IsWorking);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Selected.Cats.Count(VillageChip.IsWorking), Is.EqualTo(before + 1), "Assigning a crafter must move the at-work count (first-session-6).");
            UiTestKit.Refresh(game);
            Assert.That(UiTestKit.Find<Label>(game, VillageChip.CensusLabel).text, Does.Contain((before + 1) + " at work"));
            Assert.That(UiTestKit.Find<Label>(game, VillageChip.NameLabel).text, Is.EqualTo(game.Selected.Name));
            foreach (var label in UiTestKit.Find<VisualElement>(game, ResourcePills.RowName).Query<Label>().ToList())
                Assert.That(label.text, Does.Not.Match(@"\d,\d"), "Numbers never follow the OS locale.");
        }

        [Test]
        public void DrawerHeaderSummarisesTheColonyInsteadOfFlavourText()
        {
            game.UI.OpenPanel("Cats");
            var summary = UiTestKit.Find<Label>(game, DrawerHost.SummaryName).text;
            int alive = game.Selected.Cats.Count(c => c.Alive);
            Assert.That(summary, Does.StartWith(alive + " cats"));
            Assert.That(summary, Does.Contain("at work").And.Contain("idle"));
            Assert.That(UiTestKit.Find<Label>(game, DrawerHost.TitleName).text, Is.EqualTo("Cats"));
            UiTestKit.Click(game, UiNames.DrawerClose);
            Assert.That(game.UI.OpenSection, Is.EqualTo(""));
        }

        [UnityTest]
        public IEnumerator LiveRefreshKeepsTheDrawerScrollPlace()
        {
            yield return Resize(1280, 720);
            game.UI.OpenPanel("Build");
            yield return Frames();
            var drawer = UiTestKit.Find<ScrollView>(game, UiNames.DrawerScroll);
            drawer.scrollOffset = new Vector2(0, 150);
            yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(25), "The setup must actually scroll the building list.");
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.1f), "The ordinary live refresh must preserve the player's place.");
            game.UI.OpenPanel("Stores");
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Another section starts at the top.");
        }

        [UnityTest]
        public IEnumerator ResearchPrerequisiteNavigationStartsAtTheRequestedStudy()
        {
            yield return Resize(1280, 720);
            game.UI.OpenPanel("Research");
            yield return Frames();
            var drawer = UiTestKit.Find<ScrollView>(game, UiNames.DrawerScroll);
            drawer.scrollOffset = new Vector2(0, 250);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must actually scroll the research catalog.");
            UiTestKit.Click(UiTestKit.FindFirst<Button>(game, UiNames.Act("requires", "research_hut")));
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Following a prerequisite must reveal the requested study at the top, not reuse the old catalog offset.");
            Assert.That(UiTestKit.Find<TextField>(game, "field-search-studies").value, Is.EqualTo("research_hut"));
            var firstCard = Root.Query<VisualElement>(className: "research-card").ToList().First();
            Assert.That(firstCard.name, Is.EqualTo("study-research_hut"));
        }

        // ---- Helpers -----------------------------------------------------------------------------------

        /// <summary>Lays the HUD out at a size in panel points, the way a window of that many points would.</summary>
        private IEnumerator Resize(float width, float height)
        {
            Root.style.flexGrow = 0;
            Root.style.width = Root.style.maxWidth = Root.style.minWidth = width;
            Root.style.height = Root.style.maxHeight = Root.style.minHeight = height;
            yield return Frames();
            Assert.That(Root.layout.width, Is.EqualTo(width).Within(.5f), "The regression must exercise " + width + " points.");
            UiTestKit.Refresh(game);
            yield return Frames();
        }

        private static IEnumerator Frames()
        {
            for (int i = 0; i < 3; i++) yield return null;
        }

        /// <summary>An element's bounds relative to the HUD root, in points.</summary>
        private Rect Rect(string name)
        {
            var bound = UiTestKit.Find<VisualElement>(game, name).worldBound;
            var origin = Root.worldBound.position;
            return new Rect(bound.position - origin, bound.size);
        }

        /// <summary>Chip, pills, dock and hint stay inside the window, never overlap and never clip their text.</summary>
        private void AssertChromeClear(float width, float height)
        {
            var screen = new Rect(0, 0, width, height);
            var chip = Rect(VillageChip.ChipName);
            var dock = Rect(Dock.BarName);
            var pills = UiTestKit.WithPrefix<Button>(game, "resource-").Where(p => p.resolvedStyle.display != DisplayStyle.None).Select(p => Rect(p.name)).ToList();
            AssertInside(screen, chip, "The chip fits the window at " + width + "×" + height);
            AssertInside(screen, dock, "The dock fits the window without scrolling at " + width + "×" + height);
            foreach (var pill in pills)
            {
                AssertInside(screen, pill, "Every pill fits the window.");
                Assert.That(pill.xMin, Is.GreaterThanOrEqualTo(chip.xMax), "Pills never run under the chip at " + width + "×" + height);
            }
            foreach (var cell in UiTestKit.WithPrefix<Button>(game, "nav-").Where(b => b.resolvedStyle.display != DisplayStyle.None))
                AssertInside(dock, Rect(cell.name), cell.name + " sits inside the dock.");
            foreach (var label in UiTestKit.Find<VisualElement>(game, VillageChip.ChipName).Query<Label>().ToList().Where(l => l.resolvedStyle.display != DisplayStyle.None && l.text.Length > 0))
                AssertInside(chip, Bounds(label), "Chip text '" + label.text + "' fits the chip.");
            foreach (var label in UiTestKit.Find<VisualElement>(game, ResourcePills.RowName).Query<Label>(className: "pill-count").ToList().Where(l => l.parent.resolvedStyle.display != DisplayStyle.None))
                AssertInside(Bounds(label.parent), Bounds(label), "Pill counts fit their pill.");
            var hintElement = UiTestKit.Find<VisualElement>(game, HintLine.LineName);
            if (hintElement.resolvedStyle.display != DisplayStyle.None)
            {
                var hint = Rect(HintLine.LineName);
                AssertInside(screen, hint, "The hint line fits the window.");
                Assert.That(hint.xMax <= dock.xMin + .5f || hint.yMax <= dock.yMin + .5f, Is.True, "The hint line never runs under the dock at " + width + "×" + height + ": " + hint + " vs " + dock);
            }
        }

        private Rect Bounds(VisualElement element)
        {
            var origin = Root.worldBound.position;
            return new Rect(element.worldBound.position - origin, element.worldBound.size);
        }

        private static void AssertInside(Rect outer, Rect inner, string reason)
        {
            Assert.That(inner.xMin, Is.GreaterThanOrEqualTo(outer.xMin - .5f), reason);
            Assert.That(inner.xMax, Is.LessThanOrEqualTo(outer.xMax + .5f), reason);
            Assert.That(inner.yMin, Is.GreaterThanOrEqualTo(outer.yMin - .5f), reason);
            Assert.That(inner.yMax, Is.LessThanOrEqualTo(outer.yMax + .5f), reason);
        }
    }
}
