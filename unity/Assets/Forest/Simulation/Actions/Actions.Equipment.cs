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
                return ActionResult.Fail("Exact item identity required");
            if (kind == "repairitem")
            {
                string station = item.Material == "metal" ? "smithy" : item.Material == "fibre" ? "clothier" : item.Material == "leather" ? "tannery" : "woodworking";
                if (!v.Stockpiles.Any(p => p.Id == item.LocationId) || !v.Buildings.Any(b => b.Kind == station && b.Completed && b.Slots.Any(s => Cat(s.CatId)?.Alive == true)))
                    return ActionResult.Fail("Stored item and staffed repair station required");
                string material = item.Material == "wood" ? "planks" : item.Material;
                if (!Spend(v, material, 1))
                    return ActionResult.Fail("Matching repair material required");
                item.Condition = item.MaxCondition;
                return ActionResult.Ok();
            }
            if (c == null)
                return ActionResult.Fail("Living bearer required");
            if (IsDungeonExplorer(c))
                return ActionResult.Fail("Recall the explorer before changing its equipment");
            if (kind == "equipitem")
            {
                if (!new[] { "tool", "weapon", "armor" }.Contains(item.Kind) || item.Condition <= 0 || !v.Stockpiles.Any(p => p.Id == item.LocationId) || v.Items.Any(i => i.LocationId == c.Id && i.Kind == item.Kind))
                    return ActionResult.Fail("Unavailable or occupied equipment slot");
                item.LocationId = c.Id;
                c.Equipment.Add(item.Id);
            }
            else
            {
                if (item.LocationId != c.Id || !c.Equipment.Contains(item.Id))
                    return ActionResult.Fail("Item not equipped by this cat");
                var storage = Storage(v, ItemResource(item.Kind), 1, c.Position);
                if (storage == null)
                    return ActionResult.Fail("No reachable equipment storage");
                item.LocationId = storage.Id;
                c.Equipment.Remove(item.Id);
            }
            return ActionResult.Ok();
        }
        private static string ItemResource(string kind) => kind == "tool" ? "tools" : kind == "weapon" ? "weapons" : kind == "armor" ? "armor" : kind + "s";
    }
}
