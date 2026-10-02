using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>A 0-1 ring with a caption. Wave 1 stub: a Painter2D arc and a label.</summary>
    public class Ring : VisualElement
    {
        private readonly Label caption;
        private float value01;

        public Ring(float value01, string label)
        {
            AddToClassList("ring");
            caption = new Label(label);
            caption.AddToClassList("ring-label");
            Add(caption);
            generateVisualContent += Draw;
            Value = value01;
        }

        public float Value
        {
            get => value01;
            set
            {
                value01 = Mathf.Clamp01(value);
                MarkDirtyRepaint();
            }
        }

        public string Caption
        {
            get => caption.text;
            set => caption.text = value;
        }

        private void Draw(MeshGenerationContext context)
        {
            var rect = contentRect;
            float radius = Mathf.Min(rect.width, rect.height) * .5f - 2;
            if (radius <= 0 || value01 <= 0) return;
            var painter = context.painter2D;
            painter.lineWidth = 3;
            painter.strokeColor = resolvedStyle.color;
            painter.BeginPath();
            painter.Arc(rect.center, radius, Angle.Degrees(-90), Angle.Degrees(-90 + 360 * value01));
            painter.Stroke();
        }
    }
}
