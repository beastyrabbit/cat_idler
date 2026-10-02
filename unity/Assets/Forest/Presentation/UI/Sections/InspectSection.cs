using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The drawer inspector for the selected cat or building, with its staffing and production queues.</summary>
    public sealed class InspectSection : ISection
    {
        public string Id => DrawerHost.InspectId;
        public string Label => DrawerHost.InspectId;
        public string Group => "";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) => ctx.Drawer.RebuildKeepingScroll();

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var c = ctx.Game.View.SelectedCat;
            var b = ctx.Game.View.SelectedBuilding;
            if (c != null) BuildCat(ctx, root, c);
            else if (b != null) BuildBuilding(ctx, root, b);
            else UiKit.Text(root, "Click a cat or a building in the forest, or choose a cat from the census.");
            return root;
        }

        private static void BuildCat(UiContext ctx, VisualElement root, Cat c)
        {
            var game = ctx.Game;
            var village = ctx.Village;
            var picks = ctx.Picks;
            ctx.Drawer.SetHeading(c.Name, c.Alive ? UiText.Pretty(c.Goal) + (c.OfficerRole == "" ? "" : " · " + UiText.Pretty(c.OfficerRole)) : "Dead");
            var needs = UiKit.Element(root, "need-grid");
            UiKit.Meter(ctx, needs, "Health", () => ctx.Game.View.SelectedCat?.Health ?? 0);
            UiKit.Meter(ctx, needs, "Hunger", () => ctx.Game.View.SelectedCat?.Hunger ?? 0);
            UiKit.Meter(ctx, needs, "Thirst", () => ctx.Game.View.SelectedCat?.Thirst ?? 0);
            UiKit.Meter(ctx, needs, "Rest", () => ctx.Game.View.SelectedCat?.Rest ?? 0);
            UiKit.Text(root, "Age " + c.AgeHours.ToString("0.0") + "h · " + c.Migration + " · " + (c.BedId == "" ? "No permanent bed" : "Permanent bed assigned"));
            if (c.PregnantUntil >= 0) UiKit.Text(root, "Kitten expected at hour " + (c.PregnantUntil / 3600).ToString("0.0"));
            if (c.BlockedReason != "") UiKit.Text(root, UiText.HumanReason(c.BlockedReason), "warning");
            var job = village.Jobs.Find(j => j.Id == c.JobId);
            if (job != null)
            {
                UiKit.Text(root, "Job: " + UiText.Pretty(job.Kind) + " · " + UiText.Pretty(job.Phase));
                if (job.RequiredWork > 0)
                    UiKit.WorkMeter(ctx, root, () => ctx.Village.Jobs.Find(j => j.Id == job.Id)?.Progress ?? 0, () => ctx.Village.Jobs.Find(j => j.Id == job.Id)?.RequiredWork ?? 1);
                if (job.BlockedReason != "" && job.BlockedReason != c.BlockedReason) UiKit.Text(root, UiText.HumanReason(job.BlockedReason), "warning");
            }
            var carriedItems = job != null && !job.Completed && job.Phase != "item_fetch" ? village.Items.Where(item => item.LocationId == job.Id).ToList() : new List<Item>();
            if (c.Cargo.Count > 0 || carriedItems.Count == 0) UiKit.Text(root, "Cargo: " + UiText.Goods(c.Cargo));
            foreach (var item in carriedItems)
                UiKit.Text(root, "Carried item: " + UiText.Pretty(item.Material) + " " + item.Kind + " · " + item.Id + " · " + item.Condition.ToString("0") + " / " + item.MaxCondition.ToString("0") + " condition").Named("carried-item-" + item.Id);
            if (c.ControlledBy == game.PlayerId && c.ControlledBy != "")
            {
                UiKit.Button(root, "Return to management", game.View.LeaveCat).Named(UiNames.Act("return", c.Id)).AddToClassList("primary");
                if (c.Position.Level < 0) UiKit.Button(root, "Collect reached dungeon chest · E", () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id })).Named(UiNames.Act("collect-chest", c.Id));
                picks.ResourceChoice(root);
                picks.AmountField(root);
                foreach (var p in village.Stockpiles.Where(p => Int2.Distance(p.Position, c.Position) <= 2))
                {
                    string id = p.Id;
                    UiKit.Button(root, (c.Cargo.Count > 0 ? "Deposit at " : "Take from ") + id, () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = id, Resource = picks.SelectedResource, Amount = picks.Amount })).Named(UiNames.Act("pile", id));
                }
                UiKit.Button(root, "Interact with shrine", () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id })).Named(UiNames.Act("shrine", c.Id));
                UiKit.Button(root, "Eat carried food at shrine", () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "eat" })).Named(UiNames.Act("eat", c.Id));
                UiKit.Button(root, "Drink carried water at shrine", () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "drink" })).Named(UiNames.Act("drink", c.Id));
                UiKit.Button(root, "Rest at assigned Den", () => ctx.Act(new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "rest" })).Named(UiNames.Act("rest", c.Id));
            }
            else if (c.Alive) UiKit.Button(root, "Control this cat · Tab", game.View.EnterSelectedCat).Named(UiNames.Act("control", c.Id)).AddToClassList("primary");
            UiKit.Separator(root);
            UiKit.Label(root, "Work preferences", "subtitle");
            var preferences = UiKit.Element(root, "labor-grid");
            foreach (var labor in Catalog.Labors)
            {
                string id = labor;
                var toggle = new Toggle(UiText.Pretty(labor)) { value = c.Preferences.Contains(labor), name = UiNames.Act("labor", labor) };
                toggle.RegisterValueChangedCallback(e => ctx.Act(new GameAction { Kind = "SetCatLaborPreference", CatId = c.Id, Labor = id, Enabled = e.newValue }));
                preferences.Add(toggle);
            }
            UiKit.Text(root, "Skills: " + UiText.Goods(c.Skills));
            foreach (var item in village.Items.Where(i => i.LocationId == c.Id))
            {
                string id = item.Id;
                UiKit.Text(root, UiText.Pretty(item.Material) + " " + item.Kind + " · " + item.Condition.ToString("0") + "% condition");
                UiKit.Button(root, "Unequip " + item.Kind, () => ctx.Act(new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id })).Named(UiNames.Act("unequip", id));
            }
            UiKit.Button(root, "Release assigned workplace", () => ctx.Act(new GameAction { Kind = "AssignWorker", CatId = c.Id })).Named(UiNames.Act("release-workplace", c.Id));
            UiKit.Button(root, "Boost cat", () => ctx.Act(new GameAction { Kind = "BoostCat", CatId = c.Id, Enabled = true })).Named(UiNames.Act("boost-cat", c.Id));
        }

        private static void BuildBuilding(UiContext ctx, VisualElement root, Building b)
        {
            var village = ctx.Village;
            var picks = ctx.Picks;
            ctx.Drawer.SetHeading(UiText.Pretty(b.Kind), b.Width + " × " + b.Depth + " tiles · " + b.Position);
            UiKit.Text(root, b.Completed ? "Completed workplace" : "Under construction · " + b.Progress.ToString("0") + " / " + b.RequiredWork.ToString("0"));
            if (!b.Completed)
                UiKit.WorkMeter(ctx, root, () => ctx.Game.View.SelectedBuilding?.Progress ?? 0, () => ctx.Game.View.SelectedBuilding?.RequiredWork ?? 1);
            UiKit.Text(root, "Inputs: " + UiText.Goods(b.Inputs) + "\nOutputs waiting: " + UiText.Goods(b.Outputs));
            if (b.BlockedReason != "") UiKit.Text(root, UiText.HumanReason(b.BlockedReason), "warning");
            UiKit.Label(root, "Staff this workplace", "subtitle");
            picks.CatChoice(ctx, root);
            UiKit.Button(root, "Assign worker", () => ctx.Act(new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = picks.SelectedCat })).Named(UiNames.Act("assign-worker", b.Id)).AddToClassList("primary");
            if (!Catalog.Recipes.Any(r => r.Building == b.Kind)) return;
            UiKit.Separator(root);
            UiKit.Label(root, "Production", "subtitle");
            var options = Catalog.Recipes.Where(r => r.Building == b.Kind).ToList();
            if (!options.Any(r => r.Id == picks.SelectedRecipe)) picks.SelectedRecipe = options[0].Id;
            var recipeChoice = UiKit.Choice(root, "Recipe", options.Select(r => r.Id), picks.SelectedRecipe, x => picks.SelectedRecipe = x).Named("choice-recipe");
            var ingredients = UiKit.Label(root, "", "muted");
            void DescribeRecipe()
            {
                var recipe = options.Find(r => r.Id == picks.SelectedRecipe);
                if (recipe != null) ingredients.text = UiText.Goods(recipe.Inputs) + " → " + (recipe.ItemKind == "" ? UiText.Goods(recipe.Outputs) : UiText.Pretty(recipe.Material) + " " + recipe.ItemKind);
            }
            recipeChoice.RegisterValueChangedCallback(_ => DescribeRecipe());
            DescribeRecipe();
            UiKit.Text(root, "Each worker follows one queue. Repeating recipes move to the back after each batch.", "muted");
            UiKit.Button(root, "Add recipe to station queue", () => ctx.Act(new GameAction { Kind = "EditProductionQueue", BuildingId = b.Id, Edit = "add", RecipeId = picks.SelectedRecipe })).Named(UiNames.Act("queue-add", b.Id));
            if (b.Slots.Count == 0) QueuePanel(ctx, root, b, b.Queue, "", b.Paused);
            foreach (var slot in b.Slots)
            {
                var position = UiKit.Element(root, "work-position");
                if (slot.CatId == "")
                {
                    UiKit.Label(position, "Vacant work position", "subtitle");
                    UiKit.Label(position, "Assign a cat above to run this queue.", "muted");
                    if (b.Slots.IndexOf(slot) == 0) QueuePanel(ctx, position, b, slot.Queue, "", slot.Paused);
                    else foreach (var entry in slot.Queue) UiKit.Label(position, UiText.Pretty(entry.RecipeId), "muted");
                    continue;
                }
                string id = slot.CatId;
                UiKit.Label(position, "Work position · " + (village.Cats.Find(c => c.Id == id)?.Name ?? id), "subtitle");
                if (slot.BlockedReason != "") UiKit.Label(position, UiText.HumanReason(slot.BlockedReason), "warning");
                UiKit.Button(position, "Add selected recipe to this cat's queue", () => ctx.Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = id, Edit = "add", RecipeId = picks.SelectedRecipe })).Named(UiNames.Act("slot-add", b.Id, id));
                QueuePanel(ctx, position, b, slot.Queue, id, slot.Paused);
            }
        }

        /// <summary>
        /// A station or work-slot queue. Controls are named <c>act-queue-&lt;verb&gt;-&lt;building&gt;-&lt;cat&gt;[-&lt;index&gt;]</c>
        /// (cat empty for the station queue) and entries <c>queue-entry-&lt;building&gt;-&lt;cat&gt;-&lt;index&gt;</c>.
        /// </summary>
        private static void QueuePanel(UiContext ctx, VisualElement parent, Building b, List<QueueEntry> queue, string catId, bool paused)
        {
            string kind = catId == "" ? "EditProductionQueue" : "EditProductionWorkSlot";
            UiKit.Label(parent, (paused ? "Paused" : "Queue running") + " · " + queue.Count + " recipes", "muted");
            var row = UiKit.Element(parent, "row");
            UiKit.Button(row, "Pause", () => ctx.Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = true })).Named(UiNames.Act("queue-pause", b.Id, catId)).EnableInClassList("active", paused);
            UiKit.Button(row, "Resume", () => ctx.Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = "pause", Enabled = false })).Named(UiNames.Act("queue-resume", b.Id, catId)).EnableInClassList("active", !paused);
            if (queue.Count == 0) UiKit.Label(parent, "No recipes yet. Add the selected recipe to begin.", "muted");
            for (int i = 0; i < queue.Count; i++)
            {
                int index = i;
                var q = queue[i];
                string at = index.ToString(System.Globalization.CultureInfo.InvariantCulture);
                var card = UiKit.Element(parent, "queue-entry").Named("queue-entry-" + b.Id + "-" + catId + "-" + at);
                UiKit.Label(card, (i + 1) + ". " + UiText.Pretty(q.RecipeId), "subtitle");
                row = UiKit.Element(card, "row");
                void Edit(string edit, int direction = 0, bool enabled = true) => ctx.Act(new GameAction { Kind = kind, BuildingId = b.Id, CatId = catId, Edit = edit, Index = index, Direction = direction, Enabled = enabled });
                UiKit.Button(row, "↑", () => Edit("move", -1)).Named(UiNames.Act("queue-up", b.Id, catId, at));
                UiKit.Button(row, "↓", () => Edit("move", 1)).Named(UiNames.Act("queue-down", b.Id, catId, at));
                UiKit.Button(row, q.Repeat ? "Repeat on" : "Repeat off", () => Edit("repeat", 0, !q.Repeat)).Named(UiNames.Act("queue-repeat", b.Id, catId, at));
                UiKit.Button(row, "Remove", () => Edit("remove")).Named(UiNames.Act("queue-remove", b.Id, catId, at));
            }
        }
    }
}
