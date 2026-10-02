using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.RegularExpressions;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Why something waits or fails, in the player's words. A blocked reason has a full sentence with a
    /// next step (<see cref="Reason(string)"/>) and a short lowercase cause for "because …" lines
    /// (<see cref="Cause"/>). Action results map their stable code and arguments to the same voice.
    /// </summary>
    public static class Reasons
    {
        private static readonly Dictionary<string, (string sentence, string cause)> Blocks = new Dictionary<string, (string, string)>
        {
            // The frozen Blocked list (docs/DEMO.md 3.8).
            [Blocked.ForeignTerritory] = ("This land belongs to another colony. Choose a spot inside your fence.", "the land belongs to another colony"),
            [Blocked.ExpansionFootprintBlocked] = ("The new fence would cross a building, entrance or stockpile. Move it out of the way and the fence goes on; its materials stay reserved.", "the new fence would cross a building"),
            [Blocked.InfrastructureFootprintBlocked] = ("Something stands on the planned road. Clear the building, stockpile or field in its way to carry on.", "something stands on the planned road"),
            [Blocked.BuildingEntranceDisconnected] = ("The entrance is cut off. Connect its road back to the shrine to carry on building.", "its entrance has no road to the shrine"),
            [Blocked.QueuePaused] = ("Production is paused. Resume the queue when you are ready.", "production is paused"),
            [Blocked.MissingScaffoldInput] = ("Building materials are still on the way. Haulers bring them from storage.", "building materials are still on the way"),
            [Blocked.UnknownRecipe] = ("No recipe is chosen. Pick something for this workshop to make.", "no recipe is chosen"),
            [Blocked.MissingRecipeInput] = ("Waiting for ingredients. Stock them in a stockpile the workers can reach.", "ingredients are missing"),
            [Blocked.WallMaterialsMissing] = ("The fence is waiting for its materials to arrive.", "fence materials are still on the way"),
            [Blocked.PavingInputMissing] = ("Road work is waiting for paving stones.", "paving stones are still on the way"),
            [Blocked.OutputStorageFullOrUnreachable] = ("Finished goods have nowhere to go. Make room in a matching stockpile and check its path.", "storage is full or out of reach"),
            [Blocked.WorkerRequired] = ("Nobody works here yet. Assign a free cat.", "nobody works here yet"),
            [Blocked.SourceChanged] = ("The supply ran out or moved. The cats are looking for another one.", "the supply ran out or moved"),
            [Blocked.BuildingResearchRequired] = ("This needs a study first. Open Research to unlock it.", "it needs a study first"),
            [Blocked.SiteRequired] = ("Choose a clear building site connected to the colony's roads.", "it needs a clear building site"),
            [Blocked.BlockedRoute] = ("No path gets there. Check the gate, roads and any water along the way.", "there is no path"),
            [Blocked.MissingMaterial] = ("Waiting for materials. Bring them to a stockpile the builders can reach.", "materials are missing"),
            [Blocked.RouteBlocked] = ("No path gets there. Check the gate, roads and any water along the way.", "there is no path"),
            [Blocked.SourceExcluded] = ("The usual source keeps failing, so the cats are trying another one for a while.", "the usual source keeps failing"),

            // Reasons the simulation still writes as literals.
            ["queue_empty"] = ("Ready for work. Add a recipe to this worker's queue.", "nothing is queued"),
            ["recipe_locked"] = ("This needs a study first. Open Research to unlock it.", "it needs a study first"),
            ["missing_input"] = ("Waiting for ingredients. Stock them in a stockpile the workers can reach.", "ingredients are missing"),
            ["missing_grain"] = ("Waiting for grain. Grow or gather grain and bring it to storage.", "there is no grain"),
            ["need_supply_missing"] = ("Can't find food or water. Check the stores and the paths to them.", "no food or water is in reach"),
            ["output_storage_full"] = ("Finished goods have nowhere to go. Make room in a matching stockpile and check its path.", "storage is full or out of reach"),
            ["station_local_full"] = ("The workshop is full. Haul its finished goods into storage.", "the workshop is full"),
            ["input_claimed"] = ("Another worker has the ingredients. Waiting for more supplies.", "another worker has the ingredients"),
            ["blocked_harvest_route"] = ("The harvest can't reach storage. Clear a path for it.", "the harvest has no path to storage"),
            ["blocked_handoff_route"] = ("The field's collection spot is out of reach. Check the path.", "the collection spot is out of reach"),
            ["handoff_full"] = ("The field's collection spot is full. Haul its goods to storage.", "the collection spot is full"),
            ["return_route_blocked"] = ("The way back is blocked. Open a path for this cargo.", "the way back is blocked"),
            ["return_storage_full"] = ("The storage at the other end is full. Free some space or add a matching stockpile.", "the storage there is full"),
            ["storage_full"] = ("The storage at the other end is full. Free some space or add a matching stockpile.", "the storage there is full"),
            ["blocked_reboarding_route"] = ("The driver can't get back to the vehicle. Clear a path to it.", "the driver can't reach the vehicle"),
            ["driver_required"] = ("This route needs a driver. Build a bridge so a cat can reach the wreck.", "it needs a driver"),
            ["expedition_storage_required"] = ("The explorer needs a stockpile with room for the loot. Make space near the gate.", "there is no room for the loot"),
            ["expedition_return_for_safety"] = ("The explorer came home to rest and heal before going back down.", "the explorer needed rest"),
            ["expedition_enemy_too_dangerous"] = ("The explorer turned back: the creatures ahead are too strong. Train or equip them first.", "the creatures ahead are too strong"),
            ["expedition_guardian_unreachable"] = ("The explorer can't reach the guardian. The way down is blocked.", "the guardian is out of reach"),
            ["resident"] = ("", ""),
        };

        private static readonly Dictionary<string, string> Results = new Dictionary<string, string>
        {
            // Successes read as the confirmation verb.
            [ResultCode.Ok] = "Done",
            [ResultCode.Queued] = "Queued",
            [ResultCode.Planned] = "Planned",
            [ResultCode.Assigned] = "Assigned",
            [ResultCode.Released] = "Released from work",
            [ResultCode.Studied] = "Studied",
            [ResultCode.Boosted] = "Sped up",
            [ResultCode.Cancelled] = "Cancelled",
            [ResultCode.Demolished] = "Taken down",
            // Refusals say what is wrong and what to do.
            [ResultCode.NotAWorkplace] = "Nobody works there. Pick a workshop, hut or station.",
            [ResultCode.UnknownTarget] = "That's no longer there. Pick it again.",
            [ResultCode.RequiresPlayers] = "That needs other players, and this colony is on its own.",
            [ResultCode.FullGameOnly] = "That comes with the full game.",
            [ResultCode.MissingMaterial] = "Not enough materials yet.",
            [ResultCode.AllGroundsBusy] = "Every known hunting and gathering ground is in use. Send a scout to find more.",
            [ResultCode.NotImplemented] = "That isn't ready yet.",
        };

        private static readonly string[] SuccessCodes =
        {
            ResultCode.Ok, ResultCode.Queued, ResultCode.Planned, ResultCode.Assigned, ResultCode.Released,
            ResultCode.Studied, ResultCode.Boosted, ResultCode.Cancelled, ResultCode.Demolished,
        };

        /// <summary>
        /// Engine messages of actions that do not carry a code yet, matched by prefix. W2-06 replaces them
        /// with codes; until then the player still reads the colony's voice.
        /// </summary>
        private static readonly (string prefix, string text)[] Legacy =
        {
            ("Insufficient research", "Not enough research yet. Cats study at the Research Hut."),
            ("Insufficient blessings", "Not enough blessings yet. Offer food at the Shrine to earn more."),
            ("Insufficient coin", "Not enough coins."),
            ("Missing prerequisites", "Study the earlier steps first."),
            ("Already researched", "Already studied."),
            ("Unknown study", "That study isn't known."),
            ("Occupied, unmapped, or invalid footprint", "That spot is taken or not explored yet. Pick a clear patch next to a road."),
            ("Building entrance must touch", "The entrance needs a road that leads back to the shrine."),
            ("Road must join", "A road has to connect to the shrine's roads."),
            ("No available worker", "Every cat is busy. Free one up or wait a moment."),
            ("No available builder", "No builder is free right now."),
            ("No available carrier", "No hauler is free right now."),
            ("All work positions staffed", "Every spot here is already taken."),
            ("Available living cat required", "Pick a cat first."),
            ("Living cat required", "Pick a cat first."),
            ("Living leader required", "The colony needs a Leader for that."),
            ("Unknown action", "That can't be done here."),
            ("Research unlock required", "This needs a study first. Open Research to unlock it."),
            ("Recipe locked or wrong station", "This workshop can't make that yet."),
            ("Queue limit reached", "The queue is full."),
            ("Food reserve protected", "The colony needs that food to eat. Keep it."),
            ("Survival and construction reserve protected", "The colony needs that for food and building. Keep it."),
            ("Upgrade already maxed", "Already at the highest level."),
            ("Unknown upgrade", "That blessing isn't known."),
            ("Cat controlled by another player", "Someone else is steering that cat."),
            ("Existing control lease required", "Take control of the cat first."),
            ("Control lease required", "Take control of the cat first."),
            ("Blocked route", "No path gets there. Check the gate, roads and any water along the way."),
            ("Foreign territory", "That land belongs to another colony."),
            ("Existing footprint or entrance blocks expansion", "The new fence would cross a building, entrance or stockpile. Clear the way first."),
            ("No clear Den footprint", "There's no clear spot for a Den. Make room inside the fence."),
            ("Expansion already active", "The fence is already growing."),
            ("Dungeon risk exceeds", "Too dangerous for this cat right now. Rest, train or equip them first."),
            ("No known dungeon entrance", "No dungeon has been found yet. Send a scout."),
            ("Recall the current expedition first", "Call the current explorer home first."),
            ("Recall the explorer", "Call the explorer home first."),
            ("Choose a living explorer", "Pick a cat who is exploring."),
            ("The dungeon entrance has no accessible surface route", "No path leads to the dungeon entrance."),
            ("No active raid", "There's no raid to defend against right now."),
            ("Completed Barracks required", "Build a Barracks first."),
            ("Completed station required", "Finish building the workshop first."),
            ("Completed Field required", "Finish the field first."),
            ("Office requires completed", "That office needs its building and study first."),
            ("Cat already holds an office", "That cat already holds an office."),
            ("Office already vacant", "Nobody holds that office."),
            ("Unknown constructible building", "That can't be built."),
            ("Construction requires", "That needs a study first. Open Research to unlock it."),
            ("Construction inputs claimed", "The materials are already promised to another build."),
            ("Scaffold already has a builder", "Someone is already building there."),
            ("Storage full or incompatible", "No storage has room for that."),
            ("Storage unavailable", "No storage has room for that."),
            ("No accepting destination", "No storage has room for that."),
            ("Source unreachable", "No path gets there."),
            ("Empty source", "There's nothing left there."),
            ("No source goods", "There's nothing left there."),
            ("Pile has active claims", "Cats are still using that pile. Try again in a moment."),
            ("Unknown pile", "That pile is gone."),
            ("Walk to the pile first", "Walk to the pile first."),
            ("No interactable target within reach", "Nothing to use within reach. Walk closer."),
            ("Rest at this cat's reserved Den bed", "Walk to this cat's own Den to rest."),
            ("Movement must be one cardinal direction", "Move one step at a time."),
            ("Fishing needs a walkable shore", "Fishing needs a shore the cats can walk to."),
            ("Farm requires reachable mapped exterior ground", "Fields go on explored ground outside the fence that the cats can reach."),
            ("Unknown crop", "That crop can't be planted."),
            ("Invalid stockpile area", "That area can't hold a stockpile."),
            ("Zone must be a bounded mapped area", "Mark an explored area."),
            ("Avoid zones cannot cover shrine roads", "A no-go zone can't cover the shrine's roads or an entrance."),
            ("Survey a dry reachable outer perimeter first", "Explore dry land around the fence first."),
            ("No reachable survey", "Nowhere left to scout that the cats can reach."),
            ("Developer control", "That needs developer mode."),
            ("Developer controls", "That needs developer mode."),
            ("Explicit AdvanceTime", "That needs developer mode."),
            ("Village ownership denied", "That belongs to another colony."),
            ("Connect to a shared world first", "That needs other players, and this colony is on its own."),
            ("Mutual shrine-delivered contact required", "That needs other players, and this colony is on its own."),
            ("Trade controller required", "That needs other players, and this colony is on its own."),
            ("Authenticate before sending actions", "Sign in first."),
            ("Action is malformed", "That can't be done here."),
            ("Invalid finite action arguments", "That can't be done here."),
            ("Missing action or identity", "That can't be done here."),
        };

        private static readonly Regex EntityId = new Regex(@"\b[a-z]+(?:_[a-z]+)*-\d+\b", RegexOptions.CultureInvariant);
        private static readonly Regex Coordinates = new Regex(@"-?\d+,-?\d+(?:,-?\d+)?", RegexOptions.CultureInvariant);
        private static readonly Regex Spaces = new Regex(@"\s{2,}", RegexOptions.CultureInvariant);

        /// <summary>The full sentence for a blocked or failure code, with its next step.</summary>
        public static string Reason(string code)
        {
            if (string.IsNullOrEmpty(code))
                return "";
            string key = Key(code);
            if (Blocks.TryGetValue(key, out var block))
                return block.sentence;
            if (Results.TryGetValue(key, out var result) && !IsSuccess(key))
                return result;
            // A sentence (for example an older engine message) is cleaned, never echoed raw.
            if (code.IndexOf(' ') >= 0)
                return Message(code);
            return Names.Humanize(key) + ".";
        }

        /// <summary>A reason with typed arguments: "Needs 5 research · you have 0.4 · about 4 min with 1 scholar".</summary>
        public static string Reason(string code, IReadOnlyDictionary<string, string> args)
        {
            if (args == null || args.Count == 0)
                return Reason(code);
            string key = Key(code);
            string resource = Arg(args, "resource");
            if (resource != "" && (key == ResultCode.MissingMaterial || key == Blocked.MissingMaterial || Number(args, "need", out _)))
                return Shortfall(resource, args);
            string sentence = Reason(code);
            string target = Arg(args, "target");
            return target == "" || sentence == "" ? sentence : sentence.TrimEnd('.') + " · " + Names.Humanize(target);
        }

        /// <summary>The short lowercase cause for "because …" lines and "Waiting at the Sawmill — …".</summary>
        public static string Cause(string code)
        {
            if (string.IsNullOrEmpty(code))
                return "";
            string key = Key(code);
            if (Blocks.TryGetValue(key, out var block))
                return block.cause;
            if (Results.TryGetValue(key, out var result) && !IsSuccess(key))
                return Names.Lower(result.Split('.')[0]);
            return Names.Humanize(key).ToLowerInvariant();
        }

        /// <summary>The text for any action result code: a confirmation word or a refusal sentence.</summary>
        public static string Result(string code) =>
            code != null && Results.TryGetValue(code, out var text) ? text : Reason(code);

        /// <summary>True for result codes that mean the action succeeded.</summary>
        public static bool IsSuccess(string code) => Array.IndexOf(SuccessCodes, code) >= 0;

        /// <summary>True when the blocked code has authored copy (Coverage).</summary>
        public static bool IsAuthored(string code) => code != null && Blocks.ContainsKey(Key(code)) && Blocks[Key(code)].sentence != "";

        /// <summary>True when the result code has authored copy (Coverage).</summary>
        public static bool IsAuthoredResult(string code) => code != null && Results.ContainsKey(code);

        /// <summary>
        /// The player text for a refused action: the code with its arguments when there is one, otherwise
        /// the engine message mapped to the colony's voice. Never an exception message, id or coordinate.
        /// </summary>
        public static string ActionError(ActionResult result)
        {
            if (result == null || result.Success)
                return "";
            if (!string.IsNullOrEmpty(result.Code) && (Results.ContainsKey(result.Code) || Blocks.ContainsKey(result.Code)))
                return Reason(result.Code, result.Args);
            if (!string.IsNullOrEmpty(result.Error))
                return Message(result.Error);
            return string.IsNullOrEmpty(result.Code) ? "That didn't work." : Reason(result.Code, result.Args);
        }

        /// <summary>An engine or legacy message in the colony's voice: mapped when known, cleaned otherwise.</summary>
        public static string Message(string message)
        {
            if (string.IsNullOrWhiteSpace(message))
                return "";
            foreach (var (prefix, text) in Legacy)
                if (message.StartsWith(prefix, StringComparison.Ordinal))
                    return text;
            if (message.StartsWith("Insufficient ", StringComparison.Ordinal))
                return "Not enough " + Names.Resource(message.Substring("Insufficient ".Length).Trim().TrimEnd('.')) + " yet.";
            if (message.StartsWith("No known reachable ", StringComparison.Ordinal))
            {
                string what = message.Substring("No known reachable ".Length).Split(';')[0].Trim();
                return "Nobody knows where to find " + Names.Resource(what) + " yet. Send a scout first.";
            }
            return Clean(message);
        }

        /// <summary>
        /// Makes arbitrary text safe to show: drops entity ids and coordinates, turns underscores into
        /// spaces and starts with a capital letter.
        /// </summary>
        public static string Clean(string text)
        {
            if (string.IsNullOrWhiteSpace(text))
                return "";
            string cleaned = EntityId.Replace(text, "");
            cleaned = Coordinates.Replace(cleaned, "");
            cleaned = cleaned.Replace('_', ' ');
            cleaned = Spaces.Replace(cleaned, " ").Trim().Trim(':', '·', '-').Trim();
            return Names.Sentence(cleaned);
        }

        private static string Shortfall(string resource, IReadOnlyDictionary<string, string> args)
        {
            var parts = new List<string>();
            if (Number(args, "need", out double need))
                parts.Add("Needs " + Names.Amount(need, resource));
            else
                parts.Add("Needs more " + Names.Resource(resource));
            if (Number(args, "have", out double have))
                parts.Add("you have " + Numbers.Floor(have, have < 10 ? 1 : 0));
            double eta = 0;
            if (Number(args, "eta", out eta) || Number(args, "seconds", out eta))
            {
                string about = GameClock.About(eta);
                if (Number(args, "workers", out double workers) && workers > 0)
                    about += " with " + Numbers.Plural(workers, Worker(resource, false), Worker(resource, true));
                parts.Add(about);
            }
            else
            {
                string hint = Hint(resource);
                if (hint != "")
                    return string.Join(" · ", parts) + ". " + hint;
            }
            return string.Join(" · ", parts);
        }

        private static string Worker(string resource, bool many) =>
            resource == "research" ? (many ? "scholars" : "scholar") : (many ? "workers" : "worker");

        private static string Hint(string resource)
        {
            switch (resource)
            {
                case "research":
                    return "Cats study at the Research Hut.";
                case "blessings":
                    return "Offer food at the Shrine to earn more.";
                case "logs":
                    return "Woodcutters bring logs from the forest.";
                case "stone":
                    return "Quarry workers bring stone.";
                case "food":
                    return "Hunters bring food.";
                case "water":
                    return "Water carriers fill the Water Bowl.";
                default:
                    return "";
            }
        }

        private static string Key(string code)
        {
            // Some reasons carry a free-text detail after " · " (for example a driver note).
            int separator = code.IndexOf(" · ", StringComparison.Ordinal);
            return separator > 0 ? code.Substring(0, separator) : code;
        }

        private static string Arg(IReadOnlyDictionary<string, string> args, string key) =>
            args != null && args.TryGetValue(key, out var value) && value != null ? value : "";

        private static bool Number(IReadOnlyDictionary<string, string> args, string key, out double value)
        {
            value = 0;
            string raw = Arg(args, key);
            return raw != "" && double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out value) && !double.IsNaN(value) && !double.IsInfinity(value);
        }
    }
}
