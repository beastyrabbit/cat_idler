using System;
using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    [Serializable]
    public struct Int2 : IEquatable<Int2>
    {
        public int X, Z;
        public Int2(int x, int z)
        {
            X = x;
            Z = z;
        }
        public bool Equals(Int2 p) => X == p.X && Z == p.Z;
        public override bool Equals(object o) => o is Int2 p && Equals(p);
        public override int GetHashCode() => unchecked(X * 397 ^ Z);
        public static int Distance(Int2 a, Int2 b) => Math.Abs(a.X - b.X) + Math.Abs(a.Z - b.Z);
        public override string ToString() => X + "," + Z;
    }
    [Serializable]
    public partial class PlayerContext
    {
        public string PlayerId = "", VillageId = "communal"; public bool IsDeveloper;
    }
    [Serializable]
    public partial class Player
    {
        public string Id = "", SelectedVillageId = "communal", PersonalVillageId = "";
    }
    [Serializable]
    public partial class GameAction
    {
        public string Kind = "", TargetId = "", CatId = "", BuildingId = "", Resource = "", RecipeId = "", NodeId = "", Role = "", Name = "", OtherResource = "", OtherVillageId = "", Edit = "", Labor = "", Mode = "";
        public Int2 Position, End;
        public double Amount = 1, OtherAmount = 1;
        public int Index, Direction;
        public bool Enabled = true, Repeat;
        public List<string> Accepts = new List<string>();
        public List<Int2> Path = new List<Int2>();
    }
    [Serializable]
    public partial class ActionResult
    {
        public bool Success; public string Error = "", EntityId = "", VillageId = "";
        public static ActionResult Ok(string id = "") => new ActionResult { Success = true, EntityId = id };
        public static ActionResult Fail(string message) => new ActionResult { Error = message };
    }
    [Serializable]
    public partial class Stack
    {
        public string Resource = ""; public double Amount; public Stack()
        {
        }
        public Stack(string resource, double amount)
        {
            Resource = resource;
            Amount = amount;
        }
    }
    [Serializable]
    public partial class Stockpile
    {
        public string Id = "", Kind = "storage", ManagedBy = "";
        public Int2 Position; public int Width = 2, Depth = 2; public double Capacity = 500;
        public List<string> Accepts = new List<string>(); public List<Stack> Goods = new List<Stack>(), Report = new List<Stack>();
        public double CountedAt = -1, ExpiresAt = -1;
    }
    [Serializable]
    public partial class Item
    {
        public string Id = "", Kind = "", Material = "", VillageId = "", LocationId = "";
        public int Quality = 1; public double Condition = 100, MaxCondition = 100, Weight = 1;
    }
    [Serializable]
    public partial class QueueEntry
    {
        public string RecipeId = "", AutomatedBy = ""; public bool Repeat;
    }
    [Serializable]
    public partial class WorkSlot
    {
        public string CatId = "", JobId = "", BlockedReason = ""; public bool Paused; public double Progress; public List<QueueEntry> Queue = new List<QueueEntry>(); public List<Stack> Inputs = new List<Stack>(), Outputs = new List<Stack>();
    }
    [Serializable]
    public partial class Reservation
    {
        public string OwnerId = "", VillageId = "", PileId = "", Resource = ""; public double Amount;
    }
    [Serializable]
    public partial class Building
    {
        public string Id = "", Kind = "", WorkerId = "", BlockedReason = "";
        public Int2 Position, Entrance; public bool Completed, Paused, HasEntrance; public int Width = 2, Depth = 2;
        public double Progress, RequiredWork = 60;
        public List<Stack> Required = new List<Stack>(), Inputs = new List<Stack>(), Outputs = new List<Stack>();
        public List<QueueEntry> Queue = new List<QueueEntry>(); public List<string> ExtraWorkerIds = new List<string>();
        public List<WorkSlot> Slots = new List<WorkSlot>();
    }
    [Serializable]
    public partial class Cat
    {
        public string NeedSourceId = ""; public double NextNeedAttemptAt;
        public string Id = "", Name = "", VillageId = "", JobId = "", BuildingId = "", OfficerRole = "", Goal = "idle", BlockedReason = "", ControlledBy = "", BedId = "", Migration = "resident";
        public Int2 Position; public double X, Z, Hunger = 100, Thirst = 100, Rest = 100, Health = 100, AgeHours = 24, PregnantUntil = -1, ProbationUntil = -1, ControlLeaseUntil;
        public bool Alive = true, Boosted; public List<Stack> Cargo = new List<Stack>(), Skills = new List<Stack>();
        public List<string> Preferences = new List<string>(), Equipment = new List<string>();
        public List<Int2> Path = new List<Int2>(), ScoutNotes = new List<Int2>(); public string ResumeJobId = "";
    }
    [Serializable]
    public partial class Job
    {
        public double NextPlanningAt, NextStorageAttemptAt; public string DeliveryPileId = ""; public bool HasObservedPosition; public Int2 ObservedPosition;
        public bool HasWorkStand; public Int2 WorkStand; public int WorkStandIndex = -1;
        public string Id = "", Kind = "", CatId = "", TargetId = "", Resource = "", Phase = "travel", BlockedReason = "", SourceId = "", RecipeId = "", SuspendedCargoPileId = "", AutomatedBy = "";
        public Int2 Position, Origin; public double Progress, RequiredWork = 10, Amount, StartedAt; public bool Manual, Completed; public int PathIndex;
        public List<Stack> Reserved = new List<Stack>(); public List<Int2> Path = new List<Int2>();
        public List<Stack> Local = new List<Stack>(); public List<string> ItemIds = new List<string>();
    }
    [Serializable]
    public partial class Farm
    {
        public string Id = "", Crop = "grain", WorkerId = "", Phase = "soil", BlockedReason = "", AutomatedBy = "";
        public Int2 Position; public int Width = 1, Depth = 1; public double Growth, Harvest; public Int2 Handoff;
    }
    [Serializable]
    public partial class Tile
    {
        public Int2 Position; public string Biome = "forest", Resource = "", ClaimId = ""; public double Amount, RegrowAt = -1;
        public List<Stack> Deposits = new List<Stack>();
        public bool Water, Mountain, Wall, Road, Dirt, Rail, Bridge, Dock;
    }
    [Serializable]
    public partial class Officer
    {
        public string Role = "", CatId = "";
    }
    [Serializable]
    public partial class Event
    {
        public double Time; public string Kind = "", Text = "", EntityId = "";
    }
    [Serializable]
    public partial class TradeOffer
    {
        public Int2 Position; public bool HasContinuousPosition;
        public string Id = "", FromVillageId = "", ToVillageId = "", Status = "offered", CarrierId = "";
        public Stack Offered = new Stack(), Requested = new Stack(); public List<Int2> Path = new List<Int2>(); public int PathIndex; public double Progress;
        public List<Item> OfferedItems = new List<Item>(), RequestedItems = new List<Item>();
    }
    [Serializable]
    public partial class TransportRoute
    {
        public string Id = "", Mode = "", CatId = "", VehicleId = "", SourceId = "", DestinationId = "", Resource = "", Phase = "boarding", BlockedReason = "";
        public double Amount; public bool Repeat, CancelRequested; public List<Int2> Path = new List<Int2>(); public int PathIndex;
    }
    [Serializable]
    public partial class Vehicle
    {
        public bool HasContinuousPosition; public double X, Z, Progress;
        public string Id = "", Mode = "", RouteId = ""; public Int2 Position; public List<Stack> Cargo = new List<Stack>(); public List<string> ItemIds = new List<string>();
    }
    [Serializable]
    public partial class Raid
    {
        public bool HasContinuousPosition; public double X, Z;
        public string Id = "", Phase = "approaching"; public Int2 Position; public double Health = 30, Strength = 30, Progress; public List<Int2> Path = new List<Int2>(); public List<Stack> Loot = new List<Stack>();
    }
    [Serializable]
    public partial class Election
    {
        public double EndAt; public List<Ballot> Votes = new List<Ballot>();
    }
    [Serializable]
    public partial class Ballot
    {
        public string PlayerId = "", CatId = "";
    }
    [Serializable]
    public partial class Trader
    {
        public bool HasContinuousPosition; public double X, Z;
        public string Phase = "absent"; public Int2 Position; public double NextAt = 3600, Until, Coins = 1000; public List<Stack> Goods = new List<Stack>(); public List<Item> Items = new List<Item>();
        public List<Int2> Path = new List<Int2>(); public int PathIndex; public double Progress; public string BlockedReason = "";
    }
    [Serializable]
    public partial class Village
    {
        public int LayoutVersion;
        public string Id = "", Name = "", OwnerId = "", LeaderId = ""; public bool Communal; public Int2 Center; public int Radius = 6, Run = 1;
        public double FoundedAt, ResearchPoints, Blessings, Coins, LastLeaderResearch = -86400, LastMigration, NextElection = 86400;
        public long BoostMinute = -1; public int BoostsUsed;
        public List<Cat> Cats = new List<Cat>(); public List<Building> Buildings = new List<Building>(); public List<Stockpile> Stockpiles = new List<Stockpile>();
        public List<Job> Jobs = new List<Job>(); public List<Farm> Farms = new List<Farm>(); public List<Item> Items = new List<Item>();
        public List<Officer> Officers = new List<Officer>(); public List<string> Research = new List<string>(), Contacts = new List<string>();
        public List<Int2> Known = new List<Int2>(); public List<Event> Events = new List<Event>(); public List<TransportRoute> Routes = new List<TransportRoute>();
        public List<Vehicle> Vehicles = new List<Vehicle>(); public List<Raid> Raids = new List<Raid>(); public Election Election; public Trader Trader = new Trader();
    }
}
