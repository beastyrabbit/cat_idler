using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Sun, ambient light and fog.</summary>
    public sealed partial class ForestView
    {
        private Light sun;

        private void CreateLights()
        {
            var existingLights = FindObjectsByType<Light>();
            foreach (var light in existingLights) light.enabled = false;
            sun = new GameObject("Afternoon light").AddComponent<Light>();
            sun.type = LightType.Directional; sun.color = new Color(1, .89f, .72f); sun.intensity = 1.35f;
            sun.transform.rotation = Quaternion.Euler(48, -32, 0);
            sun.shadows = LightShadows.Soft; sun.shadowStrength = .55f; sun.shadowBias = .08f;
            RenderSettings.ambientMode = UnityEngine.Rendering.AmbientMode.Flat;
            RenderSettings.ambientLight = new Color(.52f, .62f, .58f);
            RenderSettings.fog = true; RenderSettings.fogColor = Camera.backgroundColor;
            RenderSettings.fogMode = FogMode.Linear; RenderSettings.fogStartDistance = 45; RenderSettings.fogEndDistance = 110;
            QualitySettings.shadowDistance = 45;
        }

        /// <summary>Fog distances for the active camera; called every frame by UpdateCamera.</summary>
        private static void ApplyFogForCamera(bool directControl)
        {
            if (directControl) { RenderSettings.fogStartDistance = 24; RenderSettings.fogEndDistance = 58; }
            else { RenderSettings.fogStartDistance = 65; RenderSettings.fogEndDistance = 120; }
        }
    }
}
