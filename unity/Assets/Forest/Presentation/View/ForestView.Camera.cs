using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Management (orthographic) and direct-control (follow) cameras.</summary>
    public sealed partial class ForestView
    {
        private const float ManagementStandoff = 50;
        private static readonly Vector3 DirectOffset = new Vector3(0, 2.5f, -3.8f);
        private Vector3 focus, managementFocus;
        private float zoom = 14, managementZoom = 14, yaw;
        private bool wasDirect, ownsCamera;

        public float ManagementZoom => DirectControl ? managementZoom : zoom;
        /// <summary>Orthographic half-height of the management camera, also while it is parked in direct control.</summary>
        internal float Zoom => zoom;
        /// <summary>Yaw of the direct-control camera around its cat, in degrees.</summary>
        internal float CameraYaw => yaw;
        /// <summary>Distance from the camera to the point it frames.</summary>
        internal float CameraStandoff => DirectControl ? DirectOffset.magnitude : ManagementStandoff;

        private void CreateCamera()
        {
            Camera = UnityEngine.Camera.main;
            if (Camera == null) { var go = new GameObject("Forest camera"); Camera = go.AddComponent<Camera>(); go.tag = "MainCamera"; ownsCamera = true; }
            Camera.clearFlags = CameraClearFlags.SolidColor;
            Camera.backgroundColor = new Color(.11f, .18f, .17f);
            Camera.nearClipPlane = .1f; Camera.farClipPlane = 240;
            Camera.allowHDR = false; Camera.allowMSAA = true;
        }

        public void ZoomBy(float steps)
        {
            if (DirectControl || float.IsNaN(steps) || float.IsInfinity(steps)) return;
            // Input System 1.x uses normalized wheel steps, including fractional trackpad motion.
            zoom = Mathf.Clamp(zoom * Mathf.Exp(Mathf.Clamp(-steps * .12f, -20, 20)), 3.5f, 36);
            ApplyManagementCamera();
        }

        public void FocusVillage()
        {
            viewedLevel = 0;
            var v = Game.Selected; if (v == null) return;
            var center = new Vector3(v.Center.X, 0, v.Center.Z);
            if (DirectControl) { managementFocus = center; managementZoom = Mathf.Max(10, v.Radius + 5); return; }
            focus = center; zoom = Mathf.Max(10, v.Radius + 5); villageId = "";
            ApplyManagementCamera();
            SelectedCatId = ""; SelectedBuildingId = "";
        }

        private void ApplyManagementCamera()
        {
            Camera.orthographic = true; Camera.orthographicSize = zoom;
            Camera.transform.rotation = Quaternion.Euler(55, 0, 0);
            float covered = Game.UI?.WorldLeftFraction ?? 0;
            Camera.transform.position = focus - Camera.transform.forward * ManagementStandoff - Camera.transform.right * (zoom * Camera.aspect * covered);
        }

        internal void UpdateCamera()
        {
            var cat = ControlledCat;
            if (cat != null)
            {
                if (!wasDirect) { managementFocus = focus; managementZoom = zoom; }
                Camera.orthographic = false; Camera.fieldOfView = 58;
                var target = Position(cat) + Vector3.up * .55f;
                var offset = Quaternion.Euler(0, yaw, 0) * DirectOffset;
                Camera.transform.position = Vector3.Lerp(Camera.transform.position, target + offset, 1 - Mathf.Exp(-Time.unscaledDeltaTime * 8));
                Camera.transform.LookAt(target);
            }
            else
            {
                if (wasDirect) { focus = managementFocus; zoom = managementZoom; }
                ApplyManagementCamera();
            }
            ApplyFogForCamera(cat != null);
            wasDirect = cat != null;
        }
    }
}
