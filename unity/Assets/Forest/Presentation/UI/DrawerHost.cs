using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The drawer frame: heading, Back and Close, and a scroll view that hosts one section. It builds the
    /// open section, runs its live updates every frame and calls its Refresh on the HUD's 0.5 s cadence.
    /// </summary>
    public sealed class DrawerHost
    {
        public const string InspectId = "Inspect";

        private readonly UiContext ctx;
        private readonly SectionRegistry registry;
        private readonly VisualElement drawer, content;
        private readonly ScrollView scroll;
        private readonly Label title, subtitle;
        private readonly Button back;
        private readonly List<Action> liveUpdates = new List<Action>();
        private string inspectOrigin = "Cats";
        private string renderedVillageId = "", renderedSubjectId = "";
        private bool renderedRemote;

        public DrawerHost(UiContext ctx, SectionRegistry registry, VisualElement parent)
        {
            this.ctx = ctx;
            this.registry = registry;
            drawer = UiKit.Element(parent, "drawer glass-panel interactive");
            drawer.name = UiNames.Drawer;
            drawer.style.display = DisplayStyle.None;
            var heading = UiKit.Element(drawer, "drawer-heading row");
            var titleGroup = UiKit.Element(heading, "drawer-title-group");
            title = UiKit.Label(titleGroup, "", "title");
            subtitle = UiKit.Label(titleGroup, "", "drawer-subtitle");
            back = UiKit.Button(heading, "Back", () => ctx.Hud.OpenPanel(inspectOrigin)).Named(UiNames.DrawerBack);
            back.AddToClassList("quiet");
            UiKit.Button(heading, "Close", ctx.Hud.ClosePanel).Named(UiNames.DrawerClose).AddToClassList("quiet");
            scroll = new ScrollView(ScrollViewMode.Vertical) { name = UiNames.DrawerScroll };
            scroll.AddToClassList("drawer-scroll");
            drawer.Add(scroll);
            content = scroll.contentContainer;
        }

        public VisualElement Element => drawer;
        public ScrollView Scroll => scroll;

        /// <summary>The navigation entry shown as active: the roster an inspector was opened from.</summary>
        public string ActiveNavigationId => State.OpenSection == InspectId ? inspectOrigin : State.OpenSection;

        private UiState State => ctx.State;
        private ForestView View => ctx.Game.View;
        private string SubjectId => View.SelectedCat?.Id ?? View.SelectedBuilding?.Id ?? "";

        public void Open(string section)
        {
            bool newPage = section != State.OpenSection || section == InspectId && SubjectId != renderedSubjectId;
            if (section == InspectId && State.OpenSection != InspectId)
                inspectOrigin = SectionRegistry.NavigationOrder.Contains(State.OpenSection) ? State.OpenSection : View.SelectedCat != null ? "Cats" : "Work";
            State.OpenSection = section;
            drawer.style.display = DisplayStyle.Flex;
            drawer.EnableInClassList("wide", registry.Find(section)?.Wide == true);
            if (newPage) RenderAtTop();
            else Render();
        }

        public void Close()
        {
            State.OpenSection = "";
            liveUpdates.Clear();
            drawer.style.display = DisplayStyle.None;
        }

        /// <summary>Placement hides the drawer while the player picks tiles.</summary>
        public void HideForPlacement()
        {
            drawer.style.display = DisplayStyle.None;
            State.OpenSection = "";
        }

        public void RenderAtTop()
        {
            Render();
            scroll.scrollOffset = Vector2.zero;
        }

        /// <summary>Rebuilds the open section from scratch.</summary>
        public void Render()
        {
            var village = ctx.Village;
            if (village == null) return;
            liveUpdates.Clear();
            content.Clear();
            string id = State.OpenSection;
            title.text = id;
            subtitle.text = UiText.SectionSummary(id);
            back.style.display = id == InspectId ? DisplayStyle.Flex : DisplayStyle.None;
            ctx.Hud.UpdateNavigation();
            renderedVillageId = village.Id;
            renderedRemote = ctx.Game.IsRemote;
            renderedSubjectId = SubjectId;
            var body = registry.Find(id)?.Build(ctx);
            if (body != null) content.Add(body);
        }

        /// <summary>Rebuilds the open section and keeps the player's scroll position.</summary>
        public void RebuildKeepingScroll()
        {
            var offset = scroll.scrollOffset;
            Render();
            scroll.scrollOffset = offset;
        }

        public void SetHeading(string heading, string summary)
        {
            title.text = heading;
            subtitle.text = summary;
        }

        public void Live(Action update) => liveUpdates.Add(update);

        /// <summary>Per frame: live values of the open section.</summary>
        public void TickLive()
        {
            if (State.OpenSection == "" || ctx.Village?.Id != renderedVillageId) return;
            foreach (var update in liveUpdates) update();
        }

        /// <summary>Re-renders when the selected village or the connection changed under the open section.</summary>
        public void RerenderIfStale()
        {
            if (State.OpenSection != "" && (renderedVillageId != ctx.Village.Id || renderedRemote != ctx.Game.IsRemote)) Render();
        }

        /// <summary>The 0.5 s refresh, skipped while the player types or holds the mouse button.</summary>
        public void RefreshOpen()
        {
            var section = registry.Find(State.OpenSection);
            if (section == null) return;
            if (ctx.Hud.TextInputFocused || Mouse.current?.leftButton.isPressed == true) return;
            section.Refresh(ctx);
        }
    }
}
