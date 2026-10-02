using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>A small 0-1 fill bar. Wave 1 stub: a fill child whose width follows the value.</summary>
    public class Spark : VisualElement
    {
        private readonly VisualElement fill;
        private float value01;

        public Spark(float value01)
        {
            AddToClassList("spark");
            fill = new VisualElement();
            fill.AddToClassList("spark-fill");
            fill.style.height = Length.Percent(100);
            Add(fill);
            Value = value01;
        }

        public float Value
        {
            get => value01;
            set
            {
                value01 = Mathf.Clamp01(value);
                fill.style.width = Length.Percent(value01 * 100);
            }
        }
    }
}
