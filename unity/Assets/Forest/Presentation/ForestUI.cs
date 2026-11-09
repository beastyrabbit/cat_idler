using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>Management controls issue the same commands as connected players.</summary>
    public sealed class ForestUI : MonoBehaviour
    {
        public static readonly string[] Sections = { "Village", "Cats", "Build", "Work", "Stores", "Research", "Officers", "Shrine", "World", "Defense", "Trade", "Routes", "Events" };
        public bool HasPlacement => placement != null;
        /// <summary>The occupied left edge in panel coordinates, for centering the camera in visible forest.</summary>
        public float WorldLeftFraction
        {
            get
            {
                if (root == null || navigation == null || root.worldBound.width <= 0) return 0;
                float right = navigation.worldBound.xMax;
                if (OpenSection != "" && drawer.resolvedStyle.display != DisplayStyle.None && !drawer.ClassListContains("wide")) right = Mathf.Max(right, drawer.worldBound.xMax);
                float fraction = (right - root.worldBound.xMin) / root.worldBound.width;
                return float.IsNaN(fraction) || float.IsInfinity(fraction) ? 0 : Mathf.Clamp(fraction, 0, .8f);
            }
        }
        public bool PointerOverPanel
        {
            get
            {
                if (root?.panel == null || Mouse.current == null) return false;
                var mouse = Mouse.current.position.ReadValue();
                var point = RuntimePanelUtils.ScreenToPanel(root.panel, new Vector2(mouse.x, Screen.height - mouse.y));
                var hit = root.panel.Pick(point);
                while (hit != null) { if (hit.ClassListContains("interactive") || hit.ClassListContains(GenericDropdownMenu.ussClassName)) return true; hit = hit.parent; }
                return false;
            }
        }
        public bool TextInputFocused
        {
            get
            {
                var focused = root?.focusController?.focusedElement as VisualElement;
                for (var p = focused; p != null; p = p.parent) if (p is TextField || p is DoubleField || p is IntegerField) return true;
                return false;
            }
        }
        public string OpenSection { get; private set; } = "";
        private ForestGame Game => GetComponent<ForestGame>();
        private Village Village => Game.Selected;
        private UIDocument document;
        private VisualElement root, drawer, content, navigation, topBar, bottomBar, storesTabs, workshopNavigationHost;
        private ScrollView scroll;
        private Label headline, villageSummary, clockLabel, status, hint, feedback, drawerTitle, drawerSubtitle;
        private Button attentionSeal;
        private VisualElement tetheredInspector;
        private Vector2 tetheredInspectorSize = new Vector2(292, 150);
        private Label tetheredInspectorTitle, tetheredInspectorBody;
        private Button backButton;
        private Button inventoryTab, equipmentTab, generalStockpile;
        private DropdownField workshopNavigation;
        private Label workshopNavigationContext;
        private string workshopNavigationVillage = "", workshopNavigationBuilding = "", workshopLastJump = "";
        private const string WorkshopNavigationPrompt = "Choose a section";
        private readonly List<(string Name, VisualElement Target)> workshopDestinations = new List<(string, VisualElement)>();
        private bool storesEquipment;
        private readonly Dictionary<string, Label> resourceLabels = new Dictionary<string, Label>();
        private readonly Dictionary<string, Button> navigationButtons = new Dictionary<string, Button>();
        private readonly Dictionary<float, Button> speedButtons = new Dictionary<float, Button>();
        private readonly List<Action> liveUpdates = new List<Action>();
        // Local tokens follow exact orders through rebuilds and reorders without retaining completed entries.
        private readonly ConditionalWeakTable<QueueEntry, string> queueControlIds = new ConditionalWeakTable<QueueEntry, string>();
        private string inspectOrigin = "Cats";
        private string workerReturnBuilding = "", workerReturnVillage = "", workerReturnCat = "", supplierReturnTarget = "";
        private Vector2 workerReturnScroll;
        private bool actionFailed;
        private string query = "", selectedResource = "food", selectedRecipe = "", selectedCat = "", selectedPile = "", selectedOther = "", selectedMode = "rail";
        private string selectedRouteDestination = "", routeDraftVillage = "";
        private double amount = 5, otherAmount = 5;
        private float nextRefresh;
        private Func<Int2, GameAction> placement;
        private string placementLabel = "", placementSecondPointPrompt = "";
        private Int2? firstPoint;
        private bool linePlacement;
        private string researchCategory = "All categories";
        private bool researchMap;
        private string renderedVillageId = "", renderedSubjectId = "", serverAddress = "ws://127.0.0.1:8788/ws";
        private string renderedDefenseTopology = "";
        private bool renderedRemote;
        private int panelVersion;
        private Action cancelFarmReveal;

        private void Awake()
        {
            document = gameObject.AddComponent<UIDocument>();
            var panel = Instantiate(Resources.Load<PanelSettings>("ForestPanel"));
            // Reflow a smaller window instead of shrinking every label below readable size.
            panel.scaleMode = PanelScaleMode.ConstantPhysicalSize;
            panel.referenceDpi = 110; panel.fallbackDpi = 96;
            document.panelSettings = panel;
            root = document.rootVisualElement; root.name = "forest-ui"; root.pickingMode = PickingMode.Ignore;
            root.styleSheets.Add(Resources.Load<StyleSheet>("ForestUI"));
            root.RegisterCallback<GeometryChangedEvent>(e =>
            {
                root.EnableInClassList("compact", e.newRect.width < 1040);
                root.EnableInClassList("narrow", e.newRect.width <= 650);
                root.EnableInClassList("micro", e.newRect.width <= 560);
                root.EnableInClassList("short", e.newRect.height < 620);
            });
            var top = topBar = Element(root, "top interactive");
            var brand = Element(top, "brand"); Label(brand, "IDLE CAT FOREST", "eyebrow"); headline = Label(brand, "Opening forest…", "headline");
            villageSummary = Label(brand, "A home among the trees", "village-summary");
            var resources = Element(top, "resources");
            foreach (var resource in new[] { "food", "water", "research", "blessings" })
            {
                var chip = Element(resources, "resource");
                resourceLabels[resource] = Label(chip, "—", "resource-value");
                Label(chip, char.ToUpperInvariant(resource[0]) + resource.Substring(1), "resource-name");
                if (resource == "food" || resource == "water") chip.tooltip = "Last physical inventory count. Open Stores to see how fresh each report is.";
            }
            var clock = Element(top, "clock");
            clockLabel = Label(clock, "Hour 0.0", "clock-label");
            var speeds = Element(clock, "row");
            foreach (float speed in new[] { 0f, 1f, 4f, 8f })
            {
                float selected = speed;
                speedButtons[speed] = Button(speeds, speed == 0 ? "Ⅱ" : speed + "×", () => Game.SetSpeed(selected), speed == 0 ? "Pause local world" : "Local simulation speed");
            }
            attentionSeal = Button(top, "✓", () => { if (OpenSection == "Events") ClosePanel(); else OpenPanel("Events"); }, "Open attention"); attentionSeal.AddToClassList("attention-seal");
            var nav = new ScrollView(ScrollViewMode.Vertical); nav.AddToClassList("navigation"); nav.AddToClassList("interactive"); root.Add(nav);
            navigation = nav;
            nav.horizontalScrollerVisibility = ScrollerVisibility.Hidden;
            foreach (var section in Sections)
            {
                string name = section;
                if (name == "Village") Label(nav, "COLONY", "nav-group");
                if (name == "Build") Label(nav, "PRODUCTION", "nav-group");
                if (name == "Research") Label(nav, "GROWTH", "nav-group");
                if (name == "Defense") Label(nav, "WORLD", "nav-group");
                var button = Button(nav, name, () => { if (OpenSection == name) ClosePanel(); else OpenPanel(name); }, SectionSummary(name));
                button.AddToClassList("navigation-button"); navigationButtons[name] = button;
            }
            drawer = Element(root, "drawer glass-panel interactive"); drawer.style.display = DisplayStyle.None;
            tetheredInspector = Element(root, "tethered-inspector glass-panel interactive"); tetheredInspector.style.display = DisplayStyle.None;
            tetheredInspector.RegisterCallback<GeometryChangedEvent>(e =>
            {
                if (e.newRect.width > 0 && e.newRect.height > 0) tetheredInspectorSize = e.newRect.size;
            });
            tetheredInspectorTitle = Label(tetheredInspector, "", "title");
            tetheredInspectorBody = Label(tetheredInspector, "", "body");
            Button(tetheredInspector, "Inspect", () => OpenPanel("Inspect"));
            var heading = Element(drawer, "drawer-heading row");
            var titleGroup = Element(heading, "drawer-title-group"); drawerTitle = Label(titleGroup, "", "title"); drawerSubtitle = Label(titleGroup, "", "drawer-subtitle");
            backButton = Button(heading, "Back", BackFromInspector); backButton.AddToClassList("quiet");
            Button(heading, "Close", ClosePanel).AddToClassList("quiet");
            storesTabs = Element(drawer, "row stores-tabs"); storesTabs.style.display = DisplayStyle.None;
            inventoryTab = Button(storesTabs, "Inventory", () => OpenStores(false)); inventoryTab.name = "stores-inventory-tab";
            equipmentTab = Button(storesTabs, "Equipment", () => OpenStores(true)); equipmentTab.name = "stores-equipment-tab";
            generalStockpile = Button(storesTabs, "Designate general stockpile", DesignateGeneralStockpile);
            generalStockpile.name = "stores-designate-general-stockpile";
            generalStockpile.AddToClassList("stores-general-stockpile");
            workshopNavigationHost = Element(drawer, "workshop-navigation"); workshopNavigationHost.style.display = DisplayStyle.None;
            workshopNavigationHost.style.flexShrink = 0;
            // Intercept this chooser before the native field opens an input-width menu.
            workshopNavigationHost.RegisterCallback<PointerDownEvent>(e =>
            {
                if (e.button != 0 || !WorkshopNavigationTarget(e.target as VisualElement)) return;
                e.StopImmediatePropagation(); ShowWorkshopNavigation();
            }, TrickleDown.TrickleDown);
            workshopNavigationHost.RegisterCallback<NavigationSubmitEvent>(e =>
            {
                if (!WorkshopNavigationTarget(e.target as VisualElement)) return;
                e.StopImmediatePropagation(); ShowWorkshopNavigation();
            }, TrickleDown.TrickleDown);
            scroll = new ScrollView(ScrollViewMode.Vertical); scroll.AddToClassList("drawer-scroll"); drawer.Add(scroll); content = scroll.contentContainer;
            var bottom = bottomBar = Element(root, "bottom interactive"); var bottomCopy = Element(bottom, "bottom-copy");
            hint = Label(bottomCopy, "WASD / right drag pan · wheel zoom · click to inspect", "hint");
            feedback = Label(bottomCopy, "", "feedback"); status = Label(bottomCopy, "", "status");
            var viewControls = Element(bottom, "view-controls");
            Button(viewControls, "−", () => Game.View.ZoomBy(-1), "Zoom out");
            Button(viewControls, "+", () => Game.View.ZoomBy(1), "Zoom in");
            Button(viewControls, "Return to village", () => Game.View.FocusVillage(), "Center the village · Home");
            Game.View.TileClicked += Place;
        }

        private void Update()
        {
            long phase = Game.StartFramePhase();
            try { UpdateFrame(); }
            finally { Game.EndFramePhase(FramePhase.UI, phase); }
        }

        private void UpdateFrame()
        {
            long live = Game.StartFramePhase();
            try
            {
            root.EnableInClassList("direct-control", Game.View != null && Game.View.DirectControl);
            if (Village != null) UpdateTetheredInspector();
            if (OpenSection != "" && Village?.Id == renderedVillageId)
                foreach (var update in liveUpdates) update();
            }
            finally { Game.EndFramePhase(FramePhase.Live, live); }
            if (Time.unscaledTime < nextRefresh) return; nextRefresh = Time.unscaledTime + .5f;
            if (Village == null) { headline.text = Game.Status; return; }
            if (OpenSection != "" && (renderedVillageId != Village.Id || renderedRemote != Game.IsRemote)) RenderPanel();
            int alive = Village.Cats.Count(c => c.Alive), assigned = Village.Cats.Count(c => c.Alive && (c.JobId != "" || c.ResumeJobId != "" || c.BuildingId != ""));
            headline.text = Village.Name;
            villageSummary.text = alive + " cats · " + assigned + " assigned · " + Village.Research.Count + " studies";
            resourceLabels["food"].text = Reported("food", true); resourceLabels["water"].text = Reported("water", true);
            resourceLabels["research"].text = Village.ResearchPoints.ToString("0"); resourceLabels["blessings"].text = Village.Blessings.ToString("0");
            clockLabel.text = "Hour " + (Game.CurrentWorld.TimeSeconds / 3600).ToString("0.0") + (Game.IsRemote ? " · shared" : Game.Speed == 0 ? " · paused" : "");
            foreach (var speed in speedButtons) { speed.Value.EnableInClassList("active", !Game.IsRemote && speed.Key == Game.Speed); speed.Value.SetEnabled(!Game.IsRemote); }
            status.text = Game.Status;
            feedback.text = HumanReason(Game.LastAction); feedback.style.display = Game.LastAction.Length == 0 ? DisplayStyle.None : DisplayStyle.Flex;
            feedback.EnableInClassList("failure", actionFailed);
            var attention = Village.Raids.Count + Village.Jobs.Count(j => !j.Completed && j.BlockedReason != "");
            attentionSeal.text = attention > 0 ? "⚠ " + Math.Min(99, attention) : "✓";
            attentionSeal.EnableInClassList("has-attention", attention > 0);
            root.EnableInClassList("has-feedback", Game.LastAction.Length > 0);
            hint.text = HasPlacement ? placementLabel + " · Esc cancels" : Game.View.DirectControl ? "Controlling " + Game.View.ControlledCat.Name + " · WASD walk · right drag look · E interact · Tab return" : "WASD / right drag pan · wheel / + − zoom · click to inspect · Tab controls cat";
            if ((OpenSection == "Inspect" || OpenSection == "Cats" || OpenSection == "Stores" || OpenSection == "Events") && !PanelControlFocused && Mouse.current?.leftButton.isPressed != true)
            { var offset = scroll.scrollOffset; RenderPanel(); scroll.scrollOffset = offset; }
            if (OpenSection == "Defense" && renderedDefenseTopology != DefenseTopology() && !PanelControlFocused && Mouse.current?.leftButton.isPressed != true)
            { var offset = scroll.scrollOffset; RenderPanel(); scroll.scrollOffset = offset; }
        }

        private void UpdateTetheredInspector()
        {
            if (root.panel == null || Game.View.Camera == null) return;
            var cat = Game.View.SelectedCat;
            var building = Game.View.SelectedBuilding;
            var position = cat?.Position ?? building?.Position;
            if (Game.View.DirectControl || !position.HasValue || position.Value.Level != Game.View.RenderedLevel || OpenSection == "Inspect") { tetheredInspector.style.display = DisplayStyle.None; return; }
            string title = cat != null ? cat.Name : Pretty(building.Kind);
            string catReason = cat == null ? "" : ReportedCatReason(cat, cat.BlockedReason);
            string body = cat != null
                ? (cat.Alive ? "Doing " : "") + ReportedCatStatus(cat) + (catReason == "" ? "" : "\nBecause " + HumanReason(catReason))
                : BuildingHoverStatus(building);
            if (tetheredInspectorTitle.text != title || tetheredInspectorBody.text != body)
            {
                tetheredInspectorTitle.text = title;
                tetheredInspectorBody.text = body;
                // Lay out changed content without showing it at the previous subject's size.
                tetheredInspector.style.visibility = Visibility.Hidden;
                tetheredInspector.style.display = DisplayStyle.Flex;
                return;
            }
            Vector3 world;
            if (cat != null)
                world = new Vector3((float)cat.X, (float)Game.CurrentWorld.WalkHeight(cat.Position) + .7f, (float)cat.Z);
            else
                world = new Vector3(building.Position.X + building.Width * .5f, (float)Game.CurrentWorld.WalkHeight(building.Position) + .7f, building.Position.Z + building.Depth * .5f);
            var screen = Game.View.Camera.WorldToScreenPoint(world);
            if (screen.z <= 0) { tetheredInspector.style.display = DisplayStyle.None; return; }
            var panelPoint = RuntimePanelUtils.ScreenToPanel(root.panel, new Vector2(screen.x, Screen.height - screen.y));
            var forest = root.worldBound;
            forest.xMin = navigation.worldBound.xMax + 10;
            if (OpenSection != "" && drawer.resolvedStyle.display != DisplayStyle.None)
                forest.xMin = Mathf.Max(forest.xMin, drawer.worldBound.xMax + 10);
            forest.xMax -= 14;
            forest.yMin = topBar.worldBound.yMax + 10;
            forest.yMax = bottomBar.worldBound.yMin - 10;
            if (forest.width < tetheredInspectorSize.x || forest.height < tetheredInspectorSize.y)
            { tetheredInspector.style.display = DisplayStyle.None; return; }
            var local = root.WorldToLocal(new Vector2(
                Mathf.Clamp(panelPoint.x + 18, forest.xMin, forest.xMax - tetheredInspectorSize.x),
                Mathf.Clamp(panelPoint.y - 78, forest.yMin, forest.yMax - tetheredInspectorSize.y)));
            tetheredInspector.style.left = local.x;
            tetheredInspector.style.top = local.y;
            tetheredInspector.style.display = DisplayStyle.Flex;
            tetheredInspector.style.visibility = Visibility.Visible;
        }

        private string BuildingHoverStatus(Building building)
        {
            if (building.Completed && IsStaffedWorkplace(building))
            {
                var positions = building.Slots.Where(slot => slot.CatId != "").Select(slot =>
                {
                    var worker = Village.Cats.Find(c => c.Id == slot.CatId);
                    string reason = WorkerReasonText(building, slot, worker, WorkerReason(building, slot, worker));
                    return (worker?.Name ?? slot.CatId) + " · " + WorkerStatus(building, slot, worker)
                        + (reason == "" ? "" : "\nBecause " + reason);
                }).ToArray();
                return positions.Length == 0 ? "Waiting for a worker" : string.Join("\n", positions);
            }
            return (building.Kind == "barracks" && building.Completed ? BarracksTrainingStatus(building) : building.WorkerId == "" ? "Waiting for a worker" : "Working")
                + (building.BlockedReason == "" ? "" : "\nBecause " + HumanReason(building.BlockedReason));
        }

        private string BarracksTrainingStatus(Building barracks)
        {
            int training = 0, travelling = 0, needs = 0, blocked = 0;
            foreach (var job in Village.Jobs.Where(j => !j.Completed && j.Kind == "train" && (j.TargetId == "" ? j.Position.Equals(barracks.Position) : j.TargetId == barracks.Id)))
            {
                var cat = Village.Cats.Find(c => c.Id == job.CatId && c.Alive && (c.JobId == job.Id || c.ResumeJobId == job.Id));
                if (cat == null) continue;
                if (cat.Goal.StartsWith("need_", StringComparison.Ordinal)) needs++;
                else if (job.BlockedReason != "" || cat.BlockedReason != "") blocked++;
                else if (cat.Position.Equals(job.Position) && Math.Abs(cat.X - job.Position.X) + Math.Abs(cat.Z - job.Position.Z) < 1e-9) training++;
                else travelling++;
            }
            int assigned = training + travelling + needs + blocked;
            if (barracks.Paused) return "Training paused · " + assigned + " assigned";
            if (assigned == 0) return "Training enabled · no trainees";
            var status = new List<string>();
            if (training > 0) status.Add(training + " training");
            if (travelling > 0) status.Add(travelling + " travelling");
            if (needs > 0) status.Add(needs + " attending to needs");
            if (blocked > 0) status.Add(blocked + " blocked");
            return "Training enabled\n" + string.Join(" · ", status);
        }

        private string Reported(string resource, bool compact = false)
        {
            bool any = Village.Stockpiles.Any(p => p.CountedAt >= 0);
            return any ? Game.CurrentWorld.ReportedTotal(Village, resource).ToString("0") + "*" : compact ? "—" : "uncounted";
        }
        public void OpenPanel(string section)
        {
            if (workerReturnBuilding != "" && (section != "Inspect" || Village?.Id != workerReturnVillage || !MatchesWorkshopReturn())) workerReturnBuilding = "";
            string subject = Game.View.SelectedCat?.Id ?? Game.View.SelectedBuilding?.Id ?? "";
            bool newPage = section != OpenSection || section == "Inspect" && subject != renderedSubjectId;
            if (section == "Inspect" && OpenSection != "Inspect") inspectOrigin = Sections.Contains(OpenSection) ? OpenSection : Game.View.SelectedCat != null ? "Cats" : "Work";
            OpenSection = section; drawer.style.display = DisplayStyle.Flex; drawer.EnableInClassList("wide", section == "Research");
            if (newPage) RenderPanelAtTop(); else RenderPanel();
        }
        public void ClosePanel() { cancelFarmReveal?.Invoke(); OpenSection = ""; workerReturnBuilding = ""; liveUpdates.Clear(); drawer.style.display = DisplayStyle.None; placement = null; firstPoint = null; placementSecondPointPrompt = ""; UpdateNavigation(); }
        internal void CancelFarmReveal() => cancelFarmReveal?.Invoke();
        public void InspectFarm(string farmId)
        {
            int level = Game.View.RenderedLevel;
            if (Game.View.DirectControl || HasPlacement || Village?.Farms.Any(f => f.Id == farmId && f.Position.Level == level) != true) return;
            string villageId = Village.Id;
            var focused = root.focusController?.focusedElement as VisualElement;
            if (focused != null && content.Contains(focused)) focused.Blur();
            OpenPanel("Work");
            // Deliberate navigation supersedes the rebuild's old scroll/focus restoration.
            int version = ++panelVersion;
            int attempts = 0;
            bool finished = false;
            IVisualElementScheduledItem scheduled = null;
            EventCallback<GeometryChangedEvent> layout = null;
            EventCallback<WheelEvent> wheel = null;
            EventCallback<PointerDownEvent> pointer = null;
            EventCallback<KeyDownEvent> key = null;
            void Finish()
            {
                if (finished) return;
                finished = true; scheduled?.Pause();
                content.UnregisterCallback(layout); scroll.contentViewport.UnregisterCallback(layout);
                scroll.UnregisterCallback(wheel, TrickleDown.TrickleDown); scroll.UnregisterCallback(pointer, TrickleDown.TrickleDown);
                scroll.UnregisterCallback(key, TrickleDown.TrickleDown);
                cancelFarmReveal = null;
            }
            bool Reveal()
            {
                if (version != panelVersion || OpenSection != "Work" || Village?.Id != villageId || Game.View.DirectControl || HasPlacement || Game.View.RenderedLevel != level
                    || !Village.Farms.Any(f => f.Id == farmId && f.Position.Level == level)) { Finish(); return false; }
                // Resolve the current card, never a captured farm object or list index.
                var card = content.Q<VisualElement>("farm-card-" + farmId);
                if (card?.panel != root.panel || !content.Contains(card)) { Finish(); return false; }
                var viewport = scroll.contentViewport.worldBound;
                if (!(card.layout.height > 0 && viewport.height > 0 && content.layout.height > 0)) return false;
                // A short previous page can still clamp the scroller during layout.
                float extent = Mathf.Max(0, content.layout.height - scroll.contentViewport.layout.height);
                float destination = Mathf.Clamp(scroll.scrollOffset.y + card.worldBound.yMin - viewport.yMin, 0, extent);
                scroll.scrollOffset = new Vector2(0, destination);
                var bounds = card.worldBound;
                return Mathf.Abs(scroll.verticalScroller.highValue - extent) < 1f
                    && Mathf.Abs(scroll.scrollOffset.y - destination) < .5f && bounds.yMin >= viewport.yMin - .5f
                    && (bounds.height > viewport.height ? bounds.yMin <= viewport.yMin + .5f : bounds.yMax <= viewport.yMax + .5f);
            }
            void AfterLayout()
            {
                if (finished) return;
                if (Reveal() || ++attempts >= 8) { Finish(); return; }
                if (!finished) scheduled = root.schedule.Execute(AfterLayout);
            }
            // Geometry handles the first measured layout; the bounded panel updates
            // confirm its scroll range. Nothing keeps pinning the card after completion.
            layout = _ => { if (!finished) Reveal(); };
            wheel = _ => Finish(); pointer = _ => Finish(); key = _ => Finish();
            content.RegisterCallback(layout); scroll.contentViewport.RegisterCallback(layout);
            scroll.RegisterCallback(wheel, TrickleDown.TrickleDown); scroll.RegisterCallback(pointer, TrickleDown.TrickleDown);
            scroll.RegisterCallback(key, TrickleDown.TrickleDown);
            cancelFarmReveal = Finish;
            scheduled = root.schedule.Execute(AfterLayout);
        }
        private void OpenStores(bool equipment)
        {
            storesEquipment = equipment;
            if (OpenSection == "Stores") RenderPanelAtTop();
            else OpenPanel("Stores");
        }
        private void DesignateGeneralStockpile()
        {
            if (OpenSection != "Stores" || storesEquipment || HasPlacement || Game.View.DirectControl ||
                Village == null || Village.Id != renderedVillageId || Game.IsRemote != renderedRemote ||
                root != document.rootVisualElement || root.panel == null || generalStockpile.panel != root.panel ||
                generalStockpile.parent != storesTabs || !root.Contains(generalStockpile) ||
                root.Q<Button>("stores-designate-general-stockpile") != generalStockpile) return;
            Placement("Stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Name = "General" }, true, "Choose opposite corner");
        }
        private void InspectWorker(Building building, string catId)
        {
            workerReturnBuilding = building.Id; workerReturnVillage = Village.Id; workerReturnCat = catId; supplierReturnTarget = ""; workerReturnScroll = scroll.scrollOffset;
            Game.View.InspectCat(catId); OpenPanel("Inspect");
        }
        private void InspectSupplier(Building origin, string supplierId)
        {
            workerReturnBuilding = origin.Id; workerReturnVillage = Village.Id; workerReturnCat = ""; supplierReturnTarget = supplierId; workerReturnScroll = scroll.scrollOffset;
            Game.View.InspectBuilding(supplierId); OpenPanel("Inspect");
        }
        private bool MatchesWorkshopReturn() => supplierReturnTarget != "" ? Game.View.SelectedBuilding?.Id == supplierReturnTarget : Game.View.SelectedCat?.Id == workerReturnCat;
        private void BackFromInspector()
        {
            if (workerReturnBuilding != "" && Village?.Id == workerReturnVillage && MatchesWorkshopReturn() && Village.Buildings.Any(b => b.Id == workerReturnBuilding))
            {
                string buildingId = workerReturnBuilding, villageId = workerReturnVillage;
                var offset = workerReturnScroll; workerReturnBuilding = "";
                Game.View.InspectBuilding(buildingId); OpenPanel("Inspect"); scroll.scrollOffset = offset;
                // The shorter subject page can clamp offsets until the workshop's new layout is measured.
                content.RegisterCallbackOnce<GeometryChangedEvent>(_ =>
                {
                    if (OpenSection == "Inspect" && Village?.Id == villageId && Game.View.SelectedBuilding?.Id == buildingId) scroll.scrollOffset = offset;
                });
                return;
            }
            workerReturnBuilding = ""; OpenPanel(inspectOrigin);
        }
        private void UpdateNavigation()
        {
            foreach (var button in navigationButtons) button.Value.EnableInClassList("active", button.Key == (OpenSection == "Inspect" ? inspectOrigin : OpenSection));
        }
        private void RenderPanelAtTop() { RenderPanel(false); scroll.scrollOffset = Vector2.zero; }
        private void RenderPanel(bool preserveInteraction = true)
        {
            cancelFarmReveal?.Invoke();
            long phase = Game.StartFramePhase();
            try
            {
                var focused = root.focusController?.focusedElement as VisualElement;
                string key = preserveInteraction && focused != null && content.Contains(focused) ? focused.name : "";
                float anchorY = focused?.worldBound.yMin ?? 0;
                var offset = scroll.scrollOffset;
                int version = ++panelVersion;
                RenderPanelContent();
                if (!preserveInteraction) return;
                scroll.scrollOffset = offset;
                var replacement = string.IsNullOrEmpty(key) ? null : content.Q<VisualElement>(key);
                replacement?.Focus();
                content.RegisterCallbackOnce<GeometryChangedEvent>(_ =>
                {
                    if (version != panelVersion) return;
                    // Newly revealed bills can grow above a focused action. Keep its
                    // screen position after layout, rather than retaining only an offset.
                    if (replacement?.panel != null)
                        scroll.scrollOffset += new Vector2(0, replacement.worldBound.yMin - anchorY);
                    else scroll.scrollOffset = offset;
                });
            }
            finally { Game.EndFramePhase(FramePhase.Rebuild, phase); }
        }
        private void RenderPanelContent()
        {
            if (Village == null) return; liveUpdates.Clear(); content.Clear(); drawerTitle.text = OpenSection;
            drawerSubtitle.text = SectionSummary(OpenSection); backButton.style.display = OpenSection == "Inspect" ? DisplayStyle.Flex : DisplayStyle.None; UpdateNavigation();
            storesTabs.style.display = OpenSection == "Stores" ? DisplayStyle.Flex : DisplayStyle.None;
            generalStockpile.style.display = OpenSection == "Stores" && !storesEquipment ? DisplayStyle.Flex : DisplayStyle.None;
            workshopNavigationHost.style.display = DisplayStyle.None; workshopDestinations.Clear();
            inventoryTab.EnableInClassList("active", !storesEquipment); equipmentTab.EnableInClassList("active", storesEquipment);
            renderedVillageId = Village.Id; renderedRemote = Game.IsRemote;
            renderedSubjectId = Game.View.SelectedCat?.Id ?? Game.View.SelectedBuilding?.Id ?? "";
            switch (OpenSection)
            {
                case "Village": VillagePanel(); break;
                case "Cats": CatsPanel(); break;
                case "Inspect": InspectPanel(); break;
                case "Build": BuildPanel(); break;
                case "Work": WorkPanel(); break;
                case "Stores": StoresPanel(); break;
                case "Research": ResearchPanel(); break;
                case "Officers": OfficersPanel(); break;
                case "Shrine": ShrinePanel(); break;
                case "Defense": DefensePanel(); break;
                case "World": WorldPanel(); break;
                case "Trade": TradePanel(); break;
                case "Routes": RoutesPanel(); break;
                case "Events": foreach (var e in Village.Events.AsEnumerable().Reverse().Take(100)) Label(content, (e.Time / 3600).ToString("0.0") + "h  " + e.Text, "body"); break;
            }
        }
        private void VillagePanel()
        {
            Text("The Commons is shared by everyone. Found a personal village to manage a home of your own.");
            foreach (var v in Game.CurrentWorld.Villages) { string id = v.Id; Button(content, v.Name + (v.Id == Village.Id ? " · selected" : ""), () => Act(new GameAction { Kind = "JoinVillage", TargetId = id }, true)); }
            var name = Field(content, "New village name", "Mosslight");
            Button(content, "Found personal village", () => Act(new GameAction { Kind = "FoundVillage", Name = name.value }, true));
            Separator(); Text("Shared server"); var server = Field(content, "Server address", serverAddress);
            server.RegisterValueChangedCallback(e => serverAddress = e.newValue);
            Button(content, "Connect", () => { _ = Game.Connect(serverAddress); });
            if (Game.IsRemote) Button(content, "Return to local world", Game.Disconnect);
            Button(content, "Save local world", Game.Save);
            Text(Game.IsRemote ? "Your shared village continues on the server when you leave." : "This world saves on your Mac as you play and when you quit.", "muted");
            Separator(); Label(content, "Governance", "subtitle");
            if (Village.Election != null)
            { Text("Election open until hour " + (Village.Election.EndAt / 3600).ToString("0.0")); foreach (var c in Village.Cats.Where(c => c.Alive)) { string id = c.Id; Button(content, "Vote for " + c.Name, () => Act(new GameAction { Kind = "CastVote", CatId = id })); } }
            else Text("Next election at hour " + (Village.NextElection / 3600).ToString("0.0"));
            if (Village.KickPetition != null) Text(Village.KickPetition.Votes.Count + " / 5 signatures for leader removal · closes at hour " + (Village.KickPetition.EndAt / 3600).ToString("0.0"));
            Button(content, "Sign leader removal petition", () => Act(new GameAction { Kind = "RequestVoteKick" }));
        }
        private void CatsPanel()
        {
            Text("Select a cat to inspect needs, work, preferences and carried goods.");
            foreach (var c in Village.Cats.Where(c => c.Alive))
            {
                var card = Element(content, "card"); var row = Element(card, "card-heading row"); string id = c.Id;
                Label(row, c.Name + (c.Id == Village.LeaderId ? " · Leader" : ""), "subtitle");
                Button(row, "Inspect " + c.Name, () => { Game.View.InspectCat(id); OpenPanel("Inspect"); });
                Label(card, ReportedCatStatus(c) + (c.OfficerRole != "" ? " · " + Pretty(c.OfficerRole) : ""), "body");
                Label(card, "Hunger " + c.Hunger.ToString("0") + "  Thirst " + c.Thirst.ToString("0") + "  Rest " + c.Rest.ToString("0"), "muted");
                string reason = ReportedCatReason(c, c.BlockedReason);
                if (reason != "") Label(card, HumanReason(reason), "warning");
            }
        }
        private void InspectPanel()
        {
            var c = Game.View.SelectedCat; var b = Game.View.SelectedBuilding;
            if (c != null)
            {
                drawerTitle.text = c.Name; drawerSubtitle.text = ReportedCatStatus(c) + (c.Alive && c.OfficerRole != "" ? " · " + Pretty(c.OfficerRole) : "");
                var needs = Element(content, "need-grid");
                Meter(needs, "Health", () => Game.View.SelectedCat?.Health ?? 0);
                Meter(needs, "Hunger", () => Game.View.SelectedCat?.Hunger ?? 0);
                Meter(needs, "Thirst", () => Game.View.SelectedCat?.Thirst ?? 0);
                Meter(needs, "Rest", () => Game.View.SelectedCat?.Rest ?? 0);
                Text("Age " + c.AgeHours.ToString("0.0") + "h · " + c.Migration + " · " + (c.BedId == "" ? "No permanent bed" : "Permanent bed assigned"));
                if (c.PregnantUntil >= 0) Text("Kitten expected at hour " + (c.PregnantUntil / 3600).ToString("0.0"));
                var job = Village.Jobs.Find(j => j.Id == c.JobId)
                    ?? Village.Jobs.Find(j => !j.Completed && j.Id == c.ResumeJobId && IsEquipmentOrder(j));
                string catReason = ReportedCatReason(c, c.BlockedReason);
                if (catReason != "" && !IsEquipmentOrder(job)) Text(HumanReason(catReason), "warning");
                if (job != null)
                {
                    Text("Job: " + Pretty(job.Kind) + " · " + Pretty(job.Phase));
                    if (job.RequiredWork > 0 && !IsEquipmentOrder(job))
                        WorkMeter(() => Village.Jobs.Find(j => j.Id == job.Id)?.Progress ?? 0, () => Village.Jobs.Find(j => j.Id == job.Id)?.RequiredWork ?? 1);
                    string jobReason = ReportedCatReason(c, job.BlockedReason);
                    if (jobReason != "" && jobReason != catReason && !IsEquipmentOrder(job)) Text(HumanReason(jobReason), "warning");
                    if (IsEquipmentOrder(job)) EquipmentOrderCard(content, job);
                }
                var carriedItems = job != null && !job.Completed && job.Phase != "item_fetch" ? Village.Items.Where(item => item.LocationId == job.Id).ToList() : new List<Item>();
                if (c.Cargo.Count > 0 || carriedItems.Count == 0) Text("Cargo: " + Goods(c.Cargo));
                foreach (var item in carriedItems) Text("Carried item: " + Pretty(item.Material) + " " + item.Kind + " · " + item.Id + " · " + item.Condition.ToString("0") + " / " + item.MaxCondition.ToString("0") + " condition");
                Label(content, "Equipment", "subtitle");
                foreach (var item in Village.Items.Where(i => i.LocationId == c.Id && c.Equipment.Contains(i.Id)))
                {
                    string id = item.Id;
                    Text(Pretty(item.Material) + " " + item.Kind + " · " + item.Condition.ToString("0") + " / " + item.MaxCondition.ToString("0") + " condition");
                    var returns = Button(content, "Return " + item.Kind + " to storage", () => Act(new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id }));
                    returns.SetEnabled(!Village.Jobs.Any(j => !j.Completed && IsEquipmentOrder(j) && j.CatId == c.Id));
                }
                if (c.Alive) Button(content, "Choose equipment for this cat", () => { selectedCat = c.Id; OpenStores(true); });
                if (c.ControlledBy == Game.PlayerId && c.ControlledBy != "")
                {
                    Button(content, "Return to management", Game.View.LeaveCat).AddToClassList("primary");
                    if (c.Position.Level < 0) Button(content, "Collect reached dungeon chest · E", () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id }));
                    ResourceChoice(content); Number(content, "Amount", amount, x => amount = x);
                    foreach (var p in Village.Stockpiles.Where(p => Game.CurrentWorld.CanInteractWithPile(Village, c, p))) { string id = p.Id; Button(content, (c.Cargo.Count > 0 ? "Deposit at " : "Take from ") + id, () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = id, Resource = selectedResource, Amount = amount })); }
                    Button(content, "Interact with shrine", () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id }));
                    Button(content, "Eat carried food at shrine", () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "eat" }));
                    Button(content, "Drink carried water at shrine", () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "drink" }));
                    Button(content, "Rest at assigned Den", () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "rest" }));
                }
                else if (c.Alive) Button(content, "Control this cat · Tab", Game.View.EnterSelectedCat).AddToClassList("primary");
                Separator(); Label(content, "Work preferences", "subtitle");
                var preferences = Element(content, "labor-grid");
                foreach (var labor in Catalog.Labors) { string id = labor; var toggle = new Toggle(Pretty(labor)) { value = c.Preferences.Contains(labor) }; toggle.RegisterValueChangedCallback(e => Act(new GameAction { Kind = "SetCatLaborPreference", CatId = c.Id, Labor = id, Enabled = e.newValue })); preferences.Add(toggle); }
                Text("Skills: " + Goods(c.Skills));
                Button(content, "Release assigned workplace", () => Act(new GameAction { Kind = "AssignWorker", CatId = c.Id }));
                Button(content, "Boost cat", () => Act(new GameAction { Kind = "BoostCat", CatId = c.Id, Enabled = true }));
            }
            else if (b != null) BuildingPanel(b);
            else Text("Click a cat or a building in the forest, or choose a cat from the census.");
        }
        private void BuildingPanel(Building b)
        {
            drawerTitle.text = Pretty(b.Kind); drawerSubtitle.text = b.Width + " × " + b.Depth + " tiles · " + b.Position;
            Text(b.Completed ? IsStaffedWorkplace(b) ? "Completed workplace" : "Completed building" : "Under construction · " + b.Progress.ToString("0") + " / " + b.RequiredWork.ToString("0"));
            if (!b.Completed)
                WorkMeter(() => Game.View.SelectedBuilding?.Progress ?? 0, () => Game.View.SelectedBuilding?.RequiredWork ?? 1);
            Text("Inputs: " + Goods(b.Inputs) + "\nOutputs waiting: " + Goods(b.Outputs));
            if (b.BlockedReason != "" && (!b.Completed || b.Slots.Count == 0)) Text(HumanReason(b.BlockedReason), "warning");
            if (b.Completed && !IsStaffedWorkplace(b))
            {
                if (b.Kind == "field") Text("Designate and staff farm plots in Work. This building provides farm support.");
                else if (b.Kind == "barracks")
                {
                    Text("Start training from Work or Defense. Trainees walk here without a permanent workplace assignment.");
                    bool paused = b.Paused;
                    Text(paused ? "Training paused · assigned trainees keep their orders." : "Training enabled.", "muted");
                    Button(content, paused ? "Resume training" : "Pause training", () => Act(new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "pause", Enabled = !paused }));
                }
                else Text("This building does not need a permanent worker assignment.");
                foreach (var slot in b.Slots.Where(s => s.CatId != ""))
                {
                    string id = slot.CatId;
                    var cat = Village.Cats.Find(c => c.Id == id);
                    Label(content, (cat?.Name ?? id) + " · assigned here", "muted");
                    Button(content, "Release worker · " + (cat?.Name ?? id), () => Act(new GameAction { Kind = "AssignWorker", CatId = id }));
                }
                return;
            }
            int capacity = WorkCapacity(b);
            if (b.Completed) Label(content, OccupiedPositions(b) + " / " + capacity + " work positions staffed", "staff-summary");
            Label(content, "Staff this workplace", "subtitle"); CatChoice(content);
            var assign = Button(content, "Assign worker", () => Act(new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = selectedCat }));
            assign.AddToClassList("primary"); assign.SetEnabled(!b.Completed || OccupiedPositions(b) < capacity);
            if (b.Completed && OccupiedPositions(b) >= capacity) Text("All positions are staffed. Release a worker below before assigning another cat.", "muted");
            bool recipes = Catalog.Recipes.Any(r => r.Building == b.Kind);
            if (b.Completed) workshopDestinations.Add((recipes ? "Staffing & recipes" : "Staffing", content));
            if (recipes)
            {
                Separator(); Label(content, "Production", "subtitle");
                var allRecipes = Catalog.Recipes.Where(r => r.Building == b.Kind).ToList();
                var options = allRecipes.Where(r => Catalog.RecipeAvailable(Village, r)).ToList();
                if (!options.Any(r => r.Id == selectedRecipe)) selectedRecipe = options.FirstOrDefault()?.Id ?? "";
                if (options.Count > 0)
                {
                    var recipeChoice = Choice(content, "Recipe", options.Select(r => r.Id), selectedRecipe, x => selectedRecipe = x);
                    var ingredients = Label(content, "", "muted"); ingredients.name = "recipe-ingredients";
                    var preview = Element(content, "recipe-preview"); preview.name = "recipe-preview";
                    void DescribeRecipe()
                    {
                        var recipe = options.Find(r => r.Id == selectedRecipe);
                        if (recipe == null) return;
                        ingredients.text = Goods(recipe.Inputs.Select(s => new Stack(s.Resource, Game.CurrentWorld.RecipeInput(Village, recipe, s.Amount))), "0.##") + " → "
                            + (recipe.ItemKind == "" ? Goods(recipe.Outputs.Select(s => new Stack(s.Resource, Game.CurrentWorld.RecipeOutput(Village, recipe, s.Amount))), "0.##") : "1 " + Pretty(recipe.Material) + " " + recipe.ItemKind);
                        preview.Clear();
                        if (OccupiedPositions(b) == 0) RecipeSupplyPanel(preview, b, selectedRecipe);
                    }
                    recipeChoice.RegisterValueChangedCallback(_ => DescribeRecipe()); DescribeRecipe();
                }
                else Text("No recipes are unlocked for this workplace.", "muted");
                if (options.Count < allRecipes.Count)
                {
                    Text("More recipes require research.", "muted");
                    Button(content, "Study recipe unlocks", () => { query = ""; researchCategory = "recipe_resource"; researchMap = false; OpenPanel("Research"); });
                }
                Text("Each worker follows one queue. Repeating recipes move to the back after each batch.", "muted");
                var stationAdd = Button(content, "Add recipe to station queue", () => Act(new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = selectedRecipe }));
                stationAdd.name = "station-add-" + b.Id; stationAdd.SetEnabled(options.Count > 0);
            }
            else if (b.Kind == "research_hut" || b.Kind == "school") Text("Research needs no recipe queue. Staffed cats walk here to study.", "muted");
            if (!b.Completed) return;
            for (int index = 0; index < Math.Max(capacity, b.Slots.Count); index++)
            {
                var slot = index < b.Slots.Count ? b.Slots[index] : null;
                var position = Element(content, "work-position");
                var cat = Village.Cats.Find(c => c.Id == slot?.CatId);
                Label(position, "Work position " + (index + 1) + " · " + (slot == null || slot.CatId == "" ? "Vacant" : cat?.Name ?? slot.CatId), "subtitle");
                if (slot == null || slot.CatId == "")
                {
                    position.AddToClassList("vacant");
                    Label(position, "Assign a cat above to fill this position.", "muted");
                    if (recipes && index == 0)
                    {
                        var queue = QueuePanel(position, b, slot?.Queue ?? b.Queue, "", slot?.Paused == true || b.Paused);
                        workshopDestinations.Add(("Position 1 · Vacant · Queue", queue));
                    }
                    else if (slot != null) foreach (var entry in slot.Queue) Label(position, Pretty(entry.RecipeId), "muted");
                    continue;
                }
                string id = slot.CatId;
                string destination = "Position " + (index + 1) + " · " + (cat?.Name ?? id) + " · ";
                var ownership = Label(position, slot.AutomatedBy == "" ? "Manual assignment" : "Staffed by " + Pretty(slot.AutomatedBy), "muted");
                workshopDestinations.Add((destination + "Work & controls", ownership));
                Label(position, WorkerStatus(b, slot, cat), "worker-status");
                string reason = WorkerReasonText(b, slot, cat, WorkerReason(b, slot, cat));
                if (reason != "") Label(position, reason, "warning");
                var job = WorkerJob(cat);
                if (job != null && job.RequiredWork > 0 && job.Phase == "working")
                    WorkMeter(() => Village.Jobs.Find(j => j.Id == job.Id)?.Progress ?? 0, () => Village.Jobs.Find(j => j.Id == job.Id)?.RequiredWork ?? 1, position);
                var ownershipActions = Element(position, "row");
                var controls = Element(position, "row");
                Button(controls, "Inspect worker · " + (cat?.Name ?? id), () => InspectWorker(b, id));
                Button(controls, "Release worker · " + (cat?.Name ?? id), () => Act(new GameAction { Kind = "AssignWorker", CatId = id }));
                bool paused = slot.Paused || b.Paused;
                DelegationControls(position, ownershipActions, b, slot, cat, SupportsWorkerPause(b) && paused);
                var supply = RecipeSupplyPanel(position, b, !string.IsNullOrEmpty(job?.RecipeId) ? job.RecipeId : slot.Queue.FirstOrDefault()?.RecipeId);
                if (supply != null) workshopDestinations.Add((destination + "Inputs", supply));
                if (recipes)
                {
                    var workerAdd = Button(position, "Add selected recipe to this cat's queue", () => Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = id, Edit = "add", RecipeId = selectedRecipe }));
                    workerAdd.name = "worker-add-" + b.Id + "-" + id; workerAdd.SetEnabled(Catalog.RecipeAvailable(Village, Catalog.Recipe(selectedRecipe)));
                    var queue = QueuePanel(position, b, slot.Queue, id, paused);
                    workshopDestinations.Add((destination + "Queue", queue));
                }
                else if (SupportsWorkerPause(b)) Button(controls, paused ? "Resume work" : "Pause work", () => Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = id, Edit = "pause", Enabled = !paused }));
            }
            ConfigureWorkshopNavigation(b);
        }
        private void ConfigureWorkshopNavigation(Building building)
        {
            string villageId = Village.Id, buildingId = building.Id;
            if (workshopNavigation == null || workshopNavigationVillage != villageId || workshopNavigationBuilding != buildingId)
            {
                workshopNavigationHost.Clear();
                workshopLastJump = "";
                var choice = new DropdownField("Go to") { name = "workshop-navigation" };
                choice.style.flexShrink = 0;
                choice.labelElement.style.minWidth = 46; choice.labelElement.style.maxWidth = 46;
                choice.RegisterValueChangedCallback(e =>
                {
                    choice.SetValueWithoutNotify(WorkshopNavigationPrompt);
                    if (choice != workshopNavigation || OpenSection != "Inspect" || Village?.Id != villageId || Game.View.SelectedBuilding?.Id != buildingId) return;
                    var target = workshopDestinations.Find(d => d.Name == e.newValue).Target;
                    if (target == null || target.panel != root.panel || !content.Contains(target) && target != content) return;
                    workshopLastJump = e.newValue;
                    RefreshWorkshopContext();
                    // A pending rebuild's layout restoration must not undo an explicit jump.
                    panelVersion++;
                    scroll.scrollOffset += new Vector2(0, target.worldBound.yMin - scroll.contentViewport.worldBound.yMin);
                });
                workshopNavigation = choice; workshopNavigationVillage = villageId; workshopNavigationBuilding = buildingId;
                workshopNavigationHost.Add(choice);
                workshopNavigationContext = Label(workshopNavigationHost, "", "muted");
                workshopNavigationContext.name = "workshop-navigation-context";
                workshopNavigationContext.style.fontSize = 12;
                workshopNavigationContext.style.marginTop = 0; workshopNavigationContext.style.marginBottom = 2;
                workshopNavigationContext.style.flexShrink = 0;
            }
            workshopNavigation.choices = new[] { WorkshopNavigationPrompt }.Concat(workshopDestinations.Select(d => d.Name)).ToList();
            workshopNavigation.SetValueWithoutNotify(WorkshopNavigationPrompt);
            RefreshWorkshopContext();
            workshopNavigationHost.style.display = DisplayStyle.Flex;
        }
        private bool WorkshopNavigationTarget(VisualElement target) => workshopNavigation != null
            && (target == workshopNavigation || target != null && workshopNavigation.Contains(target));
        private void ShowWorkshopNavigation()
        {
            var choice = workshopNavigation;
            if (choice == null || choice.panel != root.panel || OpenSection != "Inspect" || Village?.Id != workshopNavigationVillage || Game.View.SelectedBuilding?.Id != workshopNavigationBuilding) return;
            var menu = new GenericDropdownMenu();
            foreach (string name in choice.choices.Where(c => c != WorkshopNavigationPrompt))
            {
                string destination = name;
                menu.AddItem(destination, false, () => choice.value = destination);
            }
            choice.Focus();
            var input = choice.Q<VisualElement>(className: DropdownField.inputUssClassName);
            menu.DropDown(input.worldBound, choice, DropdownMenuSizeMode.Content);
        }
        private void RefreshWorkshopContext()
        {
            if (!workshopDestinations.Any(d => d.Name == workshopLastJump)) workshopLastJump = "";
            workshopNavigationContext.text = workshopLastJump == "" ? "" : "Jumped to " + workshopLastJump;
            workshopNavigationContext.style.display = workshopLastJump == "" ? DisplayStyle.None : DisplayStyle.Flex;
        }
        private void DelegationControls(VisualElement parent, VisualElement actions, Building building, WorkSlot slot, Cat cat, bool paused)
        {
            string role = World.WorkplaceOfficerRole(building.Kind);
            if (role == "" || cat == null || !cat.Alive || cat.ControlledBy != "") return;
            string name = UpperFirst(Pretty(role));
            bool recipes = Catalog.Recipes.Any(r => r.Building == building.Kind);
            if (recipes && paused)
                Button(actions, "Resume work", () => Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = building.Id, CatId = cat.Id, Edit = "pause", Enabled = false }));
            if (slot.AutomatedBy == "")
            {
                bool appointed = Village.Officers.Any(o => o.Role == role && Village.Cats.Any(c => c.Id == o.CatId && c.Alive));
                if (appointed)
                {
                    var button = Button(actions, "Return to " + name, () => Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = building.Id, CatId = cat.Id, Edit = "delegate" }));
                    button.SetEnabled(!paused);
                    Label(parent, paused ? "Resume work, then return this position to the " + name + "." : recipes ? "Keeps this cat and its current orders. The " + name + " chooses new recipes when the queue empties." : "Keeps this cat, current work and existing pauses.", "muted");
                }
                else
                {
                    Label(parent, "Appoint a " + name + " in Officers to delegate this position.", "muted");
                    Button(parent, "Manage " + name + " in Officers", () => OpenPanel("Officers"));
                }
            }
            else if (slot.Queue.Any(q => q.AutomatedBy == ""))
                Label(parent, "Current manual orders stay unchanged. The " + name + " chooses new recipes when the queue empties.", "muted");
            if (slot.Queue.Any(q => q.AutomatedBy == "" && q.Repeat))
                Label(parent, "Repeating orders continue until you change them.", "muted");
        }
        private VisualElement RecipeSupplyPanel(VisualElement parent, Building building, string recipeId)
        {
            if (string.IsNullOrEmpty(recipeId)) return null;
            var report = Game.CurrentWorld.WorkshopSupply(Village, building, recipeId);
            if (report.Inputs.Count == 0) return null;
            var panel = Element(parent, "recipe-supply");
            Label(panel, "Recipe inputs · " + Pretty(recipeId), "subtitle");
            foreach (var input in report.Inputs)
            {
                string recorded = report.CountedPiles == 0 ? "uncounted" : input.Reported.ToString("0.##") + "*";
                Label(panel, UpperFirst(Pretty(input.Resource)) + " · needs " + input.Required.ToString("0.##") + " · reported " + recorded, "muted");
            }
            if (report.CountedPiles == 0)
                Label(panel, "No counted storage reports. Quantities remain uncounted.", "muted");
            else
            {
                string counted = (report.OldestCountedAt / 3600).ToString("0.0") + "h";
                if (report.NewestCountedAt != report.OldestCountedAt) counted += " to " + (report.NewestCountedAt / 3600).ToString("0.0") + "h";
                Label(panel, "* Reports counted at " + counted + " · " + report.UncountedPiles + " storage piles uncounted.", "muted");
                if (report.Inputs.Any(input => input.Reported + 1e-8 < input.Required))
                    Label(panel, "Reported stock is below this bill. Uncounted and changed goods are not included.", "muted");
            }
            Label(panel, "Workers collect ingredients from reachable piles, including external piles. Haul from pile in Inventory can move goods into accepting storage.", "muted").name = "recipe-hauling-guidance";
            Label(panel, "Storage reports are historical and exclude external piles. A pile can still list moved goods; storage reports refresh when counted again.", "muted").name = "recipe-reports-guidance";
            Button(panel, "View inventory & hauling", () => OpenStores(false)).name = "recipe-inventory-hauling";
            var shown = new HashSet<string>(StringComparer.Ordinal);
            foreach (var input in report.Inputs)
            {
                var producers = Catalog.Recipes.Where(r => r.Outputs.Any(o => o.Resource == input.Resource) && Catalog.RecipeAvailable(Village, r)).ToArray();
                foreach (string kind in producers.Select(r => r.Building).Distinct())
                {
                    var supplier = Village.Buildings.Where(b => b.Kind == kind && b.Completed && b.Id != building.Id)
                        .OrderBy(b => Int2.Distance(building.Position, b.Position)).ThenBy(b => b.Id, StringComparer.Ordinal).FirstOrDefault();
                    if (supplier != null && shown.Add(supplier.Id))
                    {
                        string id = supplier.Id;
                        Button(panel, "Inspect supplier · " + Pretty(kind), () => InspectSupplier(building, id));
                    }
                    else if (kind == building.Kind && shown.Add("here:" + kind))
                        Label(panel, UpperFirst(Pretty(input.Resource)) + " can be made here with " + string.Join(", ", producers.Where(r => r.Building == kind).Select(r => Pretty(r.Id))) + ".", "muted");
                }
            }
            return panel;
        }
        private static string UpperFirst(string value) => string.IsNullOrEmpty(value) ? value : char.ToUpperInvariant(value[0]) + value.Substring(1);
        private int WorkCapacity(Building b) => Math.Max(1, (int)Catalog.BuildingEffect(Village, b.Kind, "worker_slots", 1));
        private static int OccupiedPositions(Building b) => b.Slots.Count(s => s.CatId != "");
        private static bool IsStaffedWorkplace(Building b) => b.Kind == "research_hut" || b.Kind == "school" || b.Kind == "accounting_tent" || b.Kind == "mouse_farm" || Catalog.Recipes.Any(r => r.Building == b.Kind);
        private static bool SupportsWorkerPause(Building b) => b.Kind == "research_hut" || b.Kind == "school" || Catalog.Recipes.Any(r => r.Building == b.Kind);
        private Job WorkerJob(Cat cat) => cat == null ? null : Village.Jobs.Find(j => j.Id == cat.JobId && !j.Completed);
        private bool TryProductionPosition(Cat cat, out Building building, out WorkSlot slot, out Job job)
        {
            building = null; slot = null; job = WorkerJob(cat);
            if (job == null || job.Kind != "production" || job.CatId != cat.Id) return false;
            string target = job.TargetId;
            building = Village.Buildings.Find(b => b.Id == cat.BuildingId && b.Id == target && b.Completed);
            slot = building?.Slots.Find(s => s.CatId == cat.Id);
            return slot != null;
        }
        private string ReportedCatStatus(Cat cat)
        {
            if (!cat.Alive) return "Dead";
            if (cat.ControlledBy != "") return "Under direct control";
            if (cat.Goal.StartsWith("need_", StringComparison.Ordinal)) return Pretty(cat.Goal);
            return TryProductionPosition(cat, out var building, out var slot, out _) ? WorkerStatus(building, slot, cat) : Pretty(cat.Goal);
        }
        private string ReportedCatReason(Cat cat, string reason)
        {
            // Pause/Resume can precede another simulation step. Only discard a
            // proven obsolete production marker; keep every other recorded reason.
            if (reason != "queue_paused" || !TryProductionPosition(cat, out var building, out var slot, out var job)) return reason;
            return (building.Paused || slot.Paused) && job.Phase != "output_delivery" ? reason : "";
        }
        private string WorkerStatus(Building b, WorkSlot slot, Cat cat)
        {
            if (cat == null || !cat.Alive) return "Worker unavailable";
            if (cat.ControlledBy != "") return "Under direct control";
            switch (cat.Goal)
            {
                case "need_eat": return "Getting food · work position kept";
                case "need_drink": return "Getting water · work position kept";
                case "need_sleep": return "Resting · work position kept";
                case "need_heal": return "Seeking care · work position kept";
            }
            var job = WorkerJob(cat);
            if (SupportsWorkerPause(b) && (slot.Paused || b.Paused) && !(job?.Kind == "production" && job.Phase == "output_delivery")) return "Paused · resume work to continue";
            if (job != null)
            {
                string phase;
                switch (job.Phase)
                {
                    case "fetch": case "item_fetch": phase = "Collecting ingredients"; break;
                    case "input_delivery": phase = "Carrying ingredients to workshop"; break;
                    case "working": phase = "Working"; break;
                    case "output_delivery": phase = "Delivering finished goods"; break;
                    case "travel": phase = "Walking to work"; break;
                    default: phase = Pretty(job.Phase); break;
                }
                return Pretty(job.RecipeId == "" ? job.Kind : job.RecipeId) + " · " + phase;
            }
            if (b.Kind == "research_hut" || b.Kind == "school")
                return cat.Position.Equals(b.Position) && Math.Abs(cat.X - b.Position.X) + Math.Abs(cat.Z - b.Position.Z) < 1e-6 ? "Researching" : "Walking to " + Pretty(b.Kind);
            if (b.Kind == "mouse_farm") return "Waiting to start next mouse-farm batch";
            if (b.Kind == "accounting_tent") return Pretty(cat.Goal);
            string officer = EmptyQueueOfficer(b, slot, cat);
            if (officer != "" && CanWaitForOfficer(b, slot, cat)) return "Waiting for " + officer + " to choose work";
            return slot.Queue.Count == 0 ? "Waiting for a recipe" : "Ready for next batch";
        }
        private string EmptyQueueOfficer(Building b, WorkSlot slot, Cat cat)
        {
            if (b == null || slot == null || cat == null || !b.Completed || !cat.Alive || cat.ControlledBy != ""
                || !ReferenceEquals(Village.Buildings.Find(value => value.Id == b.Id), b)
                || !ReferenceEquals(Village.Cats.Find(value => value.Id == cat.Id), cat)
                || !b.Slots.Contains(slot) || slot.CatId != cat.Id || cat.BuildingId != b.Id || slot.Queue.Count != 0
                || !Catalog.Recipes.Any(recipe => recipe.Building == b.Kind)) return "";
            string role = World.WorkplaceOfficerRole(b.Kind);
            if (role == "" || slot.AutomatedBy != role
                || !Village.Officers.Any(appointment => appointment.Role == role && Village.Cats.Any(holder => holder.Id == appointment.CatId && holder.Alive))) return "";
            return UpperFirst(Pretty(role));
        }
        private bool CanWaitForOfficer(Building b, WorkSlot slot, Cat cat)
        {
            if (b.Paused || slot.Paused || cat.ResumeJobId != "" || cat.Goal.StartsWith("need_", StringComparison.Ordinal)
                || WorkerJob(cat) != null || Village.Jobs.Any(job => !job.Completed && (job.CatId == cat.Id || job.Id == slot.JobId))) return false;
            string reason = WorkerReason(b, slot, cat);
            return reason == "" || reason == "queue_empty";
        }
        private string EmptyQueueText(Building b, WorkSlot slot, Cat cat)
        {
            string officer = EmptyQueueOfficer(b, slot, cat);
            if (officer == "") return "No recipes yet. Add the selected recipe to begin.";
            return CanWaitForOfficer(b, slot, cat)
                ? "The " + officer + " chooses work automatically. Add a recipe to take manual control of this position."
                : "No recipes queued. Add a recipe to take manual control of this position.";
        }
        private string WorkerReasonText(Building b, WorkSlot slot, Cat cat, string reason)
        {
            if (reason != "queue_empty") return HumanReason(reason);
            if (slot.Queue.Count != 0) return "";
            return EmptyQueueOfficer(b, slot, cat) == "" ? HumanReason(reason) : EmptyQueueText(b, slot, cat);
        }
        private string WorkerReason(Building b, WorkSlot slot, Cat cat)
            => ReportedWorkerReason(b, slot, cat, WorkerJob(cat));
        private static string ReportedWorkerReason(Building b, WorkSlot slot, Cat cat, Job job)
        {
            if (cat == null || !cat.Alive || cat.ControlledBy != "" || SupportsWorkerPause(b) && (slot.Paused || b.Paused)) return "";
            // Resume can precede the next simulation step, especially at global pause.
            // Read current pause flags without changing the world's recorded blockers.
            string Current(string reason) => reason == "queue_paused" && SupportsWorkerPause(b) ? "" : reason;
            string catReason = Current(cat.BlockedReason);
            if (cat.Goal.StartsWith("need_", StringComparison.Ordinal)) return catReason;
            if (job != null) return Current(job.BlockedReason) != "" ? Current(job.BlockedReason) : catReason;
            return catReason != "" ? catReason : Current(slot.BlockedReason) != "" ? Current(slot.BlockedReason) : b.Kind == "mouse_farm" ? b.BlockedReason : "";
        }
        private VisualElement QueuePanel(VisualElement parent, Building b, List<QueueEntry> queue, string catId, bool paused)
        {
            string kind = catId == "" ? "EditProductionQueue" : "EditProductionWorkSlot";
            bool remoteQueue = Game.IsRemote;
            var slot = catId == "" ? b.Slots.FirstOrDefault() : b.Slots.Find(s => s.CatId == catId);
            bool currentBatch = Game.CurrentWorld.ProductionBatchInProgress(Village, b, slot);
            var heading = Label(parent, (paused ? "Paused" : "Queue ready") + " · " + queue.Count + " recipes", "muted");
            if (catId != "") Label(parent, "Pause and Resume take manual control of this position.", "muted");
            if (currentBatch) Label(parent, "The current batch stays first until its output is ready. Future recipes remain editable.", "muted");
            var row = Element(parent, "row"); Button(row, "Pause", () => Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = true })).EnableInClassList("active", paused); Button(row, "Resume", () => Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = false })).EnableInClassList("active", !paused);
            if (queue.Count == 0) Label(parent, EmptyQueueText(b, slot, Village.Cats.Find(cat => cat.Id == catId)), "muted");
            for (int i = 0; i < queue.Count; i++)
            {
                int index = i; var q = queue[i]; var card = Element(parent, "queue-entry"); Label(card, (i + 1) + ". " + Pretty(q.RecipeId), "subtitle"); row = Element(card, "row");
                void Edit(string edit, int direction = 0, bool enabled = true)
                {
                    if (Game.IsRemote != remoteQueue) return;
                    int currentIndex = index;
                    if (!remoteQueue)
                    {
                        // Local batches can finish or move before refresh. Retain the exact order.
                        if (!ReferenceEquals(Village?.Buildings.Find(building => building.Id == b.Id), b)) return;
                        var currentSlot = catId == "" ? b.Slots.FirstOrDefault() : b.Slots.Find(position => position.CatId == catId);
                        var currentQueue = currentSlot?.Queue ?? (catId == "" ? b.Queue : null);
                        if (!ReferenceEquals(currentQueue, queue)) return;
                        currentIndex = queue.IndexOf(q);
                        if (currentIndex < 0) return;
                    }
                    // Remote snapshots have no order IDs. Preserve the protocol's rendered-index dispatch.
                    Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = edit, Index = currentIndex, Direction = direction, Enabled = enabled });
                }
                Button(row, "↑", () => Edit("move", -1)).SetEnabled(index > 0 && (!currentBatch || index > 1));
                Button(row, "↓", () => Edit("move", 1)).SetEnabled(index < queue.Count - 1 && (!currentBatch || index > 0));
                var repeat = Button(row, q.Repeat ? "Repeat on" : "Repeat off", () => Edit("repeat", 0, !q.Repeat));
                repeat.name = "queue-repeat-" + b.Id + "-" + catId + "-" + queueControlIds.GetValue(q, _ => Guid.NewGuid().ToString("N"));
                Button(row, "Remove", () => Edit("remove")).SetEnabled(!currentBatch || index != 0);
            }
            return heading;
        }
        private void BuildPanel()
        {
            Text("Choose a clear 2 × 2 site inside the village. Its entrance needs a road connected to the shrine. Cats carry materials to the site; farms belong outside the fence.");
            Label(content, "Paths and space", "subtitle");
            var paths = Element(content, "row");
            Button(paths, "Build stone road", () => Placement("Road: choose start, then end", p => new GameAction { Kind = "BuildRoad", Position = p }, true));
            Button(paths, "Build bridge", () => Placement("Place bridge", p => new GameAction { Kind = "BuildBridge", Position = p }));
            Button(content, "Expand village", () => Act(new GameAction { Kind = "RequestJob", Name = "expand" }));
            Separator(); Label(content, "Buildings", "subtitle");
            foreach (var kind in Catalog.Buildings.Where(k => k != "shrine"))
            {
                string id = kind; bool available = Catalog.BuildingAvailable(Village, id); var card = Card(Pretty(id));
                TextInto(card, available ? "Available" : "Research required", "muted");
                Button(card, "Place " + Pretty(id), () => Placement("Place " + Pretty(id) + (id == "field" ? " on known exterior land" : " on clear land inside the village"), p => new GameAction { Kind = "PlanBuilding", Name = id, Position = p }));
            }
            Separator(); Label(content, "Gathering and boundaries", "subtitle");
            Button(content, "Mark gathering zone", () => Placement("Gathering zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "gather" }, true, "Choose opposite corner"));
            Button(content, "Mark avoid zone", () => Placement("Avoid zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "avoid" }, true, "Choose opposite corner"));
            foreach (var zone in Village.Stockpiles.Where(p => p.Kind.StartsWith("zone_", StringComparison.Ordinal))) { string id = zone.Id; Button(content, "Remove " + zone.Kind + " " + zone.Position, () => Act(new GameAction { Kind = "RemoveZone", TargetId = id })); }
        }
        private void WorkPanel()
        {
            Text("Manual jobs remain available while an officer's seat is vacant. Personal needs can interrupt work.");
            var catChoices = new WorkCatChoices(this);
            catChoices.Add(content, "Cat", "work-cat-choice"); liveUpdates.Add(catChoices.Refresh);
            foreach (var labor in new[] { "hunt", "fetch_water", "woodcut", "quarry", "forage", "fish", "replant", "scout", "train" })
            {
                string name = labor;
                string job = name == "woodcut" ? "logs" : name == "forage" ? "fibre" : name;
                Button(content, Pretty(name), () => Act(new GameAction { Kind = name == "scout" ? "DispatchScout" : "RequestJob", Name = job, CatId = selectedCat }));
            }
            Button(content, "Haul goods · choose source", () => OpenStores(false));
            Button(content, "Research work · choose workplace", () =>
            {
                var hut = Village.Buildings.FirstOrDefault(b => b.Kind == "research_hut");
                if (hut == null) { OpenPanel("Build"); return; }
                Game.View.InspectBuilding(hut.Id); OpenPanel("Inspect");
            });
            Separator(); Label(content, "Farms and gathering", "subtitle");
            ResourceChoice(content);
            Button(content, "Quarry selected resource", () => Act(new GameAction { Kind = "RequestJob", Name = "quarry", CatId = selectedCat, Resource = selectedResource }));
            Button(content, "Scout for selected resource", () => Act(new GameAction { Kind = "DispatchScout", CatId = selectedCat, Resource = selectedResource }));
            foreach (var crop in new[] { "grain", "catnip", "herbs" }) { string id = crop; Button(content, "Designate " + crop + " farm", () => Placement("Exterior " + id + " farm: choose opposite corners", p => new GameAction { Kind = "DesignateFarm", Name = id, Resource = id, Position = p, End = p }, true, "Choose opposite corner")); }
            ResourceChoice(content); Button(content, "Designate gathering", () => Placement("Gathering area: choose opposite corners on known exterior land", p => new GameAction { Kind = "DesignateGatherSpot", Position = p, Resource = selectedResource }, true, "Choose opposite corner"));
            Button(content, "Designate fishing", () => Placement("Choose a known dry shore tile beside water", p => new GameAction { Kind = "DesignateFishingSpot", Position = p, Resource = "fish" }));
            FarmCards(catChoices);
            Separator(); Label(content, "Workplaces", "subtitle");
            var duplicatedKinds = new HashSet<string>(Village.Buildings.GroupBy(b => b.Kind).Where(group => group.Skip(1).Any()).Select(group => group.Key));
            foreach (var b in Village.Buildings.Where(b => !b.Completed || IsStaffedWorkplace(b) || b.Kind == "field" || b.Kind == "barracks"))
            {
                string id = b.Id;
                var button = Button(content, "", () => { Game.View.InspectBuilding(id); OpenPanel("Inspect"); });
                void Refresh()
                {
                    var building = Village.Buildings.Find(candidate => candidate.Id == id);
                    if (building == null) return;
                    string purpose = "under construction";
                    if (building.Completed)
                        purpose = building.Kind == "field" ? "farm support" : building.Kind == "barracks" ? "training site" : OccupiedPositions(building) + " / " + WorkCapacity(building) + " staffed";
                    button.text = Pretty(building.Kind) + " · " + purpose + (duplicatedKinds.Contains(building.Kind) ? " · " + building.Position : "");
                }
                Refresh(); liveUpdates.Add(Refresh);
            }
            foreach (var job in Village.Jobs.Where(j => !j.Completed))
            {
                string id = job.Id;
                var cat = Village.Cats.Find(c => c.Alive && c.Id == job.CatId && c.JobId == id);
                string reason = cat == null ? job.BlockedReason : ReportedCatReason(cat, job.BlockedReason);
                Text(Pretty(job.Kind) + " · " + Pretty(job.Phase)); if (reason != "") Text(HumanReason(reason), "warning");
                Button(content, "Boost " + Pretty(job.Kind) + " · " + id, () => Act(new GameAction { Kind = "Boost", TargetId = id }));
            }
        }
        private void FarmCards(WorkCatChoices catChoices)
        {
            string villageId = Village.Id;
            var farms = Element(content, "farm-cards");
            var cards = new Dictionary<string, (VisualElement Card, Action<Farm> Refresh)>();
            var order = new List<string>();
            void RefreshFarms()
            {
                var current = Village.Farms;
                for (int index = order.Count - 1; index >= 0; index--)
                {
                    string id = order[index]; bool exists = false;
                    foreach (var farm in current) if (farm.Id == id) { exists = true; break; }
                    if (exists) continue;
                    catChoices.Remove(cards[id].Card);
                    cards[id].Card.RemoveFromHierarchy(); cards.Remove(id); order.RemoveAt(index);
                }
                foreach (var farm in current)
                {
                    string id = farm.Id;
                    if (!cards.TryGetValue(id, out var shown))
                    {
                        var card = Element(farms, "card farm-card"); card.name = "farm-card-" + id;
                        var title = Label(card, "", "subtitle"); var plot = Label(card, "", "body");
                        var worker = Label(card, "", "body"); var ownership = Label(card, "", "body"); var warning = Label(card, "", "warning");
                        bool IsCurrentFarm() => OpenSection == "Work" && Village?.Id == villageId && farms.parent == content && card.parent == farms && Village.Farms.Any(value => value.Id == id);
                        catChoices.Add(card, "Choose", "farm-worker-choice-" + id, id);
                        Button(card, "Assign selected cat", () => { if (IsCurrentFarm()) Act(new GameAction { Kind = "AssignWorker", CatId = selectedCat, TargetId = id }); }).AddToClassList("farm-assign");
                        Button(card, "Clear farm", () => { if (IsCurrentFarm()) Act(new GameAction { Kind = "ClearFarm", TargetId = id }); });
                        var previous = (Crop: (string)null, Phase: (string)null, Position: default(Int2), Width: 0, Depth: 0, WorkerId: (string)null, WorkerName: (string)null, Ownership: (string)null, Reason: (string)null);
                        void RefreshFarm(Farm value)
                        {
                            var cat = Village.Cats.Find(candidate => candidate.Id == value.WorkerId);
                            // Assignment can precede the next farm tick while paused.
                            string reason = value.BlockedReason == "worker_required" && cat?.Alive == true ? "" : value.BlockedReason;
                            var values = (value.Crop, value.Phase, value.Position, value.Width, value.Depth, value.WorkerId, cat?.Name, value.AutomatedBy, reason);
                            if (previous.Equals(values)) return;
                            previous = values;
                            title.text = Pretty(value.Crop) + " · " + Pretty(value.Phase);
                            plot.text = "Plot · " + id + " · " + value.Position + " · " + value.Width + " × " + value.Depth;
                            worker.text = "Worker · " + (value.WorkerId == "" ? "unassigned" : cat == null ? value.WorkerId + " · unavailable" : cat.Name + " · " + cat.Id);
                            ownership.text = "Assignment · " + (value.WorkerId == "" ? "Unstaffed" : value.AutomatedBy == "" ? "Manual" : value.AutomatedBy == "farmer" ? "Farmer" : Pretty(value.AutomatedBy));
                            warning.text = reason == "" ? "" : HumanReason(reason);
                            warning.style.display = reason == "" ? DisplayStyle.None : DisplayStyle.Flex;
                        }
                        shown = (card, RefreshFarm); cards.Add(id, shown); order.Add(id);
                    }
                    // Resolve this frame's farm by stable ID, retaining the card
                    // and its controls across authority snapshot replacement.
                    shown.Refresh(farm);
                }
            }
            RefreshFarms(); liveUpdates.Add(RefreshFarms);
        }
        private void StoresPanel()
        {
            if (storesEquipment) { EquipmentPanel(); return; }
            Text("* Inventory is the last physical count. An Accountant refreshes each pile by visiting it; stale counts are not current stock.");
            CatChoice(content, true);
            Text("The chosen carrier walks to the source, then delivers to reachable accepting storage. Choose available cat to assign automatically.", "muted");
            foreach (var resource in Catalog.Resources.Where(r => r != "blessings")) Label(content, Pretty(resource) + "   " + Reported(resource), "stock-row");
            Separator(); ResourceChoice(content);
            Button(content, "Designate selected-resource pile", () => Placement("Filtered stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Accepts = new List<string> { selectedResource } }, true, "Choose opposite corner"));
            foreach (var p in Village.Stockpiles)
            {
                var pile = p; var card = Card(p.Kind + " · " + p.Id); Label(card, "Capacity " + p.Capacity.ToString("0") + " · " + (p.CountedAt < 0 ? "uncounted" : "counted at " + (p.CountedAt / 3600).ToString("0.0") + "h"), "muted");
                Label(card, Goods(p.Report), "body"); Button(card, "Haul from pile", () => Act(new GameAction { Kind = "HaulGatherSpot", TargetId = pile.Id, CatId = selectedCat })); Button(card, "Remove pile", () => Act(new GameAction { Kind = pile.Kind == "storage" ? "RemoveStockpile" : "RemoveGatherSpot", TargetId = pile.Id }));
            }
        }
        private void EquipmentPanel()
        {
            Label(content, "Equipment", "subtitle"); CatChoice(content);
            Text("Choose a free adult without cargo. Retrieval and return take a physical trip; existing work must be released first.", "muted");
            foreach (var order in Village.Jobs.Where(j => !j.Completed && IsEquipmentOrder(j))) EquipmentOrderCard(content, order);
            foreach (var item in Village.Items)
            {
                var i = item; var card = Card(Pretty(i.Material) + " " + i.Kind + " · quality " + i.Quality); card.name = "equipment-item-" + i.Id;
                Label(card, i.Id + " · " + i.Condition.ToString("0") + " / " + i.MaxCondition.ToString("0") + " condition", "body");
                var pile = Village.Stockpiles.Find(p => p.Id == i.LocationId);
                var bearer = Village.Cats.Find(c => c.Id == i.LocationId && c.Equipment.Contains(i.Id));
                var claim = Village.Jobs.Find(j => !j.Completed && j.Id == i.LocationId);
                Label(card, pile != null ? "Stored at " + pile.Id + " · " + pile.Position : bearer != null ? "Equipped by " + bearer.Name : claim != null ? "Reserved for " + Pretty(claim.Kind) : "In transport or workshop storage", "muted");
                var retrieve = Button(card, "Retrieve for selected cat", () => Act(new GameAction { Kind = "EquipItem", TargetId = i.Id, CatId = selectedCat }));
                retrieve.SetEnabled(pile != null && i.Condition > 0 && (i.Kind == "tool" || i.Kind == "weapon" || i.Kind == "armor"));
                var repair = Button(card, "Repair at staffed station", () => Act(new GameAction { Kind = "RepairItem", TargetId = i.Id }));
                repair.SetEnabled(pile?.Kind == "storage");
                if (bearer != null) Button(card, "Inspect " + bearer.Name, () => { Game.View.InspectCat(bearer.Id); OpenPanel("Inspect"); });
            }
        }
        private static bool IsEquipmentOrder(Job job) => job != null && (job.Kind == "equip" || job.Kind == "unequip");
        private void EquipmentOrderCard(VisualElement parent, Job order)
        {
            var cat = Village.Cats.Find(c => c.Id == order.CatId);
            var item = Village.Items.Find(i => order.Kind == "equip" ? i.Id == order.TargetId : order.ItemIds.Contains(i.Id));
            string itemName = item == null ? "equipment" : Pretty(item.Material) + " " + item.Kind + " · " + item.Id;
            var card = Element(parent, "card"); card.name = "equipment-order-" + order.Id;
            Label(card, (order.Kind == "equip" ? "Retrieve for " : "Return with ") + (cat?.Name ?? order.CatId), "subtitle");
            Label(card, itemName, "body");
            if (order.Kind == "equip")
            {
                var source = Village.Stockpiles.Find(p => p.Id == order.SourceId);
                Label(card, "Reserved at " + order.SourceId + " · " + (source?.Position ?? order.Position) + ". Equipment applies only after arrival.", "muted");
            }
            else Label(card, "Still equipped until delivered to " + order.TargetId + " · " + order.Position + ". Cancellation keeps it on the cat.", "muted");
            if (cat?.Goal.StartsWith("need_", StringComparison.Ordinal) == true) Label(card, "Meeting needs; this equipment order will resume afterward.", "muted");
            if (order.BlockedReason != "")
            {
                string reason = order.Kind == "unequip" && order.BlockedReason == "storage_full"
                    ? "Destination storage is full. Free space in " + order.TargetId + ", or cancel and issue a new return to another pile."
                    : order.Kind == "unequip" && order.BlockedReason == "storage_missing"
                    ? "The destination pile was removed; cancel and issue a new return to another pile."
                    : HumanReason(order.BlockedReason);
                Label(card, reason, "warning");
            }
            string jobId = order.Id, catId = order.CatId;
            Button(card, "Cancel equipment order", () => Act(new GameAction { Kind = "CancelEquipment", CatId = catId, TargetId = jobId }));
            if (OpenSection != "Inspect" && cat != null) Button(card, "Inspect " + cat.Name, () => { Game.View.InspectCat(catId); OpenPanel("Inspect"); });
        }
        private void ResearchPanel()
        {
            Text("All " + Catalog.Research.Count + " studies. A purchase applies its listed effects immediately; workshops still need workers, materials and queues.");
            var controls = Element(content, "row");
            Button(controls, "Study cards", () => { researchMap = false; RenderPanel(); });
            Button(controls, "Dependency map", () => { researchMap = true; RenderPanel(); });
            Choice(content, "Category", new[] { "All categories" }.Concat(Catalog.Research.Select(n => n.Category).Distinct()), researchCategory, x => researchCategory = x).RegisterValueChangedCallback(_ => RenderPanel());
            var search = Field(content, "Search studies", query);
            var researchCards = Element(content, researchMap ? "research-map" : "research-cards");
            search.RegisterValueChangedCallback(e => { query = e.newValue; RenderResearchCards(researchCards); }); RenderResearchCards(researchCards);
        }
        private void RenderResearchCards(VisualElement parent)
        {
            parent.Clear();
            var nodes = Catalog.Research.Where(n => (researchCategory == "All categories" || n.Category == researchCategory) && (query == "" || (n.Id + " " + n.Name + " " + n.Description + " " + n.Category).IndexOf(query, StringComparison.OrdinalIgnoreCase) >= 0)).ToList();
            var positions = new Dictionary<string, Vector2>();
            VisualElement canvas = parent;
            if (researchMap)
            {
                var viewport = new ScrollView(ScrollViewMode.VerticalAndHorizontal); viewport.style.height = 420; parent.Add(viewport);
                canvas = new VisualElement(); viewport.Add(canvas);
                var levels = new Dictionary<string, int>();
                int Level(string id)
                {
                    if (levels.TryGetValue(id, out int existing)) return existing;
                    var study = Catalog.Research.Find(n => n.Id == id);
                    int level = study == null || study.Prerequisites.Count == 0 ? 0 : study.Prerequisites.Max(Level) + 1; levels[id] = level; return level;
                }
                var rows = new Dictionary<int, int>();
                foreach (var node in nodes)
                { int level = Level(node.Id); rows.TryGetValue(level, out int row); positions[node.Id] = new Vector2(level * 330, row * 330); rows[level] = row + 1; }
                canvas.style.width = positions.Count == 0 ? 300 : positions.Values.Max(p => p.x) + 310;
                canvas.style.height = positions.Count == 0 ? 300 : positions.Values.Max(p => p.y) + 320;
                canvas.generateVisualContent += context =>
                {
                    var painter = context.painter2D; painter.strokeColor = new Color(.55f, .63f, .39f); painter.lineWidth = 2;
                    foreach (var node in nodes) foreach (var prerequisite in node.Prerequisites)
                    {
                        if (!positions.TryGetValue(prerequisite, out var from)) continue;
                        var to = positions[node.Id]; from += new Vector2(300, 130); to += new Vector2(0, 130);
                        painter.BeginPath(); painter.MoveTo(from); painter.BezierCurveTo(from + Vector2.right * 20, to - Vector2.right * 20, to); painter.Stroke();
                    }
                };
            }
            foreach (var node in nodes)
            {
                var n = node; bool owned = Village.Research.Contains(n.Id), prereqs = n.Prerequisites.All(Village.Research.Contains);
                var card = Element(canvas, "research-card");
                card.EnableInClassList("complete", owned); card.EnableInClassList("locked", !prereqs);
                if (researchMap) { var p = positions[n.Id]; card.style.position = Position.Absolute; card.style.left = p.x; card.style.top = p.y; card.style.width = 300; card.style.height = 310; }
                Label(card, n.Category.ToUpperInvariant(), "eyebrow"); Label(card, n.Name, "subtitle"); Label(card, n.Description, "body");
                Label(card, "Cost " + n.Cost.ToString("0") + " research · " + (owned ? "complete" : prereqs ? "prerequisites met" : "needs " + string.Join(", ", n.Prerequisites.Where(p => !Village.Research.Contains(p)))), owned ? "muted" : "body");
                Label(card, string.Join(" · ", n.Payloads.Select(p => Pretty(p.Kind) + ": " + p.Id + (p.Value != 0 ? " " + p.Value.ToString("0.##") : ""))), "muted");
                foreach (var prerequisite in n.Prerequisites)
                {
                    string id = prerequisite; string title = Catalog.Research.Find(study => study.Id == id)?.Name ?? id;
                    Button(card, "Requires " + title, () => { query = id; researchCategory = "All categories"; RenderPanelAtTop(); });
                }
                if (!owned) Button(card, "Research " + n.Name, () => Act(new GameAction { Kind = "ResearchNode", NodeId = n.Id }));
            }
        }
        private void OfficersPanel()
        {
            Text("The founding Leader keeps primitive food, water and scouting work moving. Specialist offices automate their own category after the required study and building."); CatChoice(content);
            for (int i = 0; i < Catalog.Roles.Length; i++)
            {
                string role = Catalog.Roles[i], studyId = Catalog.RoleStudies[i], buildingKind = Catalog.RoleBuildings[i];
                var officer = Village.Officers.Find(o => o.Role == role); var card = Card(Pretty(role));
                card.AddToClassList("officer-card"); card.name = role;
                string studyName = Catalog.Study(studyId)?.Name ?? Pretty(studyId);
                string buildingName = Pretty(buildingKind); buildingName = char.ToUpperInvariant(buildingName[0]) + buildingName.Substring(1);
                bool studied = Village.Research.Contains(studyId), built = Village.Buildings.Any(b => b.Kind == buildingKind && b.Completed);
                Label(card, officer == null ? "Vacant · manual control" : Village.Cats.Find(c => c.Id == officer.CatId)?.Name ?? "Missing holder", "body");
                Label(card, "Study · " + studyName + " · " + (studied ? "complete" : "needed"), studied ? "muted" : "body");
                Label(card, "Building · " + buildingName + " · " + (built ? "complete" : "needs completion"), built ? "muted" : "body");
                if (!studied) Button(card, "Find study · " + studyName, () => { query = studyId; researchCategory = "All categories"; researchMap = false; OpenPanel("Research"); });
                Button(card, "Appoint selected cat", () => Act(new GameAction { Kind = "AssignOfficer", Role = role, CatId = selectedCat })); Button(card, "Vacate office", () => Act(new GameAction { Kind = "UnassignOfficer", Role = role }));
            }
        }
        private void ShrinePanel()
        {
            Text("Blessings " + Village.Blessings.ToString("0.0") + " · separate from research points. Offerings travel with a living cat to the shrine.");
            Choice(content, "Offering", new[] { "food", "herbs", "materials" }, selectedResource, x => selectedResource = x); Number(content, "Amount", amount, x => amount = x);
            Button(content, "Carry offering to shrine", () => Act(new GameAction { Kind = "OfferResource", Resource = selectedResource, Amount = amount }));
            Button(content, "Offer materials", () => Act(new GameAction { Kind = "OfferMaterials", Amount = amount })); Button(content, "Offer surplus tithe", () => Act(new GameAction { Kind = "OfferTithe" }));
            Button(content, "Assign ritual work", () => Act(new GameAction { Kind = "RequestJob", Name = "ritual" }));
            foreach (var upgrade in Catalog.Upgrades)
            {
                string key = upgrade;
                Button(content, Pretty(key) + " · level " + Game.CurrentWorld.UpgradeLevel(Village, key) + " · " + Game.CurrentWorld.UpgradeCost(Village, key) + " blessings", () => Act(new GameAction { Kind = "PurchaseUpgrade", Name = key }));
            }
            foreach (var n in Catalog.Research.Take(24).Where(n => !Village.Research.Contains(n.Id))) { string id = n.Id; Button(content, "Blessing: " + n.Name + " · " + n.Cost.ToString("0"), () => Act(new GameAction { Kind = "UnlockNode", NodeId = id })); }
        }
        private void WorldPanel()
        {
            var world = Game.CurrentWorld;
            Text(world.GenerationVersion > 0 ? "Explore the forest, follow rivers and descend into the ruins. Cats carry recovered goods home." : "This saved forest keeps its original terrain. The new landscape generator is used for new worlds.");
            var levels = Element(content, "row");
            Button(levels, "Surface", () => { Game.View.ViewLevel(0); RenderPanel(); });
            foreach (int depth in Village.Known.Select(p => p.Level).Where(l => l < 0).Distinct().OrderByDescending(l => l))
            { int level = depth; Button(levels, "Depth " + -level, () => { Game.View.ViewLevel(level); RenderPanel(); }); }
            Text("Viewing " + (Game.View.RenderedLevel == 0 ? "surface" : "depth " + -Game.View.RenderedLevel) + " · Page Up / Page Down changes discovered levels");
            var terrain = Label(content, "", "body");
            liveUpdates.Add(() => { var tile = world.GetTile(Game.View.CursorTile); terrain.text = tile == null || !Village.Known.Contains(tile.Position) ? "Unexplored terrain" : Pretty(tile.Biome) + " · elevation " + tile.Elevation.ToString("0.0") + " m" + (tile.Water ? " · water depth " + tile.WaterDepth.ToString("0.0") + " m" : ""); });
            CatChoice(content);
            foreach (var site in world.Dungeons.Where(d => d.RumoredBy.Contains(Village.Id)))
            {
                var dungeon = site; var card = Card(site.Name);
                Label(card, site.Depth + " levels · entrance danger " + site.Threat.ToString("0.0"), "body");
                var readiness = Label(card, "", "body");
                liveUpdates.Add(() => { var cat = Village.Cats.Find(c => c.Id == selectedCat); double strength = world.DungeonReadiness(Village, cat); readiness.text = cat == null ? "Choose an explorer" : cat.Name + " · readiness " + strength.ToString("0.0") + (strength < dungeon.Threat ? " · train or equip this cat first" : " · ready for the entrance"); });
                Button(card, "Show entrance", () => Game.View.FocusDungeon(dungeon.Id));
                Button(card, "Explore with selected cat", () => Act(new GameAction { Kind = "ExploreDungeon", CatId = selectedCat, TargetId = dungeon.Id }));
            }
            foreach (var cat in Village.Cats.Where(c => c.Alive && world.IsDungeonExplorer(c)))
            {
                string id = cat.Id; var card = Card(cat.Name + " · " + Pretty(cat.DungeonPhase));
                Label(card, (cat.Position.Level == 0 ? "Surface" : "Depth " + -cat.Position.Level) + " · " + Goods(cat.Cargo), "body");
                if (cat.BlockedReason != "") Label(card, HumanReason(cat.BlockedReason), "warning");
                Button(card, "Follow explorer", () => { Game.View.InspectCat(id); OpenPanel("Inspect"); });
                Button(card, "Recall to village", () => Act(new GameAction { Kind = "RecallExplorer", CatId = id }));
            }
            Text("Depth increases danger. Explorers compare it with their skills, equipment, health and needs and return before taking on a stronger floor. In direct control, walk along stairs to change level and press E beside an enemy to fight.");
        }
        private void DefensePanel()
        {
            Text("Choose an available adult for defense. Untrained cats can defend; occupied workers keep their jobs.");
            var readiness = Element(content, "card defense-readiness");
            var captain = Label(readiness, "", "subtitle");
            var coverage = Label(readiness, "", "body");
            var preparation = Label(readiness, "", "muted");
            float nextReadiness = 0;
            void RefreshReadiness()
            {
                var officer = Village.Officers.Find(o => o.Role == "captain" && Village.Cats.Any(c => c.Id == o.CatId && c.Alive));
                captain.text = "Captain · " + (officer == null ? "Vacant" : Village.Cats.Find(c => c.Id == officer.CatId).Name);
                var state = Game.CurrentWorld.CaptainPreparationStatus(Village);
                coverage.text = state.Trained + " trained / " + state.Target + " target · " + state.Training + " training";
                preparation.text = CaptainPreparationReason(state.Blocker);
                nextReadiness = Time.unscaledTime + .5f;
            }
            RefreshReadiness(); liveUpdates.Add(() => { if (Time.unscaledTime >= nextReadiness) RefreshReadiness(); });
            Button(readiness, "Manage Captain in Officers", () => OpenPanel("Officers"));
            Text("Barracks training adds three fighting skill. Weapons and armor are equipped manually in Stores.", "muted");
            CatChoice(content);
            var selected = Label(content, "", "muted");
            void RefreshSelection()
            {
                var cat = Village.Cats.Find(c => c.Id == selectedCat && c.Alive);
                selected.text = cat == null ? "Choose a living adult." : cat.Name + " · health " + cat.Health.ToString("0") + " · fighting skill " + World.Amount(cat.Skills, "fight").ToString("0.#") + " · " + ReportedCatStatus(cat);
            }
            RefreshSelection(); liveUpdates.Add(RefreshSelection);
            var preparationControls = Element(content, "row");
            Button(preparationControls, "Train selected cat", () => Act(new GameAction { Kind = "TrainWarrior", CatId = selectedCat }));
            Button(preparationControls, "Choose equipment in Stores", () => OpenStores(true));
            Separator(); Label(content, "Active raids", "subtitle");
            foreach (var raid in Village.Raids)
            {
                string id = raid.Id;
                var card = Element(content, "card raid-card"); card.name = id;
                Label(card, "Raid · " + id, "subtitle");
                var state = Label(card, "", "body");
                var defend = Button(card, "Defend with selected cat", () => Act(new GameAction { Kind = "DefendRaid", CatId = selectedCat, TargetId = id }));
                void RefreshRaid()
                {
                    var current = Village.Raids.Find(r => r.Id == id);
                    state.text = current == null ? "Raid ended" : "Health " + Math.Max(0, current.Health).ToString("0.0") + " · " + current.Position + " · " + Pretty(current.Phase);
                    defend.SetEnabled(current != null && current.Health > 0);
                }
                RefreshRaid(); liveUpdates.Add(RefreshRaid);
            }
            if (Village.Raids.Count == 0) Text("No active raids.");
            Separator(); Label(content, "Defenders", "subtitle");
            var orders = ActiveDefenseOrders().ToList();
            foreach (var order in orders)
            {
                string id = order.Id, catId = order.CatId;
                var card = Element(content, "card defender-order"); card.name = id;
                var title = Label(card, "", "subtitle");
                Label(card, (order.Manual ? "Manual order" : order.AutomatedBy == "captain" ? "Captain order" : Pretty(order.AutomatedBy) + " order") + " · target " + order.TargetId, "muted");
                var state = Label(card, "", "body");
                var blocker = Label(card, "", "warning");
                void RefreshDefender()
                {
                    var cat = Village.Cats.Find(c => c.Id == catId);
                    var current = Village.Jobs.Find(j => j.Id == id && !j.Completed);
                    title.text = cat?.Name ?? catId;
                    state.text = cat == null || !cat.Alive || current == null ? "Defense order ended" : DefenseDutyStatus(cat, current) + " · health " + cat.Health.ToString("0");
                    string reason = current == null ? "" : cat != null && cat.Goal.StartsWith("need_", StringComparison.Ordinal) ? cat.BlockedReason : current.BlockedReason != "" ? current.BlockedReason : cat?.BlockedReason ?? "";
                    blocker.text = HumanReason(reason); blocker.style.display = reason == "" ? DisplayStyle.None : DisplayStyle.Flex;
                }
                RefreshDefender(); liveUpdates.Add(RefreshDefender);
                Button(card, "Inspect defender", () => { Game.View.InspectCat(catId); OpenPanel("Inspect"); });
                if (order.Manual) Button(card, "Cancel defense order", () => Act(new GameAction { Kind = "CancelDefense", TargetId = id, CatId = catId }));
                else Button(card, "Manage Captain in Officers", () => OpenPanel("Officers"));
            }
            if (orders.Count == 0) Text("No defense orders. A Captain dispatches available trained cats; without a Captain, issue orders here.", "muted");
            Text("Defenders walk to their target and fight in reach. Food, water, rest and healing can interrupt duty; the order remains assigned.", "muted");
            renderedDefenseTopology = DefenseTopology();
        }
        private IEnumerable<Job> ActiveDefenseOrders() => Village.Jobs.Where(j => !j.Completed && j.Kind == "defend_raid" && Village.Cats.Any(c => c.Id == j.CatId && c.Alive && (c.JobId == j.Id || c.ResumeJobId == j.Id)));
        private string DefenseTopology() => string.Join("|", Village.Raids.Select(r => r.Id)) + "/" + string.Join("|", ActiveDefenseOrders().Select(j => j.Id));
        private bool PanelControlFocused
        {
            get
            {
                if (TextInputFocused || root?.panel?.visualTree.Q<VisualElement>(className: GenericDropdownMenu.ussClassName) != null) return true;
                for (var element = root?.focusController?.focusedElement as VisualElement; element != null; element = element.parent)
                    // The fixed destination chooser survives content refreshes. Its open menu
                    // is still protected above, without leaving worker feedback stale afterward.
                    if (element is DropdownField && element != workshopNavigation) return true;
                return false;
            }
        }
        private static string DefenseDutyStatus(Cat cat, Job order)
        {
            switch (cat.Goal)
            {
                case "need_eat": return "Getting food · defense order kept";
                case "need_drink": return "Getting water · defense order kept";
                case "need_sleep": return "Resting · defense order kept";
                case "need_heal": return "Seeking care · defense order kept";
                default: return order.Phase == "fighting" ? "Fighting · " + order.Progress.ToString("0.0") + " combat seconds" : "Pursuing raid";
            }
        }
        private static string CaptainPreparationReason(string reason)
        {
            switch (reason)
            {
                case "captain_unassigned": return "Training and defense are manual. Appoint a Captain in Officers to delegate them.";
                case "raid_active": return "An active raid takes priority over new training.";
                case "barracks_missing": return "Complete a Barracks to prepare warriors.";
                case "barracks_paused": return "The Barracks is paused. Open it from Work and resume training.";
                case "barracks_unreachable": return "No available trainee can reach the Barracks. Check paths and gates.";
                case "idle_reserve": return "Training is waiting to preserve the colony's idle reserve. Free another adult from work.";
                case "no_eligible_trainee": return "No available adult can train. Check jobs, needs and direct control.";
                case "training_active": return "Training is underway. The Captain waits for this crew before adding another trainee.";
                case "ready": return "The trained crew meets the current target. Equipment remains your choice.";
                default: return "The Captain is ready to assign a trainee.";
            }
        }
        private void TradePanel()
        {
            Text("Trade uses finite goods. Village exchange needs returned-scout contact and a passable physical route.");
            ResourceChoice(content); Number(content, "Offer / buy amount", amount, x => amount = x);
            Label(content, "Visiting trader · " + Village.Trader.Phase, "subtitle"); Text("Coins " + Village.Coins.ToString("0") + " · trader stock " + Goods(Village.Trader.Goods));
            Button(content, "Buy resource", () => Act(new GameAction { Kind = "BuyResource", Resource = selectedResource, Amount = amount }));
            var saleItems = Village.Items.Where(i => Village.Stockpiles.Any(p => p.Id == i.LocationId && p.Kind == "storage")).ToList();
            string saleItem = "";
            Choice(content, "Stored item to sell", saleItems.Select(i => Pretty(i.Material) + " " + i.Kind + " · " + i.Id), "", x => saleItem = saleItems.Find(i => x.EndsWith(" · " + i.Id, StringComparison.Ordinal))?.Id ?? "");
            Button(content, "Sell goods", () => Act(new GameAction { Kind = "SellGoods", TargetId = saleItem }));
            Separator(); var other = Field(content, "Recipient village ID", selectedOther); other.RegisterValueChangedCallback(e => selectedOther = e.newValue);
            Text("Known contacts: " + string.Join(", ", Village.Contacts));
            string wanted = selectedResource == "water" ? "food" : "water"; Choice(content, "Request resource", Catalog.Resources.Where(r => r != "blessings"), wanted, x => wanted = x); Number(content, "Request amount", otherAmount, x => otherAmount = x);
            Button(content, "Offer exchange", () => Act(new GameAction { Kind = "OfferVillageTrade", OtherVillageId = selectedOther, Resource = selectedResource, Amount = amount, OtherResource = wanted, OtherAmount = otherAmount }));
            foreach (var offer in Game.CurrentWorld.TradeOffers.Where(t => t.FromVillageId == Village.Id || t.ToVillageId == Village.Id))
            { string id = offer.Id; var card = Card(offer.Id + " · " + offer.Status); Label(card, offer.Offered.Amount + " " + offer.Offered.Resource + " ⇄ " + offer.Requested.Amount + " " + offer.Requested.Resource, "body"); Button(card, "Accept", () => Act(new GameAction { Kind = "AcceptVillageTrade", TargetId = id })); Button(card, "Cancel / return escrow", () => Act(new GameAction { Kind = "CancelVillageTrade", TargetId = id })); }
        }
        private void RoutesPanel()
        {
            if (routeDraftVillage != Village.Id) { routeDraftVillage = Village.Id; selectedRouteDestination = ""; }
            Text("Build the route infrastructure and a vehicle, then assign a cat and endpoints."); Choice(content, "Mode", new[] { "rail", "shipping" }, selectedMode, x => selectedMode = x);
            Button(content, "Lay rail", () => Placement("Rail: choose start, then end", p => new GameAction { Kind = "DesignateRail", Position = p }, true)); Button(content, "Build dock", () => Placement("Place dock on known dry shore beside water", p => new GameAction { Kind = "BuildDock", Position = p }));
            Button(content, "Build vehicle", () => Placement("Place vehicle at infrastructure", p => new GameAction { Kind = "BuildTransportVehicle", Position = p, Mode = selectedMode }));
            CatChoice(content); ResourceChoice(content); Number(content, "Cargo amount", amount, x => amount = x);
            var source = Choice(content, "Source pile", Village.Stockpiles.Select(p => p.Id), selectedPile, x => selectedPile = x);
            var destination = Choice(content, "Destination pile", Village.Stockpiles.Select(p => p.Id), selectedRouteDestination, x => selectedRouteDestination = x);
            void RefreshDestination()
            {
                var piles = Village.Stockpiles; var options = destination.choices;
                bool changed = piles.Count == 0 ? options.Count != 1 || options[0] != "None available" : options.Count != piles.Count;
                for (int index = 0; !changed && index < piles.Count; index++) changed = options[index] != piles[index].Id;
                if (changed) { options = piles.Select(p => p.Id).ToList(); if (options.Count == 0) options.Add("None available"); destination.choices = options; }
                if (!options.Contains(selectedRouteDestination)) selectedRouteDestination = options[0];
                if (destination.value != selectedRouteDestination) destination.SetValueWithoutNotify(selectedRouteDestination);
            }
            liveUpdates.Add(RefreshDestination);
            Text("An available vehicle of the chosen mode will be assigned by the colony.");
            Button(content, "Create transport route", () =>
            {
                if (routeDraftVillage != Village.Id || !Village.Stockpiles.Any(p => p.Id == selectedRouteDestination)) { RenderPanel(); return; }
                var from = Village.Stockpiles.Find(p => p.Id == selectedPile); var to = Village.Stockpiles.Find(p => p.Id == selectedRouteDestination);
                var path = from == null || to == null ? new List<Int2>() : TransportPath(from.Position, to.Position);
                Act(new GameAction { Kind = "CreateTransportRoute", Mode = selectedMode, CatId = selectedCat, TargetId = selectedPile, BuildingId = selectedRouteDestination, Path = path, Resource = selectedResource, Amount = amount, Repeat = true });
            });
            var routes = Element(content, "transport-routes");
            var shown = new List<(string Id, string Mode, string Phase, string Resource, string Reason, string DriverId, string DriverName, string SourceId, Int2? SourcePosition, string DestinationId, Int2? DestinationPosition)>();
            (string Id, string Mode, string Phase, string Resource, string Reason, string DriverId, string DriverName, string SourceId, Int2? SourcePosition, string DestinationId, Int2? DestinationPosition) Snapshot(TransportRoute route)
            {
                Cat cat = null; foreach (var candidate in Village.Cats) if (candidate.Id == route.CatId) { cat = candidate; break; }
                Stockpile from = null, to = null;
                foreach (var pile in Village.Stockpiles)
                {
                    if (pile.Id == route.SourceId) from = pile; if (pile.Id == route.DestinationId) to = pile;
                    if (from != null && to != null) break;
                }
                return (route.Id, route.Mode, route.Phase, route.Resource, route.BlockedReason, route.CatId, cat?.Name, route.SourceId, from?.Position, route.DestinationId, to?.Position);
            }
            string Endpoint(string id, Int2? position) => id + " · " + (position.HasValue ? position.Value.ToString() : "unavailable");
            void RefreshRoutes()
            {
                var current = Village.Routes; bool changed = current.Count != shown.Count;
                for (int index = 0; !changed && index < current.Count; index++) changed = !shown[index].Equals(Snapshot(current[index]));
                if (!changed) return;
                shown.Clear(); routes.Clear();
                foreach (var item in current)
                {
                    var route = Snapshot(item);
                    shown.Add(route);
                    string id = route.Id;
                    var card = Element(routes, "card"); Label(card, Pretty(route.Mode) + " · " + Pretty(route.Phase), "subtitle");
                    Label(card, Pretty(route.Resource), "body"); Label(card, "Driver · " + (route.DriverName == null ? route.DriverId + " · unavailable" : route.DriverName + " · " + route.DriverId), "body");
                    Label(card, "Source · " + Endpoint(route.SourceId, route.SourcePosition), "body"); Label(card, "Destination · " + Endpoint(route.DestinationId, route.DestinationPosition), "body");
                    if (route.Reason != "") Label(card, HumanReason(route.Reason), "warning");
                    Button(card, "Cancel route", () => Act(new GameAction { Kind = "CancelTransportRoute", TargetId = id }));
                }
            }
            RefreshRoutes(); liveUpdates.Add(RefreshRoutes);
        }
        private void Placement(string label, Func<Int2, GameAction> action, bool line = false, string secondPointPrompt = "Choose end of route")
        {
            placement = action; placementLabel = label; placementSecondPointPrompt = secondPointPrompt; firstPoint = null; linePlacement = line; drawer.style.display = DisplayStyle.None; OpenSection = ""; UpdateNavigation();
        }
        private List<Int2> TransportPath(Int2 start, Int2 end)
        {
            var tiles = Game.CurrentWorld.Tiles.ToDictionary(t => t.Position);
            var parents = new Dictionary<Int2, Int2>(); var queue = new Queue<Int2>(); queue.Enqueue(start); parents[start] = start;
            while (queue.Count > 0 && parents.Count < 4096)
            {
                var point = queue.Dequeue(); if (point.Equals(end))
                { var path = new List<Int2>(); while (!point.Equals(start)) { path.Add(point); point = parents[point]; } path.Add(start); path.Reverse(); return path; }
                foreach (var step in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                { var p = new Int2(point.X + step.X, point.Z + step.Z); if (parents.ContainsKey(p) || !tiles.TryGetValue(p, out var tile)) continue; if (selectedMode == "rail" ? !tile.Rail : (!tile.Water && !p.Equals(end))) continue; parents[p] = point; queue.Enqueue(p); }
            }
            return new List<Int2>();
        }
        private void Place(Int2 point)
        {
            if (placement == null) return;
            if (linePlacement && firstPoint == null) { firstPoint = point; placementLabel = placementSecondPointPrompt; return; }
            var action = placement(firstPoint ?? point); action.End = point;
            if (linePlacement)
            {
                var p = action.Position; action.Path.Add(p); int guard = 0;
                while (!p.Equals(point) && guard++ < 128) { p = p.X != point.X ? new Int2(p.X + Math.Sign(point.X - p.X), p.Z) : new Int2(p.X, p.Z + Math.Sign(point.Z - p.Z)); action.Path.Add(p); }
            }
            Act(action); placement = null; firstPoint = null; placementSecondPointPrompt = "";
        }
        private async void Act(GameAction action, bool focus = false)
        {
            var result = await Game.Send(action); actionFailed = !result.Success; if (result.Success && focus) Game.View.FocusVillage();
            feedback.text = HumanReason(Game.LastAction); feedback.style.display = DisplayStyle.Flex; feedback.EnableInClassList("failure", actionFailed);
            root.EnableInClassList("has-feedback", true);
            if (OpenSection != "") RenderPanel();
        }
        private void CatChoice(VisualElement parent, bool automatic = false)
        {
            var cats = Village.Cats.Where(c => c.Alive).ToList(); if (!cats.Any(c => c.Id == selectedCat)) selectedCat = automatic ? "" : cats.FirstOrDefault()?.Id ?? "";
            var names = cats.Select(c => c.Name + " · " + c.Id).ToList(); if (automatic) names.Insert(0, "Choose available cat");
            string current = selectedCat == "" ? names.FirstOrDefault() : names.Find(n => n.EndsWith(" · " + selectedCat));
            Choice(parent, "Cat", names, current, x => selectedCat = x == "Choose available cat" ? "" : cats.Find(c => x.EndsWith(" · " + c.Id))?.Id ?? "");
        }
        // Work has several views of one prospective worker. Keep IDs and labels,
        // never Cat objects from a replaceable authority snapshot.
        private sealed class WorkCatChoices
        {
            private const string Empty = "Choose available cat";
            private readonly ForestUI ui;
            private readonly string villageId;
            private readonly List<DropdownField> fields = new List<DropdownField>();
            private readonly List<(string Id, string Name)> roster = new List<(string, string)>();
            private List<string> labels;
            private DropdownField anchor;

            public WorkCatChoices(ForestUI ui)
            {
                this.ui = ui; villageId = ui.Village.Id; ReadRoster();
            }
            private bool IsCurrent() => ui.OpenSection == "Work" && ui.Village?.Id == villageId
                && anchor?.parent == ui.content && anchor.panel != null && anchor.panel == ui.root.panel;
            private bool RosterMatches()
            {
                int index = 0;
                foreach (var cat in ui.Village.Cats)
                {
                    if (!cat.Alive) continue;
                    if (index >= roster.Count || roster[index].Id != cat.Id || roster[index].Name != cat.Name) return false;
                    index++;
                }
                return index == roster.Count;
            }
            private void ReadRoster()
            {
                roster.Clear(); labels = new List<string> { Empty };
                bool selectedAlive = ui.selectedCat == "";
                foreach (var cat in ui.Village.Cats)
                {
                    if (!cat.Alive) continue;
                    roster.Add((cat.Id, cat.Name)); labels.Add(cat.Name + " · " + cat.Id);
                    if (cat.Id == ui.selectedCat) selectedAlive = true;
                }
                if (!selectedAlive) ui.selectedCat = "";
            }
            private string SelectedLabel()
            {
                for (int index = 0; index < roster.Count; index++)
                    if (roster[index].Id == ui.selectedCat) return labels[index + 1];
                return Empty;
            }
            private void SyncValues()
            {
                string value = SelectedLabel();
                foreach (var field in fields)
                    if (field.value != value) field.SetValueWithoutNotify(value);
            }
            public void Add(VisualElement parent, string label, string name, string farmId = "")
            {
                var field = new DropdownField(label) { name = name, choices = labels };
                field.SetValueWithoutNotify(SelectedLabel()); parent.Add(field);
                if (anchor == null) anchor = field;
                fields.Add(field);
                if (farmId != "") field.tooltip = "Choose a named cat for this plot. The empty choice cannot assign a farm worker.";
                field.RegisterValueChangedCallback(e =>
                {
                    if (!IsCurrent() || field.panel != ui.root.panel || !ui.content.Contains(field)
                        || farmId != "" && !ui.Village.Farms.Any(f => f.Id == farmId)) return;
                    // An old menu event must not be reinterpreted against a new
                    // roster. Reconcile it only after refusing that event.
                    if (!RosterMatches()) { Refresh(); return; }
                    int index = labels.IndexOf(e.newValue);
                    if (index >= 0) ui.selectedCat = index == 0 ? "" : roster[index - 1].Id;
                    SyncValues();
                });
            }
            public void Remove(VisualElement card) => fields.RemoveAll(field => card.Contains(field));
            public void Refresh()
            {
                if (!IsCurrent()) return;
                bool changed = !RosterMatches();
                if (changed) ReadRoster();
                for (int index = fields.Count - 1; index >= 0; index--)
                {
                    var field = fields[index];
                    if (field.panel != ui.root.panel || !ui.content.Contains(field)) { fields.RemoveAt(index); continue; }
                    // Leave an unchanged popup, focus and scroll alone.
                    if (changed) field.choices = labels;
                }
                SyncValues();
            }
        }
        private void ResourceChoice(VisualElement parent) => Choice(parent, "Resource", Catalog.Resources.Where(r => r != "blessings"), selectedResource, x => selectedResource = x);
        private void Meter(VisualElement parent, string name, Func<double> read)
        {
            var bar = new ProgressBar { lowValue = 0, highValue = 100 }; parent.Add(bar);
            void Refresh() { double value = read(); bar.value = (float)value; bar.title = name + " " + value.ToString("0"); }
            Refresh(); liveUpdates.Add(Refresh);
        }
        private void WorkMeter(Func<double> progress, Func<double> required, VisualElement parent = null)
        {
            var bar = new ProgressBar { name = "live-work-progress", lowValue = 0 }; (parent ?? content).Add(bar);
            void Refresh()
            {
                double value = progress(), total = Math.Max(1, required());
                bar.highValue = (float)total; bar.value = (float)value;
                bar.title = "Work " + value.ToString("0.0") + " / " + total.ToString("0");
            }
            Refresh(); liveUpdates.Add(Refresh);
        }
        private void Separator() => Element(content, "separator");
        private void Text(string text, string cls = "body") => Label(content, text, cls);
        private static void TextInto(VisualElement parent, string text, string cls) => Label(parent, text, cls);
        private VisualElement Card(string title) { var card = Element(content, "card"); Label(card, title, "subtitle"); return card; }
        private static VisualElement Element(VisualElement parent, string classes) { var e = new VisualElement(); foreach (var cls in classes.Split(' ')) e.AddToClassList(cls); parent.Add(e); return e; }
        private static Label Label(VisualElement parent, string text, string cls) { var e = new Label(text); e.AddToClassList(cls); parent.Add(e); return e; }
        private static Button Button(VisualElement parent, string text, Action click, string tooltip = "") { var b = new Button(click) { text = text, tooltip = tooltip }; parent.Add(b); return b; }
        private static TextField Field(VisualElement parent, string label, string value) { var f = new TextField(label) { value = value }; parent.Add(f); return f; }
        private static void Number(VisualElement parent, string label, double value, Action<double> set) { var f = new DoubleField(label) { value = value }; f.RegisterValueChangedCallback(e => set(e.newValue)); parent.Add(f); }
        private static DropdownField Choice(VisualElement parent, string label, IEnumerable<string> options, string value, Action<string> changed)
        {
            var list = options.ToList(); if (list.Count == 0) list.Add("None available"); int index = Mathf.Max(0, list.IndexOf(value));
            var f = new DropdownField(label, list, index); changed(list[index]); f.RegisterValueChangedCallback(e => changed(e.newValue)); parent.Add(f); return f;
        }
        private static string Pretty(string name) => ForestGame.Describe(name);
        private static string SectionSummary(string section)
        {
            switch (section)
            {
                case "Village": return "Your home, neighbors and shared world";
                case "Cats": return "Every pair of paws has a purpose";
                case "Build": return "Plan a place, connect it, bring materials";
                case "Work": return "Direct today's work and staff workshops";
                case "Stores": return "Physical goods, counts and equipment";
                case "Research": return "New craft, buildings and ways to thrive";
                case "Officers": return "Give a specialist responsibility";
                case "Shrine": return "Offerings, blessings and rituals";
                case "World": return "Terrain, exploration and dungeon levels";
                case "Defense": return "Train and protect your village";
                case "Trade": return "Exchange goods with passing visitors";
                case "Routes": return "Move cargo by rail and water";
                case "Events": return "What has happened in your village";
                default: return "Needs, work and next actions";
            }
        }
        private static string HumanReason(string reason)
        {
            switch (reason)
            {
                case "": return "";
                case "blocked_route": case "route_blocked": return "No walkable route. Check the gate, roads and any water along the way.";
                case "foreign_territory": return "This site belongs to another village. Choose land within your village.";
                case "building_entrance_disconnected": return "The entrance is cut off. Connect its road back to the shrine to resume construction.";
                case "infrastructure_footprint_blocked": return "Road or rail work is paused. A building, stockpile or farm occupies the reserved route. Clear that footprint to resume.";
                case "expansion_footprint_blocked": return "Expansion is paused because the new fence would cross a building, entrance or stockpile. Its materials and completed fence sections remain reserved.";
                case "queue_empty": return "Ready for work. Add a recipe to this worker's queue.";
                case "queue_paused": return "Production is paused. Resume the queue when you are ready.";
                case "worker_required": return "Waiting for a worker. Assign an available cat.";
                case "recipe_locked": case "building_research_required": return "Research required. Open Research to unlock this work.";
                case "missing_input": case "missing_recipe_input": return "Waiting for recipe ingredients in a reachable stockpile.";
                case "missing_grain": return "Waiting for grain. Grow or gather grain and deliver it to storage.";
                case "need_supply_missing": return "Food or water is missing or unreachable. Check supplies and paths.";
                case "output_storage_full": case "output_storage_full_or_unreachable": return "Output needs space in a matching stockpile and a reachable route.";
                case "station_local_full": return "The workshop is full. Move finished goods into storage.";
                case "input_claimed": return "Another worker has reserved these ingredients. Waiting for more supplies.";
                case "input_unreachable": return "Recipe ingredients cannot reach this workshop. Check the stockpile access and workshop route.";
                case "missing_scaffold_input": case "wall_materials_missing": return "Construction is waiting for its reserved materials to arrive.";
                case "paving_input_missing": return "Road work is waiting for paving materials.";
                case "blocked_harvest_route": return "The harvest cannot reach storage. Clear a walkable route.";
                case "blocked_handoff_route": return "The field's collection point is unreachable. Check the path.";
                case "handoff_full": return "The field's collection point is full. Haul its goods to storage.";
                case "return_route_blocked": return "The return route is blocked. Restore a path for this cargo.";
                case "return_storage_full": case "storage_full": return "Destination storage is full. Free space or add a matching stockpile.";
                case "source_changed": return "The original supply source has changed. Check its remaining goods.";
                case "site_required": return "Choose a clear construction site connected to the village.";
                default: return Pretty(reason);
            }
        }
        private static string Goods(IEnumerable<Stack> goods, string format = "0.#") => goods.Any() ? string.Join(", ", goods.Select(s => s.Amount.ToString(format) + " " + s.Resource)) : "none";
        private void OnDestroy()
        {
            if (Game?.View != null) Game.View.TileClicked -= Place;
            if (document != null && document.panelSettings != null) Destroy(document.panelSettings);
        }
    }
}
