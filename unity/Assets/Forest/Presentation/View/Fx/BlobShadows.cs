using System;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Soft contact shadows: one instanced draw of dark blob quads lying on the ground under every rendered
    /// cat, raider, cave creature, cart and trader wagon, stretched along the body's heading. They ground the
    /// small bodies even where cascaded shadows are coarse or out of range. Alpha <see cref="Alpha"/>.
    /// </summary>
    internal sealed class BlobShadows : IDisposable
    {
        public const float Alpha = .35f;
        private const float Lift = .09f;
        private static readonly Quaternion Flat = Quaternion.Euler(90, 0, 0);
        private readonly FxInstances blobs;
        private readonly Color tint = new Color(0, 0, 0, Alpha);


        public int Drawn => blobs.Count;

        public BlobShadows(FxAssets assets, Mesh quad) { blobs = new FxInstances(assets.Billboard(FxSprites.Blob), quad, 512); }

        public void Render(ForestView view, World world, Village village, Camera camera)
        {
            blobs.Clear();
            if (camera == null) return;
            int level = view.RenderedLevel;
            foreach (var c in village.Cats)
            {
                if (!c.Alive || c.Position.Level != level) continue;
                if (view.FxEntity(ForestFx.CatKey(c.Id), out var root)) Add(root, 1.2f, 1.45f, c.Cargo.Count > 0 && c.Cargo[0].Amount > 0 ? 1.1f : 1);
            }
            foreach (var raid in village.Raids)
                if (raid.Position.Level == level && view.FxEntity(ForestFx.Key("raid:", raid.Id), out var root)) Add(root, 1.2f, 1.45f, 1);
            foreach (var vehicle in village.Vehicles)
                if (vehicle.Position.Level == level && view.FxEntity(ForestFx.Key("vehicle:", vehicle.Id), out var root)) Add(root, 1.3f, 1.7f, 1);
            if (level == 0 && village.Trader.Phase != "absent" && view.FxEntity(ForestFx.Key("trader:", village.Id), out var wagon)) Add(wagon, 1.3f, 1.7f, 1);
            if (level == 0)
                foreach (var trade in world.TradeOffers)
                    if (view.FxEntity(ForestFx.Key("trade:", trade.Id), out var root)) Add(root, 1.3f, 1.7f, 1);
            foreach (var creature in world.Creatures)
                if (creature.Health > 0 && creature.Position.Level == level && view.FxEntity(ForestFx.Key("creature:", creature.Id), out var root)) Add(root, 1.2f, 1.45f, 1);
            blobs.Draw();
        }

        // width and length are fractions of the entity root's scale (the reconciled footprint width).
        private void Add(Transform root, float width, float length, float grow)
        {
            float s = root.localScale.x * grow;
            float yaw = root.eulerAngles.y;
            var rotation = Quaternion.Euler(0, yaw, 0) * Flat;
            blobs.Add(Matrix4x4.TRS(root.position + Vector3.up * Lift, rotation, new Vector3(width * s, length * s, 1)), tint);
        }

        public void Dispose() => blobs.Dispose();
    }
}
