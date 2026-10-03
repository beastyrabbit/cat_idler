using System.Globalization;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A 0-1 ring with its number inside (need rings in the card). Painter2D draws a dim track and an arc
    /// coloured by tone: by default ok at 60 % and above, sun from 35 %, bad below (GUIDE section 5.5).
    /// Repaints only when the value or tone changes.
    /// </summary>
    public class Ring : VisualElement
    {
        public const string UssClass = "ring";
        public const string LabelClass = "ring-label";

        // The GUIDE section 3 tokens; UI Toolkit cannot read USS variables from Painter2D code.
        public static readonly Color Ok = new Color32(126, 211, 122, 255);
        public static readonly Color Sun = new Color32(255, 209, 102, 255);
        public static readonly Color Bad = new Color32(255, 107, 87, 255);
        public static readonly Color Info = new Color32(124, 198, 230, 255);
        public static readonly Color Track = new Color(1, 1, 1, .10f);
        public static readonly Color Hole = new Color32(21, 34, 26, 255);

        private readonly Label caption;
        private float value01 = -1;
        private Tone? tone;

        public Ring(float value01, string label)
        {
            AddToClassList(UssClass);
            caption = new Label(label ?? "") { pickingMode = PickingMode.Ignore };
            caption.AddToClassList(LabelClass);
            Add(caption);
            generateVisualContent += Draw;
            Value = value01;
            WidgetStyles.Use(this);
        }

        /// <summary>Stroke width in panel points.</summary>
        public float Thickness { get; set; } = 4;

        public float Value
        {
            get => Mathf.Max(0, value01);
            set
            {
                float clamped = float.IsNaN(value) ? 0 : Mathf.Clamp01(value);
                if (Mathf.Approximately(clamped, value01)) return;
                value01 = clamped;
                MarkDirtyRepaint();
            }
        }

        /// <summary>A fixed tone; null picks the tone from the value.</summary>
        public Tone? Tone
        {
            get => tone;
            set
            {
                if (tone == value) return;
                tone = value;
                MarkDirtyRepaint();
            }
        }

        public string Caption
        {
            get => caption.text;
            set
            {
                if (caption.text != (value ?? "")) caption.text = value ?? "";
            }
        }

        /// <summary>Sets the value and shows it as a whole percentage inside the ring.</summary>
        public void SetPercent(float value01)
        {
            Value = value01;
            Caption = Mathf.RoundToInt(Value * 100).ToString(CultureInfo.InvariantCulture);
        }

        public static Tone ToneFor(float value01) => value01 >= .6f ? Presentation.Tone.Good : value01 >= .35f ? Presentation.Tone.Sun : Presentation.Tone.Bad;

        public static Color ColorOf(Tone tone)
        {
            switch (tone)
            {
                case Presentation.Tone.Good: return Ok;
                case Presentation.Tone.Sun: return Sun;
                case Presentation.Tone.Bad: return Bad;
                case Presentation.Tone.Info: return Info;
                default: return Track;
            }
        }

        // Two half arcs: a single 360 degree arc has no sweep.
        private static void Circle(Painter2D painter, Vector2 center, float radius)
        {
            painter.BeginPath();
            painter.Arc(center, radius, Angle.Degrees(0), Angle.Degrees(180));
            painter.Arc(center, radius, Angle.Degrees(180), Angle.Degrees(360));
            painter.ClosePath();
        }

        private void Draw(MeshGenerationContext context)
        {
            var rect = contentRect;
            float radius = Mathf.Min(rect.width, rect.height) * .5f - Thickness * .5f - 1;
            if (radius <= 0) return;
            var painter = context.painter2D;
            painter.lineWidth = Thickness;
            painter.lineCap = LineCap.Round;
            painter.fillColor = Hole;
            Circle(painter, rect.center, radius);
            painter.Fill();
            painter.strokeColor = Track;
            Circle(painter, rect.center, radius);
            painter.Stroke();
            float value = Value;
            if (value <= 0) return;
            painter.strokeColor = ColorOf(tone ?? ToneFor(value));
            painter.BeginPath();
            painter.Arc(rect.center, radius, Angle.Degrees(-90), Angle.Degrees(-90 + 360 * Mathf.Min(value, .9999f)));
            painter.Stroke();
        }
    }
}
