using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The "now line" of GUIDE section 7: what a cat is doing, in one line, for the card, the census and
    /// the attention popover ("Bringing water to the Food Store · 9 tiles", "Napping in Den 2",
    /// "You're steering Hazel"), plus the cause line from its blocked reason.
    /// </summary>
    public static class NowLines
    {
        /// <summary>Context-free text for the fixed goal values (GoalKey).</summary>
        private static readonly Dictionary<string, string> Goals = new Dictionary<string, string>
        {
            [GoalKey.Idle] = "Idle — waiting for a job",
            [GoalKey.RestingAtHome] = "Resting at home",
            [GoalKey.Leaving] = "Leaving the colony",
            [GoalKey.Research] = "Studying at the Research Hut",
            [GoalKey.Defending] = "Defending the colony",
            [GoalKey.PlayerControl] = "Steered by you",
            [GoalKey.PlayerRest] = "Resting while you steer",
            [GoalKey.ProductionPaused] = "Waiting — production is paused",
            [GoalKey.NeedDrink] = "Going for a drink",
            [GoalKey.NeedEat] = "Going for a meal",
            [GoalKey.NeedHeal] = "Getting medicine",
            [GoalKey.NeedSleep] = "Going to sleep",
            [GoalKey.AccountingReturning] = "Heading back to the Accounting Tent",
            [GoalKey.ExploringDungeon] = "Exploring the dungeon",
            [GoalKey.ReturningFromDungeon] = "Heading home from the dungeon",
            [GoalKey.DescendingDungeonStairs] = "Taking the stairs down",
            [GoalKey.FightingDungeonGuardian] = "Fighting the guardian",
            [GoalKey.CollectingDungeonSupplies] = "Gathering supplies in the dungeon",
            [GoalKey.ReturningRailVehicle] = "Bringing the rail cart back",
            [GoalKey.ReturningVesselToDock] = "Sailing the boat back to the dock",
            [GoalKey.PlayerControlReturningToDock] = "Sailing back to the dock",
            [GoalKey.TransportReturning] = "Heading back after a delivery",
            [GoalKey.IdleNap] = "Napping",
            [GoalKey.IdleGroom] = "Grooming",
            [GoalKey.IdleSocial] = "Chatting with friends",
            [GoalKey.IdlePatrol] = "Patrolling the fence",
            [GoalKey.IdleShrine] = "Visiting the shrine",
            [GoalKey.IdlePlay] = "Playing",
        };

        /// <summary>Goal prefixes followed by a detail, with the text when the detail is unknown.</summary>
        private static readonly (string prefix, string text)[] Prefixes =
        {
            (GoalKey.FarmPrefix, "Tending the field"),
            (GoalKey.AccountingPrefix, "Counting the stores"),
            (GoalKey.ReturningToPrefix, "Heading back after a delivery"),
            (GoalKey.NeedPrefix, "Seeing to a need"),
        };

        private static readonly Dictionary<string, string> FarmPhases = new Dictionary<string, string>
        {
            ["soil"] = "Preparing the soil",
            ["sprout"] = "Tending the seedlings",
            ["growing"] = "Tending the field",
            ["mature"] = "Tending the field",
            ["flowering"] = "Tending the field",
            ["harvesting"] = "Harvesting",
            ["hauling"] = "Carrying the harvest",
        };

        /// <summary>Request aliases and legacy kinds fold into the kind whose verbs they share.</summary>
        private static readonly Dictionary<string, string> KindAlias = new Dictionary<string, string>
        {
            [JobKind.SupplyFood] = JobKind.Hunt,
            [JobKind.LeaderPlanHunt] = JobKind.Hunt,
            [JobKind.HuntExpedition] = JobKind.Hunt,
            [JobKind.SupplyWater] = JobKind.Water,
            [JobKind.FetchWater] = JobKind.Water,
            [JobKind.GatherLogs] = JobKind.Logs,
            [JobKind.ForageFibre] = JobKind.Fibre,
            [JobKind.ReplantTree] = JobKind.Replant,
            [JobKind.Explore] = JobKind.Scout,
            [JobKind.TrainWarrior] = JobKind.Train,
            [JobKind.ExpandVillage] = JobKind.Expand,
            [JobKind.LeaderPlanHouse] = JobKind.Build,
            [JobKind.BuildHouse] = JobKind.Build,
            [JobKind.Ritual] = JobKind.Offering,
            [JobKind.CarryOffering] = JobKind.Offering,
            [JobKind.PerformOffering] = JobKind.Offering,
            ["zone_gather"] = "gather",
        };

        /// <summary>What a cat does at the work spot, per job kind ("Hunting", "Cutting wood").</summary>
        private static readonly Dictionary<string, string> Working = new Dictionary<string, string>
        {
            [JobKind.Hunt] = "Hunting",
            [JobKind.Water] = "Filling up with water",
            [JobKind.Logs] = "Cutting wood",
            [JobKind.Quarry] = "Quarrying stone",
            [JobKind.Fibre] = "Gathering fibre",
            [JobKind.Fish] = "Fishing",
            [JobKind.Scout] = "Scouting the forest",
            [JobKind.Replant] = "Planting a sapling",
            [JobKind.Train] = "Training to fight",
            [JobKind.Expand] = "Building the new fence",
            [JobKind.Haul] = "Hauling",
            [JobKind.Build] = "Building",
            [JobKind.Production] = "Working at the workshop",
            [JobKind.Offering] = "Preparing an offering",
            [JobKind.MouseFarm] = "Tending the mice",
            [JobKind.Road] = "Laying a road",
            [JobKind.Rail] = "Laying track",
            [JobKind.Bridge] = "Building a bridge",
            [JobKind.Dock] = "Building a dock",
            [JobKind.Wagon] = "Building a wagon",
            [JobKind.Vessel] = "Building a boat",
            ["gather"] = "Gathering",
            ["rail"] = "Driving the rail cart",
            ["shipping"] = "Sailing a boat",
        };

        /// <summary>Where a cat is heading before the work starts ("Heading out to hunt").</summary>
        private static readonly Dictionary<string, string> Travelling = new Dictionary<string, string>
        {
            [JobKind.Hunt] = "Heading out to hunt",
            [JobKind.Water] = "Going to the water",
            [JobKind.Logs] = "Heading into the woods for logs",
            [JobKind.Quarry] = "Heading to the quarry",
            [JobKind.Fibre] = "Going to gather fibre",
            [JobKind.Fish] = "Going fishing",
            [JobKind.Scout] = "Scouting the forest",
            [JobKind.Replant] = "Going to plant a sapling",
            [JobKind.Train] = "Going to train",
            [JobKind.Expand] = "Walking to the new fence",
            [JobKind.Build] = "Walking to the building site",
            [JobKind.Production] = "Walking to the workshop",
            [JobKind.Offering] = "Walking to the Shrine",
            [JobKind.MouseFarm] = "Walking to the Mouse Farm",
            [JobKind.Road] = "Walking to the new road",
            [JobKind.Rail] = "Walking to the new track",
            [JobKind.Bridge] = "Walking to the bridge site",
            [JobKind.Dock] = "Walking to the dock site",
            [JobKind.Wagon] = "Walking to the wagon",
            [JobKind.Vessel] = "Walking to the boat",
            ["gather"] = "Going to gather",
        };

        private static readonly Dictionary<string, string> Crafting = new Dictionary<string, string>
        {
            ["process"] = "Preparing",
            ["craft"] = "Crafting",
            ["textile"] = "Weaving",
            ["metalwork"] = "Forging",
            ["mill"] = "Milling",
            ["woodcut"] = "Sawing",
            ["quarry"] = "Cutting",
            ["research"] = "Studying",
        };

        /// <summary>
        /// The goal value alone as a short line, for lists without world context: "Hunting",
        /// "Going for a drink", "Napping". Never the raw goal.
        /// </summary>
        public static string GoalText(string goal)
        {
            if (string.IsNullOrEmpty(goal))
                return Goals[GoalKey.Idle];
            if (Goals.TryGetValue(goal, out var fixedText))
                return fixedText;
            if (goal.StartsWith(GoalKey.FarmPrefix, StringComparison.Ordinal))
                return FarmPhases.TryGetValue(goal.Substring(GoalKey.FarmPrefix.Length), out var farm) ? farm : "Tending the field";
            foreach (var (prefix, text) in Prefixes)
                if (goal.StartsWith(prefix, StringComparison.Ordinal))
                    return text;
            SplitJob(goal, out string kind, out string phase);
            return JobText(kind, phase);
        }

        /// <summary>True when the goal value resolves to authored text (Coverage).</summary>
        public static bool IsAuthoredGoal(string goal)
        {
            if (string.IsNullOrEmpty(goal))
                return false;
            if (Goals.ContainsKey(goal))
                return true;
            foreach (var (prefix, _) in Prefixes)
                if (goal.StartsWith(prefix, StringComparison.Ordinal))
                    return true;
            SplitJob(goal, out string kind, out _);
            return Working.ContainsKey(Normalise(kind));
        }

        /// <summary>True when the job kind has authored now-line verbs (Coverage).</summary>
        public static bool IsAuthoredJob(string kind) => Working.ContainsKey(Normalise(kind));

        /// <summary>The now line for a cat in its village. Empty for no cat.</summary>
        public static string NowLine(World world, Village village, Cat cat)
        {
            if (cat == null)
                return "";
            if (!cat.Alive)
                return "Remembered by the colony";
            string goal = cat.Goal ?? "";
            if (cat.ControlledBy != "")
                return goal == GoalKey.PlayerRest ? "Resting · you're steering " + cat.Name : "You're steering " + cat.Name;
            if (cat.Migration == "departing" || goal == GoalKey.Leaving)
                return "Leaving the colony";
            if (cat.Migration == "arriving")
                return "Arriving at the gate" + Distance(cat);
            if (world != null && world.IsDungeonExplorer(cat))
                return Dungeon(world, cat, goal);

            if (goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                return Need(world, village, cat, goal);

            switch (goal)
            {
                case GoalKey.IdleNap:
                    return "Napping" + In(village, cat.BedId, cat.Path.Count == 0);
                case GoalKey.RestingAtHome:
                    if (cat.Path.Count > 0)
                    {
                        string home = Names.Building(village, cat.BedId);
                        return (home == "" ? "Heading home" : "Heading to " + home) + Distance(cat);
                    }
                    return "Resting" + In(village, cat.BedId, true, " at home");
                case GoalKey.Research:
                    {
                        string hut = Names.Building(village, cat.BuildingId);
                        return "Studying at the " + (hut == "" ? Names.Building("research_hut") : hut);
                    }
                case GoalKey.ProductionPaused:
                    return "Waiting at the " + Station(village, cat) + " — production is paused";
            }
            if (Goals.ContainsKey(goal) && goal != GoalKey.Idle)
                return Goals[goal];
            if (goal.StartsWith(GoalKey.FarmPrefix, StringComparison.Ordinal))
                return Farm(village, cat, goal.Substring(GoalKey.FarmPrefix.Length));
            if (goal.StartsWith(GoalKey.AccountingPrefix, StringComparison.Ordinal))
            {
                string pile = Names.Pile(village, goal.Substring(GoalKey.AccountingPrefix.Length));
                return (pile == "" ? "Counting the stores" : "Counting the " + pile) + Distance(cat);
            }

            var job = village?.Jobs.Find(j => j.Id == cat.JobId && !j.Completed);
            if (job != null)
                return Job(world, village, cat, job);
            if ((goal == GoalKey.Idle || goal == "") && cat.BuildingId != "")
                return AtStation(village, cat);
            if (goal == GoalKey.Idle || goal == "")
                return Idle(world, cat);
            return GoalText(goal) + Distance(cat);
        }

        /// <summary>
        /// "Because there is no path · then back to hauling": the cause behind a waiting cat, or "" when
        /// nothing blocks it. Names the officer who ordered automated work.
        /// </summary>
        public static string CauseLine(World world, Village village, Cat cat)
        {
            if (cat == null || !cat.Alive)
                return "";
            string code = cat.BlockedReason;
            var job = village?.Jobs.Find(j => j.Id == cat.JobId && !j.Completed);
            if (string.IsNullOrEmpty(code) && job != null)
                code = job.BlockedReason;
            if (!string.IsNullOrEmpty(code))
            {
                string cause = Reasons.Cause(code);
                return cause == "" ? "" : "Because " + cause;
            }
            if (job == null || cat.ControlledBy != "")
                return "";
            string owner = Owner(village, job);
            return owner == "" ? "" : "Because " + owner;
        }

        /// <summary>
        /// Who asked for a job: "you asked", "Steward Hazel ordered a haul", "the Leader's safety floor".
        /// </summary>
        public static string Owner(Village village, Job job)
        {
            if (job == null)
                return "";
            string noun = Names.Lower(Names.Job(job.Kind));
            if (job.Manual)
                return "you asked for a " + noun;
            if (!string.IsNullOrEmpty(job.AutomatedBy))
            {
                var officer = village?.Officers.Find(o => o.Role == job.AutomatedBy);
                var cat = officer == null ? null : village.Cats.Find(c => c.Id == officer.CatId && c.Alive);
                string title = Names.Role(job.AutomatedBy);
                return (cat == null ? "the " + title : title + " " + cat.Name) + " ordered " + Article(noun);
            }
            return "";
        }

        private static string Job(World world, Village village, Cat cat, Job job)
        {
            string kind = Normalise(job.Kind);
            string phase = job.Phase ?? "";
            string blocked = job.BlockedReason ?? "";
            string station = kind == JobKind.Production || kind == JobKind.Build ? Names.Building(village, job.TargetId) : "";

            if (blocked != "" && phase != JobPhase.Travel && Reasons.Cause(blocked) != "")
            {
                string at = station != "" ? station : Names.Place(village, job.TargetId);
                return "Waiting" + (at == "" ? "" : " at the " + at) + " — " + Reasons.Cause(blocked);
            }

            switch (phase)
            {
                case JobPhase.Fetch:
                case JobPhase.ItemFetch:
                    {
                        string source = Names.Place(village, job.SourceId);
                        if (source == "" && world != null)
                        {
                            var claim = world.Reservations.Find(r => r.OwnerId == job.Id);
                            source = claim == null ? "" : Names.Pile(village, claim.PileId);
                        }
                        string what = phase == JobPhase.ItemFetch ? "an item" : Names.Resource(job.Resource);
                        if (cat.Cargo.Count > 0)
                            break;
                        if (what == "")
                            what = "supplies";
                        return (source == "" ? "Fetching " + what : "Going to the " + source + " for " + what) + Distance(cat);
                    }
                case JobPhase.InputDelivery:
                case JobPhase.OutputDelivery:
                    break;
                case JobPhase.Travel:
                    return Travel(village, kind, job) + Distance(cat);
                case JobPhase.Working:
                    return Work(world, village, cat, kind, job);
                case JobPhase.Return:
                    if (kind == JobKind.Scout)
                        return "Bringing scouting notes home" + Distance(cat);
                    return "Heading home" + Distance(cat);
                default:
                    return JobText(kind, phase) + Distance(cat);
            }

            // Carrying: delivery phases and fetches that already hold the cargo.
            string destination;
            if (phase == JobPhase.InputDelivery && station != "")
                destination = station + (kind == JobKind.Build ? " site" : "");
            else
            {
                destination = Names.Place(village, job.DeliveryPileId);
                if (destination == "")
                    destination = Names.Place(village, job.TargetId);
            }
            string cargo = Cargo(cat, job);
            if (cargo == "")
                return (destination == "" ? "Carrying supplies home" : "Heading to the " + destination) + Distance(cat);
            return "Bringing " + cargo + (destination == "" ? " home" : " to the " + destination) + Distance(cat);
        }

        private static string Work(World world, Village village, Cat cat, string kind, Job job)
        {
            switch (kind)
            {
                case JobKind.Production:
                    {
                        var recipe = world?.RecipeOf(job.RecipeId) ?? Catalog.Recipe(job.RecipeId);
                        string station = Names.Building(village, job.TargetId);
                        if (station == "")
                            station = recipe == null ? "workshop" : Names.Building(recipe.Building);
                        if (recipe == null)
                            return "Working at the " + station;
                        string verb = Crafting.TryGetValue(recipe.Labor ?? "", out var v) ? v : "Making";
                        return verb + " " + recipe.Name + " at the " + station;
                    }
                case JobKind.Build:
                    {
                        string site = Names.Building(village, job.TargetId);
                        return site == "" ? "Building" : "Building the " + site;
                    }
                case JobKind.Haul:
                    return "Hauling" + Distance(cat);
                case JobKind.Water:
                    return "Filling up with water";
                default:
                    return (Working.TryGetValue(kind, out var text) ? text : JobText(kind, JobPhase.Working)) + Distance(cat);
            }
        }

        private static string Travel(Village village, string kind, Job job)
        {
            if (kind == JobKind.Build)
            {
                string site = Names.Building(village, job.TargetId);
                return site == "" ? "Walking to the building site" : "Walking to the " + site + " site";
            }
            if (kind == JobKind.Production)
            {
                string station = Names.Building(village, job.TargetId);
                return station == "" ? "Walking to the workshop" : "Walking to the " + station;
            }
            return Travelling.TryGetValue(kind, out var text) ? text : JobText(kind, JobPhase.Travel);
        }

        private static string Need(World world, Village village, Cat cat, string goal)
        {
            if (goal == GoalKey.NeedSleep)
            {
                string den = Names.Building(village, cat.BedId);
                if (cat.Path.Count > 0)
                    return "Going to sleep" + (den == "" ? "" : " in " + den) + Distance(cat);
                string wake = Wake(world, village, cat);
                return "Sleeping" + (den == "" ? "" : " · " + den) + wake;
            }
            string want = goal == GoalKey.NeedDrink ? "a drink" : goal == GoalKey.NeedHeal ? "medicine" : "a meal";
            string doing = goal == GoalKey.NeedDrink ? "Drinking" : goal == GoalKey.NeedHeal ? "Taking medicine" : "Eating";
            if (cat.Cargo.Count == 0)
            {
                string source = Names.Pile(village, cat.NeedSourceId);
                return (source == "" ? "Looking for " + want : "Going to the " + source + " for " + want) + Distance(cat);
            }
            return cat.Path.Count > 0 ? "Taking " + want + " to the shrine" + Distance(cat) : doing + " at the shrine";
        }

        private static string Wake(World world, Village village, Cat cat)
        {
            if (world == null)
                return "";
            var pace = world.CurrentPacing();
            double rate = pace.SleepRecoveryPerSecond * (village == null ? 1 : Catalog.Effect(village, "restRecovery", 1));
            if (rate <= 0 || cat.Rest >= pace.SleepWakeRest)
                return "";
            double seconds = (pace.SleepWakeRest - cat.Rest) / rate;
            return " · wakes at " + GameClock.TimeOfDay(world.TimeSeconds + seconds, pace.DayScale);
        }

        private static string Farm(Village village, Cat cat, string phase)
        {
            var farm = village?.Farms.Find(f => f.WorkerId == cat.Id);
            string field = farm == null ? "" : Names.Farm(village, farm);
            string text = FarmPhases.TryGetValue(phase, out var t) ? t : "Tending the field";
            if (field == "")
                return text + Distance(cat);
            switch (phase)
            {
                case "harvesting":
                    return "Harvesting · " + field;
                case "hauling":
                    return "Carrying the harvest from the " + field + Distance(cat);
                case "soil":
                    return "Preparing the soil · " + field;
                default:
                    return "Tending the " + field + Distance(cat);
            }
        }

        private static string Dungeon(World world, Cat cat, string goal)
        {
            var site = world.Dungeons.Find(d => d.Id == cat.DungeonId);
            string name = site == null || site.Name == "" ? "the dungeon" : site.Name;
            int floor = cat.Position.Level < 0 ? -cat.Position.Level : cat.DungeonFloorIndex + 1;
            string floors = site != null && site.Depth > 0 && cat.Position.Level < 0 ? " · floor " + Numbers.Of(floor, site.Depth) : "";
            string cargo = cat.Cargo.Count > 0 ? " · carrying " + UiGoods(cat.Cargo) : "";
            switch (goal)
            {
                case GoalKey.FightingDungeonGuardian:
                    return "Fighting the guardian of " + name + floors;
                case GoalKey.DescendingDungeonStairs:
                    return "Taking the stairs down in " + name + floors;
                case GoalKey.CollectingDungeonSupplies:
                    return "Gathering supplies in " + name + floors + cargo;
                case GoalKey.ReturningFromDungeon:
                    return "Heading home from " + name + cargo + Distance(cat);
                default:
                    return "Exploring " + name + floors + cargo;
            }
        }

        /// <summary>A cat staffing a station between jobs: "Waiting at the Sawmill — nothing is queued".</summary>
        private static string AtStation(Village village, Cat cat)
        {
            var building = village?.Buildings.Find(b => b.Id == cat.BuildingId);
            string station = building == null ? "" : Names.Building(village, building);
            if (station == "")
                return "Waiting at the workshop";
            if (cat.Path.Count > 0)
                return "Walking to the " + station + Distance(cat);
            string reason = building.Slots.Find(s => s.CatId == cat.Id)?.BlockedReason;
            if (string.IsNullOrEmpty(reason))
                reason = building.BlockedReason;
            string cause = Reasons.Cause(reason);
            return "Waiting at the " + station + (cause == "" ? "" : " — " + cause);
        }

        private static string Idle(World world, Cat cat)
        {
            if (cat.Path.Count > 0)
                return "Wandering" + Distance(cat);
            if (world != null && cat.AgeHours < world.CurrentPacing().WorkerMinAgeHours)
                return "Idle — too young to work yet";
            return "Idle — waiting for a job";
        }

        private static string JobText(string kind, string phase)
        {
            kind = Normalise(kind);
            switch (phase)
            {
                case JobPhase.Travel:
                    return Travelling.TryGetValue(kind, out var travel) ? travel : "On the way to work";
                case JobPhase.Return:
                    return "Heading home";
                case JobPhase.Fetch:
                case JobPhase.ItemFetch:
                    return kind == JobKind.Build ? "Fetching building materials" : "Fetching supplies";
                case JobPhase.InputDelivery:
                    return kind == JobKind.Build ? "Bringing materials to the building site" : "Bringing supplies to work";
                case JobPhase.OutputDelivery:
                    return "Bringing the haul home";
            }
            if (Working.TryGetValue(kind, out var working))
                return working;
            string noun = Names.Job(kind);
            return noun == "" ? "Busy" : "Busy with " + Article(Names.Lower(noun));
        }

        private static string Cargo(Cat cat, Job job)
        {
            var stack = cat.Cargo.OrderByDescending(s => s.Amount).FirstOrDefault();
            if (stack == null)
                return job.Resource == "" ? "" : Names.Resource(job.Resource);
            // Masses read without a number ("water"); pieces with one ("3 logs").
            string load = Names.Countable(stack.Resource) ? Names.Amount(stack.Amount, stack.Resource) : Names.Resource(stack.Resource);
            return cat.Cargo.Count > 1 ? load + " and more" : load;
        }

        private static string UiGoods(IEnumerable<Stack> goods) =>
            Names.List(goods.Where(s => s.Amount > 0).Select(s => Names.Amount(s.Amount, s.Resource)));

        private static string In(Village village, string buildingId, bool arrived, string fallback = "")
        {
            string place = Names.Building(village, buildingId);
            if (place == "" || !arrived)
                return fallback;
            return " in " + place;
        }

        private static string Station(Village village, Cat cat)
        {
            var job = village?.Jobs.Find(j => j.Id == cat.JobId && !j.Completed);
            string station = job == null ? "" : Names.Building(village, job.TargetId);
            if (station == "")
                station = Names.Building(village, cat.BuildingId);
            return station == "" ? "workshop" : station;
        }

        private static string Distance(Cat cat) => cat.Path.Count > 0 ? " · " + Numbers.Tiles(cat.Path.Count) : "";

        private static string Normalise(string kind)
        {
            if (string.IsNullOrEmpty(kind))
                return "";
            return KindAlias.TryGetValue(kind, out var alias) ? alias : kind;
        }

        private static void SplitJob(string goal, out string kind, out string phase)
        {
            int separator = goal.IndexOf(GoalKey.Separator, StringComparison.Ordinal);
            kind = separator > 0 ? goal.Substring(0, separator) : goal;
            phase = separator > 0 ? goal.Substring(separator + GoalKey.Separator.Length) : "";
        }

        private static string Article(string noun)
        {
            if (string.IsNullOrEmpty(noun))
                return "";
            return ("aeiou".IndexOf(char.ToLowerInvariant(noun[0])) >= 0 ? "an " : "a ") + noun;
        }
    }
}
