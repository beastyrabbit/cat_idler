using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The card's 52 px portrait well. It renders the subject's actual model: a copy of the entity's
    /// <see cref="ForestView.VisualName"/> child is posed like the original every frame it renders (so a walking or
    /// grooming cat moves in its portrait), lit on its own layer far outside the world and drawn by a dedicated
    /// camera into a small RenderTexture. Cats get a front three-quarter close-up, buildings and piles a
    /// three-quarter view of the whole model. Without a graphics device, or before the entity exists, the well shows
    /// the subject's initial.
    /// </summary>
    public sealed class CardPortrait : VisualElement
    {
        /// <summary>The render layer only the portrait camera and its lights see.</summary>
        public const int Layer = 31;
        private const int Pixels = 128;
        private static readonly Vector3 Stage = new Vector3(-20000, 4000, -20000);

        private readonly Label letter;
        private GameObject rig, copy;
        private Camera lens;
        private RenderTexture texture;
        private Transform source;
        private Transform[] sourceParts, copyParts;
        private Renderer[] sourceRenderers, copyRenderers;
        private MaterialPropertyBlock block;
        private string entityId = "";
        private bool closeUp, hasImage;
        private float nextRender;

        public CardPortrait()
        {
            name = "card-portrait";
            AddToClassList("card-portrait");
            pickingMode = PickingMode.Ignore;
            letter = new Label { name = "card-portrait-letter", pickingMode = PickingMode.Ignore };
            letter.AddToClassList("card-portrait-letter");
            Add(letter);
            RegisterCallback<DetachFromPanelEvent>(_ => Release());
        }

        /// <summary>True once the well shows a rendered model rather than the initial.</summary>
        public bool HasImage => hasImage;

        /// <summary>The entity id ("cat:&lt;id&gt;", "building:&lt;id&gt;", "pile:&lt;id&gt;") the portrait shows.</summary>
        public string EntityId => entityId;

        private static bool CanRender => Application.isPlaying && SystemInfo.graphicsDeviceType != GraphicsDeviceType.Null;

        /// <summary>
        /// Shows <paramref name="entity"/>; <paramref name="live"/> re-renders about 20 times a second (cats), otherwise
        /// once a second (buildings and piles change slowly).
        /// </summary>
        public void Show(ForestView view, string entity, string initial, bool live)
        {
            if (letter.text != initial) letter.text = initial;
            if (entity != entityId)
            {
                DropCopy();
                entityId = entity ?? "";
                closeUp = entityId.StartsWith("cat:", System.StringComparison.Ordinal);
                SetImage(false);
                nextRender = 0;
            }
            if (!CanRender || view == null || entityId == "") return;
            if (source == null)
            {
                if (copy != null) DropCopy();
                if (!view.TryGetEntity(entityId, out var root) || root == null) { SetImage(false); return; }
                source = ForestView.VisualOf(root);
                if (source == null) return;
                MakeCopy();
            }
            if (Time.unscaledTime < nextRender) return;
            nextRender = Time.unscaledTime + (live ? .05f : 1f);
            if (!Pose()) { DropCopy(); return; }
            Render();
        }

        private void MakeCopy()
        {
            EnsureRig();
            copy = Object.Instantiate(source.gameObject);
            copy.name = "Card portrait of " + entityId;
            foreach (var behaviour in copy.GetComponentsInChildren<MonoBehaviour>(true)) Object.Destroy(behaviour);
            foreach (var collider in copy.GetComponentsInChildren<Collider>(true)) Object.Destroy(collider);
            foreach (var part in copy.GetComponentsInChildren<Transform>(true)) part.gameObject.layer = Layer;
            sourceParts = source.GetComponentsInChildren<Transform>(true);
            copyParts = copy.GetComponentsInChildren<Transform>(true);
            sourceRenderers = source.GetComponentsInChildren<Renderer>(true);
            copyRenderers = copy.GetComponentsInChildren<Renderer>(true);
            foreach (var renderer in copyRenderers)
            {
                renderer.shadowCastingMode = ShadowCastingMode.Off;
                renderer.receiveShadows = false;
            }
        }

        /// <summary>Copies the original's pose, coat and visibility; false when its hierarchy changed.</summary>
        private bool Pose()
        {
            if (source == null || copy == null || source.parent == null) return false;
            var parts = source.GetComponentsInChildren<Transform>(true);
            if (parts.Length != copyParts.Length) return false;
            for (int i = 1; i < parts.Length; i++)
            {
                if (parts[i] != sourceParts[i]) return false;
                var from = parts[i]; var to = copyParts[i];
                to.localPosition = from.localPosition;
                to.localRotation = from.localRotation;
                to.localScale = from.localScale;
                if (to.gameObject.activeSelf != from.gameObject.activeSelf) to.gameObject.SetActive(from.gameObject.activeSelf);
            }
            var root = source.parent;
            copy.transform.position = Stage + Quaternion.Inverse(root.rotation) * (source.position - root.position);
            copy.transform.rotation = Quaternion.Inverse(root.rotation) * source.rotation;
            copy.transform.localScale = source.lossyScale;
            block ??= new MaterialPropertyBlock();
            for (int i = 0; i < copyRenderers.Length && i < sourceRenderers.Length; i++)
            {
                var from = sourceRenderers[i]; var to = copyRenderers[i];
                if (from == null || to == null) return false;
                to.enabled = from.enabled;
                if (to.sharedMaterials.Length != from.sharedMaterials.Length || to.sharedMaterial != from.sharedMaterial) to.sharedMaterials = from.sharedMaterials;
                if (from.HasPropertyBlock()) { from.GetPropertyBlock(block); to.SetPropertyBlock(block); }
            }
            return true;
        }

        private void Render()
        {
            var bounds = new Bounds(Stage, Vector3.zero);
            bool any = false;
            foreach (var renderer in copyRenderers)
            {
                if (renderer == null || !renderer.enabled) continue;
                if (!any) { bounds = renderer.bounds; any = true; }
                else bounds.Encapsulate(renderer.bounds);
            }
            if (!any) return;
            // Cats: a close three-quarter view from the front, aimed a little above the body's middle so the face
            // reads. Buildings and piles: the whole model from a raised three-quarter angle.
            var target = closeUp ? bounds.center + Vector3.up * bounds.extents.y * .3f : bounds.center;
            float radius = closeUp ? Mathf.Max(bounds.extents.y * 1.25f, bounds.extents.x) : bounds.extents.magnitude;
            var direction = closeUp ? Quaternion.Euler(-12, 32, 0) * Vector3.forward : Quaternion.Euler(-28, 38, 0) * Vector3.forward;
            float distance = radius / Mathf.Sin(lens.fieldOfView * .5f * Mathf.Deg2Rad) * (closeUp ? .95f : 1.02f);
            lens.transform.position = target + direction * distance;
            lens.transform.LookAt(target);
            lens.nearClipPlane = Mathf.Max(.01f, distance - radius * 3);
            lens.farClipPlane = distance + radius * 3;

            bool fog = RenderSettings.fog;
            var shadows = QualitySettings.shadows;
            RenderSettings.fog = false;
            QualitySettings.shadows = ShadowQuality.Disable;
            try { lens.Render(); }
            finally
            {
                RenderSettings.fog = fog;
                QualitySettings.shadows = shadows;
            }
            SetImage(true);
            MarkDirtyRepaint();
        }

        private void EnsureRig()
        {
            if (rig != null) return;
            rig = new GameObject("Card portrait rig");
            texture = new RenderTexture(Pixels, Pixels, 24, RenderTextureFormat.ARGB32) { name = "Card portrait", antiAliasing = 4 };
            texture.Create();
            lens = new GameObject("Card portrait camera").AddComponent<Camera>();
            lens.transform.SetParent(rig.transform, false);
            lens.enabled = false;
            lens.cullingMask = 1 << Layer;
            lens.clearFlags = CameraClearFlags.SolidColor;
            lens.backgroundColor = new Color(0, 0, 0, 0);
            lens.fieldOfView = 24;
            lens.allowHDR = false;
            lens.allowMSAA = true;
            lens.useOcclusionCulling = false;
            lens.targetTexture = texture;
            Light(rig.transform, "Card portrait key", new Color(1f, .93f, .8f), 1.15f, Quaternion.Euler(28, 210, 0));
            Light(rig.transform, "Card portrait fill", new Color(.72f, .82f, 1f), .45f, Quaternion.Euler(12, 120, 0));
            Light(rig.transform, "Card portrait rim", new Color(1f, .86f, .55f), .55f, Quaternion.Euler(20, 20, 0));
        }

        private static void Light(Transform parent, string label, Color color, float intensity, Quaternion rotation)
        {
            var light = new GameObject(label).AddComponent<Light>();
            light.transform.SetParent(parent, false);
            light.transform.rotation = rotation;
            light.type = LightType.Directional;
            light.color = color;
            light.intensity = intensity;
            light.shadows = LightShadows.None;
            light.cullingMask = 1 << Layer;
        }

        private void SetImage(bool image)
        {
            if (image && !hasImage) style.backgroundImage = Background.FromRenderTexture(texture);
            else if (!image && hasImage) style.backgroundImage = StyleKeyword.None;
            hasImage = image;
            letter.style.display = image ? DisplayStyle.None : DisplayStyle.Flex;
        }

        private void DropCopy()
        {
            if (copy != null) Object.Destroy(copy);
            copy = null; source = null;
            sourceParts = copyParts = null;
            sourceRenderers = copyRenderers = null;
        }

        private void Release()
        {
            DropCopy();
            SetImage(false);
            if (rig != null) Object.Destroy(rig);
            if (texture != null) { texture.Release(); Object.Destroy(texture); }
            rig = null; lens = null; texture = null;
            entityId = "";
        }
    }
}
