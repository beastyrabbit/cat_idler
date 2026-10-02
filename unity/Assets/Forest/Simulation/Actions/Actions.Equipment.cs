using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleEquipment(Village village, GameAction action, string kind, Cat cat) => Equipment(village, cat, action, kind);
        private ActionResult Equipment(Village v, Cat c, GameAction a, string kind)
        {
            var item = v.Items.Find(i => i.Id == a.TargetId);
            if (item == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Exact item identity required", (ResultArg.Target, a.TargetId ?? ""));
            if (kind == "repairitem")
            {
                string station = item.Material == "metal" ? "smithy" : item.Material == "fibre" ? "clothier" : item.Material == "leather" ? "tannery" : "woodworking";
                if (!v.Stockpiles.Any(p => p.Id == item.LocationId) || !v.Buildings.Any(b => b.Kind == station && b.Completed && b.Slots.Any(s => Cat(s.CatId)?.Alive == true)))
                    return ActionResult.Fail(ResultCode.RepairStationRequired, "Stored item and staffed repair station required", (ResultArg.Target, item.Id), (ResultArg.Building, station));
                string material = item.Material == "wood" ? "planks" : item.Material;
                double available = Available(v, material);
                if (!Spend(v, material, 1))
                    return ActionResult.Fail(ResultCode.MissingMaterial, "Matching repair material required", (ResultArg.Resource, material), (ResultArg.Need, "1"), (ResultArg.Have, ActionResult.Number(available)));
                item.Condition = item.MaxCondition;
                return ActionResult.Ok(ResultCode.Repaired, item.Id, (ResultArg.Kind, item.Kind));
            }
            if (c == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living bearer required", (ResultArg.Target, a.CatId ?? ""));
            if (IsDungeonExplorer(c))
                return ActionResult.Fail(ResultCode.ExplorerAway, "Recall the explorer before changing its equipment", (ResultArg.Target, c.Id));
            if (kind == "equipitem")
            {
                if (!new[] { "tool", "weapon", "armor" }.Contains(item.Kind) || item.Condition <= 0 || !v.Stockpiles.Any(p => p.Id == item.LocationId) || v.Items.Any(i => i.LocationId == c.Id && i.Kind == item.Kind))
                    return ActionResult.Fail(ResultCode.SlotOccupied, "Unavailable or occupied equipment slot", (ResultArg.Target, item.Id), (ResultArg.Kind, item.Kind));
                item.LocationId = c.Id;
                c.Equipment.Add(item.Id);
                return ActionResult.Ok(ResultCode.Equipped, item.Id, (ResultArg.Target, c.Id), (ResultArg.Kind, item.Kind));
            }
            if (item.LocationId != c.Id || !c.Equipment.Contains(item.Id))
                return ActionResult.Fail(ResultCode.NotEquipped, "Item not equipped by this cat", (ResultArg.Target, item.Id));
            var storage = Storage(v, ItemResource(item.Kind), 1, c.Position);
            if (storage == null)
                return ActionResult.Fail(ResultCode.NoStorage, "No reachable equipment storage", (ResultArg.Resource, ItemResource(item.Kind)));
            item.LocationId = storage.Id;
            c.Equipment.Remove(item.Id);
            return ActionResult.Ok(ResultCode.Unequipped, item.Id, (ResultArg.Target, storage.Id), (ResultArg.Kind, item.Kind));
        }
        private static string ItemResource(string kind) => kind == "tool" ? "tools" : kind == "weapon" ? "weapons" : kind == "armor" ? "armor" : kind + "s";
    }
}
