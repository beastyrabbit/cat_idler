using System;

namespace IdleCatForest.Simulation
{
    /// <summary>What a cat is visibly doing, for animation, sound and text. Never parse display strings instead.</summary>
    public enum CatActivity
    {
        Idle, Nap, Groom, Social, Patrol, Play, Walk, Carry, Gather, Hunt, Fish, Build, Craft, Farm, Research, Count,
        Eat, Drink, Sleep, Fight, Explore, Train, Controlled, Leave
    }

    public partial class World
    {
        /// <summary>
        /// Maps a cat's saved Goal, its job's kind and phase, and its migration state to one activity.
        /// Pure: reads the world and changes nothing. Unknown values map to Walk while the cat has a
        /// path and to Idle otherwise.
        /// </summary>
        public CatActivity ActivityOf(Village v, Cat c)
        {
            if (c == null)
                return CatActivity.Idle;
            string goal = c.Goal ?? "";
            if (c.ControlledBy != "")
                return goal == GoalKey.PlayerRest ? CatActivity.Sleep : CatActivity.Controlled;
            if (c.Migration == "departing" || goal == GoalKey.Leaving)
                return CatActivity.Leave;
            if (c.Migration == "arriving")
                return CatActivity.Walk;
            if (IsDungeonExplorer(c))
                return goal == GoalKey.FightingDungeonGuardian ? CatActivity.Fight : CatActivity.Explore;
            switch (goal)
            {
                case GoalKey.NeedDrink:
                    return CatActivity.Drink;
                case GoalKey.NeedEat:
                case GoalKey.NeedHeal:
                    return CatActivity.Eat;
                case GoalKey.NeedSleep:
                case GoalKey.PlayerRest:
                    return CatActivity.Sleep;
                case GoalKey.Defending:
                    return CatActivity.Fight;
                case GoalKey.Research:
                    return CatActivity.Research;
                case GoalKey.IdleNap:
                    return CatActivity.Nap;
                case GoalKey.IdleGroom:
                    return CatActivity.Groom;
                case GoalKey.IdleSocial:
                    return CatActivity.Social;
                case GoalKey.IdlePatrol:
                    return CatActivity.Patrol;
                case GoalKey.IdlePlay:
                    return CatActivity.Play;
                case GoalKey.IdleShrine:
                case GoalKey.RestingAtHome:
                    return Wandering(c);
                case GoalKey.ReturningRailVehicle:
                case GoalKey.ReturningVesselToDock:
                case GoalKey.PlayerControlReturningToDock:
                case GoalKey.TransportReturning:
                    return CatActivity.Carry;
            }
            if (goal.StartsWith(GoalKey.FarmPrefix, StringComparison.Ordinal))
                return CatActivity.Farm;
            if (goal.StartsWith(GoalKey.AccountingPrefix, StringComparison.Ordinal))
                return CatActivity.Count;
            if (goal.StartsWith(GoalKey.ReturningToPrefix, StringComparison.Ordinal)
                || goal.StartsWith("rail" + GoalKey.Separator, StringComparison.Ordinal)
                || goal.StartsWith("shipping" + GoalKey.Separator, StringComparison.Ordinal))
                return CatActivity.Carry;
            var job = v?.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job != null)
                return JobActivity(job.Kind, job.Phase, c);
            // A goal set from a job reads "<kind>" or "<kind> · <phase>".
            int separator = goal.IndexOf(GoalKey.Separator, StringComparison.Ordinal);
            if (separator > 0)
                return JobActivity(goal.Substring(0, separator), goal.Substring(separator + GoalKey.Separator.Length), c);
            return JobActivity(goal, "", c);
        }

        private static CatActivity Wandering(Cat c) => c.Path.Count > 0 ? CatActivity.Walk : CatActivity.Idle;

        private static CatActivity JobActivity(string kind, string phase, Cat c)
        {
            switch (phase)
            {
                case "input_delivery":
                case "output_delivery":
                    return CatActivity.Carry;
                case "fetch":
                case "item_fetch":
                case "return":
                    return c.Cargo.Count > 0 ? CatActivity.Carry : CatActivity.Walk;
                case "travel":
                    return CatActivity.Walk;
            }
            switch (kind)
            {
                case "hunt":
                    return CatActivity.Hunt;
                case "fish":
                    return CatActivity.Fish;
                case "water":
                case "logs":
                case "quarry":
                case "fibre":
                case "gather":
                case "zone_gather":
                    return CatActivity.Gather;
                case "build":
                case "expand":
                case "expand_village":
                case "road":
                case "rail":
                case "bridge":
                case "dock":
                case "wagon":
                case "vessel":
                    return CatActivity.Build;
                case "production":
                case "mouse_farm":
                case "offering":
                    return CatActivity.Craft;
                case "replant":
                    return CatActivity.Farm;
                case "train":
                    return CatActivity.Train;
                case "scout":
                    return CatActivity.Explore;
                case "haul":
                    return CatActivity.Carry;
            }
            // Idle, "production paused" and anything unknown.
            return Wandering(c);
        }
    }
}
