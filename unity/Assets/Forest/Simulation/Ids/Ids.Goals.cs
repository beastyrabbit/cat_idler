namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Values of <c>Cat.Goal</c>. They are saved, so every value is byte-identical to the literal it
    /// replaced. A prefix constant ends where a job kind, phase or other suffix is appended.
    /// </summary>
    public static class GoalKey
    {
        public const string Idle = "idle";
        public const string RestingAtHome = "resting at home";
        public const string Leaving = "leaving";
        public const string Research = "research";
        public const string Defending = "defending";
        public const string PlayerControl = "player_control";
        public const string PlayerRest = "player_rest";
        public const string ProductionPaused = "production paused";

        // Needs. Every need goal starts with NeedPrefix.
        public const string NeedPrefix = "need_";
        public const string NeedDrink = "need_drink";
        public const string NeedEat = "need_eat";
        public const string NeedHeal = "need_heal";
        public const string NeedSleep = "need_sleep";

        /// <summary>Separates a goal from its detail, for example a job kind from its phase.</summary>
        public const string Separator = " · ";
        public const string FarmPrefix = "farm · ";
        public const string AccountingPrefix = "accounting · ";
        public const string AccountingReturning = "accounting · returning to tent";

        // Dungeon expeditions.
        public const string ExploringDungeon = "exploring dungeon";
        public const string ReturningFromDungeon = "returning from dungeon";
        public const string DescendingDungeonStairs = "descending dungeon stairs";
        public const string FightingDungeonGuardian = "fighting dungeon guardian";
        public const string CollectingDungeonSupplies = "collecting dungeon supplies";

        // Transport drivers.
        public const string ReturningToPrefix = "returning to ";
        public const string ReturningRailVehicle = "returning rail vehicle to its reached tile";
        public const string ReturningVesselToDock = "returning vessel to dock";
        public const string PlayerControlReturningToDock = "player_control · returning to dock";
        public const string TransportReturning = "transport returning";

        // Idle activities, chosen by TickIdle; World.IsIdleActivity recognises exactly these.
        public const string IdleNap = "idle_nap";
        public const string IdleGroom = "idle_groom";
        public const string IdleSocial = "idle_social";
        public const string IdlePatrol = "idle_patrol";
        public const string IdleShrine = "idle_shrine";
        public const string IdlePlay = "idle_play";
    }
}
