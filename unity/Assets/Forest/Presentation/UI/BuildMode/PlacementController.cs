using System;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The placement state machine: a single tile, or for line placements a first and a last tile with
    /// the L-shaped path between them. A finished placement is sent through the HUD's action path.
    /// </summary>
    public sealed class PlacementController
    {
        /// <summary>Longest line a placement rasterises, in steps after the first tile.</summary>
        public const int MaxLineSteps = 128;

        private readonly UiState state;
        private readonly Action<GameAction> send;
        private Func<Int2, GameAction> placement;
        private Int2? firstPoint;
        private bool line;

        public PlacementController(UiState state, Action<GameAction> send)
        {
            this.state = state;
            this.send = send;
        }

        public bool Active => placement != null;

        /// <summary>What the hint line shows while placing.</summary>
        public string Label { get; private set; } = "";

        public void Begin(string label, Func<Int2, GameAction> action, bool linePlacement = false)
        {
            placement = action;
            Label = label;
            firstPoint = null;
            line = linePlacement;
            state.Mode = UiMode.Placement;
        }

        public void Cancel()
        {
            placement = null;
            firstPoint = null;
            if (state.Mode == UiMode.Placement) state.Mode = UiMode.None;
        }

        /// <summary>A world tile click while placing (ForestView.TileClicked).</summary>
        public void Click(Int2 point)
        {
            if (placement == null) return;
            if (line && firstPoint == null)
            {
                firstPoint = point;
                Label = "Choose end of route";
                return;
            }
            var action = placement(firstPoint ?? point);
            action.End = point;
            if (line) Rasterise(action, point);
            send(action);
            Cancel();
        }

        /// <summary>Appends the L-shaped path from the action's position to <paramref name="end"/>: X first, then Z.</summary>
        public static void Rasterise(GameAction action, Int2 end)
        {
            var p = action.Position;
            action.Path.Add(p);
            int guard = 0;
            while (!p.Equals(end) && guard++ < MaxLineSteps)
            {
                p = p.X != end.X ? new Int2(p.X + Math.Sign(end.X - p.X), p.Z) : new Int2(p.X, p.Z + Math.Sign(end.Z - p.Z));
                action.Path.Add(p);
            }
        }
    }
}
