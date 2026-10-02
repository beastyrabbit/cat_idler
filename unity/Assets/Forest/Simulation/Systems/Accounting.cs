using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>The accountant's round: visit each reachable storage, count it, return to the tent and wait.</summary>
        private void TickAccounting(Village v, double dt)
        {
            var tent = v.Buildings.Find(b => b.Kind == "accounting_tent" && b.Completed && b.Slots.Any(s => s.CatId != ""));
            if (tent == null || !HasOfficer(v, "accountant"))
            {
                v.Accounting = null;
                return;
            }
            var c = v.Cats.Find(c => c.Id == tent.Slots.First(s => s.CatId != "").CatId && c.Alive);
            if (c == null || c.ControlledBy != "" || c.JobId != "" || c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                return;
            var round = v.Accounting;
            if (round == null || round.WorkerId != c.Id || round.BuildingId != tent.Id)
            {
                round = new AccountingRound { WorkerId = c.Id, BuildingId = tent.Id, Phase = "returning" };
                v.Accounting = round;
            }
            if (round.Phase == "waiting")
            {
                round.DwellSeconds += SpendActorTime(c);
                if (round.DwellSeconds + 1e-9 < Pace.AccountingWaitSeconds)
                    return;
                round.Phase = "returning";
                round.DwellSeconds = 0;
            }
            if (round.Phase == "returning" || round.Phase == "return_to_tent" || round.Phase == "")
            {
                c.Goal = GoalKey.AccountingReturning;
                if (!Move(c, tent.Position, dt))
                    return;
                round.Pending = v.Stockpiles.Where(s => s.Kind == "storage").OrderBy(s => Int2.Distance(tent.Position, s.Position)).ThenBy(s => s.Id, StringComparer.Ordinal).Select(s => s.Id).ToList();
                round.Unreachable.Clear();
                round.TargetId = "";
                round.Phase = "visiting";
            }
            if (round.TargetId == "")
            {
                while (round.Pending.Count > 0)
                {
                    string id = round.Pending[0];
                    round.Pending.RemoveAt(0);
                    var next = v.Stockpiles.Find(s => s.Id == id);
                    if (next == null)
                        continue;
                    if (Path(c.Position, next.Position, v) == null)
                    {
                        round.Unreachable.Add(id);
                        continue;
                    }
                    round.TargetId = id;
                    round.DwellSeconds = 0;
                    break;
                }
                if (round.TargetId == "")
                {
                    if (Move(c, tent.Position, dt))
                    {
                        round.Phase = "waiting";
                        round.DwellSeconds = 0;
                    }
                    return;
                }
            }
            var pile = v.Stockpiles.Find(s => s.Id == round.TargetId);
            if (pile == null)
            {
                round.TargetId = "";
                return;
            }
            c.Goal = GoalKey.AccountingPrefix + pile.Id;
            if (!Move(c, pile.Position, dt))
            {
                if (c.BlockedReason == "blocked_route")
                {
                    round.Unreachable.Add(pile.Id);
                    round.TargetId = "";
                }
                return;
            }
            round.Phase = "counting";
            round.DwellSeconds += SpendActorTime(c) * Service(v, "accounting_tent", "accountingSpeed", 1);
            tent.Progress = round.DwellSeconds;
            if (round.DwellSeconds + 1e-9 >= Pace.AccountingCountSeconds)
            {
                pile.Report = pile.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToList();
                foreach (var kind in new[] { "tools", "weapons", "armor" })
                {
                    double amount = v.Items.Count(i => i.LocationId == pile.Id && i.Kind == EquipmentKind(kind));
                    if (amount > 0)
                        Add(pile.Report, kind, amount);
                }
                pile.CountedAt = TimeSeconds;
                tent.Progress = 0;
                round.DwellSeconds = 0;
                round.TargetId = "";
                round.Phase = "visiting";
            }
        }
    }
}
