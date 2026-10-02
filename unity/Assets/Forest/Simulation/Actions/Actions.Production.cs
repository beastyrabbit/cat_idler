using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleRequestJob(Village village, GameAction action, Cat cat) => Request(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, cat, true, action.Resource);
        private ActionResult HandleDispatchScout(Village village, GameAction action, Cat cat) => Request(village, "scout", cat, true, action.Resource);
        private ActionResult HandleBoost(Village village, GameAction action) => BoostJob(village, village.Jobs.Find(j => j.Id == action.TargetId && !j.Completed));
        private ActionResult HandleBoostCat(GameAction action, Cat cat)
        {
            if (cat == null)
                return ActionResult.Fail("Living cat required");
            cat.Boosted = action.Enabled;
            return ActionResult.Ok();
        }
        private ActionResult HandleSetCatLaborPreference(GameAction action, Cat cat)
        {
            if (cat == null || !Catalog.Labors.Contains(action.Labor))
                return ActionResult.Fail("Living cat and maintained labor required");
            cat.Preferences.Remove(action.Labor);
            if (action.Enabled)
                cat.Preferences.Add(action.Labor);
            return ActionResult.Ok();
        }
        private ActionResult HandleAssignWorker(Village village, GameAction action, Cat cat, Building building) => Assign(village, cat, building, action);
        private ActionResult HandleEditProductionQueue(Village village, GameAction action, string kind, Cat cat, Building building) => EditQueue(village, building, cat, action, kind == "editproductionworkslot");
        private ActionResult HandleHaulGatherSpot(Village village, GameAction action, Cat cat)
        {
            var source = village.Stockpiles.Find(s => s.Id == action.TargetId && (s.Goods.Any(g => g.Amount > 0) || village.Items.Any(i => i.LocationId == s.Id)));
            if (source == null)
                return ActionResult.Fail("No source goods");
            return StartHaul(village, cat, source);
        }
        private ActionResult EditQueue(Village v, Building b, Cat c, GameAction a, bool exact)
        {
            if (b == null || !b.Completed)
                return ActionResult.Fail("Completed station required");
            if (exact && (c == null || !b.Slots.Any(s => s.CatId == c.Id)))
                return ActionResult.Fail("Exact staffed slot required");
            var slot = exact ? b.Slots.Find(s => s.CatId == c.Id) : b.Slots.FirstOrDefault();
            var queue = slot?.Queue ?? b.Queue;
            if (slot != null)
                slot.AutomatedBy = "";
            foreach (var entry in queue)
                entry.AutomatedBy = "";
            string edit = (a.Edit ?? "").Replace("_", "").ToLowerInvariant();
            if (edit == "add")
            {
                var r = RecipeOf(a.RecipeId);
                if (r == null || r.Building != b.Kind || !Catalog.RecipeAvailable(v, r))
                    return ActionResult.Fail("Recipe locked or wrong station");
                if (queue.Count >= 64)
                    return ActionResult.Fail("Queue limit reached");
                queue.Add(new QueueEntry { RecipeId = r.Id, Repeat = a.Repeat });
            }
            else if (edit == "setpaused" || edit == "pause")
            {
                if (slot != null)
                    slot.Paused = a.Enabled;
                else
                    b.Paused = a.Enabled;
            }
            else
            {
                if (a.Index < 0 || a.Index >= queue.Count)
                    return ActionResult.Fail("Queue index out of range");
                if (edit == "remove")
                    queue.RemoveAt(a.Index);
                else if (edit == "setrepeat" || edit == "repeat")
                    queue[a.Index].Repeat = a.Enabled;
                else if (edit == "move")
                {
                    int to = a.Index + (a.Direction < 0 ? -1 : 1);
                    if (to < 0 || to >= queue.Count)
                        return ActionResult.Fail("Queue edge");
                    var q = queue[a.Index];
                    queue.RemoveAt(a.Index);
                    queue.Insert(to, q);
                }
                else
                    return ActionResult.Fail("Unknown queue edit");
            }
            if (slot != null && b.Slots.IndexOf(slot) == 0)
                b.Queue = slot.Queue;
            return ActionResult.Ok();
        }
        private ActionResult Assign(Village v, Cat c, Building b, GameAction a)
        {
            if (c == null || c.ControlledBy != "")
                return ActionResult.Fail("Available living cat required");
            if (IsDungeonExplorer(c))
                return ActionResult.Fail("Recall the explorer and wait for its return before assigning work");
            if (v.Routes.Any(r => r.CatId == c.Id && (r.Mode == "shipping" && r.PathIndex > 0 || v.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9))))
                return ActionResult.Fail("Return the vehicle to a safe stopping point before reassigning its driver");
            if (b == null && a.TargetId != "")
            {
                var farm = v.Farms.Find(f => f.Id == a.TargetId);
                if (farm != null)
                {
                    var previous = v.Cats.Find(x => x.Id == farm.WorkerId);
                    if (previous != null && previous.Id != c.Id)
                        CancelWork(v, previous);
                    CancelWork(v, c);
                    farm.WorkerId = c.Id;
                    c.BuildingId = farm.Id;
                    return ActionResult.Ok();
                }
            }
            if (b != null && !b.Completed)
            {
                if (v.Jobs.Any(j => !j.Completed && j.TargetId == b.Id))
                    return ActionResult.Fail("Scaffold already has a builder");
                CancelWork(v, c);
                StartJob(v, c, new Job { Kind = "build", TargetId = b.Id, Position = b.Position, RequiredWork = b.RequiredWork, Manual = true });
                return ActionResult.Ok();
            }
            if (b != null && b.Slots.Count(s => s.CatId != "" && s.CatId != c.Id) >= (int)Catalog.BuildingEffect(v, b.Kind, "worker_slots", 1))
                return ActionResult.Fail("All work positions staffed");
            CancelWork(v, c);
            if (b == null)
                return ActionResult.Ok();
            var vacancy = b.Slots.FirstOrDefault(s => s.CatId == "");
            if (vacancy != null)
            {
                vacancy.CatId = c.Id;
                vacancy.AutomatedBy = "";
            }
            else
                b.Slots.Add(new WorkSlot { CatId = c.Id, Queue = b.Slots.Count == 0 ? b.Queue : new List<QueueEntry>() });
            if (b.WorkerId == "")
                b.WorkerId = c.Id;
            else
                b.ExtraWorkerIds.Add(c.Id);
            c.BuildingId = b.Id;
            if (b.Kind == "field")
            {
                var plot = v.Farms.FirstOrDefault(f => f.WorkerId == "");
                if (plot != null)
                    plot.WorkerId = c.Id;
            }
            return ActionResult.Ok();
        }
    }
}
