using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The roles a building card is drawn for.</summary>
    public enum BuildingRole { Construction, Housing, Storage, Workplace, Shrine, Other }

    /// <summary>
    /// The tethered card body for a building, by role (GUIDE 5.5 building variant, a key-value block):
    /// a construction site (materials delivered of required, builders, ETA, Help, Send a builder, Cancel asked
    /// inline), housing (beds and who sleeps there), storage (counted contents and capacity), a workplace (crew slots
    /// with names, the running recipe's progress, "Assign idle cat", Boost and an inline queue editor) and the shrine.
    /// Element names: <c>card-building</c>, <c>card-material-&lt;resource&gt;</c>, <c>live-build-progress</c>,
    /// <c>card-crew-&lt;slot&gt;</c>, <c>live-work-progress</c>, <c>act-assign-idle-&lt;b&gt;</c>, <c>act-card-boost-&lt;b&gt;</c>,
    /// <c>act-edit-queue-&lt;b&gt;</c>, <c>act-card-cancel-&lt;b&gt;</c> then <c>act-card-cancel-yes|no-&lt;b&gt;</c>,
    /// <c>act-open-shrine-card</c>, and the queue editor's <c>choice-recipe</c>, <c>act-queue-add-&lt;b&gt;</c>,
    /// <c>act-slot-add-&lt;b&gt;-&lt;cat&gt;</c>, <c>act-queue-pause|resume-&lt;b&gt;-&lt;cat&gt;</c>,
    /// <c>act-queue-up|down|repeat|remove-&lt;b&gt;-&lt;cat&gt;-&lt;index&gt;</c> and entries
    /// <c>queue-entry-&lt;b&gt;-&lt;cat&gt;-&lt;index&gt;</c> (cat empty for the station queue).
    /// </summary>
    public class BuildingCard : CardBody
    {
        private static readonly string[] Studies = { "research_hut", "school" };
        private static readonly string[] OtherWorkplaces = { "research_hut", "school", "barracks", "accounting_tent", "mouse_farm" };
        private ProgressBar progress;
        private Label eta;
        private string recipe = "";
        private double sampleProgress = -1, sampleTime, rate;

        public BuildingCard(UiContext ctx) : base(ctx, "building") { }

        public override bool Exists => Building != null;

        private Building Building => Village?.Buildings.Find(b => b.Id == SubjectId);

        protected override void OnBind()
        {
            recipe = "";
            sampleProgress = -1;
            rate = 0;
        }

        /// <summary>Recipe stations and the research, school, barracks, accounting and mouse-farm buildings take crew.</summary>
        public static bool IsWorkplace(Building b) => OtherWorkplaces.Contains(b.Kind) || Catalog.Recipes.Any(r => r.Building == b.Kind);

        public static int CrewSlots(Village v, Building b) => Math.Max(1, (int)Catalog.BuildingEffect(v, b.Kind, "worker_slots", 1));

        /// <summary>The labor a building's crew practise (used for skill matching and role names).</summary>
        public static string LaborOf(string kind)
        {
            if (Studies.Contains(kind)) return "research";
            if (kind == "barracks") return "train";
            if (kind == "accounting_tent") return "haul";
            if (kind == "mouse_farm" || kind == "field") return "farm";
            return Catalog.Recipes.FirstOrDefault(r => r.Building == kind)?.Labor ?? "process";
        }

        public static BuildingRole RoleOf(Building b)
        {
            if (!b.Completed) return BuildingRole.Construction;
            if (b.Kind == "den") return BuildingRole.Housing;
            if (b.Kind == "food_storage" || b.Kind == "water_bowl") return BuildingRole.Storage;
            if (b.Kind == "shrine") return BuildingRole.Shrine;
            return IsWorkplace(b) ? BuildingRole.Workplace : BuildingRole.Other;
        }

        /// <summary>
        /// The idle cat "Assign idle cat" sends: alive, of working age, not the Leader, not an officer, not controlled,
        /// without a job or workplace; the best skill in the building's labor wins, a stated preference breaks ties
        /// upward, then the nearest.
        /// </summary>
        public static Cat BestIdleCat(World w, Village v, Building b)
        {
            string labor = LaborOf(b.Kind);
            double minAge = w.CurrentPacing().WorkerMinAgeHours;
            return v.Cats
                .Where(c => c.Alive && c.Id != v.LeaderId && c.OfficerRole == "" && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "" && c.AgeHours >= minAge)
                .OrderByDescending(c => CatCard.Skill(c, labor) + (c.Preferences.Contains(labor) ? .5 : 0) + (c.Boosted ? .25 : 0))
                .ThenBy(c => Int2.Distance(c.Position, b.Position))
                .FirstOrDefault();
        }

        protected override void Describe()
        {
            var b = Building;
            var v = Village;
            Title = CardFormat.BuildingName(v, b);
            PortraitEntity = "building:" + b.Id;
            WhyIsProblem = b.BlockedReason != "";
            Why = b.BlockedReason != "" ? UiText.HumanReason(b.BlockedReason) : "";
            switch (RoleOf(b))
            {
                case BuildingRole.Construction:
                    Facts = "Building site · " + CardFormat.Whole(Fraction(b.Progress, b.RequiredWork) * 100) + "% built";
                    var builders = Builders(v, b);
                    bool missing = Missing(b).Any();
                    Now = builders.Count > 0 ? string.Join(" and ", builders.Select(c => c.Name)) + (builders.Count == 1 ? " is" : " are") + " building it"
                        : missing ? "Waiting for materials" : "Waiting for a builder";
                    if (Why == "" && builders.Count == 0) Why = missing ? "Haulers bring each material to the site first." : "Send a builder, or wait for one to come free.";
                    break;
                case BuildingRole.Housing:
                    int beds = World.DenCapacity(v), sleepers = v.Cats.Count(c => c.Alive && c.BedId == b.Id);
                    Facts = "Home · " + CardFormat.Count(beds, "bed", "beds");
                    Now = sleepers == 0 ? "Nobody sleeps here yet" : CardFormat.Count(sleepers, "cat sleeps", "cats sleep") + " here";
                    break;
                case BuildingRole.Storage:
                    var pile = Container(v, b);
                    Facts = "Storage · holds " + CardFormat.Whole(pile?.Capacity ?? 0);
                    Now = pile == null ? "Empty" : pile.CountedAt < 0 ? "Not counted yet" : "Counted " + CardFormat.Duration(World.TimeSeconds - pile.CountedAt) + " ago";
                    break;
                case BuildingRole.Workplace:
                    int crew = b.Slots.Count(s => s.CatId != ""), slots = CrewSlots(v, b);
                    Facts = "Workplace · " + CardFormat.Count(slots, "crew slot", "crew slots");
                    var worker = b.Slots.Select(s => v.Cats.Find(c => c.Id == s.CatId && c.Alive)).FirstOrDefault(c => c != null);
                    Now = worker == null ? "Nobody works here yet" : crew == 1 ? worker.Name + " works here" : worker.Name + " and " + CardFormat.Count(crew - 1, "other", "others") + " work here";
                    if (Why == "" && worker == null) Why = "Assign an idle cat to start.";
                    if (b.Paused) { Why = UiText.HumanReason(Blocked.QueuePaused); WhyIsProblem = true; }
                    break;
                case BuildingRole.Shrine:
                    Facts = "Shrine · " + CardFormat.Whole(v.Blessings) + " blessings";
                    Now = "Offerings here become blessings";
                    break;
                default:
                    Facts = "Village building";
                    Now = "Ready";
                    break;
            }
        }

        protected override string Shape()
        {
            var b = Building;
            var role = RoleOf(b);
            string shape = role + "|" + b.Kind;
            switch (role)
            {
                case BuildingRole.Construction:
                    shape += "|" + string.Join(",", b.Required.Select(s => s.Resource)) + "|" + string.Join(",", Builders(Village, b).Select(c => c.Id)) + "|" + (BestIdleCat(World, Village, b) != null);
                    break;
                case BuildingRole.Housing:
                    shape += "|" + string.Join(",", Village.Cats.Where(c => c.Alive && c.BedId == b.Id).Select(c => c.Id));
                    break;
                case BuildingRole.Storage:
                    var pile = Container(Village, b);
                    shape += "|" + (pile == null ? "" : pile.CountedAt + ":" + string.Join(",", pile.Report.Select(s => s.Resource + s.Amount.ToString("0", CultureInfo.InvariantCulture))));
                    break;
                case BuildingRole.Workplace:
                    shape += "|" + string.Join(",", b.Slots.Select(s => s.CatId + ":" + s.JobId + ":" + s.Paused + ":" + string.Join(";", s.Queue.Select(q => q.RecipeId + q.Repeat))))
                        + "|" + b.Paused + ":" + string.Join(";", b.Queue.Select(q => q.RecipeId + q.Repeat)) + "|" + (BestIdleCat(World, Village, b)?.Id ?? "") + "|" + recipe
                        + "|" + string.Join(",", Village.Jobs.Where(j => !j.Completed && j.TargetId == b.Id).Select(j => j.Id));
                    break;
                case BuildingRole.Shrine:
                    break;
            }
            return shape;
        }

        protected override void Build()
        {
            progress = null; eta = null;
            var b = Building;
            switch (RoleOf(b))
            {
                case BuildingRole.Construction: BuildConstruction(b); break;
                case BuildingRole.Housing: BuildHousing(b); break;
                case BuildingRole.Storage: BuildStorage(b); break;
                case BuildingRole.Workplace: BuildWorkplace(b); break;
                case BuildingRole.Shrine:
                    var kv = Kv();
                    Row(kv, "Blessings", CardFormat.Whole(Village.Blessings));
                    Row(kv, "Studies", CardFormat.Whole(Village.ResearchPoints) + " research points");
                    Plain(Actions(), "Open shrine", UiNames.Act("open-shrine", "card"), () => Context.Hud.OpenPanel("Shrine"), primary: true);
                    break;
                default:
                    if (b.BlockedReason == "") UiKit.Label(this, "Finished and in use.", "card-dim");
                    break;
            }
        }

        private void BuildConstruction(Building b)
        {
            progress = Progress(this, "live-build-progress");
            var list = UiKit.Element(this, "card-checklist");
            foreach (var need in b.Required)
            {
                double have = b.Inputs.Where(s => s.Resource == need.Resource).Sum(s => s.Amount);
                bool done = have + 1e-6 >= need.Amount;
                var row = UiKit.Element(list, "card-check").Named("card-material-" + need.Resource);
                row.EnableInClassList("card-check-done", done);
                UiKit.Label(row, done ? "✓" : "○", "card-check-mark");
                UiKit.Label(row, CardFormat.Resource(need.Resource), "card-check-name");
                UiKit.Label(row, CardFormat.Amount(Math.Min(have, need.Amount)) + " / " + CardFormat.Amount(need.Amount), "card-check-value");
            }
            var kv = Kv();
            var builders = Builders(Village, b);
            Row(kv, "Builders", builders.Count == 0 ? "None yet" : string.Join(", ", builders.Select(c => c.Name)));
            eta = Row(kv, "Done in", "—", "card-eta");
            var actions = Actions();
            var job = Village.Jobs.Find(j => !j.Completed && j.Kind == JobKind.Build && j.TargetId == b.Id);
            if (job != null)
                Act(actions, "Help", UiNames.Act("card-boost", b.Id), () => new GameAction { Kind = "Boost", TargetId = job.Id, BuildingId = b.Id }, primary: true).tooltip = "Lend a paw: the site goes up sooner";
            else
            {
                var idle = BestIdleCat(World, Village, b);
                var send = Act(actions, idle == null ? "No idle builder" : "Send " + idle.Name, UiNames.Act("assign-idle", b.Id), () => new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = BestIdleCat(World, Village, b)?.Id ?? "" }, primary: idle != null);
                send.SetEnabled(idle != null);
            }
            // CancelConstruction is W2-05's action (wired in W2-INT-a); it refunds what was delivered.
            Destructive(actions, "Cancel", "Cancel this site?", "card-cancel", b.Id, () => new GameAction { Kind = "CancelConstruction", BuildingId = b.Id, TargetId = b.Id });
        }

        private void BuildHousing(Building b)
        {
            var kv = Kv();
            int beds = World.DenCapacity(Village);
            var sleepers = Village.Cats.Where(c => c.Alive && c.BedId == b.Id).ToList();
            Row(kv, "Beds", CardFormat.Whole(sleepers.Count) + " of " + CardFormat.Whole(beds) + " taken", "card-beds");
            Row(kv, "Sleepers", sleepers.Count == 0 ? "Nobody yet" : string.Join(", ", sleepers.Take(6).Select(c => c.Name)) + (sleepers.Count > 6 ? " and " + (sleepers.Count - 6) + " more" : ""));
        }

        private void BuildStorage(Building b)
        {
            var kv = Kv();
            var pile = Container(Village, b);
            if (pile == null) { Row(kv, "Holds", "nothing yet"); return; }
            Row(kv, "Holds", pile.CountedAt < 0 ? "Not counted yet" : CardFormat.Goods(pile.Report), "card-contents");
            Row(kv, "Capacity", CardFormat.Whole(pile.Capacity));
            Row(kv, "Takes", pile.Accepts.Count == 0 ? "Anything" : string.Join(", ", pile.Accepts.Select(CardFormat.Resource)));
        }

        private void BuildWorkplace(Building b)
        {
            var v = Village;
            var kv = Kv();
            int slots = CrewSlots(v, b);
            for (int i = 0; i < slots; i++)
            {
                var slot = i < b.Slots.Count ? b.Slots[i] : null;
                var cat = slot == null ? null : v.Cats.Find(c => c.Id == slot.CatId);
                Row(kv, "Crew " + (i + 1).ToString(CultureInfo.InvariantCulture), cat == null ? "Vacant" : cat.Name, "card-crew-" + i.ToString(CultureInfo.InvariantCulture), cat == null);
            }
            var running = RunningJob(v, b);
            if (running != null && running.RequiredWork > 0)
            {
                Row(kv, "Making", running.RecipeId != "" ? CardFormat.Sentence(UiText.Name(NameKind.Recipe, running.RecipeId)) : CardFormat.Sentence(UiText.Name(NameKind.Job, running.Kind)));
                progress = Progress(this, UiNames.WorkProgress);
            }
            else if (Studies.Contains(b.Kind) && b.Slots.Any(s => s.CatId != ""))
                Row(kv, "Making", "Research points · " + CardFormat.Whole(v.ResearchPoints) + " so far");
            if (b.Inputs.Count > 0 || b.Outputs.Count > 0)
                Row(kv, "On site", CardFormat.Goods(b.Inputs) + (b.Outputs.Count > 0 ? " · ready: " + CardFormat.Goods(b.Outputs) : ""));

            var actions = Actions();
            var idle = BestIdleCat(World, v, b);
            bool vacancy = b.Slots.Count(s => s.CatId != "") < slots;
            if (vacancy)
            {
                var assign = Act(actions, idle == null ? "No idle cat" : "Assign " + idle.Name, UiNames.Act("assign-idle", b.Id), () => new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = BestIdleCat(World, Village, b)?.Id ?? "" }, primary: idle != null);
                assign.SetEnabled(idle != null);
                assign.tooltip = "Sends the idle cat best at this work";
            }
            var boostJob = running ?? v.Jobs.Find(j => !j.Completed && j.TargetId == b.Id);
            if (boostJob != null)
                Act(actions, "Help", UiNames.Act("card-boost", b.Id), () => new GameAction { Kind = "Boost", TargetId = boostJob.Id, BuildingId = b.Id }, primary: !vacancy).tooltip = "Lend a paw: the current batch finishes sooner";
            bool recipes = Catalog.Recipes.Any(r => r.Building == b.Kind);
            if (recipes) Plain(actions, MoreOpen ? "Close queue" : "Edit queue", UiNames.Act("edit-queue", b.Id), ToggleMore);
            if (recipes) BuildQueue(b);
        }

        /// <summary>The inline queue editor (GUIDE: "Edit queue"), shown inside the card's expandable area.</summary>
        private void BuildQueue(Building b)
        {
            var more = MoreArea();
            var options = Catalog.Recipes.Where(r => r.Building == b.Kind && Catalog.RecipeAvailable(Village, r)).ToList();
            if (options.Count == 0)
            {
                UiKit.Label(more, "No recipe is unlocked here yet. Research opens them.", "card-dim");
                return;
            }
            if (!options.Any(r => r.Id == recipe)) recipe = options[0].Id;
            var names = options.Select(r => CardFormat.Sentence(UiText.Name(NameKind.Recipe, r.Id))).ToList();
            var choice = new DropdownField("Recipe", names, options.FindIndex(r => r.Id == recipe)) { name = "choice-recipe" };
            choice.AddToClassList("card-choice");
            choice.RegisterValueChangedCallback(e => { int at = names.IndexOf(e.newValue); if (at >= 0) recipe = options[at].Id; });
            more.Add(choice);
            var selected = options.Find(r => r.Id == recipe);
            UiKit.Label(more, CardFormat.Goods(selected.Inputs) + " → " + (selected.ItemKind == "" ? CardFormat.Goods(selected.Outputs) : CardFormat.Sentence(UiText.Name(NameKind.Resource, selected.Material) + " " + UiText.Name(NameKind.Item, selected.ItemKind))), "card-dim");
            if (b.Slots.Count == 0)
            {
                Act(more, "Add to the queue", UiNames.Act("queue-add", b.Id), () => new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe }).AddToClassList("card-row-act");
                Queue(more, b, b.Queue, "", b.Paused);
                return;
            }
            for (int i = 0; i < b.Slots.Count; i++)
            {
                var slot = b.Slots[i];
                var cat = Village.Cats.Find(c => c.Id == slot.CatId);
                UiKit.Label(more, cat == null ? "Vacant crew slot" : cat.Name + "'s queue", "card-subtitle");
                if (slot.CatId == "")
                {
                    // Only the first slot's queue is the station queue; a vacant later slot cannot route edits.
                    if (i == 0)
                    {
                        Act(more, "Add to the queue", UiNames.Act("queue-add", b.Id), () => new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = recipe }).AddToClassList("card-row-act");
                        Queue(more, b, slot.Queue, "", slot.Paused);
                    }
                    else foreach (var entry in slot.Queue) UiKit.Label(more, CardFormat.Sentence(UiText.Name(NameKind.Recipe, entry.RecipeId)), "card-dim");
                    continue;
                }
                string id = slot.CatId;
                if (slot.BlockedReason != "" && slot.BlockedReason != b.BlockedReason) UiKit.Label(more, UiText.HumanReason(slot.BlockedReason), "card-problem");
                Act(more, "Add to " + (cat?.Name ?? "this") + "'s queue", UiNames.Act("slot-add", b.Id, id), () => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = id, Edit = "add", RecipeId = recipe }).AddToClassList("card-row-act");
                Queue(more, b, slot.Queue, id, slot.Paused);
            }
        }

        private void Queue(VisualElement parent, Building b, List<QueueEntry> queue, string catId, bool paused)
        {
            string kind = catId == "" ? "EditProductionQueue" : "EditProductionWorkSlot";
            var row = UiKit.Element(parent, "card-line");
            UiKit.Label(row, (paused ? "Paused" : "Running") + " · " + CardFormat.Count(queue.Count, "recipe", "recipes"), "card-line-text");
            if (paused) Act(row, "Resume", UiNames.Act("queue-resume", b.Id, catId), () => new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = false }).AddToClassList("card-small");
            else Act(row, "Pause", UiNames.Act("queue-pause", b.Id, catId), () => new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = true }).AddToClassList("card-small");
            if (queue.Count == 0) UiKit.Label(parent, "Nothing queued. Add a recipe to begin.", "card-dim");
            for (int i = 0; i < queue.Count; i++)
            {
                int index = i;
                var entry = queue[i];
                string at = index.ToString(CultureInfo.InvariantCulture);
                var line = UiKit.Element(parent, "card-line card-queue-entry").Named("queue-entry-" + b.Id + "-" + catId + "-" + at);
                UiKit.Label(line, (i + 1).ToString(CultureInfo.InvariantCulture) + ". " + CardFormat.Sentence(UiText.Name(NameKind.Recipe, entry.RecipeId)), "card-line-text");
                GameAction Edit(string edit, int direction = 0, bool enabled = true) => new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = edit, Index = index, Direction = direction, Enabled = enabled };
                Act(line, "↑", UiNames.Act("queue-up", b.Id, catId, at), () => Edit("move", -1)).AddToClassList("card-small");
                Act(line, "↓", UiNames.Act("queue-down", b.Id, catId, at), () => Edit("move", 1)).AddToClassList("card-small");
                Act(line, entry.Repeat ? "Repeat ✓" : "Once", UiNames.Act("queue-repeat", b.Id, catId, at), () => Edit("repeat", 0, !entry.Repeat)).AddToClassList("card-small");
                Act(line, "✕", UiNames.Act("queue-remove", b.Id, catId, at), () => Edit("remove")).AddToClassList("card-small");
            }
        }

        protected override void Update()
        {
            var b = Building;
            var role = RoleOf(b);
            if (role == BuildingRole.Construction)
            {
                SetProgress(progress, b.Progress, b.RequiredWork, CardFormat.Whole(Fraction(b.Progress, b.RequiredWork) * 100) + "% built");
                if (eta != null) TetheredCard.SetText(eta, Eta(b));
            }
            else if (role == BuildingRole.Workplace && progress != null)
            {
                var job = RunningJob(Village, b);
                if (job != null) SetProgress(progress, job.Progress, job.RequiredWork, CardFormat.Whole(Fraction(job.Progress, job.RequiredWork) * 100) + "%");
            }
        }

        /// <summary>Time left from the observed build rate, in real seconds at the current game speed.</summary>
        private string Eta(Building b)
        {
            double time = World.TimeSeconds;
            if (sampleProgress < 0 || b.Progress < sampleProgress) { sampleProgress = b.Progress; sampleTime = time; }
            else if (time - sampleTime >= 2)
            {
                double observed = (b.Progress - sampleProgress) / (time - sampleTime);
                rate = rate <= 0 ? observed : rate * .6 + observed * .4;
                sampleProgress = b.Progress; sampleTime = time;
            }
            if (Builders(Village, b).Count == 0) return Missing(b).Any() ? "Waiting for materials" : "Waiting for a builder";
            if (rate <= 1e-6) return "Working it out…";
            float speed = Math.Max(1, Context.Game.Speed);
            return "About " + CardFormat.Duration((b.RequiredWork - b.Progress) / rate / speed);
        }

        private static double Fraction(double progress, double total) => total <= 0 ? 0 : Math.Max(0, Math.Min(1, progress / total));

        private static IEnumerable<Stack> Missing(Building b) =>
            b.Required.Where(need => b.Inputs.Where(s => s.Resource == need.Resource).Sum(s => s.Amount) + 1e-6 < need.Amount);

        private static List<Cat> Builders(Village v, Building b) =>
            v.Jobs.Where(j => !j.Completed && j.Kind == JobKind.Build && j.TargetId == b.Id && j.CatId != "")
                .Select(j => v.Cats.Find(c => c.Id == j.CatId && c.Alive)).Where(c => c != null).Distinct().ToList();

        private static Stockpile Container(Village v, Building b) => v.Stockpiles.Find(p => p.ManagedBy == b.Id);

        /// <summary>The crew's running station job (a recipe batch or delivery), if any.</summary>
        private static Job RunningJob(Village v, Building b)
        {
            foreach (var slot in b.Slots)
            {
                var job = v.Jobs.Find(j => !j.Completed && (j.Id == slot.JobId || j.CatId == slot.CatId && j.TargetId == b.Id));
                if (job != null) return job;
            }
            return null;
        }
    }
}
