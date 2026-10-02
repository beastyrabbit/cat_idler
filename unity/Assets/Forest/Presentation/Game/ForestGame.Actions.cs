using System;
using System.Threading.Tasks;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>Player actions routed to the local or remote authority.</summary>
    public sealed partial class ForestGame
    {
        public async Task<ActionResult> Send(GameAction action)
        {
            if (!ready) return ActionResult.Fail("World is still opening.");
            try
            {
                ActionResult result = remote != null ? await remote.SendAsync(action) : local.Apply(action);
                if (!result.Success || action.Kind != "KeepCatControl")
                    LastAction = result.Success ? Describe(action.Kind) + " accepted" : result.Error;
                if (result.Success) Revision++;
                return result;
            }
            catch (Exception e) { LastAction = "Action failed: " + e.Message; return ActionResult.Fail(LastAction); }
        }

        public void Act(GameAction action) { _ = Send(action); }

        public static string Describe(string text)
        {
            if (string.IsNullOrEmpty(text)) return "";
            return System.Text.RegularExpressions.Regex.Replace(text.Replace('_', ' '), "(?<=[a-z])(?=[A-Z])", " ");
        }
    }
}
