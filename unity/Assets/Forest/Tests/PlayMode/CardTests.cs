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
using Stack = IdleCatForest.Simulation.Stack;

namespace IdleCatForest.Tests
{
    /// <summary>The tethered card is the only inspector, never leaves the screen and shows the right body.</summary>
    public class CardTests
    {
        private static readonly Vector2[] Sizes = { new Vector2(1920, 1080), new Vector2(1280, 720) };
        private ForestGame game;
        private string directory;
        private RenderTexture target;

        private TetheredCard Card => UiTestKit.Find<TetheredCard>(game, UiNames.Card("tethered"));

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-card-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated card test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            // A batch Editor's default screen is tiny; every test sees the card at the reference size.
            yield return Resize(Sizes[0]);
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (target != null) { target.Release(); UnityEngine.Object.Destroy(target); }
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        // ---------------------------------------------------------------- placement rule (GUIDE 5.5)

        [Test]
        public void PlacementPrefersRightAndAboveThenMirrorsAndDrops()
        {
            var usable = Rect.MinMaxRect(20, 82, 1900, 984);
            var size = new Vector2(340, 300);
            Assert.That(TetheredCard.PlaceNear(new Vector2(900, 600), size, usable, new Rect[0], out var at), Is.True);
            Assert.That(at, Is.EqualTo(new Vector2(900 + TetheredCard.Right, 600 - TetheredCard.Above)), "The preferred spot is 120 right of and 180 above the anchor.");
            Assert.That(TetheredCard.PlaceNear(new Vector2(1700, 600), size, usable, new Rect[0], out at), Is.True);
            Assert.That(at.x, Is.EqualTo(1700 - TetheredCard.Right - size.x), "Near the right edge the card mirrors to the left.");
            Assert.That(TetheredCard.PlaceNear(new Vector2(900, 150), size, usable, new Rect[0], out at), Is.True);
            Assert.That(at.y, Is.EqualTo(150 + TetheredCard.Below), "Near the top the card drops below its anchor.");
        }

        [Test]
        public void PlacementStaysInsideTheScreenAtTheFourCornersAtBothSizes()
        {
            foreach (var screen in Sizes)
            {
                bool compact = screen.x < TetheredCard.CompactBelow;
                float margin = compact ? 12 : 20;
                var usable = Rect.MinMaxRect(margin, margin, screen.x - margin, screen.y - margin - 76);
                var size = new Vector2(compact ? 300 : 340, 320);
                var drawer = Rect.MinMaxRect(20, 82, 20 + (compact ? 560 : 660), screen.y - 96);
                foreach (var blocked in new[] { new Rect[0], new[] { drawer } })
                    foreach (var anchor in Corners(screen, 2).Append(screen * .5f))
                    {
                        string where = screen + " anchor " + anchor + (blocked.Length > 0 ? " with a drawer" : "");
                        Assert.That(TetheredCard.PlaceNear(anchor, size, usable, blocked, out var at), Is.True, where);
                        var card = new Rect(at, size);
                        Assert.That(card.xMin, Is.GreaterThanOrEqualTo(usable.xMin - .01f), where);
                        Assert.That(card.yMin, Is.GreaterThanOrEqualTo(usable.yMin - .01f), where);
                        Assert.That(card.xMax, Is.LessThanOrEqualTo(usable.xMax + .01f), where);
                        Assert.That(card.yMax, Is.LessThanOrEqualTo(usable.yMax + .01f), where);
                        Assert.That(blocked.Any(b => b.Overlaps(card)), Is.False, "The card never overlaps an open drawer: " + where);
                        Assert.That(card.Contains(anchor), Is.False, "The card never covers its own subject: " + where);
                    }
            }
        }

