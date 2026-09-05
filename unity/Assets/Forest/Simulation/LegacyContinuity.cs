using System;
using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    // Typed continuity fields used when a maintained SQLite save crosses the engine boundary.
    public partial class Cat
    {
        public List<string> ParentIds = new List<string>();
        public List<Stack> Stats = new List<Stack>(), RoleExperience = new List<Stack>();
        public long BirthUnixMilliseconds;
        public long? DeathUnixMilliseconds;
        public string PregnancyMateId = "", Specialization = "", AppearanceJson = "";
        public Int2? MigrationExterior;
        public string ImportedCargoSource = "";
        public bool PersonalBrewUsed;
    }
    public partial class Item
    {
        public string LocationKind = "stockpile", StationCompartment = "", ActiveJobId = "";
        public bool Credited = true, AutomaticallyIssued;
    }
    public partial class Building
    {
        public int Level = 1;
        public string AutomatedBy = "";
        public bool ConstructionConsumed;
    }
    public partial class WorkSlot
    {
        public string AutomatedBy = "";
    }
    public partial class Job
    {
        public List<BoundaryEdge> CompletionBoundaryEdges = new List<BoundaryEdge>();
        public bool AgriculturalExpansion;
        public string OriginalKind = "", Requester = "player", PendingBuildingKind = "";
        public double YieldMultiplier = 1, SpeedMultiplier = 1;
        public int BoostCount;
        public bool Accepted;
        public string LinkedJobId = "", ScoutMission = "";
        public double CompletionTime = -1;
    }
    [Serializable]
    public class AccountingRound
    {
        public string WorkerId = "", BuildingId = "", Phase = "", TargetId = "";
        public List<string> Pending = new List<string>(), Unreachable = new List<string>();
        public double DwellSeconds;
        public ulong TopologySignature;
    }
    public partial class Village
    {
        public Election KickPetition;
        public bool GrandfatheredRecipes;
        public List<BoundaryEdge> BoundaryEdges = new List<BoundaryEdge>();
        public List<Stack> UpgradeLevels = new List<Stack>();
        public List<Int2> ClaimedTiles = new List<Int2>(), AgriculturalTiles = new List<Int2>();
        public double AutomationTier, GlobalUpgradePoints, ThreatPressure, LastRaidAt = -1, RaidClicks;
        public double LastTitheAt = -1, LastOfferingAt = -1, LastPlayerActivityAt = -1, CriticalSince = -1, RitualRequestedAt = -1;
        public string Status = "active", ActiveRaidId = "";
        public long MigrationDepartures, TraderVisitCount;
        public long? MigrationCohortBucket;
        public AccountingRound Accounting;
        public List<HistoricElection> ElectionHistory = new List<HistoricElection>();
        public List<StewardPile> StewardPiles = new List<StewardPile>();
    }
    [Serializable]
    public class HistoricElection
    {
        public string Id = "", Kind = "", WinnerId = "", TargetId = "";
        public double OpenedAt, ClosesAt, ResolvedAt = -1;
        public List<string> Candidates = new List<string>();
        public List<Ballot> Votes = new List<Ballot>();
    }
    [Serializable]
    public class StewardPile
    {
        public string PileId = "", StationId = "", Resource = ""; public bool Active;
    }
    public partial class Farm
    {
        public double Fertility = 1, PlantedAt, CycleSeconds = 7200, YieldPerTile = 8; public bool FertilityAffectsGrowth; public string Stage = "soil";
    }
    public partial class Tile
    {
        public List<Stack> MaximumDeposits = new List<Stack>();
        public double Danger, FishCapacity, FishReplenishedAt, PathWear, LastDepletedAt;
        public string Overlay = "";
    }
    public partial class Event
    {
        public string Id = "", ActorName = "";
    }
    public partial class TradeOffer
    {
        public bool OfferedDelivered;
        public List<Int2> ReturnPath = new List<Int2>();
        public List<Reservation> OfferedSources = new List<Reservation>(), RequestedSources = new List<Reservation>();
        public double X, Z, AcceptedAt, LastAdvancedAt;
        public string ActorId = "";
    }
    public partial class Trader
    {
        public string Id = ""; public double LastDepartedAt = -1; public Int2? Exterior, VisitDestination;
    }
    public partial class TransportRoute
    {
        public double SegmentProgress; public Int2 Position;
    }
    public partial class Raid
    {
        public string RaidId = ""; public double Defense; public Int2? Destination;
    }
    public partial class Ballot
    {
        public double Weight = 1;
    }
    public partial class Election
    {
        public string Id = "", Kind = "scheduled", TargetId = "";
    }
    [Serializable]
    public class BoundaryEdge
    {
        public Int2 From, To;
    }
    public partial class Stockpile
    {
        public List<Stack> ResourceLimits = new List<Stack>();
    }
}
