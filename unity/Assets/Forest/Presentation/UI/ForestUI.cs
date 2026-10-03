using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Management controls issue the same commands as connected players. This MonoBehaviour is the HUD
    /// shell of the Glass Clearing chrome: it owns the UIDocument and its scale, the breakpoint classes,
    /// the mount slots, the drawer host, placement and the HUD's input bindings, and keeps the public facade
    /// ForestView and tools use. The chrome itself (chip, pills, dock, hint) lives in UI/Chrome.
    /// </summary>
    public sealed class ForestUI : MonoBehaviour
    {
        /// <summary>GUIDE §4 breakpoints in panel points: below this the layout reflows to narrow.</summary>
        public const float NarrowBelow = 1100;
        /// <summary>From this width the 1920 reference sizes and the minimap slot apply.</summary>
        public const float MinimapFrom = 1600;
        /// <summary>Screen.dpi from which the display is treated as a 2x (Retina) backing store.</summary>
        public const float RetinaDpi = 160;

        /// <summary>Style sheets added to the HUD root, in cascade order. Missing ones (other packages) are skipped.</summary>
        public static readonly string[] StyleSheets = { "UI/Tokens", "ForestUI", "UI/Chrome", "UI/Drawer", "UI/Widgets", "UI/Card", "UI/Attention", "UI/Toasts" };

        public static readonly string[] Sections = SectionRegistry.NavigationOrder;
        public bool HasPlacement => placement != null && placement.Active;

        /// <summary>
        /// The occupied left edge as a fraction of the panel width, for centering the camera in visible forest.
        /// Glass drawers overlay the map, so the camera never shifts for them: always 0.
        /// </summary>
        public float WorldLeftFraction => 0;

        /// <summary>True when the pointer is over any HUD element (including dropdown popups), not the bare map.</summary>
        public bool PointerOverPanel
        {
            get
            {
                if (root?.panel == null || Mouse.current == null) return false;
                var mouse = Mouse.current.position.ReadValue();
                var point = RuntimePanelUtils.ScreenToPanel(root.panel, new Vector2(mouse.x, Screen.height - mouse.y));
                return IsHudElement(root.panel.Pick(point));
            }
        }

        /// <summary>A picked element is HUD unless it is nothing, the HUD root or one of the root's ancestors.</summary>
        internal bool IsHudElement(VisualElement hit)
        {
            if (hit == null) return false;
            for (VisualElement a = root; a != null; a = a.parent)
                if (hit == a) return false;
            return true;
        }

        public bool TextInputFocused
        {
            get
            {
                var focused = root?.focusController?.focusedElement as VisualElement;
                return IsTextInput(focused);
            }
        }

        /// <summary>The open drawer section ("" when closed).</summary>
        public string OpenSection => state.OpenSection;
        public UiState State => state;
        /// <summary>The narrow layout (panel width below <see cref="NarrowBelow"/>) is active.</summary>
        public bool IsNarrow => root != null && root.ClassListContains("narrow");
        /// <summary>The chip's panel-space bounds, so the attention seal can sit 8 points to its right.</summary>
        public Rect ChipBounds => chip != null ? chip.worldBound : Rect.zero;
        /// <summary>The panel scale in use: backing factor times the player's interface size.</summary>
        public float PanelScale => document != null && document.panelSettings != null ? document.panelSettings.scale : 1;
        /// <summary>When the player's last action result arrived (Time.unscaledTime) and whether it failed.</summary>
        internal float LastActionAt { get; private set; } = -100;
        internal bool LastActionFailed { get; private set; }
        internal UiContext Context { get; private set; }
        internal DrawerHost Drawer => drawer;
        internal PlacementController Placement => placement;
        internal SectionRegistry Registry => registry;
        internal VisualElement Root => root;

        private readonly UiState state = new UiState();
        private readonly Dictionary<string, VisualElement> slots = new Dictionary<string, VisualElement>();
        private readonly List<IHudPart> parts = new List<IHudPart>();
        private ForestGame game;
        private SectionRegistry registry;
        private DrawerHost drawer;
        private PlacementController placement;
        private TetheredCard card;
        private VillageChip chip;
        private UIDocument document;
        private VisualElement root;
        private IDisposable inputBindings;
        private InputRouter boundRouter, seenRouter;
        private IPanel navigationPanel;
        private float appliedDpi = -1, appliedUiScale = -1;
        private float nextRefresh;
        private bool wasPlacing, wasPointerDown, wasControl;
        private string restoreAfterPlacement = "", restoreAfterControl = "", lastActionSeen = "";

        private ForestGame Game => game;
        private Village Village => Game.Selected;

        private void Awake()
        {
            game = GetComponent<ForestGame>();
            document = gameObject.AddComponent<UIDocument>();
            var panel = Instantiate(Resources.Load<PanelSettings>("ForestPanel"));
            panel.scaleMode = PanelScaleMode.ConstantPixelSize;
            document.panelSettings = panel;
            ApplyScale();
            root = document.rootVisualElement;
            root.name = "forest-ui";
            root.pickingMode = PickingMode.Ignore;
            foreach (var sheet in StyleSheets)
            {
                var loaded = Resources.Load<StyleSheet>(sheet);
                if (loaded != null) root.styleSheets.Add(loaded);
            }
            root.RegisterCallback<GeometryChangedEvent>(OnGeometryChanged);
            root.RegisterCallback<FocusInEvent>(OnFocusIn, TrickleDown.TrickleDown);
            root.RegisterCallback<FocusOutEvent>(OnFocusOut, TrickleDown.TrickleDown);
            Context = new UiContext(game, state, this);
            registry = SectionRegistry.CreateDefault();
            placement = new PlacementController(state, action => Act(action));
            state.CloseDrawer = ClosePanel;
            state.CancelPlacement = CancelPlacement;
            state.Deselect = () => Select(Subject.None);
            state.BlurTextField = BlurFocus;
            state.TextFieldFocused = () => TextInputFocused;
            // Back to front per GUIDE §2: chrome, drawer, card, seal and popover, then toasts, modes and modals. The dock
            // sits above the drawer so its active label stays readable over a wide drawer.
            foreach (var slot in new[] { UiSlots.Chip, UiSlots.Pills, UiSlots.Hint, UiSlots.Drawer, UiSlots.Dock, UiSlots.Card, UiSlots.Seal, UiSlots.Toasts, UiSlots.Mode, UiSlots.Reins, UiSlots.Objective, UiSlots.Modal }) CreateSlot(slot);
            drawer = new DrawerHost(Context, registry, slots[UiSlots.Drawer]);
            card = new TetheredCard(Context);
            slots[UiSlots.Card].Add(card);
            MountChrome();
            ApplySettingsClasses();
            GameSettings.Changed += OnSettingsChanged;
            Game.View.TileClicked += placement.Click;
            seenRouter = InputRouter.Current;
            BindInput(seenRouter);
        }

        /// <summary>
        /// ConstantPixelSize with scale = backing factor × the player's interface size: 2 on a Retina display
        /// (Screen.dpi ≥ 160), else 1. The single source of truth for the panel scale.
        /// </summary>
        public static float PanelScaleFor(float dpi, float uiScale)
        {
            float backing = dpi >= RetinaDpi ? 2 : 1;
            float user = float.IsNaN(uiScale) || uiScale <= 0 ? 1 : uiScale;
            return backing * user;
        }

        private void ApplyScale()
        {
            var panel = document.panelSettings;
            if (panel == null || panel.targetTexture != null) return;
            float dpi = Screen.dpi, uiScale = GameSettings.UiScale;
            if (dpi == appliedDpi && uiScale == appliedUiScale) return;
            appliedDpi = dpi;
            appliedUiScale = uiScale;
            panel.scale = PanelScaleFor(dpi, uiScale);
        }

        private void OnSettingsChanged()
        {
            ApplyScale();
            ApplySettingsClasses();
        }

        private void ApplySettingsClasses()
        {
            if (root == null) return;
            root.EnableInClassList("reduced-motion", GameSettings.ReducedMotion);
            root.EnableInClassList("larger-text", GameSettings.LargerText);
        }

        /// <summary>GUIDE §4 columns: reference (≥ 1600, minimap slot), compact (1100 to 1599) and narrow (below 1100).</summary>
        private void OnGeometryChanged(GeometryChangedEvent e) => ApplyBreakpoints(e.newRect.width);

        internal void ApplyBreakpoints(float width)
        {
            if (root == null || float.IsNaN(width) || width <= 0) return;
            root.EnableInClassList("has-minimap", width >= MinimapFrom);
            root.EnableInClassList("compact", width < MinimapFrom);
            root.EnableInClassList("narrow", width < NarrowBelow);
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

        private void MountChrome()
        {
            chip = new VillageChip(Context);
            Mount(UiSlots.Chip, chip);
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

        /// <summary>
        /// Binds the HUD's keys on a router: the Escape levels from <see cref="UiState"/> and the chip's pause
        /// and speed keys. Rebinding to another router releases the previous bindings.
        /// </summary>
        internal void BindInput(InputRouter router)
        {
            if (router == boundRouter) return;
            inputBindings?.Dispose();
            inputBindings = null;
            boundRouter = router;
            if (router == null) return;
            var bindings = new UiState.Bindings();
            bindings.Add(state.Bind(router));
            bindings.Add(chip.Bind(router));
            inputBindings = bindings;
        }

        private void Update()
        {
            ApplyScale();
            // Follow a new view router; a router bound explicitly (tests, tools) stays bound until then.
            if (InputRouter.Current != seenRouter)
            {
                seenRouter = InputRouter.Current;
                BindInput(seenRouter);
            }
            Tick();
            if (Time.unscaledTime < nextRefresh) return;
            nextRefresh = Time.unscaledTime + .5f;
            RefreshHud();
        }

        /// <summary>The per-frame part of Update: mode class, focus, placement exit, tethered card and live values.</summary>
        internal void Tick()
        {
            HookPanelNavigation();
            bool control = Game.View != null && Game.View.DirectControl;
            root.EnableInClassList("direct-control", control);
            if (control != wasControl) ControlModeChanged(control);
            BlurOnWorldPress();
            TrackLastAction();
            if (Village != null) card.Refresh();
            drawer.TickLive();
            SyncState();
            bool placing = HasPlacement;
            root.EnableInClassList("placing", placing);
            if (wasPlacing && !placing) RestoreAfterPlacement();
            wasPlacing = placing;
        }

        /// <summary>Direct control is a mode (GUIDE §6): it closes the drawer and brings it back afterwards.</summary>
        private void ControlModeChanged(bool control)
        {
            wasControl = control;
            if (control)
            {
                restoreAfterControl = state.OpenSection;
                if (restoreAfterControl != "") drawer.Close();
            }
            else
            {
                string section = restoreAfterControl;
                restoreAfterControl = "";
                if (section != "" && state.OpenSection == "") drawer.Open(section);
            }
            UpdateNavigation();
        }

        /// <summary>The 0.5 s part of Update: chrome parts, stale drawer and the open section's Refresh.</summary>
        internal void RefreshHud()
        {
            if (Village == null)
            {
                foreach (var part in parts) part.Refresh();
                return;
            }
            drawer.RerenderIfStale();
            foreach (var part in parts) part.Refresh();
            drawer.RefreshOpen();
        }

        private void TrackLastAction()
        {
            string last = Game.LastAction;
            if (last == lastActionSeen) return;
            lastActionSeen = last;
            if (last.Length > 0) LastActionAt = Time.unscaledTime;
        }

        private void SyncState()
        {
            var view = Game.View;
            if (view == null) return;
            if (view.SelectedCatId != "") state.Selected = new Subject(SubjectKind.Cat, view.SelectedCatId);
            else if (view.SelectedBuildingId != "") state.Selected = new Subject(SubjectKind.Building, view.SelectedBuildingId);
            else if (view.SelectedPileId != "") state.Selected = new Subject(SubjectKind.Pile, view.SelectedPileId);
            else if (state.Selected.Kind == SubjectKind.Cat || state.Selected.Kind == SubjectKind.Building || state.Selected.Kind == SubjectKind.Pile) state.Selected = Subject.None;
            state.Mode = HasPlacement ? UiMode.Placement : view.DirectControl ? UiMode.Control : UiMode.None;
        }

        /// <summary>
        /// Opens a section's drawer (replacing any open one and leaving a placement). The former "Inspect" page
        /// is not a drawer any more: asking for it keeps the open drawer and shows the selected subject's card.
        /// </summary>
        public void OpenPanel(string section)
        {
            if (section == DrawerHost.InspectId)
            {
                SyncState();
                return;
            }
            // The attention popover is not a drawer: tools and captures open it by this name (W2-16).
            if (section == AttentionPopover.PanelId)
            {
                state.PopoverOpen = true;
                return;
            }
            if (registry.Find(section) == null) return;
            if (HasPlacement)
            {
                restoreAfterPlacement = "";
                placement.Cancel();
            }
            drawer.Open(section);
            UpdateNavigation();
        }

        public void ClosePanel()
        {
            restoreAfterPlacement = "";
            drawer.Close();
            placement.Cancel();
            UpdateNavigation();
        }

        /// <summary>Selects a subject the way a world click does: the tethered card shows it; drawers stay as they are.</summary>
        public void Select(Subject s)
        {
            state.Selected = s;
            switch (s.Kind)
            {
                case SubjectKind.Cat:
                    Game.View.InspectCat(s.Id);
                    break;
                case SubjectKind.Building:
                    Game.View.InspectBuilding(s.Id);
                    break;
                case SubjectKind.Pile:
                    Game.View.InspectPile(s.Id);
                    break;
                case SubjectKind.None:
                    Game.View.InspectBuilding("");
                    break;
            }
        }

        internal void UpdateNavigation()
        {
            foreach (var part in parts)
                if (part is Dock dock) dock.Refresh();
        }

        internal void BeginPlacement(string label, Func<Int2, GameAction> action, bool line = false)
        {
            // Placement is a mode: the drawer steps aside and comes back when the placement ends (GUIDE §6).
            string previous = state.OpenSection;
            placement.Begin(label, action, line);
            drawer.HideForPlacement();
            restoreAfterPlacement = previous;
            wasPlacing = true;
            UpdateNavigation();
        }

        /// <summary>Esc while placing: leave the mode and bring back the drawer it hid.</summary>
        internal void CancelPlacement()
        {
            placement.Cancel();
            RestoreAfterPlacement();
        }

        private void RestoreAfterPlacement()
        {
            wasPlacing = false;
            string section = restoreAfterPlacement;
            restoreAfterPlacement = "";
            if (section != "" && state.OpenSection == "") drawer.Open(section);
            UpdateNavigation();
        }

        /// <summary>Sends an action; its result shows in the hint line and the open section re-renders.</summary>
        internal async void Act(GameAction action, bool focus = false)
        {
            var result = await Game.Send(action);
            LastActionFailed = !result.Success;
            LastActionAt = Time.unscaledTime;
            lastActionSeen = Game.LastAction;
            if (result.Success && focus) Game.View.GoHome();
            if (OpenSection != "") drawer.Render();
            foreach (var part in parts)
                if (part is HintLine hint) hint.Refresh();
        }

        // ---- Focus and keyboard navigation -------------------------------------------------------------

        private static bool IsTextInput(VisualElement element)
        {
            for (var p = element; p != null; p = p.parent)
                if (p is TextField || p is DoubleField || p is IntegerField || p is FloatField || p is LongField) return true;
            return false;
        }

        private void OnFocusIn(FocusInEvent e)
        {
            var target = e.target as VisualElement;
            if (IsTextInput(target))
            {
                SetRouterTextFocus(true);
                return;
            }
            // Buttons never keep keyboard focus: WASD and arrows must not walk across them and Enter must not
            // re-fire the last clicked one (game-code-2).
            if (target is Button button)
            {
                button.focusable = false;
                button.schedule.Execute(() => { if (button.focusController?.focusedElement == button) button.Blur(); });
            }
        }

        private void OnFocusOut(FocusOutEvent e)
        {
            if (IsTextInput(e.target as VisualElement) && !IsTextInput(e.relatedTarget as VisualElement)) SetRouterTextFocus(false);
        }

        private static void SetRouterTextFocus(bool focused)
        {
            if (InputRouter.Current != null) InputRouter.Current.TextFieldFocused = focused;
        }

        private void BlurFocus()
        {
            (root?.focusController?.focusedElement as Focusable)?.Blur();
            SetRouterTextFocus(false);
        }

        /// <summary>A press on the bare map takes keyboard focus away from any HUD field.</summary>
        private void BlurOnWorldPress()
        {
            var router = InputRouter.Current;
            bool down = router != null && router.DevicesPresent && (router.PrimaryHeld || router.SecondaryHeld);
            if (down && !wasPointerDown && root.focusController?.focusedElement != null && !PointerOverPanel) BlurFocus();
            wasPointerDown = down;
        }

        /// <summary>Keyboard navigation events (Tab, arrows, WASD through UI/Navigate) never move HUD focus.</summary>
        private void HookPanelNavigation()
        {
            var panel = root.panel;
            if (panel == null || panel == navigationPanel) return;
            navigationPanel = panel;
            panel.visualTree.RegisterCallback<NavigationMoveEvent>(SwallowNavigation, TrickleDown.TrickleDown);
        }

        private void SwallowNavigation(NavigationMoveEvent e)
        {
            root.focusController?.IgnoreEvent(e);
            e.StopPropagation();
        }

        private void OnDestroy()
        {
            GameSettings.Changed -= OnSettingsChanged;
            inputBindings?.Dispose();
            navigationPanel?.visualTree.UnregisterCallback<NavigationMoveEvent>(SwallowNavigation, TrickleDown.TrickleDown);
            if (Game?.View != null && placement != null) Game.View.TileClicked -= placement.Click;
            if (document != null && document.panelSettings != null) Destroy(document.panelSettings);
        }
    }
}
