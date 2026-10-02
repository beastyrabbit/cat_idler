using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Management controls issue the same commands as connected players. This MonoBehaviour is the HUD
    /// shell: it owns the UIDocument, the legacy chrome (top bar, navigation, bottom bar), the mount
    /// slots, the drawer host and placement, and keeps the public facade ForestView and tools use.
    /// </summary>
    public sealed class ForestUI : MonoBehaviour
    {
        private const string LegacyHint = "WASD / right drag pan · wheel / + − zoom · click to inspect · Tab controls cat";
        private static readonly string[] ResourceChips = { "food", "water", "research", "blessings" };
        private static readonly float[] SpeedSteps = { 0f, 1f, 4f, 8f };

        public static readonly string[] Sections = SectionRegistry.NavigationOrder;
        public bool HasPlacement => placement != null && placement.Active;

        /// <summary>The occupied left edge in panel coordinates, for centering the camera in visible forest.</summary>
        public float WorldLeftFraction
        {
            get
            {
                if (root == null || navigation == null || root.worldBound.width <= 0) return 0;
                float right = navigation.worldBound.xMax;
                var drawerElement = drawer.Element;
                if (OpenSection != "" && drawerElement.resolvedStyle.display != DisplayStyle.None && !drawerElement.ClassListContains("wide"))
                    right = Mathf.Max(right, drawerElement.worldBound.xMax);
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
                while (hit != null)
                {
                    if (hit.ClassListContains("interactive")) return true;
                    hit = hit.parent;
                }
                return false;
            }
        }

        public bool TextInputFocused
        {
            get
            {
                var focused = root?.focusController?.focusedElement as VisualElement;
                for (var p = focused; p != null; p = p.parent)
                    if (p is TextField || p is DoubleField || p is IntegerField) return true;
                return false;
            }
        }

        /// <summary>The open drawer section ("" when closed).</summary>
        public string OpenSection => state.OpenSection;
        public UiState State => state;
        internal UiContext Context { get; private set; }
        internal DrawerHost Drawer => drawer;
        internal PlacementController Placement => placement;
        internal SectionRegistry Registry => registry;

        private readonly UiState state = new UiState();
        private readonly Dictionary<string, VisualElement> slots = new Dictionary<string, VisualElement>();
        private readonly Dictionary<string, Label> resourceLabels = new Dictionary<string, Label>();
        private readonly Dictionary<string, Button> navigationButtons = new Dictionary<string, Button>();
        private readonly Dictionary<float, Button> speedButtons = new Dictionary<float, Button>();
        private readonly List<IHudPart> parts = new List<IHudPart>();
        private ForestGame game;
        private SectionRegistry registry;
        private DrawerHost drawer;
        private PlacementController placement;
        private TetheredCard card;
        private UIDocument document;
        private VisualElement root, navigation;
        private Label headline, villageSummary, clockLabel, status, hint, feedback;
        private Button attentionSeal;
        private bool actionFailed;
        private float nextRefresh;

        private ForestGame Game => game;
        private Village Village => Game.Selected;

        private void Awake()
        {
            game = GetComponent<ForestGame>();
            document = gameObject.AddComponent<UIDocument>();
            var panel = Instantiate(Resources.Load<PanelSettings>("ForestPanel"));
            // Reflow a smaller window instead of shrinking every label below readable size.
            panel.scaleMode = PanelScaleMode.ConstantPhysicalSize;
            panel.referenceDpi = 110;
            panel.fallbackDpi = 96;
            document.panelSettings = panel;
            root = document.rootVisualElement;
            root.name = "forest-ui";
            root.pickingMode = PickingMode.Ignore;
            root.styleSheets.Add(Resources.Load<StyleSheet>("ForestUI"));
            root.RegisterCallback<GeometryChangedEvent>(OnGeometryChanged);
            Context = new UiContext(game, state, this);
            registry = SectionRegistry.CreateDefault();
            placement = new PlacementController(state, action => Act(action));
            state.CloseDrawer = ClosePanel;
            BuildTopBar();
            BuildNavigation();
            foreach (var slot in new[] { UiSlots.Chip, UiSlots.Seal, UiSlots.Pills, UiSlots.Dock, UiSlots.Hint, UiSlots.Drawer, UiSlots.Card }) CreateSlot(slot);
            drawer = new DrawerHost(Context, registry, slots[UiSlots.Drawer]);
            card = new TetheredCard(Context);
            slots[UiSlots.Card].Add(card);
            BuildBottomBar();
            foreach (var slot in new[] { UiSlots.Toasts, UiSlots.Mode, UiSlots.Reins, UiSlots.Objective, UiSlots.Modal }) CreateSlot(slot);
            MountStubs();
            Game.View.TileClicked += placement.Click;
        }

        private void OnGeometryChanged(GeometryChangedEvent e)
        {
            root.EnableInClassList("compact", e.newRect.width < 1040);
            root.EnableInClassList("narrow", e.newRect.width <= 650);
            root.EnableInClassList("micro", e.newRect.width <= 560);
            root.EnableInClassList("short", e.newRect.height < 620);
        }

        private void BuildTopBar()
        {
            var top = UiKit.Element(root, "top interactive").Named("hud-top");
            var brand = UiKit.Element(top, "brand").Named("hud-brand");
            UiKit.Label(brand, "IDLE CAT FOREST", "eyebrow");
            headline = UiKit.Label(brand, "Opening forest…", "headline");
            villageSummary = UiKit.Label(brand, "A home among the trees", "village-summary");
            var resources = UiKit.Element(top, "resources").Named("hud-resources");
            foreach (var resource in ResourceChips)
            {
                var chip = UiKit.Element(resources, "resource").Named("resource-" + resource);
                resourceLabels[resource] = UiKit.Label(chip, "—", "resource-value");
                UiKit.Label(chip, char.ToUpperInvariant(resource[0]) + resource.Substring(1), "resource-name");
                if (resource == "food" || resource == "water") chip.tooltip = "Last physical inventory count. Open Stores to see how fresh each report is.";
            }
            var clock = UiKit.Element(top, "clock").Named("hud-clock");
            clockLabel = UiKit.Label(clock, "Hour 0.0", "clock-label");
            var speeds = UiKit.Element(clock, "row");
            foreach (float speed in SpeedSteps)
            {
                float selected = speed;
                var button = UiKit.Button(speeds, speed == 0 ? "Ⅱ" : speed + "×", () => Game.SetSpeed(selected), speed == 0 ? "Pause local world" : "Local simulation speed");
                speedButtons[speed] = button.Named(UiNames.Act("speed", speed.ToString(System.Globalization.CultureInfo.InvariantCulture)));
            }
            attentionSeal = UiKit.Button(top, "✓", ToggleEvents, "Open attention").Named(UiNames.Seal);
            attentionSeal.AddToClassList("attention-seal");
        }

        private void ToggleEvents()
        {
            if (OpenSection == "Events") ClosePanel();
            else OpenPanel("Events");
        }

        private void BuildNavigation()
        {
            var nav = new ScrollView(ScrollViewMode.Vertical) { name = "hud-nav" };
            nav.AddToClassList("navigation");
            nav.AddToClassList("interactive");
            root.Add(nav);
            navigation = nav;
            nav.horizontalScrollerVisibility = ScrollerVisibility.Hidden;
            string group = null;
            foreach (var section in registry.Navigation)
            {
                string id = section.Id;
                if (section.Group != group)
                {
                    group = section.Group;
                    UiKit.Label(nav, group.ToUpperInvariant(), "nav-group");
                }
                var button = UiKit.Button(nav, section.Label, () =>
                {
                    if (OpenSection == id) ClosePanel();
                    else OpenPanel(id);
                }, UiText.SectionSummary(id)).Named(UiNames.Nav(id));
                button.AddToClassList("navigation-button");
                navigationButtons[id] = button;
            }
        }

        private void BuildBottomBar()
        {
            var bottom = UiKit.Element(root, "bottom interactive").Named("hud-bottom");
            var bottomCopy = UiKit.Element(bottom, "bottom-copy");
            hint = UiKit.Label(bottomCopy, "WASD / right drag pan · wheel zoom · click to inspect", "hint");
            feedback = UiKit.Label(bottomCopy, "", "feedback");
            status = UiKit.Label(bottomCopy, "", "status");
            var viewControls = UiKit.Element(bottom, "view-controls");
            UiKit.Button(viewControls, "−", () => Game.View.ZoomBy(-1), "Zoom out").Named(UiNames.Act("zoom", "out"));
            UiKit.Button(viewControls, "+", () => Game.View.ZoomBy(1), "Zoom in").Named(UiNames.Act("zoom", "in"));
            UiKit.Button(viewControls, "Return to village", () => Game.View.FocusVillage(), "Center the village · Home").Named(UiNames.Act("focus", "village"));
        }

        /// <summary>A full-size, non-picking overlay on the root; mounted elements position as root children.</summary>
        private void CreateSlot(string slotName)
        {
            var slot = new VisualElement { name = slotName, pickingMode = PickingMode.Ignore };
            slot.style.position = Position.Absolute;
            slot.style.left = 0;
            slot.style.top = 0;
            slot.style.right = 0;
            slot.style.bottom = 0;
            root.Add(slot);
            slots[slotName] = slot;
        }

        private void MountStubs()
        {
            Mount(UiSlots.Chip, new VillageChip(Context));
            Mount(UiSlots.Seal, new AttentionSeal(Context));
            Mount(UiSlots.Seal, new AttentionPopover(Context));
            Mount(UiSlots.Pills, new ResourcePills(Context));
            Mount(UiSlots.Dock, new Dock(Context));
            Mount(UiSlots.Hint, new HintLine(Context));
            Mount(UiSlots.Toasts, new Toasts(Context));
            Mount(UiSlots.Mode, new ModeChip(Context));
            Mount(UiSlots.Mode, new BuildTray(Context));
            Mount(UiSlots.Reins, new ReinsBar(Context));
        }

        /// <summary>Adds an element to a slot; an <see cref="IHudPart"/> is refreshed on the 0.5 s cadence.</summary>
        public void Mount(string slotName, VisualElement element)
        {
            slots[slotName].Add(element);
            if (element is IHudPart part) parts.Add(part);
        }

        public VisualElement Slot(string slotName) => slots.TryGetValue(slotName, out var slot) ? slot : null;

        private void Update()
        {
            Tick();
            if (Time.unscaledTime < nextRefresh) return;
            nextRefresh = Time.unscaledTime + .5f;
            RefreshHud();
        }

        /// <summary>The per-frame part of Update: mode class, tethered card and the open section's live values.</summary>
        internal void Tick()
        {
            root.EnableInClassList("direct-control", Game.View != null && Game.View.DirectControl);
            if (Village != null) card.Refresh();
            drawer.TickLive();
            SyncState();
        }

        /// <summary>The 0.5 s part of Update: chrome text, stale drawer and the open section's Refresh.</summary>
        internal void RefreshHud()
        {
            if (Village == null)
            {
                headline.text = Game.Status;
                return;
            }
            drawer.RerenderIfStale();
            int alive = Village.Cats.Count(c => c.Alive);
            int working = Village.Cats.Count(c => c.Alive && c.JobId != "");
            headline.text = Village.Name;
            villageSummary.text = alive + " cats · " + working + " at work · " + Village.Research.Count + " studies";
            resourceLabels["food"].text = UiText.Reported(Game.CurrentWorld, Village, "food", true);
            resourceLabels["water"].text = UiText.Reported(Game.CurrentWorld, Village, "water", true);
            resourceLabels["research"].text = Village.ResearchPoints.ToString("0");
            resourceLabels["blessings"].text = Village.Blessings.ToString("0");
            clockLabel.text = UiText.Clock(Game.CurrentWorld.TimeSeconds, Game.CurrentWorld.CurrentPacing().DayScale) + (Game.IsRemote ? " · shared" : Game.Speed == 0 ? " · paused" : "");
            foreach (var speed in speedButtons)
            {
                speed.Value.EnableInClassList("active", !Game.IsRemote && speed.Key == Game.Speed);
                speed.Value.SetEnabled(!Game.IsRemote);
            }
            status.text = Game.Status;
            feedback.text = UiText.HumanReason(Game.LastAction);
            feedback.style.display = Game.LastAction.Length == 0 ? DisplayStyle.None : DisplayStyle.Flex;
            feedback.EnableInClassList("failure", actionFailed);
            var attention = Village.Raids.Count + Village.Jobs.Count(j => !j.Completed && j.BlockedReason != "");
            attentionSeal.text = attention > 0 ? "⚠ " + Math.Min(99, attention) : "✓";
            attentionSeal.EnableInClassList("has-attention", attention > 0);
            root.EnableInClassList("has-feedback", Game.LastAction.Length > 0);
            hint.text = HasPlacement ? placement.Label + " · Esc cancels" : Game.View.DirectControl ? "Controlling " + Game.View.ControlledCat.Name + " · WASD walk · right drag look · E interact · Tab return" : LegacyHint;
            foreach (var part in parts) part.Refresh();
            drawer.RefreshOpen();
        }

        private void SyncState()
        {
            var view = Game.View;
            if (view == null) return;
            if (view.SelectedCatId != "") state.Selected = new Subject(SubjectKind.Cat, view.SelectedCatId);
            else if (view.SelectedBuildingId != "") state.Selected = new Subject(SubjectKind.Building, view.SelectedBuildingId);
            else if (state.Selected.Kind == SubjectKind.Cat || state.Selected.Kind == SubjectKind.Building) state.Selected = Subject.None;
            state.Mode = HasPlacement ? UiMode.Placement : view.DirectControl ? UiMode.Control : UiMode.None;
        }

        public void OpenPanel(string section) => drawer.Open(section);

        public void ClosePanel()
        {
            drawer.Close();
            placement.Cancel();
            UpdateNavigation();
        }

        /// <summary>Selects a subject the way a world click does today: cats and buildings open the inspector.</summary>
        public void Select(Subject s)
        {
            state.Selected = s;
            switch (s.Kind)
            {
                case SubjectKind.Cat:
                    Game.View.InspectCat(s.Id);
                    OpenPanel(DrawerHost.InspectId);
                    break;
                case SubjectKind.Building:
                    Game.View.InspectBuilding(s.Id);
                    OpenPanel(DrawerHost.InspectId);
                    break;
                case SubjectKind.None:
                    Game.View.InspectBuilding("");
                    break;
            }
        }

        internal void UpdateNavigation()
        {
            string active = drawer.ActiveNavigationId;
            foreach (var button in navigationButtons) button.Value.EnableInClassList("active", button.Key == active);
        }

        internal void BeginPlacement(string label, Func<Int2, GameAction> action, bool line = false)
        {
            placement.Begin(label, action, line);
            drawer.HideForPlacement();
            UpdateNavigation();
        }

        /// <summary>Sends an action, shows its result in the feedback line and re-renders the open section.</summary>
        internal async void Act(GameAction action, bool focus = false)
        {
            var result = await Game.Send(action);
            actionFailed = !result.Success;
            if (result.Success && focus) Game.View.FocusVillage();
            feedback.text = UiText.HumanReason(Game.LastAction);
            feedback.style.display = DisplayStyle.Flex;
            feedback.EnableInClassList("failure", actionFailed);
            root.EnableInClassList("has-feedback", true);
            if (OpenSection != "") drawer.Render();
        }

        private void OnDestroy()
        {
            if (Game?.View != null && placement != null) Game.View.TileClicked -= placement.Click;
            if (document != null && document.panelSettings != null) Destroy(document.panelSettings);
        }
    }
}
