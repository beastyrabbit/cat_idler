using System;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Cat entities, coats and the per-frame interpolation of cats and other movers.</summary>
    public sealed partial class ForestView
    {
        private void ReconcileCats(Village v, int level)
        {
            foreach (var c in v.Cats.Where(c => c.Alive && c.Position.Level == level))
            {
                var go = Entity("cat:" + c.Id, "cat", Position(c), .85f, false);
                var visual = go.GetComponent<CatMotion>();
                if (visual == null) { visual = go.AddComponent<CatMotion>(); Coat(go, c.Id); }
                visual.CatId = c.Id;
            }
        }

        internal void AnimateCats()
        {
            foreach (var c in Game.Selected.Cats)
            {
                if (!c.Alive || !entities.TryGetValue("cat:" + c.Id, out var go)) continue;
                var delta = Position(c) - go.transform.position;
                bool walking = delta.sqrMagnitude > .0004f;
                go.transform.position = Vector3.Lerp(go.transform.position, Position(c), 1 - Mathf.Exp(-Time.unscaledDeltaTime * 14));
                var heading = new Vector3(delta.x, 0, delta.z);
                if (heading.sqrMagnitude > .0004f) go.transform.rotation = Quaternion.Slerp(go.transform.rotation, Quaternion.LookRotation(heading.normalized, Vector3.up), Time.unscaledDeltaTime * 10);
                var motion = go.GetComponent<CatMotion>(); if (motion != null) motion.Animate(walking, c.Goal);
                FollowCarriedCargo(c, go);
            }
            foreach (var vehicle in Game.Selected.Vehicles)
                MoveVisual("vehicle:" + vehicle.Id, vehicle.HasContinuousPosition ? MovingPosition(vehicle.X, vehicle.Z) : At(vehicle.Position));
            foreach (var trade in Game.CurrentWorld.TradeOffers)
                MoveVisual("trade:" + trade.Id, MovingPosition(trade.X, trade.Z));
            foreach (var raid in Game.Selected.Raids)
                MoveVisual("raid:" + raid.Id, raid.HasContinuousPosition ? MovingPosition(raid.X, raid.Z) : At(raid.Position));
            foreach (var enemy in Game.CurrentWorld.Creatures)
                MoveVisual("creature:" + enemy.Id, new Vector3((float)enemy.X, (float)Game.CurrentWorld.DungeonHeight(enemy.Position), (float)enemy.Z));
            var trader = Game.Selected.Trader;
            if (trader.Phase != "absent") MoveVisual("trader:" + Game.Selected.Id, trader.HasContinuousPosition ? MovingPosition(trader.X, trader.Z) : At(trader.Position));
        }

        private Vector3 MovingPosition(double x, double z)
        {
            var p = new Int2(Mathf.RoundToInt((float)x), Mathf.RoundToInt((float)z)); var t = Game.CurrentWorld.GetTile(p);
            return new Vector3((float)x, (float)(t != null && t.Water && !t.Bridge ? t.WaterSurface : Game.CurrentWorld.WalkHeight(p)), (float)z);
        }

        private void MoveVisual(string id, Vector3 position)
        {
            if (!entities.TryGetValue(id, out var entity)) return;
            var delta = position - entity.transform.position;
            entity.transform.position = Vector3.Lerp(entity.transform.position, position, 1 - Mathf.Exp(-Time.unscaledDeltaTime * 14));
            if (delta.sqrMagnitude > .0004f)
                entity.transform.rotation = Quaternion.Slerp(entity.transform.rotation, Quaternion.LookRotation(delta.normalized, Vector3.up), Time.unscaledDeltaTime * 10);
        }

        private void Coat(GameObject go, string id)
        {
            uint hash = 2166136261; foreach (char c in id) hash = unchecked((hash ^ c) * 16777619);
            int palette = (int)(hash % 4); if (palette == 0) return;
            Color[] fur = { new Color(.82f, .60f, .38f), new Color(.42f, .46f, .47f), new Color(.29f, .25f, .22f), new Color(.78f, .74f, .64f) };
            foreach (var renderer in go.GetComponentsInChildren<Renderer>())
            {
                var originals = renderer.sharedMaterials;
                for (int i = 0; i < originals.Length; i++)
                {
                    var source = originals[i]; if (source == null || source.name.IndexOf("fur", StringComparison.OrdinalIgnoreCase) < 0) continue;
                    string key = "coat:" + palette + ":" + source.name;
                    if (!materials.TryGetValue(key, out var replacement))
                    {
                        var color = source.name.Contains("light") ? Color.Lerp(fur[palette], new Color(.94f, .88f, .72f), .8f) : source.name.Contains("dark") ? fur[palette] * .6f : fur[palette];
                        replacement = new Material(source) { name = key, color = color, enableInstancing = true }; materials[key] = replacement;
                    }
                    originals[i] = replacement;
                }
                renderer.sharedMaterials = originals;
            }
        }
    }
}
