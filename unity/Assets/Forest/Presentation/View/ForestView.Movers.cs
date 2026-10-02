using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Things that travel: carts and boats, caravans, the trader, raiders and cave creatures.</summary>
    public sealed partial class ForestView
    {
        private void ReconcileMovers(World world, Village v, int level, HashSet<Int2> known)
        {
            foreach (var vehicle in v.Vehicles.Where(e => e.Position.Level == level)) Entity("vehicle:" + vehicle.Id, vehicle.Mode == "shipping" ? "boat" : "cart", vehicle.HasContinuousPosition ? MovingPosition(vehicle.X, vehicle.Z) : At(vehicle.Position), 1, !vehicle.HasContinuousPosition);
            foreach (var trade in world.TradeOffers.Where(t => level == 0 && (t.Status == "outbound" || t.Status == "returning" || t.Status == "travelling")))
                if (trade.Path.Count > 0) Entity("trade:" + trade.Id, "cart", MovingPosition(trade.X, trade.Z), .8f, !trade.HasContinuousPosition);
            if (v.Trader.Phase != "absent" && level == 0) Entity("trader:" + v.Id, "cart", v.Trader.HasContinuousPosition ? MovingPosition(v.Trader.X, v.Trader.Z) : At(v.Trader.Position), 1, !v.Trader.HasContinuousPosition);
            foreach (var raid in v.Raids.Where(r => r.Position.Level == level)) { var go = Entity("raid:" + raid.Id, "cat", raid.HasContinuousPosition ? MovingPosition(raid.X, raid.Z) : At(raid.Position), 1.1f, !raid.HasContinuousPosition); Tint(go, Material("raider", new Color(.46f, .18f, .16f))); }
            foreach (var enemy in world.Creatures.Where(e => e.Health > 0 && e.Position.Level == level && known.Contains(e.Position)))
            {
                var go = Entity("creature:" + enemy.Id, "world_cave_beetle", new Vector3((float)enemy.X, (float)world.DungeonHeight(enemy.Position), (float)enemy.Z), enemy.Kind == "stone_guardian" ? 1.3f : .9f);
                if (enemy.Kind == "stone_guardian") Tint(go, Material("stone guardian", new Color(.48f, .55f, .53f)));
            }
        }
    }
}
