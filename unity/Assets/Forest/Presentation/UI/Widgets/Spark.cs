using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A small 0-1 fill bar for table cells (progress, stock against capacity). The fill's width follows
    /// the value with a short transition; the tone colours the fill. Writes styles only on change.
    /// </summary>
    public class Spark : VisualElement
    {
        public const string UssClass = "spark";
        public const string FillClass = "spark-fill";

        private readonly VisualElement fill;
        private float value01 = -1;
        private Tone tone = Tone.Good;

        public Spark(float value01)
        {
            AddToClassList(UssClass);
            fill = new VisualElement { pickingMode = PickingMode.Ignore };
            fill.AddToClassList(FillClass);
            fill.AddToClassList(StateChip.ToneClass(tone));
            fill.style.height = Length.Percent(100);
            Add(fill);
            Value = value01;
            WidgetStyles.Use(this);
        }

        public float Value
        {
            get => Mathf.Max(0, value01);
            set
            {
                float clamped = float.IsNaN(value) ? 0 : Mathf.Clamp01(value);
                if (Mathf.Approximately(clamped, value01)) return;
                value01 = clamped;
                fill.style.width = Length.Percent(clamped * 100);
            }
        }

        public Tone Tone
        {
            get => tone;
            set
            {
                if (value == tone) return;
                fill.RemoveFromClassList(StateChip.ToneClass(tone));
                tone = value;
                fill.AddToClassList(StateChip.ToneClass(tone));
            }
        }
    }
}
