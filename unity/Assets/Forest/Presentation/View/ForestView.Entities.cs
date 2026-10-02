using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Entity roots keyed by stable ids, authored models and the material cache. Every entity root
    /// holds one child named <see cref="VisualName"/> that carries the model; reconcile writes only the
    /// root (position, rotation, scale), so animation and effects can move the visual freely.
    /// </summary>
    public sealed partial class ForestView
    {
        public const string VisualName = "Visual";
        private readonly Dictionary<string, GameObject> entities = new Dictionary<string, GameObject>();
        private readonly Dictionary<string, string> entityAssets = new Dictionary<string, string>();
        private readonly Dictionary<string, GameObject> models = new Dictionary<string, GameObject>();
        private readonly Dictionary<string, Material> materials = new Dictionary<string, Material>();
        private readonly HashSet<string> touched = new HashSet<string>();
        private Transform worldRoot;

        /// <summary>Parent of every entity root.</summary>
        internal Transform WorldRoot => worldRoot;

        /// <summary>The child of an entity root that holds its model, or null.</summary>
        public static Transform VisualOf(GameObject root) => root == null ? null : root.transform.Find(VisualName);

        internal bool TryGetEntity(string id, out GameObject go) => entities.TryGetValue(id, out go);

        /// <summary>Every rendered cat by simulation id.</summary>
        internal IEnumerable<(string id, Transform t)> CatTransforms()
        {
            foreach (var pair in entities)
                if (pair.Key.StartsWith("cat:", StringComparison.Ordinal) && pair.Value != null)
                    yield return (pair.Key.Substring(4), pair.Value.transform);
        }

        private GameObject Entity(string id, string asset, Vector3 position, float width, bool move = true, bool naturalScale = false)
        {
            touched.Add(id);
            if (entityAssets.TryGetValue(id, out var previous) && previous != asset)
            { Destroy(entities[id]); entities.Remove(id); entityAssets.Remove(id); }
            if (!entities.TryGetValue(id, out var go))
            {
                if (!models.TryGetValue(asset, out var model)) { model = Resources.Load<GameObject>("ForestArt/" + asset); models[asset] = model; }
                go = new GameObject(id); go.transform.SetParent(worldRoot); go.transform.position = position;
                var visual = new GameObject(VisualName).transform; visual.SetParent(go.transform, false);
                if (model != null)
                {
                    var child = Instantiate(model, visual); child.name = asset;
                    var renderers = child.GetComponentsInChildren<Renderer>();
                    if (renderers.Length > 0 && !naturalScale)
                    {
                        var bounds = renderers[0].bounds; foreach (var r in renderers) bounds.Encapsulate(r.bounds);
                        float scale = 1 / Mathf.Max(.01f, Mathf.Max(bounds.size.x, bounds.size.z));
                        child.transform.localScale *= scale;
                        child.transform.localPosition -= new Vector3(bounds.center.x - go.transform.position.x, bounds.min.y - go.transform.position.y, bounds.center.z - go.transform.position.z) * scale;
                    }
                }
                else
                {
                    // A missing authored export is an explicit visual error, never silently accepted art.
                    var error = GameObject.CreatePrimitive(PrimitiveType.Cube); error.name = "MISSING ASSET " + asset; error.transform.SetParent(visual, false); error.transform.localScale = Vector3.one * .2f; error.GetComponent<Renderer>().sharedMaterial = Material("missing", Color.magenta);
                }
                entities[id] = go; entityAssets[id] = asset;
            }
            go.transform.localScale = Vector3.one * width;
            if (move) go.transform.position = position;
            return go;
        }

        private Material Material(string key, Color color, bool unlit = false)
        {
            if (materials.TryGetValue(key, out var found)) return found;
            var template = Resources.Load<Material>(unlit ? "ForestSelection" : "ForestSurface");
            if (template == null) throw new InvalidOperationException("Required forest material missing; run forest_setup.");
            var m = new Material(template) { name = key, color = color, enableInstancing = true };
            if (!unlit) { m.SetFloat("_Glossiness", .05f); m.SetFloat("_Metallic", 0); }
            materials[key] = m; return m;
        }

        private static void Tint(GameObject go, Material material) { foreach (var r in go.GetComponentsInChildren<Renderer>()) r.sharedMaterial = material; }
        private void ClearWorld() { foreach (var go in entities.Values) Destroy(go); entities.Clear(); entityAssets.Clear(); terrainSignature = -1; SelectedCatId = ""; SelectedBuildingId = ""; }
        private Vector3 At(Int2 p) => new Vector3(p.X, (float)Game.CurrentWorld.WalkHeight(p), p.Z);
        private Vector3 Position(Cat c) => new Vector3((float)c.X, (float)(c.HasHeight ? c.Y : Game.CurrentWorld.WalkHeight(c.Position)), (float)c.Z);
    }
}
