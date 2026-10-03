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
    /// The Glass drawer (GUIDE §5.6): a left glass sheet that overlays the map (left 20, top 82, bottom 96,
    /// width 660; Research opens wide at 1220 and dims the map 35 %). Its header holds the title, a summary
    /// computed from the colony and an "Esc" close. It builds the open section, runs its live updates every
    /// frame and calls its Refresh on the HUD's 0.5 s cadence.
    /// </summary>
    public sealed class DrawerHost
    {
        /// <summary>
        /// The former inspector page. It is no longer a section: the tethered card is the only inspector, and
        /// <see cref="ForestUI.OpenPanel"/> treats this id as "show the selected subject's card".
        /// </summary>
        public const string InspectId = "Inspect";

        public const string DimName = "drawer-dim";
        public const string TitleName = "drawer-title";
        public const string SummaryName = "drawer-summary";

        private readonly UiContext ctx;
        private readonly SectionRegistry registry;
        private readonly VisualElement drawer, content, dim;
        private readonly ScrollView scroll;
        private readonly Label title, subtitle;
        private readonly List<Action> liveUpdates = new List<Action>();
        private string renderedVillageId = "";
        private bool renderedRemote, headingOverridden;

        public DrawerHost(UiContext ctx, SectionRegistry registry, VisualElement parent)
        {
            this.ctx = ctx;
            this.registry = registry;
            dim = new VisualElement { name = DimName };
            dim.AddToClassList("drawer-dim");
            dim.style.display = DisplayStyle.None;
            dim.RegisterCallback<ClickEvent>(_ => ctx.Hud.ClosePanel());
            // The dim darkens the map only: it sits behind every slot, so the chrome stays bright.
            (parent.parent ?? parent).Insert(0, dim);
            drawer = UiKit.Element(parent, "drawer sheet glass2");
            drawer.name = UiNames.Drawer;
            drawer.style.display = DisplayStyle.None;
            var heading = UiKit.Element(drawer, "sheet-hd");
            title = UiKit.Label(heading, "", "sheet-title").Named(TitleName);
            subtitle = UiKit.Label(heading, "", "sheet-summary").Named(SummaryName);
            var close = UiKit.Button(heading, "Esc", ctx.Hud.ClosePanel).Named(UiNames.DrawerClose);
            close.AddToClassList("sheet-x");
            scroll = new ScrollView(ScrollViewMode.Vertical) { name = UiNames.DrawerScroll };
            scroll.AddToClassList("drawer-scroll");
            scroll.horizontalScrollerVisibility = ScrollerVisibility.Hidden;
            drawer.Add(scroll);
            content = scroll.contentContainer;
        }

        public VisualElement Element => drawer;
        public ScrollView Scroll => scroll;

        /// <summary>The dock entry shown as active.</summary>
        public string ActiveNavigationId => State.OpenSection;

        private UiState State => ctx.State;

        public void Open(string section)
        {
            if (registry.Find(section) == null) return;
            bool newPage = section != State.OpenSection;
            State.OpenSection = section;
            bool wide = registry.Find(section).Wide;
            drawer.style.display = DisplayStyle.Flex;
            drawer.EnableInClassList("wide", wide);
            dim.style.display = wide ? DisplayStyle.Flex : DisplayStyle.None;
            if (newPage) RenderAtTop();
            else Render();
        }

        public void Close()
        {
            State.OpenSection = "";
            liveUpdates.Clear();
            drawer.style.display = DisplayStyle.None;
            dim.style.display = DisplayStyle.None;
        }

        /// <summary>Placement hides the drawer while the player picks tiles; ForestUI restores it afterwards.</summary>
        public void HideForPlacement() => Close();

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
            headingOverridden = false;
            title.text = registry.Find(id)?.Label ?? id;
            subtitle.text = Summary(id);
            ctx.Hud.UpdateNavigation();
            renderedVillageId = village.Id;
            renderedRemote = ctx.Game.IsRemote;
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

        /// <summary>Lets a section replace the title and summary until the next render.</summary>
        public void SetHeading(string heading, string summary)
        {
            headingOverridden = true;
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

        /// <summary>
        /// The 0.5 s refresh: the header summary, then the section's own Refresh. The section refresh is
        /// skipped while the player types, holds the mouse button or has a dropdown menu open, because a
        /// legacy section rebuild would destroy the control in use.
        /// </summary>
        public void RefreshOpen()
        {
            var section = registry.Find(State.OpenSection);
            if (section == null) return;
            if (!headingOverridden)
            {
                string summary = Summary(section.Id);
                if (subtitle.text != summary) subtitle.text = summary;
            }
            if (ctx.Hud.TextInputFocused || Mouse.current?.leftButton.isPressed == true || DropdownOpen()) return;
            section.Refresh(ctx);
        }

        private bool DropdownOpen()
        {
            var tree = drawer.panel?.visualTree;
            return tree != null && tree.Q(className: "unity-base-dropdown") != null;
        }

        /// <summary>The dim header line of a section: counts read from the colony, never flavour text.</summary>
        public string Summary(string id) => SummaryOf(id, ctx.World, ctx.Village);

        public static string SummaryOf(string id, World world, Village v)
        {
            if (world == null || v == null) return "";
            var alive = v.Cats.Where(c => c.Alive).ToList();
            int working = alive.Count(VillageChip.IsWorking);
            string Join(params string[] parts) => string.Join(" · ", parts.Where(p => !string.IsNullOrEmpty(p)));
            switch (id)
            {
                case "Overview":
                    return Join(UiText.Plural(alive.Count, "cat", "cats"), working + " at work", UiText.Plural(v.Research.Count, "study", "studies"));
                case "Cats":
                    return Join(UiText.Plural(alive.Count, "cat", "cats"), working + " at work", VillageChip.IdleCount(world, v) + " idle");
                case "Village":
                    return Join(v.Name, UiText.Plural(v.Buildings.Count(b => b.Completed), "building", "buildings"));
                case "Build":
                    return Join(UiText.Plural(v.Buildings.Count(b => b.Completed), "building", "buildings"), v.Buildings.Count(b => !b.Completed) + " under construction");
                case "Work":
                {
                    var open = v.Jobs.Where(j => !j.Completed).ToList();
                    int blocked = open.Count(j => j.BlockedReason != "") + v.Buildings.Count(b => b.Completed && b.BlockedReason != "");
                    return Join(UiText.Plural(open.Count, "job", "jobs"), working + " at work", blocked + " blocked", open.Count(j => j.CatId == "") + " waiting");
                }
                case "Stores":
                {
                    var piles = v.Stockpiles.Where(p => p.Kind == "storage").ToList();
                    double counted = piles.Count == 0 ? -1 : piles.Max(p => p.CountedAt);
                    return Join(UiText.Plural(piles.Count, "stockpile", "stockpiles"), counted < 0 ? "not counted yet" : "counted " + ResourcePills.AgeText(world, counted) + " ago");
                }
                case "Research":
                    return Join(UiText.Plural(v.Research.Count, "study", "studies") + " done", ResourcePills.Whole(v.ResearchPoints) + " research");
                case "Officers":
                    return UiText.Plural(v.Officers.Count(o => !string.IsNullOrEmpty(o.CatId)), "officer", "officers") + " appointed";
                case "Shrine":
                    return UiText.Plural((int)Math.Floor(v.Blessings), "blessing", "blessings");
                case "World":
                    return Join(UiText.Plural(v.Known.Count, "known tile", "known tiles"), v.Cats.Count(c => c.Alive && c.Position.Level < 0) + " underground");
                case "Defense":
                    return v.Raids.Count == 0 ? "No raiders nearby" : UiText.Plural(v.Raids.Count, "raider", "raiders") + " nearby";
                case "Trade":
                    return v.Trader.Phase == "absent" ? "No trader in the clearing" : "A trader is visiting";
                case "Routes":
                    return Join(UiText.Plural(v.Routes.Count, "route", "routes"), UiText.Plural(v.Vehicles.Count, "vehicle", "vehicles"));
                case "Events":
                    return UiText.Plural(v.Events.Count, "entry", "entries");
                default:
                    return "";
            }
        }
    }
}
