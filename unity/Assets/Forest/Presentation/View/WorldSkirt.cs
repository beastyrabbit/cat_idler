using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A flat, dark forest-floor plane under the whole view, sized every frame from the camera frustum corners so the
    /// world never ends in a void. It is drawn first and without depth, so real ground and scenery always cover it.
    /// Trees beyond the explored edge are not drawn here (the canopy owns them).
    /// </summary>
    internal sealed class WorldSkirt
    {
        internal const string ObjectName = "World skirt";
        private const float Margin = .5f;
        private const float MinimumSize = 80;
        private const float MaximumSize = 2000;
        private readonly GameObject plane;

        public WorldSkirt(Transform parent, Material material, int layer)
        {
            plane = new GameObject(ObjectName, typeof(MeshFilter), typeof(MeshRenderer)) { layer = layer };
            plane.transform.SetParent(parent, false);
            var mesh = new Mesh { name = "World skirt" };
            mesh.SetVertices(new[] { new Vector3(-.5f, 0, -.5f), new Vector3(-.5f, 0, .5f), new Vector3(.5f, 0, .5f), new Vector3(.5f, 0, -.5f) });
            mesh.SetNormals(new[] { Vector3.up, Vector3.up, Vector3.up, Vector3.up });
            mesh.SetColors(new[] { Color.white, Color.white, Color.white, Color.white });
            mesh.SetTriangles(new[] { 0, 1, 2, 0, 2, 3 }, 0);
            mesh.RecalculateBounds();
            plane.GetComponent<MeshFilter>().sharedMesh = mesh;
            var renderer = plane.GetComponent<MeshRenderer>();
            renderer.sharedMaterial = material;
            renderer.shadowCastingMode = ShadowCastingMode.Off; renderer.receiveShadows = false;
            renderer.lightProbeUsage = LightProbeUsage.Off; renderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
            plane.SetActive(false);
        }

        /// <summary>The ground rectangle (x, z) the skirt currently covers.</summary>
        public Rect Covered { get; private set; }

        /// <summary>Centres and sizes the plane at height <paramref name="groundY"/> to cover the frustum footprint with a margin.</summary>
        public void Place(Camera camera, float groundY)
        {
            if (plane == null) return;
            if (camera == null || !TryFootprint(camera, groundY, groundY, camera.farClipPlane, out var footprint)) { plane.SetActive(false); return; }
            float width = Mathf.Clamp(footprint.width * (1 + 2 * Margin), MinimumSize, MaximumSize);
            float depth = Mathf.Clamp(footprint.height * (1 + 2 * Margin), MinimumSize, MaximumSize);
            var center = footprint.center;
            plane.transform.SetPositionAndRotation(new Vector3(center.x, groundY, center.y), Quaternion.identity);
            plane.transform.localScale = new Vector3(width, 1, depth);
            Covered = new Rect(center.x - width / 2, center.y - depth / 2, width, depth);
            if (!plane.activeSelf) plane.SetActive(true);
        }

        /// <summary>
        /// The axis-aligned ground rectangle (x, z) seen by <paramref name="camera"/> between heights
        /// <paramref name="lowY"/> and <paramref name="highY"/>: the four viewport-corner rays and the centre ray meet both
        /// planes, and rays that never reach them (above the horizon) stop at <paramref name="maxDistance"/> along the
        /// ground. Works for orthographic and perspective cameras. Edge scenery uses it to decide what to draw.
        /// </summary>
        public static bool TryFootprint(Camera camera, float lowY, float highY, float maxDistance, out Rect footprint)
        {
            footprint = default;
            if (camera == null) return false;
            float xMin = float.MaxValue, xMax = float.MinValue, zMin = float.MaxValue, zMax = float.MinValue;
            void Add(Vector3 p) { xMin = Mathf.Min(xMin, p.x); xMax = Mathf.Max(xMax, p.x); zMin = Mathf.Min(zMin, p.z); zMax = Mathf.Max(zMax, p.z); }
            for (int corner = 0; corner < 5; corner++)
            {
                var viewport = corner == 4 ? new Vector3(.5f, .5f, 0) : new Vector3(corner & 1, corner >> 1, 0);
                var ray = camera.ViewportPointToRay(viewport);
                for (int pass = 0; pass < 2; pass++)
                {
                    float y = pass == 0 ? lowY : highY;
                    float t = Mathf.Abs(ray.direction.y) > 1e-5f ? (y - ray.origin.y) / ray.direction.y : -1;
                    var flat = new Vector3(ray.direction.x, 0, ray.direction.z);
                    if (t > 0 && new Vector2(ray.direction.x * t, ray.direction.z * t).magnitude <= maxDistance) Add(ray.origin + ray.direction * t);
                    else if (flat.sqrMagnitude > 1e-8f) Add(ray.origin + flat.normalized * maxDistance);
                    else Add(ray.origin);
                }
            }
            if (xMin > xMax) return false;
            footprint = Rect.MinMaxRect(xMin, zMin, xMax, zMax);
            return true;
        }
    }
}
