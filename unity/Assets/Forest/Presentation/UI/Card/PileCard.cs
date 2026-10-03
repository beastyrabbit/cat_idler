using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The tethered card body for a stockpile (GUIDE 5.5 stockpile variant): what it holds as last counted, how fresh
    /// that count is, its capacity and what it takes, and Remove asked once inline. Station stores and food
    /// containers belong to their building and offer no Remove. Element names: <c>card-pile</c>,
    /// <c>card-contents</c>, <c>act-card-remove-&lt;pile&gt;</c> then
    /// <c>act-card-remove-yes|no-&lt;pile&gt;</c>.
    /// </summary>
    public class PileCard : CardBody
    {
        public PileCard(UiContext ctx) : base(ctx, "pile") { }

        public override bool Exists => Pile != null;

        private Stockpile Pile => Village?.Stockpiles.Find(p => p.Id == SubjectId);

        /// <summary>A pile the player may remove: not a station store or building container.</summary>
        public static bool Removable(Stockpile p) => p.ManagedBy == "";

        protected override void Describe()
        {
            var p = Pile;
            Title = NameOf(Village, p);
            PortraitEntity = "pile:" + p.Id;
            Facts = KindOf(p) + " · holds " + CardFormat.Whole(p.Capacity);
            Now = p.CountedAt < 0 ? "Not counted yet" : "Counted " + CardFormat.Duration(World.TimeSeconds - p.CountedAt) + " ago";
            Why = p.CountedAt < 0 ? "An accountant counts stores; until then amounts are unknown." : "";
            WhyIsProblem = false;
        }

        protected override string Shape()
        {
            var p = Pile;
            return p.Kind + "|" + p.CountedAt + "|" + string.Join(",", p.Report.Select(s => s.Resource + ":" + CardFormat.Amount(s.Amount))) + "|" + string.Join(",", p.Accepts) + "|" + p.ManagedBy;
        }

        protected override void Build()
        {
            var p = Pile;
            var kv = Kv();
            Row(kv, "Holds", p.CountedAt < 0 ? Seen(p) : CardFormat.Goods(p.Report), "card-contents");
            Row(kv, "Takes", p.Accepts.Count == 0 ? "Anything" : string.Join(", ", p.Accepts.Select(CardFormat.Resource)));
            if (!Removable(p))
            {
                var owner = Village.Buildings.Find(b => b.Id == p.ManagedBy);
                if (owner != null) Row(kv, "Belongs to", CardFormat.BuildingName(Village, owner));
                return;
            }
            var actions = Actions();
            string kind = p.Kind.StartsWith("zone_", System.StringComparison.Ordinal) ? "RemoveZone" : p.Kind == "gather" || p.Kind == "fishing" ? "RemoveGatherSpot" : "RemoveStockpile";
            Destructive(actions, "Remove", "Remove this pile?", "card-remove", p.Id, () => new GameAction { Kind = kind, TargetId = p.Id });
        }

        /// <summary>Before a count the player sees which goods lie there, not how many.</summary>
        private static string Seen(Stockpile p)
        {
            var kinds = p.Report.Concat(p.Goods).Where(s => s.Amount > 0).Select(s => s.Resource).Distinct().Select(CardFormat.Resource).ToList();
            return kinds.Count == 0 ? "Looks empty" : string.Join(", ", kinds) + " · amounts unknown";
        }

        private static string KindOf(Stockpile p)
        {
            switch (p.Kind)
            {
                case "gather": return "Gather spot";
                case "fishing": return "Fishing spot";
                case "spill": return "Dropped goods";
                default: return p.ManagedBy != "" ? "Station store" : "Stockpile";
            }
        }

        public static string NameOf(Village v, Stockpile p)
        {
            string kind = KindOf(p);
            var same = v.Stockpiles.Where(x => KindOf(x) == kind).ToList();
            return same.Count > 1 ? kind + " " + (same.IndexOf(p) + 1).ToString(System.Globalization.CultureInfo.InvariantCulture) : kind;
        }
    }
}
