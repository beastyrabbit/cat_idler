using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The line from the tethered card to its subject (GUIDE 5.5): a 2 px sun cubic Bézier from the card's near edge
    /// to the anchor over a soft dark under-stroke, ending in a 4 px dot. A full-size, non-picking overlay that
    /// TetheredCard inserts just below itself in its slot.
    /// </summary>
    public class LeaderLine : VisualElement, IHudPart
    {
        private static readonly Color Sun = new Color(1f, .82f, .4f);
        private static readonly Color Shade = new Color(0, 0, 0, .38f);
        private Vector2 from, to;
        private bool shown;

        public LeaderLine(UiContext ctx)
        {
            Context = ctx;
            name = "card-leader";
            AddToClassList("card-leader");
            pickingMode = PickingMode.Ignore;
            style.position = Position.Absolute;
            style.left = 0; style.top = 0; style.right = 0; style.bottom = 0;
            generateVisualContent += Draw;
        }

        public UiContext Context { get; }

        /// <summary>Whether a line is drawn (false while the anchor is off screen or the card is hidden).</summary>
        public bool Shown => shown;
        public Vector2 From => from;
        public Vector2 To => to;

        public void Refresh() { }

        /// <summary>Draws from <paramref name="cardEdge"/> to <paramref name="anchor"/>, both in this element's coordinates.</summary>
        public void Show(Vector2 cardEdge, Vector2 anchor)
        {
            if (shown && (cardEdge - from).sqrMagnitude < .01f && (anchor - to).sqrMagnitude < .01f) return;
            from = cardEdge; to = anchor; shown = true;
            MarkDirtyRepaint();
        }

        public void Hide()
        {
            if (!shown) return;
            shown = false;
            MarkDirtyRepaint();
        }

        private void Draw(MeshGenerationContext context)
        {
            if (!shown) return;
            var painter = context.painter2D;
            // Leave the card horizontally, arrive at the subject vertically, like the mockup's curve.
            var c1 = new Vector2(from.x + (to.x - from.x) * .55f, from.y);
            var c2 = new Vector2(to.x, from.y + (to.y - from.y) * .55f);
            painter.lineCap = LineCap.Round;
            painter.lineJoin = LineJoin.Round;
            foreach (var (width, color) in new[] { (4.5f, Shade), (2f, Sun) })
            {
                painter.lineWidth = width;
                painter.strokeColor = color;
                painter.BeginPath();
                painter.MoveTo(from);
                painter.BezierCurveTo(c1, c2, to);
                painter.Stroke();
            }
            painter.fillColor = Shade;
            painter.BeginPath();
            painter.Arc(to, 5.5f, Angle.Degrees(0), Angle.Degrees(360));
            painter.Fill();
            painter.fillColor = Sun;
            painter.BeginPath();
            painter.Arc(to, 4f, Angle.Degrees(0), Angle.Degrees(360));
            painter.Fill();
        }
    }
}
