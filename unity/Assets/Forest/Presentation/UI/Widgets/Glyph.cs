using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public enum GlyphKind { Bell, Pin, Close, Alert, Check }

    /// <summary>
    /// A line icon from <c>docs/ui-redesign/mockups/icons.js</c> drawn with Painter2D on the 24-point grid
    /// (stroke 1.8, round caps), in the element's resolved text colour, so it follows hover and state
    /// colours like text. Used by the seal, popover pins and toasts until the icon atlas lands.
    /// </summary>
    public class Glyph : VisualElement
    {
        public const string UssClass = "glyph";

        private GlyphKind kind;

        public Glyph(GlyphKind kind)
        {
            this.kind = kind;
            AddToClassList(UssClass);
            pickingMode = PickingMode.Ignore;
            generateVisualContent += Draw;
            RegisterCallback<CustomStyleResolvedEvent>(_ => MarkDirtyRepaint());
        }

        public GlyphKind Kind
        {
            get => kind;
            set
            {
                if (kind == value) return;
                kind = value;
                MarkDirtyRepaint();
            }
        }

        private void Draw(MeshGenerationContext context)
        {
            var rect = contentRect;
            float size = Mathf.Min(rect.width, rect.height);
            if (size <= 0) return;
            float s = size / 24f;
            var origin = new Vector2(rect.x + (rect.width - size) * .5f, rect.y + (rect.height - size) * .5f);
            Vector2 P(float x, float y) => origin + new Vector2(x * s, y * s);
            var p = context.painter2D;
            p.strokeColor = resolvedStyle.color;
            p.fillColor = resolvedStyle.color;
            p.lineWidth = 1.8f * s;
            p.lineCap = LineCap.Round;
            p.lineJoin = LineJoin.Round;
            switch (kind)
            {
                case GlyphKind.Bell:
                    // M6 16V11a6 6 0 0112 0v5l2 2H4z  M10 20a2 2 0 004 0
                    p.BeginPath();
                    p.MoveTo(P(6, 16));
                    p.LineTo(P(6, 11));
                    p.Arc(P(12, 11), 6 * s, Angle.Degrees(180), Angle.Degrees(360));
                    p.LineTo(P(18, 16));
                    p.LineTo(P(20, 18));
                    p.LineTo(P(4, 18));
                    p.ClosePath();
                    p.Stroke();
                    p.BeginPath();
                    p.Arc(P(12, 20), 2 * s, Angle.Degrees(180), Angle.Degrees(0), ArcDirection.CounterClockwise);
                    p.Stroke();
                    break;
                case GlyphKind.Pin:
                    // M12 21s-6-6.5-6-11a6 6 0 0112 0c0 4.5-6 11-6 11z  circle 12,10 r2
                    p.BeginPath();
                    p.MoveTo(P(12, 21));
                    p.BezierCurveTo(P(12, 21), P(6, 14.5f), P(6, 10));
                    p.Arc(P(12, 10), 6 * s, Angle.Degrees(180), Angle.Degrees(360));
                    p.BezierCurveTo(P(18, 14.5f), P(12, 21), P(12, 21));
                    p.ClosePath();
                    p.Stroke();
                    p.BeginPath();
                    p.Arc(P(12, 10), 2 * s, Angle.Degrees(0), Angle.Degrees(180));
                    p.Arc(P(12, 10), 2 * s, Angle.Degrees(180), Angle.Degrees(360));
                    p.Stroke();
                    break;
                case GlyphKind.Close:
                    p.BeginPath();
                    p.MoveTo(P(7, 7));
                    p.LineTo(P(17, 17));
                    p.MoveTo(P(17, 7));
                    p.LineTo(P(7, 17));
                    p.Stroke();
                    break;
                case GlyphKind.Alert:
                    // M12 3l9 16H3z  M12 10v4  M12 17v.5
                    p.BeginPath();
                    p.MoveTo(P(12, 3));
                    p.LineTo(P(21, 19));
                    p.LineTo(P(3, 19));
                    p.ClosePath();
                    p.Stroke();
                    p.BeginPath();
                    p.MoveTo(P(12, 10));
                    p.LineTo(P(12, 14));
                    p.MoveTo(P(12, 17));
                    p.LineTo(P(12, 17.5f));
                    p.Stroke();
                    break;
                case GlyphKind.Check:
                    p.BeginPath();
                    p.MoveTo(P(5, 12));
                    p.LineTo(P(9, 16));
                    p.LineTo(P(19, 6));
                    p.Stroke();
                    break;
            }
        }
    }
}
