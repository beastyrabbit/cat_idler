using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class RoutesSection : ISection
    {
        private static readonly string[] Modes = { "rail", "shipping" };

        public string Id => "Routes";
        public string Label => "Routes";
        public string Group => "World";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "Build the route infrastructure and a vehicle, then assign a cat and endpoints.");
            UiKit.Choice(root, "Mode", Modes, picks.SelectedMode, x => picks.SelectedMode = x).Named("choice-route-mode");
            UiKit.Button(root, "Lay rail", () => ctx.BeginPlacement("Rail: choose start, then end", p => new GameAction { Kind = "DesignateRail", Position = p }, true)).Named(UiNames.Placement("rail"));
            UiKit.Button(root, "Build dock", () => ctx.BeginPlacement("Place dock on known dry shore beside water", p => new GameAction { Kind = "BuildDock", Position = p })).Named(UiNames.Placement("dock"));
            UiKit.Button(root, "Build vehicle", () => ctx.BeginPlacement("Place vehicle at infrastructure", p => new GameAction { Kind = "BuildTransportVehicle", Position = p, Mode = picks.SelectedMode })).Named(UiNames.Placement("vehicle"));
            picks.CatChoice(ctx, root);
            picks.ResourceChoice(root);
            picks.AmountField(root, "Cargo amount");
            UiKit.Choice(root, "Source pile", village.Stockpiles.Select(p => p.Id), picks.SelectedPile, x => picks.SelectedPile = x).Named("choice-source-pile");
            string destination = "";
            UiKit.Choice(root, "Destination pile", village.Stockpiles.Select(p => p.Id), destination, x => destination = x).Named("choice-destination-pile");
            UiKit.Text(root, "An available vehicle of the chosen mode will be assigned by the colony.");
            UiKit.Button(root, "Create transport route", () =>
            {
                var from = ctx.Village.Stockpiles.Find(p => p.Id == picks.SelectedPile);
                var to = ctx.Village.Stockpiles.Find(p => p.Id == destination);
                var path = from == null || to == null ? new List<Int2>() : TransportPath(ctx.World, picks.SelectedMode, from.Position, to.Position);
                ctx.Act(new GameAction { Kind = "CreateTransportRoute", Mode = picks.SelectedMode, CatId = picks.SelectedCat, TargetId = picks.SelectedPile, BuildingId = destination, Path = path, Resource = picks.SelectedResource, Amount = picks.Amount, Repeat = true });
            }).Named(UiNames.Act("create", "route"));
            foreach (var route in village.Routes)
            {
                string id = route.Id;
                var card = UiKit.Card(root, UiText.Pretty(route.Mode) + " · " + UiText.Pretty(route.Phase));
                UiKit.Label(card, UiText.Pretty(route.Resource), "body");
                if (route.BlockedReason != "") UiKit.Label(card, UiText.HumanReason(route.BlockedReason), "warning");
                UiKit.Button(card, "Cancel route", () => ctx.Act(new GameAction { Kind = "CancelTransportRoute", TargetId = id })).Named(UiNames.Act("cancel-route", id));
            }
            return root;
        }

        /// <summary>A breadth-first route over rail tiles, or over water ending at the destination.</summary>
        private static List<Int2> TransportPath(World world, string mode, Int2 start, Int2 end)
        {
            var tiles = world.Tiles.ToDictionary(t => t.Position);
            var parents = new Dictionary<Int2, Int2>();
            var queue = new Queue<Int2>();
            queue.Enqueue(start);
            parents[start] = start;
            while (queue.Count > 0 && parents.Count < 4096)
            {
                var point = queue.Dequeue();
                if (point.Equals(end))
                {
                    var path = new List<Int2>();
                    while (!point.Equals(start))
                    {
                        path.Add(point);
                        point = parents[point];
                    }
                    path.Add(start);
                    path.Reverse();
                    return path;
                }
                foreach (var step in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                {
                    var p = new Int2(point.X + step.X, point.Z + step.Z);
                    if (parents.ContainsKey(p) || !tiles.TryGetValue(p, out var tile)) continue;
                    if (mode == "rail" ? !tile.Rail : (!tile.Water && !p.Equals(end))) continue;
                    parents[p] = point;
                    queue.Enqueue(p);
                }
            }
            return new List<Int2>();
        }
    }
}
