using System;

namespace IdleCatForest.Simulation
{
    /// <summary>What a cat is visibly doing, for animation, sound and text. Never parse display strings instead.</summary>
    public enum CatActivity
    {
        Idle, Nap, Groom, Social, Patrol, Play, Walk, Carry, Gather, Hunt, Fish, Build, Craft, Farm, Research, Count,
        Eat, Drink, Sleep, Fight, Explore, Train, Controlled, Leave,
        /// <summary>Sitting by the shrine.</summary>
        Shrine
    }

    public partial class World
    {
        /// <summary>
        /// Maps a cat's saved Goal, its job's kind and phase, and its migration state to one activity.
        /// Pure: reads the world and changes nothing. Every goal the simulation sets has its own
        /// activity; unknown values (old saves, future kinds) map to Walk while the cat has a path and
        /// to Idle otherwise.
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
                case GoalKey.PlayerControl:
                    return CatActivity.Controlled;
                case GoalKey.Defending:
                case GoalKey.FightingDungeonGuardian:
                    return CatActivity.Fight;
                case GoalKey.ExploringDungeon:
                case GoalKey.ReturningFromDungeon:
                case GoalKey.DescendingDungeonStairs:
                case GoalKey.CollectingDungeonSupplies:
                    return CatActivity.Explore;
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
                    return CatActivity.Shrine;
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
            // A live job decides first: a rail construction job and a rail route both read "rail · <phase>".
            var job = v?.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job != null)
                return JobActivity(job.Kind, job.Phase, c);
            if (goal.StartsWith(GoalKey.ReturningToPrefix, StringComparison.Ordinal)
                || goal.StartsWith(JobKind.Rail + GoalKey.Separator, StringComparison.Ordinal)
                || goal.StartsWith("shipping" + GoalKey.Separator, StringComparison.Ordinal))
                return CatActivity.Carry;
            // A goal set from a job reads "<kind>" or "<kind> · <phase>". Idle, "resting at home" and
            // "production paused" are no job kind, so they end as Walk or Idle below.
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
                case JobPhase.InputDelivery:
                case JobPhase.OutputDelivery:
                    return CatActivity.Carry;
                case JobPhase.Fetch:
                case JobPhase.ItemFetch:
                case JobPhase.Return:
                    return c.Cargo.Count > 0 ? CatActivity.Carry : CatActivity.Walk;
                case JobPhase.Travel:
                    return CatActivity.Walk;
            }
            var activity = KindActivity(kind);
            return activity ?? Wandering(c);
        }

        /// <summary>The working activity of a job kind, including legacy kinds from imported saves; null when unknown.</summary>
        private static CatActivity? KindActivity(string kind)
        {
            switch (kind)
            {
                case JobKind.Hunt:
                case JobKind.HuntExpedition:
                case JobKind.LeaderPlanHunt:
                case JobKind.SupplyFood:
                    return CatActivity.Hunt;
                case JobKind.Fish:
                    return CatActivity.Fish;
                case JobKind.Water:
                case JobKind.SupplyWater:
                case JobKind.FetchWater:
                case JobKind.Logs:
                case JobKind.GatherLogs:
                case JobKind.Quarry:
                case JobKind.Fibre:
                case JobKind.ForageFibre:
                case "gather":
                case "zone_gather":
                    return CatActivity.Gather;
                case JobKind.Build:
                case JobKind.BuildHouse:
                case JobKind.LeaderPlanHouse:
                case JobKind.Expand:
                case JobKind.ExpandVillage:
                case JobKind.Road:
                case JobKind.Rail:
                case JobKind.Bridge:
                case JobKind.Dock:
                case JobKind.Wagon:
                case JobKind.Vessel:
                    return CatActivity.Build;
                case JobKind.Production:
                case JobKind.MouseFarm:
                case JobKind.Offering:
                case JobKind.Ritual:
                case JobKind.PerformOffering:
                    return CatActivity.Craft;
                case JobKind.Replant:
                case JobKind.ReplantTree:
                    return CatActivity.Farm;
                case JobKind.Train:
                case JobKind.TrainWarrior:
                    return CatActivity.Train;
                case JobKind.Scout:
                case JobKind.Explore:
                    return CatActivity.Explore;
                case JobKind.Haul:
                case JobKind.CarryOffering:
                    return CatActivity.Carry;
            }
            return null;
        }
    }
}
