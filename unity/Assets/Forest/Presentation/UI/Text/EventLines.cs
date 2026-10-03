using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>One collapsed run of events for the log: the line plus what it stands for.</summary>
    public sealed class EventGroup
    {
        public string Kind = "";
        public string Line = "";
        public double FirstTime, LastTime;
        public long LastSeq;
        public IReadOnlyList<Event> Events = new Event[0];
        public int Count => Events.Count;
    }

    /// <summary>
    /// The colony's story: one player-voice line per event kind ("Birch came back from a hunt",
    /// "Raiders spotted to the south!") and <see cref="Collapse"/>, which folds routine repeats within a
    /// minute into one line ("Birch, Fennel and 5 others finished 7 jobs · 4 water runs, 3 hunts").
    /// </summary>
    public static class EventLines
    {
        /// <summary>Kinds that repeat many times a minute and are folded together by <see cref="Collapse"/>.</summary>
        public static readonly IReadOnlyCollection<string> Routine = new HashSet<string>
        {
            EventKind.Completed, EventKind.Discovery, EventKind.Tithe, EventKind.Emergency, EventKind.Expansion,
            EventKind.Repaired,
        };

        /// <summary>The window in simulated seconds within which routine events of one kind are folded.</summary>
        public const double CollapseWindowSeconds = 60;

        private static readonly Dictionary<string, string> Fallbacks = new Dictionary<string, string>
        {
            [EventKind.Research] = "A study was completed",
            [EventKind.Tithe] = "A food tithe earned a blessing",
            [EventKind.Completed] = "A cat finished a job",
            [EventKind.Defense] = "The raiders were driven off!",
            [EventKind.Dungeon] = "News from the dungeon",
            [EventKind.Election] = "A new Leader took office",
            [EventKind.Emergency] = "A cat dropped their work to fetch supplies",
            [EventKind.Expansion] = "The colony grew",
            [EventKind.Migration] = "A wanderer is at the gate",
            [EventKind.Pregnancy] = "A cat is expecting kittens",
            [EventKind.Raid] = "Raiders spotted to the south!",
            [EventKind.RaidBreach] = "Raiders reached the shrine and stole supplies",
            [EventKind.RaidLoss] = "The raiders escaped with stolen supplies",
            [EventKind.Recovery] = "The colony was founded anew",
            [EventKind.RecoveryBlocked] = "The colony couldn't be founded again: another colony holds the land",
            [EventKind.Trade] = "A trade was completed",
            [EventKind.Trader] = "A merchant's wagon arrived at the shrine",
            [EventKind.Upgrade] = "A blessing grew stronger",
            [EventKind.Milestone] = "A milestone was reached",
            [EventKind.RaidWarning] = "Raiders spotted to the south!",
            [EventKind.RaidRepelled] = "The raiders were driven off!",
            [EventKind.BuildingCompleted] = "A new building is finished",
            [EventKind.Discovery] = "A scout came back with a new map",
            [EventKind.StorageFull] = "A store is full",
            [EventKind.StorageEmpty] = "A store ran empty",
            [EventKind.RankUp] = "A cat got better at their work",
            [EventKind.Birth] = "A kitten was born!",
            [EventKind.Arrival] = "A wanderer joined the colony",
            [EventKind.Death] = "A cat has died",
            [EventKind.ExpeditionLoot] = "An explorer brought back loot",
            [EventKind.Error] = "Something went wrong, but the colony carried on",
            [EventKind.Appointment] = "A cat took up an office",
            [EventKind.Repaired] = "The colony sorted out a muddle in its stores and beds",
        };

        /// <summary>What a finished job adds to "Birch …" and how many of them read in a summary.</summary>
        private static readonly Dictionary<string, (string done, string one, string many)> Finished = new Dictionary<string, (string, string, string)>
        {
            [JobKind.Hunt] = ("came back from a hunt", "hunt", "hunts"),
            [JobKind.Water] = ("brought back water", "water run", "water runs"),
            [JobKind.Logs] = ("brought back logs", "wood run", "wood runs"),
            [JobKind.Quarry] = ("brought back stone", "stone run", "stone runs"),
            [JobKind.Fibre] = ("brought back fibre", "fibre run", "fibre runs"),
            [JobKind.Fish] = ("came back from fishing", "fishing trip", "fishing trips"),
            [JobKind.Scout] = ("finished scouting", "scouting trip", "scouting trips"),
            [JobKind.Replant] = ("planted a sapling", "sapling", "saplings"),
            [JobKind.Train] = ("finished training", "training session", "training sessions"),
            [JobKind.Expand] = ("built a stretch of fence", "fence section", "fence sections"),
            [JobKind.Haul] = ("finished a haul", "haul", "hauls"),
            [JobKind.Build] = ("finished building", "build", "builds"),
            [JobKind.Production] = ("finished a workshop shift", "workshop shift", "workshop shifts"),
            [JobKind.Offering] = ("made an offering", "offering", "offerings"),
            [JobKind.MouseFarm] = ("tended the mice", "mouse farm shift", "mouse farm shifts"),
            [JobKind.Road] = ("laid a stretch of road", "road section", "road sections"),
            [JobKind.Rail] = ("laid a stretch of track", "track section", "track sections"),
            [JobKind.Bridge] = ("worked on the bridge", "bridge shift", "bridge shifts"),
            [JobKind.Dock] = ("worked on the dock", "dock shift", "dock shifts"),
            [JobKind.Wagon] = ("worked on the wagon", "wagon shift", "wagon shifts"),
            [JobKind.Vessel] = ("worked on the boat", "boat shift", "boat shifts"),
            ["gather"] = ("finished gathering", "gathering trip", "gathering trips"),
            ["zone_gather"] = ("finished gathering", "gathering trip", "gathering trips"),
        };

        private static readonly Dictionary<string, string> DeathCauses = new Dictionary<string, string>
        {
            ["needs"] = "{0} has died. Cats need food, water and rest to stay well.",
            ["old age"] = "{0} passed away peacefully of old age.",
            ["old_age"] = "{0} passed away peacefully of old age.",
            ["dungeon"] = "{0} fell in the dungeon.",
            ["raid"] = "{0} fell defending the colony.",
            ["starvation"] = "{0} starved. Keep the Food Store stocked.",
            ["thirst"] = "{0} died of thirst. Keep the Water Bowl filled.",
        };

        /// <summary>Fixed engine texts the simulation writes today, in the colony's voice.</summary>
        private static readonly Dictionary<string, string> LegacyTexts = new Dictionary<string, string>
        {
            ["Raiders approach the south gate"] = "Raiders spotted to the south!",
            ["Raid repelled"] = "The raiders were driven off!",
            ["Raiders escaped with stolen supplies"] = "The raiders escaped with stolen supplies",
            ["Raiders reached the shrine and looted finite supplies"] = "Raiders reached the shrine and stole supplies",
            ["Finite merchant wagon reached the shrine"] = "A merchant's wagon arrived at the shrine",
            ["Colony refounded after extinction"] = "The colony was founded anew",
            ["Refounding blocked by foreign territory"] = "The colony couldn't be founded again: another colony holds the land",
            ["Food tithe earned one blessing"] = "A food tithe earned a blessing",
            ["Leader term began"] = "A new Leader took office",
            ["Caravan returned and barter completed"] = "The caravan came home with the traded goods",
            ["Barter delivery completed"] = "A trade delivery arrived",
            ["Restored wall project claimed one tile"] = "The fence repair took in more ground",
            ["Five players removed the leader; interim term began"] = "The Leader was voted out; a new Leader stepped in",
            ["Leader removal petition expired without five signatures"] = "The petition to replace the Leader ran out",
        };

        /// <summary>True when the event kind has an authored line (Coverage).</summary>
        public static bool IsAuthored(string kind) => kind != null && Fallbacks.ContainsKey(kind);

        /// <summary>
        /// The line for one event. With the village, entity ids resolve to names; without it, names come
        /// from the event text. A text the simulation writes in a known engine form is rewritten; any
        /// other text is shown cleaned (later writers author player text); an empty one falls back to the
        /// kind's line. Never an id, a raw key or a coordinate.
        /// </summary>
        public static string Line(Event e, Village village = null)
        {
            if (e == null)
                return "";
            string text = e.Text ?? "";
            string line = Template(e, text, village);
            if (line == "" && LegacyTexts.TryGetValue(text, out var legacy))
                line = legacy;
            if (line == "")
                line = Reasons.Clean(text);
            if (line == "")
                line = Fallbacks.TryGetValue(e.Kind ?? "", out var fallback) ? fallback : "Something happened in the colony";
            return line;
        }

        /// <summary>
        /// Folds routine events (finished jobs, scouting returns, tithes) of one kind that fall within
        /// <see cref="CollapseWindowSeconds"/> of each other, and identical consecutive lines of any kind,
        /// into one group each. Order is by time, then sequence. A scout's "completed" event is dropped when
        /// its discovery event is logged at the same moment.
        /// </summary>
        public static IReadOnlyList<EventGroup> Collapse(IEnumerable<Event> events) => Collapse(events, null);

        /// <summary><see cref="Collapse(IEnumerable{Event})"/> with names resolved from the village.</summary>
        public static IReadOnlyList<EventGroup> Collapse(IEnumerable<Event> events, Village village)
        {
            var groups = new List<EventGroup>();
            if (events == null)
                return groups;
            var ordered = events.Where(e => e != null).OrderBy(e => e.Time).ThenBy(e => e.Seq).ToList();
            // A scout's return logs both "completed scout" and the discovery; the discovery says more.
            var discoveries = new HashSet<double>(ordered.Where(e => e.Kind == EventKind.Discovery).Select(e => e.Time));
            ordered.RemoveAll(e => e.Kind == EventKind.Completed && discoveries.Contains(e.Time) && Completed(e.Text ?? "").kind == JobKind.Scout);
            var run = new List<Event>();
            string runKind = null, runLine = null;
            foreach (var e in ordered)
            {
                string line = Line(e, village);
                bool routine = Routine.Contains(e.Kind ?? "");
                bool joins = run.Count > 0 && e.Kind == runKind
                    && e.Time - run[0].Time <= CollapseWindowSeconds
                    && (routine || line == runLine);
                if (!joins)
                {
                    Flush(groups, run, village);
                    run = new List<Event>();
                    runKind = e.Kind;
                    runLine = line;
                }
                run.Add(e);
            }
            Flush(groups, run, village);
            return groups;
        }

        private static void Flush(List<EventGroup> groups, List<Event> run, Village village)
        {
            if (run.Count == 0)
                return;
            var last = run[run.Count - 1];
            groups.Add(new EventGroup
            {
                Kind = run[0].Kind ?? "",
                Line = run.Count == 1 ? Line(run[0], village) : Summary(run, village),
                FirstTime = run[0].Time,
                LastTime = last.Time,
                LastSeq = run.Max(e => e.Seq),
                Events = run.ToArray(),
            });
        }

        private static string Summary(List<Event> run, Village village)
        {
            string kind = run[0].Kind ?? "";
            var lines = run.Select(e => Line(e, village)).ToList();
            if (lines.Distinct().Count() == 1 && kind != EventKind.Completed && kind != EventKind.Discovery)
                return lines[0] + " (×" + Numbers.Whole(run.Count) + ")";
            if (kind == EventKind.Completed)
            {
                var finished = run.Select(e => Completed(e.Text ?? "")).ToList();
                var who = finished.Select(f => f.cat).Where(n => n != "").Distinct().ToList();
                var byKind = finished.GroupBy(f => f.kind).OrderByDescending(g => g.Count()).ThenBy(g => g.Key, StringComparer.Ordinal)
                    .Select(g => Count(g.Key, g.Count())).ToList();
                return People(who) + " finished " + Numbers.Plural(run.Count, "job", "jobs") + " · " + string.Join(", ", byKind);
            }
            if (kind == EventKind.Discovery)
            {
                double places = run.Sum(e => Leading(e.Text));
                var who = run.Select(e => Scout(e, village)).Where(n => n != "").Distinct().ToList();
                return (who.Count == 0 ? "Scouts" : People(who)) + " mapped " + Numbers.Plural(places, "new place", "new places");
            }
            if (kind == EventKind.Tithe)
                return "Food tithes earned " + Numbers.Plural(run.Count, "blessing", "blessings");
            return lines[lines.Count - 1] + " (×" + Numbers.Whole(run.Count) + ")";
        }

        private static string Template(Event e, string text, Village village)
        {
            switch (e.Kind)
            {
                case EventKind.Completed:
                    {
                        var (cat, kind) = Completed(text);
                        if (cat == "")
                            return "";
                        return cat + " " + (Finished.TryGetValue(kind, out var f) ? f.done : "finished a job");
                    }
                case EventKind.Research:
                    return Studied(e, text);
                case EventKind.Upgrade:
                    {
                        const string marker = " reached level ";
                        int at = text.IndexOf(marker, StringComparison.Ordinal);
                        if (at <= 0)
                            return "";
                        string level = Reasons.Clean(text.Substring(at + marker.Length));
                        return Names.Of(NameKind.Upgrade, text.Substring(0, at)) + marker + level;
                    }
                case EventKind.Death:
                    {
                        const string marker = " died from ";
                        int at = text.IndexOf(marker, StringComparison.Ordinal);
                        string cat = at > 0 ? text.Substring(0, at) : Names.Cat(village, e.EntityId);
                        string cause = at > 0 ? text.Substring(at + marker.Length).Trim() : "";
                        if (cat == "" || cat == "a cat")
                            return "";
                        if (!DeathCauses.TryGetValue(cause, out var template))
                            return cat + " has died.";
                        return string.Format(Numbers.Culture, template, cat);
                    }
                case EventKind.Emergency:
                    {
                        const string marker = " preempted work for physical ";
                        int at = text.IndexOf(marker, StringComparison.Ordinal);
                        if (at <= 0)
                            return "";
                        string resource = text.Substring(at + marker.Length).Trim();
                        return text.Substring(0, at) + " dropped their work to fetch " + Names.Resource(resource);
                    }
                case EventKind.Discovery:
                    {
                        if (!text.EndsWith(" notes delivered to shrine", StringComparison.Ordinal))
                            return "";
                        string scout = Scout(e, village);
                        return (scout == "" ? "A scout" : scout) + " mapped " + Numbers.Plural(Leading(text), "new place", "new places");
                    }
                case EventKind.Migration:
                    if (Subject(text, " approaches the gate", out string wanderer))
                        return wanderer + " is at the gate, hoping to join";
                    return text.EndsWith(" left the colony", StringComparison.Ordinal) ? text : "";
                case EventKind.Pregnancy:
                    return Subject(text, " reserved a kitten bed", out string mother) ? mother + " is expecting a kitten" : "";
                case EventKind.Birth:
                    return Subject(text, " was born", out string kitten) ? kitten + " was born!" : "";
                case EventKind.Expansion:
                    return text.StartsWith("Settlement grew", StringComparison.Ordinal) ? "The colony grew · the fence moved out" : "";
                case EventKind.Dungeon:
                    if (Subject(text, " left recovered supplies beside the full storage and sought care", out string explorer))
                        return explorer + " came home hurt and left the loot beside the full stores";
                    return "";
                case EventKind.BuildingCompleted:
                    {
                        string building = Names.Building(village, e.EntityId);
                        return building == "" ? "" : "The " + building + " is finished";
                    }
                case EventKind.StorageFull:
                case EventKind.StorageEmpty:
                    {
                        string pile = Names.Place(village, e.EntityId);
                        if (pile == "")
                            return "";
                        return "The " + pile + (e.Kind == EventKind.StorageFull ? " is full" : " ran empty");
                    }
                case EventKind.Arrival:
                    {
                        string cat = Names.Cat(village, e.EntityId);
                        return cat == "a cat" ? "" : cat + " joined the colony";
                    }
                default:
                    // Fixed engine texts are rewritten by Line; every other text is cleaned there.
                    return "";
            }
        }

        /// <summary>"Studied Textiles · the Clothier and Tannery are unlocked".</summary>
        private static string Studied(Event e, string text)
        {
            var study = Catalog.Study(e.EntityId ?? "");
            string name = study != null ? Names.Study(study.Id) : Reasons.Clean(text);
            if (name == "")
                return "";
            var unlocked = study == null
                ? new List<string>()
                : study.Payloads.Where(p => p.Kind == "unlock_building").Select(p => Names.Building(p.Id)).Distinct().ToList();
            if (unlocked.Count == 0)
                return "Studied " + name;
            return "Studied " + name + " · the " + Names.List(unlocked) + (unlocked.Count == 1 ? " is" : " are") + " unlocked";
        }

        /// <summary>True when <paramref name="text"/> is "&lt;subject&gt;&lt;suffix&gt;"; returns the subject.</summary>
        private static bool Subject(string text, string suffix, out string subject)
        {
            subject = "";
            if (!text.EndsWith(suffix, StringComparison.Ordinal) || text.Length == suffix.Length)
                return false;
            subject = text.Substring(0, text.Length - suffix.Length);
            return true;
        }

        private static (string cat, string kind) Completed(string text)
        {
            int at = text.LastIndexOf(" completed ", StringComparison.Ordinal);
            if (at <= 0)
                return ("", "");
            return (text.Substring(0, at), text.Substring(at + " completed ".Length).Trim());
        }

        private static string Count(string kind, int n)
        {
            var words = Finished.TryGetValue(kind, out var f) ? (f.one, f.many) : ("job", "jobs");
            return Numbers.Plural(n, words.Item1, words.Item2);
        }

        private static string People(List<string> names)
        {
            if (names.Count == 0)
                return "The cats";
            if (names.Count <= 3)
                return Names.List(names);
            return names[0] + ", " + names[1] + " and " + Numbers.Plural(names.Count - 2, "other", "others");
        }

        private static string Scout(Event e, Village village)
        {
            string name = Names.Cat(village, e.EntityId);
            return name == "a cat" ? "" : name;
        }

        private static double Leading(string text)
        {
            if (string.IsNullOrEmpty(text))
                return 0;
            int end = 0;
            while (end < text.Length && char.IsDigit(text[end]))
                end++;
            return end == 0 ? 0 : double.Parse(text.Substring(0, end), Numbers.Culture);
        }
    }
}
