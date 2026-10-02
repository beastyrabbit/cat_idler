using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    public partial class ActionResult
    {
        /// <summary>Stable snake_case reason or outcome (see <see cref="ResultCode"/>); "" until a handler sets one.</summary>
        public string Code = "";

        /// <summary>Typed details for the code (resource, need, have, target ...); null when there are none.</summary>
        public Dictionary<string, string> Args;

        /// <summary>A failure with a stable code, the existing English message and optional arguments.</summary>
        public static ActionResult Fail(string code, string message, params (string key, string value)[] args) =>
            new ActionResult { Code = code ?? "", Error = message ?? "", Args = ToArgs(args) };

        /// <summary>
        /// A success with a stable code, the affected entity id and optional arguments. Success has no
        /// message field on the wire, so the second argument is the entity id like <c>Ok(id)</c>.
        /// </summary>
        public static ActionResult Ok(string code, string entityId, params (string key, string value)[] args) =>
            new ActionResult { Success = true, Code = code ?? "", EntityId = entityId ?? "", Args = ToArgs(args) };

        private static Dictionary<string, string> ToArgs((string key, string value)[] args)
        {
            if (args == null || args.Length == 0)
                return null;
            var map = new Dictionary<string, string>();
            foreach (var (key, value) in args)
                map[key] = value ?? "";
            return map;
        }
    }
}
