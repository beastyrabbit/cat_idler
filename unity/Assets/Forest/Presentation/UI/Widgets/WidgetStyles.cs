using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Loads the widget, attention and toast style sheets and keeps them, and the reduced-motion class,
    /// on the HUD's top root. Widgets call <see cref="Use"/> so they look right wherever they are mounted,
    /// without ForestUI having to know about every sheet.
    /// </summary>
    public static class WidgetStyles
    {
        public const string Widgets = "UI/Widgets";
        public const string Attention = "UI/Attention";
        public const string Toasts = "UI/Toasts";

        /// <summary>Set on the top root while <see cref="GameSettings.ReducedMotion"/> is on; USS drops every transition under it.</summary>
        public const string ReducedMotionClass = "reduced-motion";

        private static readonly Dictionary<string, StyleSheet> Sheets = new Dictionary<string, StyleSheet>();
        private static readonly List<VisualElement> Roots = new List<VisualElement>();
        private static bool listening;

        /// <summary>True when transitions and pulses should be skipped.</summary>
        public static bool ReducedMotion => GameSettings.ReducedMotion;

        /// <summary>Attaches the sheets to the element's top root now (when it is in a panel) and whenever it is attached again.</summary>
        public static void Use(VisualElement element, params string[] sheets)
        {
            if (element == null) return;
            var wanted = sheets == null || sheets.Length == 0 ? new[] { Widgets } : sheets;
            element.RegisterCallback<AttachToPanelEvent>(_ => Ensure(element, wanted));
            if (element.panel != null) Ensure(element, wanted);
        }

        /// <summary>The ancestor directly under the panel's visual tree: the HUD document root.</summary>
        public static VisualElement TopRoot(VisualElement element)
        {
            if (element == null) return null;
            var tree = element.panel?.visualTree;
            var top = element;
            while (top.parent != null && top.parent != tree) top = top.parent;
            return top;
        }

        private static void Ensure(VisualElement element, string[] sheets)
        {
            var root = TopRoot(element);
            if (root == null) return;
            foreach (var path in sheets)
            {
                var sheet = Load(path);
                if (sheet != null && !root.styleSheets.Contains(sheet)) root.styleSheets.Add(sheet);
            }
            if (!Roots.Contains(root))
            {
                Roots.RemoveAll(r => r == null || r.panel == null);
                Roots.Add(root);
            }
            root.EnableInClassList(ReducedMotionClass, GameSettings.ReducedMotion);
            if (!listening)
            {
                listening = true;
                GameSettings.Changed += ApplyMotion;
            }
        }

        private static void ApplyMotion()
        {
            foreach (var root in Roots)
                if (root != null)
                    root.EnableInClassList(ReducedMotionClass, GameSettings.ReducedMotion);
        }

        private static StyleSheet Load(string path)
        {
            if (Sheets.TryGetValue(path, out var sheet) && sheet != null) return sheet;
            sheet = Resources.Load<StyleSheet>(path);
            if (sheet == null) Debug.LogWarning("Missing style sheet Resources/" + path + ".uss");
            Sheets[path] = sheet;
            return sheet;
        }

        /// <summary>Delay in milliseconds for an exit animation, 0 under reduced motion.</summary>
        public static long Exit(long milliseconds) => GameSettings.ReducedMotion ? 0 : milliseconds;
    }
}
