using System;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Attaches a lazily computed tooltip. Wave 1 stub: the text is computed when the pointer enters
    /// and stored in <see cref="VisualElement.tooltip"/>. W2-16 adds the shared glass tooltip.
    /// </summary>
    public static class Tooltip
    {
        public static void Attach(VisualElement element, Func<string> text)
        {
            if (element == null || text == null) return;
            element.RegisterCallback<PointerEnterEvent>(_ => element.tooltip = text() ?? "");
        }
    }
}