        [UnityTest]
        public IEnumerator LiveCardStaysOnScreenAtTheFourCornersAtBothSizes()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            foreach (var screen in Sizes)
            {
                yield return Resize(screen);
                var root = UiTestKit.Root(game);
                Assert.That(root.layout.width, Is.EqualTo(screen.x).Within(1), "The panel must lay out at " + screen);
                foreach (bool drawer in new[] { false, true })
                {
                    if (drawer) game.UI.OpenPanel("Cats"); else game.UI.ClosePanel();
                    foreach (var corner in Corners(screen, 4))
                    {
                        Card.AnchorOverride = corner;
                        for (int i = 0; i < 3; i++) { UiTestKit.Tick(game); yield return null; }
                        string where = screen + " corner " + corner + (drawer ? " with the Cats drawer" : "");
                        var bounds = Card.worldBound;
                        if (!Card.Showing)
                        {
                            Assert.That(drawer, Is.True, "Only a drawer that leaves no room may hide the card: " + where);
                            continue;
                        }
                        Assert.That(Card.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex), where);
                        Assert.That(bounds.width, Is.GreaterThan(200), where);
                        Assert.That(bounds.xMin, Is.GreaterThanOrEqualTo(root.worldBound.xMin + 11.9f), "left: " + where);
                        Assert.That(bounds.yMin, Is.GreaterThanOrEqualTo(root.worldBound.yMin + 11.9f), "top: " + where);
                        Assert.That(bounds.xMax, Is.LessThanOrEqualTo(root.worldBound.xMax - 11.9f), "right: " + where);
                        Assert.That(bounds.yMax, Is.LessThanOrEqualTo(root.worldBound.yMax - 11.9f), "bottom: " + where);
                        if (drawer) Assert.That(bounds.Overlaps(UiTestKit.Find<VisualElement>(game, UiNames.Drawer).worldBound), Is.False, "Never over the drawer: " + where);
                        Assert.That(Card.Leader.Shown, Is.True, "An on-screen anchor gets a leader line: " + where);
                    }
                }
            }
            Card.AnchorOverride = null;
        }

        [UnityTest]
        public IEnumerator NarrowWindowShowsABottomSheetWithoutALeaderLine()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            yield return Resize(new Vector2(1000, 700));
            Card.AnchorOverride = new Vector2(500, 300);
            for (int i = 0; i < 3; i++) { UiTestKit.Tick(game); yield return null; }
            var root = UiTestKit.Root(game).worldBound;
            Assert.That(Card.IsSheet, Is.True);
            Assert.That(Card.Showing, Is.True);
            Assert.That(Card.worldBound.width, Is.EqualTo(root.width - 24).Within(1), "Full width minus 24.");
            Assert.That(Card.worldBound.yMax, Is.LessThanOrEqualTo(root.yMax - 11.9f));
            Assert.That(Card.worldBound.yMin, Is.GreaterThanOrEqualTo(root.yMin + 11.9f));
            Assert.That(Card.Leader.Shown, Is.False, "A bottom sheet has no leader line.");
            game.UI.OpenPanel("Cats");
            for (int i = 0; i < 2; i++) { UiTestKit.Tick(game); yield return null; }
            Assert.That(Card.Showing, Is.False, "Below 1100 the card hides while a drawer is open.");
            game.UI.ClosePanel();
            Card.AnchorOverride = null;
        }

        // ---------------------------------------------------------------- one inspector

        [UnityTest]
        public IEnumerator AWorldClickOpensOnlyTheCard()
        {
            yield return null; yield return null;
            var view = game.View;
            var cat = game.Selected.Cats.First(c => c.Alive && c.Position.Level == 0 && view.TryGetEntity("cat:" + c.Id, out _));
            view.TryGetEntity("cat:" + cat.Id, out var entity);
            Vector2 point = view.Camera.WorldToScreenPoint(entity.transform.position + Vector3.up * .25f);
            Assert.That(Click(point), Is.True, "A click on a cat is handled by selection.");
            Assert.That(view.SelectedSubject.Kind, Is.EqualTo(SubjectKind.Cat));
            UiTestKit.Tick(game);
            Assert.That(game.UI.OpenSection, Is.Not.EqualTo(DrawerHost.InspectId), "The card is the only inspector; no Inspect drawer.");
            AssertOnlyBody("cat");

            var building = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            view.TryGetEntity("building:" + building.Id, out var model);
            Assert.That(Click(view.Camera.WorldToScreenPoint(model.transform.position + Vector3.up * .3f)), Is.True, "A click on a building selects it.");
            Assert.That(view.SelectedSubject, Is.EqualTo(new Subject(SubjectKind.Building, building.Id)));
            UiTestKit.Tick(game);
            Assert.That(game.UI.OpenSection, Is.Not.EqualTo(DrawerHost.InspectId));
            AssertOnlyBody("building");

            game.UI.OpenPanel("Cats");
            UiTestKit.Tick(game);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"), "A drawer stays open beside the card.");
            game.UI.ClosePanel();

            var empty = new[] { new Vector2(4, 4), new Vector2(Screen.width - 4, 4), new Vector2(4, Screen.height - 4), new Vector2(Screen.width - 4, Screen.height - 4) }
                .First(p => view.SelectableAt(p).IsNone);
            Click(empty);
            UiTestKit.Tick(game);
            Assert.That(view.SelectedSubject.IsNone, Is.True, "Clicking empty ground deselects.");
            Assert.That(Card.Showing, Is.False);
        }

        [Test]
        public void SelectingNeverOpensTheRetiredInspectDrawer()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.UI.Select(new Subject(SubjectKind.Cat, cat.Id));
            UiTestKit.Tick(game);
            Assert.That(game.UI.OpenSection, Is.Not.EqualTo(DrawerHost.InspectId));
            Assert.That(UiTestKit.Root(game).Query<VisualElement>(UiNames.Section("Inspect")).ToList().All(e => e.childCount == 0), Is.True, "Inspect builds nothing.");
            AssertOnlyBody("cat");
        }

        [UnityTest]
        public IEnumerator CardHidesInDirectControlAndDuringPlacement()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            Assert.That(Card.Showing, Is.True);
            game.View.EnterSelectedCat();
            yield return null;
            UiTestKit.Tick(game);
            Assert.That(game.View.DirectControl, Is.True);
            Assert.That(Card.Showing, Is.False, "A reins bar replaces the card in direct control.");
            Assert.That(Card.Leader.Shown, Is.False);
            game.View.LeaveCat();
            yield return null;
            UiTestKit.Tick(game);
            Assert.That(Card.Showing, Is.True);
            game.UI.BeginPlacement("Den", p => new GameAction { Kind = "PlanBuilding", Resource = "den", Position = p });
            UiTestKit.Tick(game);
            Assert.That(Card.Showing, Is.False, "Placement hides the card.");
            game.UI.ClosePanel();
            UiTestKit.Tick(game);
            Assert.That(Card.Showing, Is.True);
        }

        // ---------------------------------------------------------------- bodies

        [Test]
        public void CatCardShowsColouredRingsAndAtMostThreeActions()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
            cat.Hunger = 58; cat.Thirst = 24; cat.Rest = 71; cat.Health = 92;
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            var body = UiTestKit.Find<VisualElement>(game, UiNames.Card("cat"));
            Assert.That(body.Q<NeedRing>("ring-hunger").Tone, Is.EqualTo("sun"));
            Assert.That(body.Q<NeedRing>("ring-thirst").Tone, Is.EqualTo("bad"));
            Assert.That(body.Q<NeedRing>("ring-rest").Tone, Is.EqualTo("ok"));
            Assert.That(body.Q<NeedRing>("ring-health").Value, Is.EqualTo(92));
            var actions = body.Q(className: "card-actions").Children().OfType<Button>().ToList();
            Assert.That(actions.Count(b => b.name != UiNames.Act("more", "card")), Is.InRange(1, 3), "Up to three actions plus More…");
            Assert.That(actions.Count(b => b.ClassListContains("card-primary")), Is.LessThanOrEqualTo(1), "One primary at most.");
            Assert.That(UiTestKit.Button(game, UiNames.Act("control", cat.Id)), Is.Not.Null);
            Assert.That(UiTestKit.Find<Label>(game, "card-name").text, Is.EqualTo(cat.Name));
            Assert.That(UiTestKit.Find<Label>(game, "card-facts").text, Does.Not.Contain("_"));

            cat.Thirst = 70;
            UiTestKit.Tick(game);
            Assert.That(body.Q<NeedRing>("ring-thirst").Tone, Is.EqualTo("ok"), "Rings follow the live value without a rebuild.");
            UiTestKit.Click(game, UiNames.Act("more", "card"));
            UiTestKit.Tick(game);
            var more = body.Q<VisualElement>("card-more");
            Assert.That(more.style.display.value, Is.Not.EqualTo(DisplayStyle.None), "More… grows the card in place.");
            Assert.That(UiTestKit.WithPrefix<Toggle>(game, UiNames.Act("labor") + "-"), Has.Count.EqualTo(Catalog.Labors.Length));
        }

        [Test]
        public void LiveWorkProgressUpdatesWithoutRebuildingTheCard()
        {
            var cat = game.Selected.Cats[1];
            var job = new Job { Id = "live-card-job", Kind = "woodcut", Phase = "working", CatId = cat.Id, Position = cat.Position, Progress = 1, RequiredWork = 20 };
            game.Selected.Jobs.Add(job); cat.JobId = job.Id;
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            var progress = UiTestKit.Find<ProgressBar>(game, UiNames.WorkProgress);
            job.Progress = 1.05;
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.Find<ProgressBar>(game, UiNames.WorkProgress), Is.SameAs(progress), "Live progress must not rebuild the card.");
            Assert.That(progress.value, Is.EqualTo(1.05f).Within(1e-5));
        }

        [Test]
        public void CarriedExactItemAppearsOnTheCardOnlyAfterPickup()
        {
            var village = game.Selected;
            var cat = village.Cats.First(c => c.Alive && c.Id != village.LeaderId);
            var job = new Job { Id = "card-item-haul", Kind = "haul", Phase = "item_fetch", CatId = cat.Id, SourceId = village.Stockpiles[0].Id };
            var item = new Item { Id = "card-mug", Kind = "mug", Material = "wood", LocationId = job.Id };
            job.ItemIds.Add(item.Id); village.Jobs.Add(job); village.Items.Add(item); cat.JobId = job.Id;
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.WithPrefix<Label>(game, "carried-item-"), Is.Empty, "A reserved item is still at its source.");
            job.Phase = "output_delivery";
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.WithPrefix<Label>(game, "carried-item-").Select(l => l.name), Is.EqualTo(new[] { "carried-item-" + item.Id }));
            item.LocationId = village.Stockpiles[0].Id; job.ItemIds.Clear(); job.Completed = true; cat.JobId = "";
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.WithPrefix<Label>(game, "carried-item-"), Is.Empty, "Delivered items leave the card.");
        }

        [UnityTest]
        public IEnumerator ControlButtonTakesTheSameCatAndTheCardReturnsAfterwards()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("control", cat.Id));
            yield return null;
            Assert.That(game.View.ControlledCat, Is.SameAs(cat));
            UiTestKit.Tick(game);
            Assert.That(Card.Showing, Is.False);
            game.View.LeaveCat();
            yield return null;
            UiTestKit.Tick(game);
            Assert.That(cat.ControlledBy, Is.Empty);
            Assert.That(Card.Showing, Is.True, "The card reopens on the same cat after direct control.");
            Assert.That(Card.Shown, Is.EqualTo(new Subject(SubjectKind.Cat, cat.Id)));
        }

        [Test]
        public void AssignToListsWorkplacesAndAssigns()
        {
            var v = game.Selected;
            var hut = v.Buildings.First(b => b.Kind == "research_hut" && b.Completed);
            foreach (var slot in hut.Slots.Where(s => s.CatId != "").ToList()) Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = slot.CatId }).Result.Success, Is.True);
            var cat = v.Cats.First(c => c.Alive && c.Id != v.LeaderId && c.JobId == "" && c.BuildingId == "");
            cat.Preferences.Add("research");
            game.View.InspectCat(cat.Id);
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("assign-to", cat.Id));
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("assign", cat.Id, hut.Id));
            Assert.That(cat.BuildingId, Is.EqualTo(hut.Id), game.LastAction);
        }

        [Test]
        public void WorkplaceAssignsTheBestSkilledIdleCatButNeverTheLeader()
        {
            var v = game.Selected;
            var hut = v.Buildings.First(b => b.Kind == "research_hut" && b.Completed);
            foreach (var slot in hut.Slots.Where(s => s.CatId != "").ToList()) Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = slot.CatId }).Result.Success, Is.True);
            var leader = v.Cats.First(c => c.Id == v.LeaderId);
            leader.Skills.Add(new Stack("research", 200));
            var idle = v.Cats.Where(c => c.Alive && c.Id != v.LeaderId && c.JobId == "" && c.BuildingId == "" && c.OfficerRole == "").Take(2).ToList();
            Assert.That(idle, Has.Count.EqualTo(2), "The founding colony has idle cats.");
            idle[1].Skills.Add(new Stack("research", 100));
            Assert.That(BuildingCard.BestIdleCat(game.CurrentWorld, v, hut), Is.SameAs(idle[1]));
            game.View.InspectBuilding(hut.Id);
            UiTestKit.Tick(game);
            AssertOnlyBody("building");
            UiTestKit.Click(game, UiNames.Act("assign-idle", hut.Id));
            Assert.That(hut.Slots.Any(s => s.CatId == idle[1].Id), Is.True, game.LastAction);
            Assert.That(hut.Slots.Any(s => s.CatId == leader.Id), Is.False);
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.Find<Label>(game, "card-crew-0").text, Is.EqualTo(idle[1].Name), "Crew slots name their cats.");
        }

        [Test]
        public void StationQueueEditsTheFirstWorkersQueue()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id);
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("edit-queue", station.Id));
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.WithPrefix<VisualElement>(game, "queue-entry-").Select(e => e.name), Is.EqualTo(new[] { "queue-entry-" + station.Id + "-" + cat.Id + "-0" }));
            bool repeat = station.Slots[0].Queue[0].Repeat;
            UiTestKit.Click(game, UiNames.Act("queue-repeat", station.Id, cat.Id, "0"));
            Assert.That(station.Slots[0].Queue[0].Repeat, Is.EqualTo(!repeat), game.LastAction);
        }

        [Test]
        public void ConstructionCardShowsMaterialsAndAsksBeforeCancelling()
        {
            var v = game.Selected;
            var site = new Building { Id = "card-test-site", Kind = "den", Completed = false, Position = new Int2(v.Center.X + 2, v.Center.Z + 2), RequiredWork = 60, Progress = 15 };
            site.Required.Add(new Stack("logs", 6)); site.Required.Add(new Stack("blocks", 2));
            site.Inputs.Add(new Stack("logs", 2));
            v.Buildings.Add(site);
            game.View.InspectBuilding(site.Id);
            UiTestKit.Tick(game);
            AssertOnlyBody("building");
            var logs = UiTestKit.Find<VisualElement>(game, "card-material-logs");
            Assert.That(logs.Query<Label>().ToList().Any(l => l.text == "2 / 6"), Is.True, "Delivered of required.");
            Assert.That(logs.ClassListContains("card-check-done"), Is.False);
            Assert.That(UiTestKit.Find<VisualElement>(game, "card-material-blocks"), Is.Not.Null);
            Assert.That(UiTestKit.Find<ProgressBar>(game, "live-build-progress").value, Is.EqualTo(15).Within(.01f));
            Assert.That(UiTestKit.FindFirst<Button>(game, UiNames.Act("card-cancel", "yes", site.Id)), Is.Null, "Cancel asks first.");
            UiTestKit.Click(game, UiNames.Act("card-cancel", site.Id));
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.Button(game, UiNames.Act("card-cancel", "yes", site.Id)), Is.Not.Null);
            UiTestKit.Click(game, UiNames.Act("card-cancel", "no", site.Id));
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.FindFirst<Button>(game, UiNames.Act("card-cancel", "yes", site.Id)), Is.Null);
            Assert.That(v.Buildings.Contains(site), Is.True, "No is harmless.");
        }

        [Test]
        public void PileCardOffersRemoveOnlyForFreePiles()
        {
            var v = game.Selected;
            var owner = v.Buildings.First(b => b.Completed);
            var store = new Stockpile { Id = "card-test-store", ManagedBy = owner.Id, Position = owner.Position, Width = 1, Depth = 1, Capacity = 16 };
            var free = new Stockpile { Id = "card-test-pile", Position = new Int2(v.Center.X - 3, v.Center.Z + 3), Capacity = 400, CountedAt = game.CurrentWorld.TimeSeconds };
            free.Report.Add(new Stack("logs", 12));
            v.Stockpiles.Add(store); v.Stockpiles.Add(free);
            game.View.InspectPile(store.Id);
            UiTestKit.Tick(game);
            AssertOnlyBody("pile");
            Assert.That(UiTestKit.WithPrefix<Button>(game, UiNames.Act("card-remove")), Is.Empty, "Station stores belong to their building.");
            game.View.InspectPile(free.Id);
            UiTestKit.Tick(game);
            Assert.That(UiTestKit.Find<Label>(game, "card-contents").text, Does.Contain("12"));
            UiTestKit.Click(game, UiNames.Act("card-remove", free.Id));
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("card-remove", "yes", free.Id));
            Assert.That(v.Stockpiles.Contains(free), Is.False, game.LastAction);
        }

        // ---------------------------------------------------------------- evidence

        /// <summary>
        /// With FOREST_CARD_SHOTS=&lt;dir&gt;, renders the world and the HUD at 1920x1080 panel units into one texture and
        /// saves a PNG per card variant (cat, More…, beside a drawer, workplace, queue editor, construction, housing,
        /// pile, and a cat at 1280x720). The capture driver cannot select piles, so this is the pile card's evidence.
        /// Without the variable the test does nothing.
        /// </summary>
        [UnityTest, Timeout(180000)]
        public IEnumerator EvidenceShotsWhenRequested()
        {
            string output = Environment.GetEnvironmentVariable("FOREST_CARD_SHOTS");
            if (string.IsNullOrEmpty(output) || SystemInfo.graphicsDeviceType == UnityEngine.Rendering.GraphicsDeviceType.Null) yield break;
            Directory.CreateDirectory(output);
            var v = game.Selected;
            var view = game.View;
            view.FocusVillage();
            game.SetSpeed(1);
            for (int i = 0; i < 90; i++) yield return null;

            var station = v.Buildings.First(b => b.Kind == "wood_cutter");
            var crafter = v.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != v.LeaderId);
            game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = crafter.Id });
            game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks", Repeat = true });
            game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" });
            for (int i = 0; i < 60; i++) yield return null;

            var cat = v.Cats.FirstOrDefault(c => c.Alive && c.JobId != "" && c.Id != crafter.Id);
            if (cat == null)
            {
                // Nobody has started work this early; give one cat a woodcutting trip so the card shows its progress.
                cat = v.Cats.First(c => c.Alive && c.Id != crafter.Id && c.Id != v.LeaderId && c.JobId == "");
                var job = new Job { Id = "evidence-woodcut", Kind = "logs", Phase = "working", CatId = cat.Id, Position = cat.Position, Progress = 21, RequiredWork = 60 };
                v.Jobs.Add(job); cat.JobId = job.Id; cat.Goal = "logs · working";
            }
            cat.Thirst = 31; cat.Hunger = 58;
            view.InspectCat(cat.Id);
            yield return Shot(output, "card-cat");
            UiTestKit.Click(game, UiNames.Act("more", "card"));
            yield return Shot(output, "card-cat-more");
            UiTestKit.Click(game, UiNames.Act("more", "card"));
            game.UI.OpenPanel("Cats");
            yield return Shot(output, "card-cat-beside-drawer");
            game.UI.ClosePanel();

            var hut = v.Buildings.First(b => b.Kind == "research_hut");
            view.InspectBuilding(hut.Id);
            yield return Shot(output, "card-workplace");
            view.InspectBuilding(station.Id);
            UiTestKit.Tick(game);
            UiTestKit.Click(game, UiNames.Act("edit-queue", station.Id));
            yield return Shot(output, "card-workplace-queue");

            Building site = null;
            foreach (var (x, z) in new[] { (-2, -12), (2, 11), (11, -2), (-12, 2), (-12, -12), (11, 11), (11, -12), (-12, 11), (5, 12), (-6, -13) })
            {
                var result = game.Send(new GameAction { Kind = "PlanBuilding", Resource = "den", Position = new Int2(v.Center.X + x, v.Center.Z + z) }).Result;
                if (result.Success) { site = v.Buildings.Find(b => b.Id == result.EntityId); break; }
            }
            if (site == null)
            {
                site = new Building { Id = "evidence-site", Kind = "den", Position = new Int2(v.Center.X + 2, v.Center.Z + 6), RequiredWork = 150 };
                site.Required.Add(new Stack("planks", 8)); site.Required.Add(new Stack("blocks", 4)); site.Inputs.Add(new Stack("planks", 3));
                v.Buildings.Add(site);
            }
            for (int i = 0; i < 120; i++) yield return null;
            view.InspectBuilding(site.Id);
            yield return Shot(output, "card-construction");

            view.InspectBuilding(v.Buildings.First(b => b.Kind == "den" && b.Completed).Id);
            yield return Shot(output, "card-housing");

            var pile = v.Stockpiles.FirstOrDefault(p => p.ManagedBy == "" && p.Kind == "storage");
            if (pile != null)
            {
                if (pile.CountedAt < 0) { pile.CountedAt = Math.Max(0, game.CurrentWorld.TimeSeconds - 20); pile.Report.Clear(); foreach (var s in pile.Goods.Take(5)) pile.Report.Add(new Stack(s.Resource, s.Amount)); }
                view.InspectPile(pile.Id);
                yield return Shot(output, "card-pile");
            }

            yield return Resize(Sizes[1]);
            view.InspectCat(cat.Id);
            yield return Shot(output, "card-cat-1280");
            game.SetSpeed(0);
        }

        /// <summary>Renders the game camera and then the HUD into the panel's texture and writes it as a PNG.</summary>
        private IEnumerator Shot(string output, string name)
        {
            var settings = game.GetComponent<UIDocument>().panelSettings;
            var camera = game.View.Camera;
            settings.clearColor = false;
            camera.targetTexture = target;
            // Batch-mode players never reach WaitForEndOfFrame; the texture keeps the last rendered frame.
            for (int i = 0; i < 20; i++) yield return null;
            var previous = RenderTexture.active;
            RenderTexture.active = target;
            var image = new Texture2D(target.width, target.height, TextureFormat.RGB24, false);
            image.ReadPixels(new Rect(0, 0, target.width, target.height), 0, 0);
            image.Apply();
            RenderTexture.active = previous;
            File.WriteAllBytes(Path.Combine(output, name + ".png"), image.EncodeToPNG());
            UnityEngine.Object.Destroy(image);
            camera.targetTexture = null;
        }

        // ---------------------------------------------------------------- helpers

        private void AssertOnlyBody(string kind)
        {
            Assert.That(Card.Showing, Is.True, "The card shows its subject.");
            foreach (var other in new[] { "cat", "building", "pile" })
            {
                var body = UiTestKit.Find<VisualElement>(game, UiNames.Card(other));
                Assert.That(body.ClassListContains("card-hidden"), Is.EqualTo(other != kind), other + " body while showing a " + kind);
            }
            Assert.That(UiTestKit.Root(game).Query<TetheredCard>().ToList(), Has.Count.EqualTo(1), "Never two inspectors.");
        }

        private bool Click(Vector2 screen)
        {
            var router = InputRouter.Current;
            Assert.That(router, Is.Not.Null);
            return router.DispatchPointer(PointerAction.PrimaryClick, new PointerInfo(screen, Vector2.zero, screen, false, false));
        }

        private IEnumerator Resize(Vector2 size)
        {
            var settings = game.GetComponent<UIDocument>().panelSettings;
            if (target != null) { settings.targetTexture = null; target.Release(); UnityEngine.Object.Destroy(target); }
            // Depth with stencil: UI Toolkit masks rounded, clipped elements with the stencil buffer.
            target = new RenderTexture((int)size.x, (int)size.y, 24);
            settings.scaleMode = PanelScaleMode.ConstantPixelSize;
            settings.scale = 1;
            settings.targetTexture = target;
            for (int i = 0; i < 3; i++) yield return null;
        }

        private static IEnumerable<Vector2> Corners(Vector2 screen, float inset) => new[]
        {
            new Vector2(inset, inset), new Vector2(screen.x - inset, inset), new Vector2(inset, screen.y - inset), new Vector2(screen.x - inset, screen.y - inset),
        };
    }
}
