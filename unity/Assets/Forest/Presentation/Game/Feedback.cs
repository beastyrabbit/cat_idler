using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using System.Net.WebSockets;
using System.Threading.Tasks;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The structured result of one player action, published by <see cref="ForestGame.Send"/>. UI packages
    /// turn it into copy through <c>UiText</c>: <see cref="Code"/> is a stable snake_case key (a
    /// <see cref="ResultCode"/>, a simulation reason, or one of <see cref="FeedbackCode"/>), never an
    /// exception message. <see cref="At"/> is real time (<c>Time.realtimeSinceStartupAsDouble</c>), so
    /// the HUD can fade it; <see cref="Sequence"/> grows by one per published feedback.
    /// </summary>
    public sealed class Feedback
    {
        public Feedback(string code, string kind, bool success, string entityId, double at,
            int sequence = 0, string message = "", IReadOnlyDictionary<string, string> args = null)
        {
            Code = code ?? "";
            Kind = kind ?? "";
            Success = success;
            EntityId = entityId ?? "";
            At = at;
            Sequence = sequence;
            Message = message ?? "";
            Args = args ?? Empty;
        }

        private static readonly IReadOnlyDictionary<string, string> Empty = new Dictionary<string, string>();

        /// <summary>Stable key for the copy layer; "ok" for a success whose handler sets no code.</summary>
        public string Code { get; }

        /// <summary>The <see cref="GameAction.Kind"/> that produced it.</summary>
        public string Kind { get; }

        public bool Success { get; }

        /// <summary>The affected entity (cat, building, job ...) or "".</summary>
        public string EntityId { get; }

        /// <summary>Real seconds since startup when the result arrived.</summary>
        public double At { get; }

        /// <summary>1 for the first feedback of a session; equal codes still differ by sequence.</summary>
        public int Sequence { get; }

        /// <summary>
        /// The simulation's own English reason for an uncoded failure, a fallback until every handler sets a
        /// code. Never exception text.
        /// </summary>
        public string Message { get; }

        /// <summary>Typed details of the code (resource, need, have ...); empty when there are none.</summary>
        public IReadOnlyDictionary<string, string> Args { get; }

        /// <summary>Feedback for an action result.</summary>
        public static Feedback From(GameAction action, ActionResult result, double at, int sequence)
        {
            string code = result.Code ?? "";
            if (code.Length == 0 && result.Success) code = ResultCode.Ok;
            return new Feedback(code, action?.Kind, result.Success, result.EntityId, at, sequence,
                result.Success ? "" : result.Error, result.Args);
        }

        /// <summary>
        /// Continuous control traffic that would flood the feedback line: successes of these kinds publish
        /// nothing. <c>MoveCat</c> failures (walking into a fence) publish nothing either; the view shows them.
        /// </summary>
        public static bool IsSilent(string kind, bool success)
        {
            switch (kind)
            {
                case "MoveCat": return true;
                case "KeepCatControl": case "AttackCreature": return success;
                default: return false;
            }
        }

        /// <summary>Kinds that stream while a key is held; they skip the duplicate guard.</summary>
        public static bool IsStreaming(string kind) => kind == "MoveCat" || kind == "KeepCatControl";

        /// <summary>The in-flight identity of an action: its kind plus every field that names its target.</summary>
        public static string FlightKey(GameAction a) =>
            string.Join("|", a.Kind, a.TargetId, a.CatId, a.BuildingId, a.NodeId, a.RecipeId, a.Resource,
                a.Position.X, a.Position.Z, a.End.X, a.End.Z);

        /// <summary>Maps an exception to a stable code. The message stays in the log, never in player text.</summary>
        public static string CodeFor(Exception e)
        {
            while (e is AggregateException aggregate && aggregate.InnerException != null) e = aggregate.InnerException;
            switch (e)
            {
                case null: return FeedbackCode.Unexpected;
                case TimeoutException _:
                case OperationCanceledException _: return FeedbackCode.Timeout;
                case SocketException _:
                case WebSocketException _:
                case System.Net.Http.HttpRequestException _: return FeedbackCode.Network;
                case UriFormatException _: return FeedbackCode.InvalidAddress;
                case IOException _:
                case UnauthorizedAccessException _: return FeedbackCode.IoError;
                default: return e.InnerException != null ? CodeFor(e.InnerException) : FeedbackCode.Unexpected;
            }
        }
    }

    /// <summary>
    /// Feedback codes raised by the presentation itself (exceptions, guards and the loop), alongside
    /// <see cref="ResultCode"/>. Values are UI-text keys: never rename or reuse one.
    /// </summary>
    public static class FeedbackCode
    {
        public const string IoError = "io_error";
        public const string Timeout = "timeout";
        public const string Network = "network";
        public const string InvalidAddress = "invalid_address";
        public const string Unexpected = "unexpected_error";
        public const string AlreadySent = "already_sent";
        public const string NotReady = "not_ready";
        public const string SimulationFault = "simulation_fault";
        public const string ServerControlsTime = "server_controls_time";
    }
}
