using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>Management controls issue the same commands as connected players.</summary>
    public sealed class ForestUI : MonoBehaviour
    {
        public static readonly string[] Sections = { "Village", "Cats", "Build", "Work", "Stores", "Research", "Officers", "Shrine", "Defense", "Trade", "Routes", "Events" };
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
                while (hit != null) { if (hit.ClassListContains("interactive")) return true; hit = hit.parent; }
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
        private VisualElement root, drawer, content, navigation;
        private ScrollView scroll;
        private Label headline, villageSummary, clockLabel, status, hint, feedback, drawerTitle, drawerSubtitle;
        private Button backButton;
        private readonly Dictionary<string, Label> resourceLabels = new Dictionary<string, Label>();
        private readonly Dictionary<string, Button> navigationButtons = new Dictionary<string, Button>();
        private readonly Dictionary<float, Button> speedButtons = new Dictionary<float, Button>();
        private string inspectOrigin = "Cats";
        private bool actionFailed;
        private string query = "", selectedResource = "food", selectedRecipe = "", selectedCat = "", selectedPile = "", selectedOther = "", selectedMode = "rail";
        private double amount = 5, otherAmount = 5;
        private float nextRefresh;
        private Func<Int2, GameAction> placement;
        private string placementLabel = "";
        private Int2? firstPoint;
        private bool linePlacement;
        private string researchCategory = "All categories";
        private bool researchMap;
        private string renderedVillageId = "", renderedSubjectId = "", serverAddress = "ws://127.0.0.1:8788/ws";
        private bool renderedRemote;

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
                root.EnableInClassList("short", e.newRect.height < 620);
            });
            var top = Element(root, "top interactive");
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
            drawer = Element(root, "drawer interactive"); drawer.style.display = DisplayStyle.None;
            var heading = Element(drawer, "drawer-heading row");
            var titleGroup = Element(heading, "drawer-title-group"); drawerTitle = Label(titleGroup, "", "title"); drawerSubtitle = Label(titleGroup, "", "drawer-subtitle");
            backButton = Button(heading, "Back", () => OpenPanel(inspectOrigin)); backButton.AddToClassList("quiet");
            Button(heading, "Close", ClosePanel).AddToClassList("quiet");
            scroll = new ScrollView(ScrollViewMode.Vertical); scroll.AddToClassList("drawer-scroll"); drawer.Add(scroll); content = scroll.contentContainer;
            var bottom = Element(root, "bottom interactive"); var bottomCopy = Element(bottom, "bottom-copy");
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
            if (Time.unscaledTime < nextRefresh) return; nextRefresh = Time.unscaledTime + .5f;
            if (Village == null) { headline.text = Game.Status; return; }
            if (OpenSection != "" && (renderedVillageId != Village.Id || renderedRemote != Game.IsRemote)) RenderPanel();
            int alive = Village.Cats.Count(c => c.Alive), working = Village.Cats.Count(c => c.Alive && c.JobId != "");
            headline.text = Village.Name;
            villageSummary.text = alive + " cats · " + working + " at work · " + Village.Research.Count + " studies";
            resourceLabels["food"].text = Reported("food", true); resourceLabels["water"].text = Reported("water", true);
            resourceLabels["research"].text = Village.ResearchPoints.ToString("0"); resourceLabels["blessings"].text = Village.Blessings.ToString("0");
            clockLabel.text = "Hour " + (Game.CurrentWorld.TimeSeconds / 3600).ToString("0.0") + (Game.IsRemote ? " · shared" : Game.Speed == 0 ? " · paused" : "");
            foreach (var speed in speedButtons) { speed.Value.EnableInClassList("active", !Game.IsRemote && speed.Key == Game.Speed); speed.Value.SetEnabled(!Game.IsRemote); }
            status.text = Game.Status;
            feedback.text = HumanReason(Game.LastAction); feedback.style.display = Game.LastAction.Length == 0 ? DisplayStyle.None : DisplayStyle.Flex;
            feedback.EnableInClassList("failure", actionFailed);
            root.EnableInClassList("has-feedback", Game.LastAction.Length > 0);
            hint.text = HasPlacement ? placementLabel + " · Esc cancels" : Game.View.DirectControl ? "Controlling " + Game.View.ControlledCat.Name + " · WASD walk · right drag look · E interact · Tab return" : "WASD / right drag pan · wheel / + − zoom · click to inspect · Tab controls cat";
            if ((OpenSection == "Inspect" || OpenSection == "Cats" || OpenSection == "Stores" || OpenSection == "Events") && !TextInputFocused && Mouse.current?.leftButton.isPressed != true)
            { var offset = scroll.scrollOffset; RenderPanel(); scroll.scrollOffset = offset; }
        }

        private string Reported(string resource, bool compact = false)
        {
            bool any = Village.Stockpiles.Any(p => p.CountedAt >= 0);
            return any ? Game.CurrentWorld.ReportedTotal(Village, resource).ToString("0") + "*" : compact ? "—" : "uncounted";
        }
        public void OpenPanel(string section)
        {
            string subject = Game.View.SelectedCat?.Id ?? Game.View.SelectedBuilding?.Id ?? "";
            bool newPage = section != OpenSection || section == "Inspect" && subject != renderedSubjectId;
            if (section == "Inspect" && OpenSection != "Inspect") inspectOrigin = Sections.Contains(OpenSection) ? OpenSection : Game.View.SelectedCat != null ? "Cats" : "Work";
            OpenSection = section; drawer.style.display = DisplayStyle.Flex; drawer.EnableInClassList("wide", section == "Research");
            if (newPage) RenderPanelAtTop(); else RenderPanel();
        }
        public void ClosePanel() { OpenSection = ""; drawer.style.display = DisplayStyle.None; placement = null; firstPoint = null; UpdateNavigation(); }
        private void UpdateNavigation()
        {
            foreach (var button in navigationButtons) button.Value.EnableInClassList("active", button.Key == (OpenSection == "Inspect" ? inspectOrigin : OpenSection));
        }
        private void RenderPanelAtTop() { RenderPanel(); scroll.scrollOffset = Vector2.zero; }
        private void RenderPanel()
        {
            if (Village == null) return; content.Clear(); drawerTitle.text = OpenSection;
            drawerSubtitle.text = SectionSummary(OpenSection); backButton.style.display = OpenSection == "Inspect" ? DisplayStyle.Flex : DisplayStyle.None; UpdateNavigation();
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
                Label(card, Pretty(c.Goal) + (c.OfficerRole != "" ? " · " + Pretty(c.OfficerRole) : ""), "body");
                Label(card, "Hunger " + c.Hunger.ToString("0") + "  Thirst " + c.Thirst.ToString("0") + "  Rest " + c.Rest.ToString("0"), "muted");
                if (c.BlockedReason != "") Label(card, HumanReason(c.BlockedReason), "warning");
            }
        }
        private void InspectPanel()
        {
            var c = Game.View.SelectedCat; var b = Game.View.SelectedBuilding;
            if (c != null)
            {
                drawerTitle.text = c.Name; drawerSubtitle.text = c.Alive ? Pretty(c.Goal) + (c.OfficerRole == "" ? "" : " · " + Pretty(c.OfficerRole)) : "Dead";
                var needs = Element(content, "need-grid"); Meter(needs, "Health", c.Health); Meter(needs, "Hunger", c.Hunger); Meter(needs, "Thirst", c.Thirst); Meter(needs, "Rest", c.Rest);
                Text("Age " + c.AgeHours.ToString("0.0") + "h · " + c.Migration + " · " + (c.BedId == "" ? "No permanent bed" : "Permanent bed assigned"));
                if (c.PregnantUntil >= 0) Text("Kitten expected at hour " + (c.PregnantUntil / 3600).ToString("0.0"));
                if (c.BlockedReason != "") Text(HumanReason(c.BlockedReason), "warning");
                var job = Village.Jobs.Find(j => j.Id == c.JobId);
                if (job != null) { Text("Job: " + Pretty(job.Kind) + " · " + Pretty(job.Phase) + " · " + job.Progress.ToString("0") + " / " + job.RequiredWork.ToString("0") + " work"); if (job.BlockedReason != "" && job.BlockedReason != c.BlockedReason) Text(HumanReason(job.BlockedReason), "warning"); }
                var carriedItems = job != null && !job.Completed && job.Phase != "item_fetch" ? Village.Items.Where(item => item.LocationId == job.Id).ToList() : new List<Item>();
                if (c.Cargo.Count > 0 || carriedItems.Count == 0) Text("Cargo: " + Goods(c.Cargo));
                foreach (var item in carriedItems) Text("Carried item: " + Pretty(item.Material) + " " + item.Kind + " · " + item.Id + " · " + item.Condition.ToString("0") + " / " + item.MaxCondition.ToString("0") + " condition");
                if (c.ControlledBy == Game.PlayerId && c.ControlledBy != "")
                {
                    Button(content, "Return to management", Game.View.LeaveCat).AddToClassList("primary");
                    ResourceChoice(content); Number(content, "Amount", amount, x => amount = x);
                    foreach (var p in Village.Stockpiles.Where(p => Int2.Distance(p.Position, c.Position) <= 2)) { string id = p.Id; Button(content, (c.Cargo.Count > 0 ? "Deposit at " : "Take from ") + id, () => Act(new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = id, Resource = selectedResource, Amount = amount })); }
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
                foreach (var item in Village.Items.Where(i => i.LocationId == c.Id)) { string id = item.Id; Text(Pretty(item.Material) + " " + item.Kind + " · " + item.Condition.ToString("0") + "% condition"); Button(content, "Unequip " + item.Kind, () => Act(new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id })); }
                Button(content, "Release assigned workplace", () => Act(new GameAction { Kind = "AssignWorker", CatId = c.Id }));
                Button(content, "Boost cat", () => Act(new GameAction { Kind = "BoostCat", CatId = c.Id, Enabled = true }));
            }
            else if (b != null) BuildingPanel(b);
            else Text("Click a cat or a building in the forest, or choose a cat from the census.");
        }
        private void BuildingPanel(Building b)
        {
            drawerTitle.text = Pretty(b.Kind); drawerSubtitle.text = b.Width + " × " + b.Depth + " tiles · " + b.Position;
            Text(b.Completed ? "Completed workplace" : "Under construction · " + b.Progress.ToString("0") + " / " + b.RequiredWork.ToString("0"));
            Text("Inputs: " + Goods(b.Inputs) + "\nOutputs waiting: " + Goods(b.Outputs));
            if (b.BlockedReason != "") Text(HumanReason(b.BlockedReason), "warning");
            Label(content, "Staff this workplace", "subtitle"); CatChoice(content); Button(content, "Assign worker", () => Act(new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = selectedCat })).AddToClassList("primary");
            if (Catalog.Recipes.Any(r => r.Building == b.Kind))
            {
                Separator(); Label(content, "Production", "subtitle");
                var options = Catalog.Recipes.Where(r => r.Building == b.Kind).ToList(); if (!options.Any(r => r.Id == selectedRecipe)) selectedRecipe = options[0].Id;
                var recipeChoice = Choice(content, "Recipe", options.Select(r => r.Id), selectedRecipe, x => selectedRecipe = x);
                var ingredients = Label(content, "", "muted");
                void DescribeRecipe()
                {
                    var recipe = options.Find(r => r.Id == selectedRecipe);
                    if (recipe != null) ingredients.text = Goods(recipe.Inputs) + " → " + (recipe.ItemKind == "" ? Goods(recipe.Outputs) : Pretty(recipe.Material) + " " + recipe.ItemKind);
                }
                recipeChoice.RegisterValueChangedCallback(_ => DescribeRecipe()); DescribeRecipe();
                Text("Each worker follows one queue. Repeating recipes move to the back after each batch.", "muted");
                Button(content, "Add recipe to station queue", () => Act(new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = selectedRecipe }));
                if (b.Slots.Count == 0) QueuePanel(content, b, b.Queue, "", b.Paused);
                foreach (var slot in b.Slots)
                {
                    var position = Element(content, "work-position");
                    if (slot.CatId == "")
                    {
                        Label(position, "Vacant work position", "subtitle");
                        Label(position, "Assign a cat above to run this queue.", "muted");
                        if (b.Slots.IndexOf(slot) == 0) QueuePanel(position, b, slot.Queue, "", slot.Paused);
                        else foreach (var entry in slot.Queue) Label(position, Pretty(entry.RecipeId), "muted");
                        continue;
                    }
                    string id = slot.CatId; Label(position, "Work position · " + (Village.Cats.Find(c => c.Id == id)?.Name ?? id), "subtitle");
                    if (slot.BlockedReason != "") Label(position, HumanReason(slot.BlockedReason), "warning");
                    Button(position, "Add selected recipe to this cat's queue", () => Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = id, Edit = "add", RecipeId = selectedRecipe }));
                    QueuePanel(position, b, slot.Queue, id, slot.Paused);
                }
            }
        }
        private void QueuePanel(VisualElement parent, Building b, List<QueueEntry> queue, string catId, bool paused)
        {
            string kind = catId == "" ? "EditProductionQueue" : "EditProductionWorkSlot";
            Label(parent, (paused ? "Paused" : "Queue running") + " · " + queue.Count + " recipes", "muted");
            var row = Element(parent, "row"); Button(row, "Pause", () => Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = true })).EnableInClassList("active", paused); Button(row, "Resume", () => Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = false })).EnableInClassList("active", !paused);
            if (queue.Count == 0) Label(parent, "No recipes yet. Add the selected recipe to begin.", "muted");
            for (int i = 0; i < queue.Count; i++)
            {
                int index = i; var q = queue[i]; var card = Element(parent, "queue-entry"); Label(card, (i + 1) + ". " + Pretty(q.RecipeId), "subtitle"); row = Element(card, "row");
                void Edit(string edit, int direction = 0, bool enabled = true) => Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = edit, Index = index, Direction = direction, Enabled = enabled });
                Button(row, "↑", () => Edit("move", -1)); Button(row, "↓", () => Edit("move", 1)); Button(row, q.Repeat ? "Repeat on" : "Repeat off", () => Edit("repeat", 0, !q.Repeat)); Button(row, "Remove", () => Edit("remove"));
            }
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
            Button(content, "Mark gathering zone", () => Placement("Gathering zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "gather" }, true));
            Button(content, "Mark avoid zone", () => Placement("Avoid zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "avoid" }, true));
            foreach (var zone in Village.Stockpiles.Where(p => p.Kind.StartsWith("zone_", StringComparison.Ordinal))) { string id = zone.Id; Button(content, "Remove " + zone.Kind + " " + zone.Position, () => Act(new GameAction { Kind = "RemoveZone", TargetId = id })); }
        }
        private void WorkPanel()
        {
            Text("Manual jobs remain available while an officer's seat is vacant. Personal needs can interrupt work."); CatChoice(content, true);
            foreach (var labor in new[] { "hunt", "fetch_water", "woodcut", "quarry", "forage", "fish", "replant", "scout", "train" })
            {
                string name = labor;
                string job = name == "woodcut" ? "logs" : name == "forage" ? "fibre" : name;
                Button(content, Pretty(name), () => Act(new GameAction { Kind = name == "scout" ? "DispatchScout" : "RequestJob", Name = job, CatId = selectedCat }));
            }
            Button(content, "Haul goods · choose source", () => OpenPanel("Stores"));
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
            foreach (var crop in new[] { "grain", "catnip", "herbs" }) { string id = crop; Button(content, "Designate " + crop + " farm", () => Placement("Exterior " + id + " farm: choose opposite corners", p => new GameAction { Kind = "DesignateFarm", Name = id, Resource = id, Position = p, End = p }, true)); }
            ResourceChoice(content); Button(content, "Designate gathering", () => Placement("Gathering area: choose opposite corners on known exterior land", p => new GameAction { Kind = "DesignateGatherSpot", Position = p, Resource = selectedResource }, true));
            Button(content, "Designate fishing", () => Placement("Choose a known dry shore tile beside water", p => new GameAction { Kind = "DesignateFishingSpot", Position = p, Resource = "fish" }));
            foreach (var farm in Village.Farms) { var f = farm; var card = Card(Pretty(f.Crop) + " · " + Pretty(f.Phase)); if (f.BlockedReason != "") Label(card, HumanReason(f.BlockedReason), "warning"); Button(card, "Assign selected cat", () => Act(new GameAction { Kind = "AssignWorker", CatId = selectedCat, TargetId = f.Id })); Button(card, "Clear farm", () => Act(new GameAction { Kind = "ClearFarm", TargetId = f.Id })); }
            Separator(); Label(content, "Workplaces", "subtitle");
            foreach (var b in Village.Buildings) { string id = b.Id; Button(content, Pretty(b.Kind) + " · " + (b.WorkerId == "" ? "vacant" : "staffed"), () => { Game.View.InspectBuilding(id); OpenPanel("Inspect"); }); }
            foreach (var job in Village.Jobs.Where(j => !j.Completed))
            {
                string id = job.Id;
                Text(Pretty(job.Kind) + " · " + Pretty(job.Phase)); if (job.BlockedReason != "") Text(HumanReason(job.BlockedReason), "warning");
                Button(content, "Boost " + Pretty(job.Kind) + " · " + id, () => Act(new GameAction { Kind = "Boost", TargetId = id }));
            }
        }
        private void StoresPanel()
        {
            Text("* Inventory is the last physical count. An Accountant refreshes each pile by visiting it; stale counts are not current stock.");
            foreach (var resource in Catalog.Resources.Where(r => r != "blessings")) Label(content, Pretty(resource) + "   " + Reported(resource), "stock-row");
            Separator(); ResourceChoice(content); Button(content, "Designate general stockpile", () => Placement("Stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Name = "General" }, true));
            Button(content, "Designate selected-resource pile", () => Placement("Filtered stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Accepts = new List<string> { selectedResource } }, true));
            foreach (var p in Village.Stockpiles)
            {
                var pile = p; var card = Card(p.Kind + " · " + p.Id); Label(card, "Capacity " + p.Capacity.ToString("0") + " · " + (p.CountedAt < 0 ? "uncounted" : "counted at " + (p.CountedAt / 3600).ToString("0.0") + "h"), "muted");
                Label(card, Goods(p.Report), "body"); Button(card, "Haul from pile", () => Act(new GameAction { Kind = "HaulGatherSpot", TargetId = pile.Id })); Button(card, "Remove pile", () => Act(new GameAction { Kind = pile.Kind == "storage" ? "RemoveStockpile" : "RemoveGatherSpot", TargetId = pile.Id }));
            }
            Separator(); Label(content, "Equipment", "subtitle"); CatChoice(content);
            foreach (var item in Village.Items)
            {
                var i = item; var card = Card(Pretty(i.Material) + " " + i.Kind + " · quality " + i.Quality); Label(card, i.Condition.ToString("0") + " / " + i.MaxCondition.ToString("0") + " condition", "body");
                Button(card, "Equip selected cat", () => Act(new GameAction { Kind = "EquipItem", TargetId = i.Id, CatId = selectedCat })); Button(card, "Repair at staffed station", () => Act(new GameAction { Kind = "RepairItem", TargetId = i.Id }));
            }
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
                string role = Catalog.Roles[i]; var officer = Village.Officers.Find(o => o.Role == role); var card = Card(Pretty(role));
                Label(card, officer == null ? "Vacant · manual control" : Village.Cats.Find(c => c.Id == officer.CatId)?.Name ?? "Missing holder", "body");
                Label(card, "Requires " + Pretty(Catalog.RoleBuildings[i]) + " + " + Pretty(Catalog.RoleStudies[i]), "muted");
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
        private void DefensePanel()
        {
            Text("Train warriors and appoint a Captain to organize defense."); CatChoice(content); Button(content, "Train selected cat", () => Act(new GameAction { Kind = "TrainWarrior", CatId = selectedCat }));
            Button(content, "Defend active raid", () => Act(new GameAction { Kind = "DefendRaid" }));
            foreach (var raid in Village.Raids) Text("Raid · health " + raid.Health.ToString("0") + " · " + raid.Position);
            if (Village.Raids.Count == 0) Text("No active raids.");
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
            Text("Build the route infrastructure and a vehicle, then assign a cat and endpoints."); Choice(content, "Mode", new[] { "rail", "shipping" }, selectedMode, x => selectedMode = x);
            Button(content, "Lay rail", () => Placement("Rail: choose start, then end", p => new GameAction { Kind = "DesignateRail", Position = p }, true)); Button(content, "Build dock", () => Placement("Place dock on known dry shore beside water", p => new GameAction { Kind = "BuildDock", Position = p }));
            Button(content, "Build vehicle", () => Placement("Place vehicle at infrastructure", p => new GameAction { Kind = "BuildTransportVehicle", Position = p, Mode = selectedMode }));
            CatChoice(content); ResourceChoice(content); Number(content, "Cargo amount", amount, x => amount = x);
            var source = Choice(content, "Source pile", Village.Stockpiles.Select(p => p.Id), selectedPile, x => selectedPile = x);
            string destination = ""; Choice(content, "Destination pile", Village.Stockpiles.Select(p => p.Id), destination, x => destination = x);
            Text("An available vehicle of the chosen mode will be assigned by the colony.");
            Button(content, "Create transport route", () =>
            {
                var from = Village.Stockpiles.Find(p => p.Id == selectedPile); var to = Village.Stockpiles.Find(p => p.Id == destination);
                var path = from == null || to == null ? new List<Int2>() : TransportPath(from.Position, to.Position);
                Act(new GameAction { Kind = "CreateTransportRoute", Mode = selectedMode, CatId = selectedCat, TargetId = selectedPile, BuildingId = destination, Path = path, Resource = selectedResource, Amount = amount, Repeat = true });
            });
            foreach (var route in Village.Routes) { string id = route.Id; var card = Card(Pretty(route.Mode) + " · " + Pretty(route.Phase)); Label(card, Pretty(route.Resource), "body"); if (route.BlockedReason != "") Label(card, HumanReason(route.BlockedReason), "warning"); Button(card, "Cancel route", () => Act(new GameAction { Kind = "CancelTransportRoute", TargetId = id })); }
        }
        private void Placement(string label, Func<Int2, GameAction> action, bool line = false)
        {
            placement = action; placementLabel = label; firstPoint = null; linePlacement = line; drawer.style.display = DisplayStyle.None; OpenSection = ""; UpdateNavigation();
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
            if (linePlacement && firstPoint == null) { firstPoint = point; placementLabel = "Choose end of route"; return; }
            var action = placement(firstPoint ?? point); action.End = point;
            if (linePlacement)
            {
                var p = action.Position; action.Path.Add(p); int guard = 0;
                while (!p.Equals(point) && guard++ < 128) { p = p.X != point.X ? new Int2(p.X + Math.Sign(point.X - p.X), p.Z) : new Int2(p.X, p.Z + Math.Sign(point.Z - p.Z)); action.Path.Add(p); }
            }
            Act(action); placement = null; firstPoint = null;
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
        private void ResourceChoice(VisualElement parent) => Choice(parent, "Resource", Catalog.Resources.Where(r => r != "blessings"), selectedResource, x => selectedResource = x);
        private static void Meter(VisualElement parent, string name, double value) { var bar = new ProgressBar { title = name + " " + value.ToString("0"), value = (float)value, lowValue = 0, highValue = 100 }; parent.Add(bar); }
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
                case "output_storage_full": case "output_storage_full_or_unreachable": return "Finished goods are waiting. Make room in a matching stockpile and check its route.";
                case "station_local_full": return "The workshop is full. Move finished goods into storage.";
                case "input_claimed": return "Another worker has reserved these ingredients. Waiting for more supplies.";
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
        private static string Goods(IEnumerable<Stack> goods) => goods.Any() ? string.Join(", ", goods.Select(s => s.Amount.ToString("0.#") + " " + s.Resource)) : "none";
        private void OnDestroy()
        {
            if (Game?.View != null) Game.View.TileClicked -= Place;
            if (document != null && document.panelSettings != null) Destroy(document.panelSettings);
        }
    }
}
