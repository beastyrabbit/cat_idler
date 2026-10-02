using System;
using System.Threading.Tasks;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>What a section, card or chrome element needs: the game, the world, the HUD and its state.</summary>
    public sealed class UiContext
    {
        public UiContext(ForestGame game, UiState state, ForestUI hud)
        {
            Game = game;
            State = state;
            Hud = hud;
        }

        public ForestGame Game { get; }
        public World World => Game.CurrentWorld;
        public Village Village => Game.Selected;
        public UiState State { get; }
        public ForestUI Hud { get; }

        public Task<ActionResult> Send(GameAction a) => Game.Send(a);

        /// <summary>The selections the legacy sections share (cat, resource, amount, ...).</summary>
        internal SectionPicks Picks { get; } = new SectionPicks();

        internal DrawerHost Drawer => Hud.Drawer;

        /// <summary>Sends an action, shows its result in the feedback line and re-renders the open section.</summary>
        internal void Act(GameAction action, bool focus = false) => Hud.Act(action, focus);

        internal void BeginPlacement(string label, Func<Int2, GameAction> action, bool line = false) => Hud.BeginPlacement(label, action, line);

        /// <summary>Registers a per-frame update that runs while the current section stays open.</summary>
        internal void Live(Action update) => Hud.Drawer.Live(update);
    }
}
