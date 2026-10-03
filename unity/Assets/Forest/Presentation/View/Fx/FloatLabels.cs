using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Pooled UI Toolkit chips that rise from a world point and fade ("+3 logs", "Den built"). They live on
    /// their own panel behind the HUD, ignore the pointer and are built on first use, so a session that
    /// never floats anything pays nothing. Identical text that lands close to a live chip refreshes that
    /// chip instead of stacking a copy, which keeps 4x play readable.
    /// </summary>
    internal sealed class FloatLabels : IDisposable
    {
        public const int Max = 24;
        private const float Life = 1.5f, Rise = 46;
        private static readonly Color Ink = new Color(.933f, .953f, .918f); // --ink #eef3ea
        private static readonly Color Glass = new Color(.082f, .133f, .102f, .78f);

        private sealed class Chip
        {
            public VisualElement Box, Icon; public Label Text;
            public Vector3 World; public float Age; public bool Live; public string Key = "";
        }

        private readonly List<Chip> chips = new List<Chip>();
        private GameObject host;
        private PanelSettings panel;
        private VisualElement root;
        private int liveCount;

        public int Live => liveCount;

        public void Float(Vector3 world, string text, Background icon, Color color, float mergeDistance)
        {
            if (string.IsNullOrEmpty(text) || !EnsureRoot()) return;
            Chip spare = null, oldest = null;
            foreach (var candidate in chips)
            {
                if (!candidate.Live) { if (spare == null) spare = candidate; continue; }
                if (candidate.Key == text && candidate.Age < Life * .5f && (candidate.World - world).sqrMagnitude < mergeDistance * mergeDistance)
                { candidate.Age = 0; candidate.World = Vector3.Lerp(candidate.World, world, .5f); return; }
                if (oldest == null || candidate.Age > oldest.Age) oldest = candidate;
            }
            var chip = spare ?? (chips.Count < Max ? Create() : oldest);
            if (!chip.Live) liveCount++;
            chip.Live = true; chip.Age = 0; chip.World = world; chip.Key = text;
            chip.Text.text = text;
            chip.Text.style.color = color.a > 0 ? color : Ink;
            bool hasIcon = icon.texture != null || icon.sprite != null || icon.vectorImage != null || icon.renderTexture != null;
            chip.Icon.style.display = hasIcon ? DisplayStyle.Flex : DisplayStyle.None;
            if (hasIcon) chip.Icon.style.backgroundImage = new StyleBackground(icon);
            chip.Box.style.display = DisplayStyle.Flex;
            chip.Box.style.opacity = 0;
        }

        public void Tick(float dt, Camera camera, bool reducedMotion)
        {
            if (liveCount == 0 || root == null || root.panel == null) return;
            float scale = Mathf.Max(.5f, GameSettings.UiScale);
            foreach (var chip in chips)
            {
                if (!chip.Live) continue;
                chip.Age += dt;
                if (chip.Age >= Life || camera == null) { Retire(chip); continue; }
                var screen = camera.WorldToScreenPoint(chip.World);
                if (screen.z <= 0) { chip.Box.style.opacity = 0; continue; }
                var point = RuntimePanelUtils.ScreenToPanel(root.panel, new Vector2(screen.x, Screen.height - screen.y));
                float t = chip.Age / Life;
                float rise = reducedMotion ? 0 : Rise * (1 - (1 - t) * (1 - t));
                float pop = reducedMotion || chip.Age > .18f ? 1 : 1 + .25f * Mathf.Sin(chip.Age / .18f * Mathf.PI);
                float alpha = Mathf.Clamp01(chip.Age / .1f) * (1 - Mathf.Clamp01((t - .7f) / .3f));
                var size = chip.Box.layout;
                float width = float.IsNaN(size.width) ? 0 : size.width, height = float.IsNaN(size.height) ? 0 : size.height;
                chip.Box.style.left = point.x - width * .5f;
                chip.Box.style.top = point.y - height - rise;
                chip.Box.style.opacity = alpha;
                chip.Box.style.scale = new Scale(new Vector3(pop * scale, pop * scale, 1));
            }
        }

        public void Clear() { foreach (var chip in chips) if (chip.Live) Retire(chip); }

        private void Retire(Chip chip)
        {
            chip.Live = false; chip.Key = "";
            chip.Box.style.display = DisplayStyle.None;
            liveCount--;
        }

        private bool EnsureRoot()
        {
            if (root != null) return true;
            var template = Resources.Load<PanelSettings>("ForestPanel");
            if (template == null) return false;
            host = new GameObject("Forest effect labels");
            panel = UnityEngine.Object.Instantiate(template);
            panel.name = "Forest effect labels panel";
            panel.scaleMode = PanelScaleMode.ConstantPhysicalSize;
            panel.referenceDpi = 110;
            panel.fallbackDpi = 96;
            // Behind the HUD panel (sorting order 0): world chips never cover controls.
            panel.sortingOrder = -10;
            var document = host.AddComponent<UIDocument>();
            document.panelSettings = panel;
            root = document.rootVisualElement;
            root.name = "forest-fx-labels";
            root.pickingMode = PickingMode.Ignore;
            root.style.position = Position.Absolute;
            root.style.left = 0; root.style.top = 0; root.style.right = 0; root.style.bottom = 0;
            return true;
        }

        private Chip Create()
        {
            var box = new VisualElement { name = "fx-float", pickingMode = PickingMode.Ignore };
            box.style.position = Position.Absolute;
            box.style.flexDirection = FlexDirection.Row;
            box.style.alignItems = Align.Center;
            box.style.paddingLeft = 7; box.style.paddingRight = 8; box.style.paddingTop = 2; box.style.paddingBottom = 3;
            box.style.backgroundColor = Glass;
            box.style.borderTopLeftRadius = box.style.borderTopRightRadius = box.style.borderBottomLeftRadius = box.style.borderBottomRightRadius = 10;
            box.style.transformOrigin = new TransformOrigin(Length.Percent(50), Length.Percent(100), 0);
            box.style.display = DisplayStyle.None;
            var icon = new VisualElement { pickingMode = PickingMode.Ignore };
            icon.style.width = 16; icon.style.height = 16; icon.style.marginRight = 4;
            var text = new Label { pickingMode = PickingMode.Ignore };
            text.style.fontSize = 14;
            text.style.unityFontStyleAndWeight = FontStyle.Bold;
            text.style.color = Ink;
            text.style.marginLeft = text.style.marginRight = text.style.marginTop = text.style.marginBottom = 0;
            text.style.paddingLeft = text.style.paddingRight = text.style.paddingTop = text.style.paddingBottom = 0;
            box.Add(icon); box.Add(text);
            root.Add(box);
            var chip = new Chip { Box = box, Icon = icon, Text = text };
            chips.Add(chip);
            return chip;
        }

        public void Dispose()
        {
            chips.Clear(); liveCount = 0; root = null;
            if (host != null) UnityEngine.Object.Destroy(host);
            if (panel != null) UnityEngine.Object.Destroy(panel);
        }
    }
}
