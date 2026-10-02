using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private bool HasOfficer(Village v, string role) => v.Officers.Any(o => o.Role == role && v.Cats.Any(c => c.Id == o.CatId && c.Alive));
        /// <summary>Pre-empts the healthiest free adult for physical hunting or water when requests fail.</summary>
        private void Emergency(Village v, string kind)
        {
            if (v.Jobs.Any(j => j.Kind == kind && !j.Completed))
                return;
            var worker = v.Cats.Where(c => c.Alive && c.ControlledBy == "" && !IsDungeonExplorer(c) && c.Position.Level == 0 && c.AgeHours >= Pace.WorkingAgeHours && c.Migration != "arriving" && c.Migration != "departing" && !v.Jobs.Any(j => j.CatId == c.Id && !j.Completed && (j.Kind == "hunt" || j.Kind == "water")))
                .OrderByDescending(c => c.Health + c.Thirst + c.Hunger).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
            if (worker == null)
                return;
            CancelWork(v, worker);
            var result = Request(v, kind, worker, false);
            if (result.Success)
                Note(v, "emergency", worker.Name + " preempted work for physical " + kind, worker.Id);
        }
        /// <summary>The leader's standing orders: food, water and scouting, officer duties, housing, and the daily study.</summary>
        private void Direct(Village v)
        {
            int alive = v.Cats.Count(c => c.Alive), hunts = (int)Math.Ceiling(alive * Pace.HuntCrewPerGroup / Pace.CrewGroupSize), waters = (int)Math.Ceiling(alive * Pace.WaterCrewPerGroup / Pace.CrewGroupSize);
            if (Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive * Pace.FoodReservePerCat && v.Jobs.Count(j => j.Kind == "hunt" && !j.Completed) < hunts)
            {
                var result = Request(v, "hunt", null, false);
                if (!result.Success && Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive * Pace.EmergencyStockPerCat)
                    Emergency(v, "hunt");
            }
            if (Total(v, "water") < alive * Pace.WaterReservePerCat && v.Jobs.Count(j => j.Kind == "water" && !j.Completed) < waters)
            {
                var result = Request(v, "water", null, false);
                if (!result.Success && Total(v, "water") < alive * Pace.EmergencyStockPerCat)
                    Emergency(v, "water");
            }
            if (!v.Jobs.Any(j => j.Kind == "scout" && !j.Completed) && ((long)TimeSeconds % Pace.ScoutIntervalSeconds == 0 || !v.Known.Any(p => TileAt(p).Resource == "logs")))
                Request(v, "scout", null, false, "logs");
            if (HasOfficer(v, "forester"))
            {
                foreach (var resource in new[] { "logs", "stone" })
                    if (Total(v, resource) < Pace.ForesterStockTarget && !v.Jobs.Any(j => j.Resource == resource && !j.Completed))
                        MarkOfficerJob(v, Request(v, resource == "stone" ? "quarry" : "logs", null, false), "forester");
                if ((long)TimeSeconds % Pace.ReplantIntervalSeconds == 0)
                    MarkOfficerJob(v, Request(v, "replant", null, false), "forester");
            }
            if (HasOfficer(v, "farmer"))
            {
                if (Total(v, "fibre") < Pace.FibreStockTarget && !v.Jobs.Any(j => j.Kind == "fibre" && !j.Completed))
                    MarkOfficerJob(v, Request(v, "fibre", null, false), "farmer");
                foreach (var f in v.Farms.Where(f => f.WorkerId == ""))
                {
                    var c = AvailableCat(v, "farm");
                    if (c != null)
                    {
                        f.WorkerId = c.Id;
                        f.AutomatedBy = "farmer";
                        c.BuildingId = f.Id;
                    }
                }
            }
            if (HasOfficer(v, "steward"))
            {
                foreach (var pile in v.Stockpiles.Where(s => new[] { "gather", "spill", "farm_output", "fishing" }.Contains(s.Kind) && (s.Goods.Count > 0 || v.Items.Any(i => i.LocationId == s.Id))).ToArray())
                {
                    if (!v.Jobs.Any(j => j.SourceId == pile.Id && !j.Completed))
                        MarkOfficerJob(v, StartHaul(v, null, pile), "steward");
                }
            }
            StaffOfficers(v);
            PlanHousing(v);
            if (HasOfficer(v, "loremaster") && v.Blessings < Pace.LoremasterOfferingBlessingCap && Total(v, "materials") > Pace.LoremasterOfferingMaterialsMin && !v.Jobs.Any(j => j.Kind == "offering" && !j.Completed))
                MarkOfficerJob(v, CreateInputJob(v, null, "offering", v.Center, "materials", Pace.LoremasterOfferingMaterials, Pace.LoremasterOfferingWork, ""), "loremaster");
            if (TimeSeconds - v.LastLeaderResearch >= Pace.LeaderStudyIntervalSeconds && v.Cats.Any(c => c.Id == v.LeaderId && c.Alive))
            {
                var node = Catalog.Research.Where(n => !v.Research.Contains(n.Id) && n.Cost <= v.ResearchPoints && n.Prerequisites.All(v.Research.Contains)).OrderBy(n => n.LeaderPriority).ThenBy(n => n.Id, StringComparer.Ordinal).FirstOrDefault();
                if (node != null && Research(v, node.Id, false).Success)
                    v.LastLeaderResearch = TimeSeconds;
            }
        }
    }
}
