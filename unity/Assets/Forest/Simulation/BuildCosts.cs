using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>What one construction costs: its material bill and the work it needs.</summary>
    public sealed class BuildBill
    {
        public string Kind = "";
        public List<Stack> Inputs = new List<Stack>();
        public double Work;
        /// <summary>How many existing buildings of the kind the price counted (buildings only).</summary>
        public int Counted;
    }

    /// <summary>A material the colony lacks for a bill: how much the bill needs and how much is free to claim.</summary>
    public sealed class MaterialShortfall
    {
        public string Resource = "";
        public double Need, Have;
    }

    /// <summary>
    /// The one place construction prices come from. Plan, ResolveScaffold, the placement check and the build
    /// tray all read these functions, so the price a player sees is the price the rules charge.
    /// </summary>
    public partial class World
    {
        // PACING-HANDOFF BuildWorkBase: work for the first building of a kind (today's literal in Plan).
        private const double BuildWorkBase = 120;
        // PACING-HANDOFF BuildWorkPerExisting: extra work per counted building of the same kind.
        private const double BuildWorkPerExisting = 30;
        // PACING-HANDOFF BridgeLumber and DockLumber: today's literals in the bridge and dock handlers.
        private const double BridgeLumber = 4;
        private const double DockLumber = 8;
        // PACING-HANDOFF DemolishRefundShare: share of the bill a demolished building returns.
        private const double DemolishRefundShare = 0.5;

        /// <summary>
        /// The demo construction rules: player-built pricing, queued plans, several builders per site, full-load
        /// wall and road trips and natural expansion barriers. Classic worlds keep today's behaviour.
        /// </summary>
        private bool DemoConstruction => Pace.Profile == Pacing.DemoProfile;

        /// <summary>
        /// True for a building the players paid for through a plan (it carries a bill). Founding and imported
        /// buildings have no bill and never raise the price of the next one.
        /// </summary>
        public static bool PlayerBuilt(Building b) => b != null && b.Required.Count > 0;

        /// <summary>
        /// The bill for the next building of <paramref name="kind"/> in <paramref name="v"/>. Timber is paid in
        /// lumber when enough is free, otherwise in planks. Demo worlds count player-built buildings only;
        /// classic worlds count every building of the kind (today's rule).
        /// </summary>
        public BuildBill BuildCost(Village v, string kind)
        {
            kind = Snake(kind);
            int counted = v.Buildings.Count(b => b.Kind == kind && (!DemoConstruction || PlayerBuilt(b)));
            double timber = Pace.ScaffoldTimberBase + counted * Pace.ScaffoldTimberPerExisting, blocks = Pace.ScaffoldBlocksBase + counted * Pace.ScaffoldBlocksPerExisting;
            string timberKind = Available(v, "lumber") >= timber ? "lumber" : "planks";
            return new BuildBill { Kind = kind, Counted = counted, Work = BuildWorkBase + counted * BuildWorkPerExisting, Inputs = new List<Stack> { new Stack(timberKind, timber), new Stack("blocks", blocks) } };
        }

        /// <summary>A road or rail line: one unit of materials (road) or metal (rail) per tile.</summary>
        public BuildBill LineCost(string kind, int tiles) => new BuildBill { Kind = kind, Work = Pace.LinearWorkPerTile * tiles, Inputs = new List<Stack> { new Stack(kind == JobKind.Rail ? "metal" : "materials", tiles) } };

        /// <summary>A bridge or a dock: a fixed amount of lumber.</summary>
        public BuildBill InfrastructureCost(string kind) => new BuildBill { Kind = kind, Work = Pace.InfrastructureWork, Inputs = new List<Stack> { new Stack("lumber", kind == "dock" ? DockLumber : BridgeLumber) } };

        /// <summary>An expansion ring: one unit of materials per wall tile.</summary>
        public BuildBill ExpansionCost(int wallTiles) => new BuildBill { Kind = JobKind.Expand, Work = Pace.ExpansionWork, Inputs = new List<Stack> { new Stack("materials", wallTiles) } };

        /// <summary>The materials a demolition returns: half of what the building cost, rounded down.</summary>
        private List<Stack> DemolishRefund(Village v, Building b)
        {
            IEnumerable<Stack> bill = PlayerBuilt(b) ? b.Required : new[] { new Stack("planks", Pace.ScaffoldTimberBase), new Stack("blocks", Pace.ScaffoldBlocksBase) };
            return bill.Select(s => new Stack(s.Resource, Math.Floor(s.Amount * DemolishRefundShare))).Where(s => s.Amount > 0).ToList();
        }

        /// <summary>
        /// Every resource of <paramref name="bill"/> the colony cannot claim right now, with the amount needed and
        /// the amount free (stored and unreserved). Empty when the whole bill is affordable.
        /// </summary>
        public List<MaterialShortfall> Shortfalls(Village v, IEnumerable<Stack> bill)
        {
            var result = new List<MaterialShortfall>();
            foreach (var s in bill)
            {
                double have = Available(v, s.Resource);
                if (have + 1e-9 < s.Amount)
                    result.Add(new MaterialShortfall { Resource = s.Resource, Need = s.Amount, Have = have });
            }
            return result;
        }

        /// <summary>A coded failure that names the first missing resource with the amount needed and free.</summary>
        private static ActionResult MissingMaterialFailure(MaterialShortfall s) => ActionResult.Fail(ResultCode.MissingMaterial, "Needs " + Number(s.Need) + " " + s.Resource + " (have " + Number(s.Have) + ")", ("resource", s.Resource), ("need", Number(s.Need)), ("have", Number(s.Have)));

        private static string Number(double value) => Math.Round(value, 3).ToString(System.Globalization.CultureInfo.InvariantCulture);

        /// <summary>Claims up to <paramref name="amount"/> of a resource for an owner; returns what it claimed.</summary>
        private double ReserveUpTo(Village v, string ownerId, string resource, double amount)
        {
            double take = Math.Min(amount, Available(v, resource));
            if (take <= 1e-9 || !Reserve(v, ownerId, resource, take))
                return 0;
            return take;
        }
    }
}
