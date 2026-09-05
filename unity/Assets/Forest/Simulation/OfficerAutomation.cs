using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private void StopOfficerWork(Village v, string role)
        {
            var workers = v.Buildings.SelectMany(b => b.Slots).Where(s => s.AutomatedBy == role).Select(s => s.CatId)
                .Concat(v.Farms.Where(f => f.AutomatedBy == role).Select(f => f.WorkerId))
                .Concat(v.Jobs.Where(j => !j.Completed && j.AutomatedBy == role).Select(j => j.CatId)).Where(id => id != "").Distinct().ToArray();
            foreach (var id in workers)
            {
                var c = v.Cats.Find(c => c.Id == id);
                if (c != null)
                    CancelWork(v, c);
            }
            foreach (var b in v.Buildings)
                foreach (var s in b.Slots)
                {
                    s.Queue.RemoveAll(q => q.AutomatedBy == role);
                    if (s.AutomatedBy == role)
                        s.AutomatedBy = "";
                }
            foreach (var f in v.Farms.Where(f => f.AutomatedBy == role))
                f.AutomatedBy = "";
        }
        private void MarkOfficerJob(Village v, ActionResult result, string role)
        {
            if (result.Success)
            {
                var job = v.Jobs.Find(j => j.Id == result.EntityId || j.TargetId == result.EntityId && !j.Completed);
                if (job != null)
                {
                    job.AutomatedBy = role;
                    job.Requester = role;
                    job.Manual = false;
                }
            }
        }
        private static string StationOfficer(string kind) => new[] { "wood_cutter", "stone_prep", "woodworking", "sawmill" }.Contains(kind) ? "forester" : new[] { "clothier", "tannery" }.Contains(kind) ? "cloth_leader" : new[] { "smithy", "smelter", "barracks" }.Contains(kind) ? "captain" : new[] { "research_hut", "school" }.Contains(kind) ? "loremaster" : kind == "mill" ? "farmer" : kind == "accounting_tent" ? "accountant" : "steward";
        private double RecipeDeficit(Village v, Recipe r)
        {
            if (r.ItemKind != "")
                return Math.Max(0, v.Cats.Count(c => c.Alive) * 0.5 - v.Items.Count(i => i.Kind == r.ItemKind && i.Material == r.Material && i.Condition > 0));
            return r.Outputs.Count == 0 ? 0 : r.Outputs.Max(s => Math.Max(0, (s.Resource == "food" ? v.Cats.Count(c => c.Alive) * 8 : 32) - Total(v, s.Resource)));
        }
        private void StaffOfficers(Village v)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed))
            {
                string officer = StationOfficer(b.Kind);
                bool productive = Catalog.Recipes.Any(r => r.Building == b.Kind) || new[] { "research_hut", "school", "accounting_tent", "mouse_farm" }.Contains(b.Kind);
                if (!productive || !HasOfficer(v, officer))
                    continue;
                int reserve = Math.Max(2, (int)Math.Ceiling(v.Cats.Count(c => c.Alive) * 2.0 / 15));
                if (!b.Slots.Any(s => s.CatId != "") && v.Cats.Count(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "") > reserve)
                {
                    var worker = AvailableCat(v, "process");
                    if (worker != null && Assign(v, worker, b, new GameAction()).Success)
                        b.Slots.Find(s => s.CatId == worker.Id).AutomatedBy = officer;
                }
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == officer && s.CatId != "" && s.Queue.Count == 0 && !s.Paused))
                {
                    var recipe = Catalog.Recipes.Where(r => r.Building == b.Kind && Catalog.RecipeAvailable(v, r) && r.Inputs.All(input => Available(v, input.Resource) + Amount(b.Inputs, input.Resource) >= RecipeInput(v, r, input.Amount)))
                        .OrderByDescending(r => RecipeDeficit(v, r)).ThenBy(r => r.Id, StringComparer.Ordinal).FirstOrDefault(r => RecipeDeficit(v, r) > 0);
                    if (recipe != null)
                        slot.Queue.Add(new QueueEntry { RecipeId = recipe.Id, AutomatedBy = officer });
                }
            }
        }
        private void PlanHousing(Village v)
        {
            int adults = v.Cats.Count(c => c.Alive), beds = v.Buildings.Count(b => b.Kind == "den" && b.Completed) * DenCapacity(v);
            if (TimeSeconds - v.FoundedAt < 6 * 3600 || beds - adults - v.Cats.Count(c => c.PregnantUntil >= 0) > 1 || Total(v, "food") < adults * 3 || Total(v, "water") < adults * 4 || v.Buildings.Any(b => b.Kind == "den" && !b.Completed))
                return;
            foreach (var p in v.Known.OrderBy(p => Int2.Distance(v.Center, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (FreeSite(v, p, 2, 2))
                {
                    var result = Plan(v, "den", p, null);
                    if (result.Success)
                        MarkOfficerJob(v, result, "leader");
                    return;
                }
        }
    }
}
