using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Player actions routed to the local or remote authority, and the feedback they publish.</summary>
    public sealed partial class ForestGame
    {
        /// <summary>
        /// The legacy one-line result of the last player action (English for successes, the simulation's
        /// reason or a code for failures). New UI reads <see cref="LastFeedback"/> instead.
        /// </summary>
        public string LastAction { get; private set; } = "";

        /// <summary>The structured result of the last action that publishes feedback; null before the first one.</summary>
        public Feedback LastFeedback { get; private set; }

        /// <summary>Raised on the main thread whenever <see cref="LastFeedback"/> changes.</summary>
        public event Action<Feedback> FeedbackPosted;

        /// <summary>Test seam: runs after every local action is applied; a throw simulates a half-applied action.</summary>
        internal Action<GameAction> ApplyProbe;

        /// <summary>Actions sent and not answered yet, keyed by <see cref="Feedback.FlightKey"/>.</summary>
        internal readonly HashSet<string> InFlight = new HashSet<string>();

        private int feedbackSequence;

        /// <summary>
        /// Sends an action and publishes its feedback. Never throws: a throwing local action faults the forest
        /// (as a throwing step does), and remote exceptions become stable codes. A second identical action while
        /// the first is unanswered is refused with <see cref="FeedbackCode.AlreadySent"/> and publishes nothing.
        /// </summary>
        public async Task<ActionResult> Send(GameAction action)
        {
            if (action == null) return ActionResult.Fail(ResultCode.UnknownTarget, "");
            if (!ready || CurrentWorld == null) return Publish(action, ActionResult.Fail(FeedbackCode.NotReady, "World is still opening."));
            string key = Feedback.IsStreaming(action.Kind) ? null : Feedback.FlightKey(action);
            if (key != null && !InFlight.Add(key)) return ActionResult.Fail(FeedbackCode.AlreadySent, "");
            ActionResult result;
            try
            {
                if (remote != null)
                {
                    try { result = await remote.SendAsync(action); }
                    catch (Exception e)
                    {
                        ForestLog.Exception(e, "remote action " + action.Kind);
                        result = ActionResult.Fail(Feedback.CodeFor(e), "");
                    }
                }
                else
                {
                    try
                    {
                        result = local.Apply(action);
                        ApplyProbe?.Invoke(action);
                    }
                    catch (Exception e)
                    {
                        Fault(e, "local action " + action.Kind, false);
                        result = ActionResult.Fail(FeedbackCode.SimulationFault, "");
                    }
                }
            }
            finally
            {
                if (key != null) InFlight.Remove(key);
            }
            if (result == null) result = ActionResult.Fail(FeedbackCode.Unexpected, "");
            if (result.Success) Revision++;
            return Publish(action, result);
        }

        /// <summary>Sends an action; the task completes with its result and never faults.</summary>
        public Task<ActionResult> ActAsync(GameAction action) => Send(action);

        /// <summary>Sends an action without waiting; its result arrives as <see cref="LastFeedback"/>.</summary>
        public void Act(GameAction action) { _ = Send(action); }

        /// <summary>Records the feedback of a result unless the kind is silent for it, and returns the result.</summary>
        private ActionResult Publish(GameAction action, ActionResult result)
        {
            string kind = action?.Kind ?? "";
            if (Feedback.IsSilent(kind, result.Success)) return result;
            LastFeedback = Feedback.From(action, result, Time.realtimeSinceStartupAsDouble, ++feedbackSequence);
            LastAction = result.Success ? Describe(kind) + " accepted"
                : !string.IsNullOrEmpty(result.Error) ? result.Error : LastFeedback.Code;
            FeedbackPosted?.Invoke(LastFeedback);
            return result;
        }

        public static string Describe(string text)
        {
            if (string.IsNullOrEmpty(text)) return "";
            return System.Text.RegularExpressions.Regex.Replace(text.Replace('_', ' '), "(?<=[a-z])(?=[A-Z])", " ");
        }
    }
}
