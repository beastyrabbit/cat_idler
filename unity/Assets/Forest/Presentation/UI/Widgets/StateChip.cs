using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public enum Tone { Neutral, Good, Sun, Bad, Info }

    /// <summary>
    /// A short status word with a tone (<c>state-chip</c> plus <c>tone-*</c>). Status colours always
    /// travel with a word (GUIDE section 3), so the chip never shows colour alone. <see cref="Set"/> touches
    /// the element only when the text or tone changed.
    /// </summary>
    public class StateChip : Label
    {
        public const string UssClass = "state-chip";

        public StateChip(string text, Tone tone)
        {
            AddToClassList(UssClass);
            Tone = tone;
            AddToClassList(ToneClass(tone));
            this.text = text ?? "";
            WidgetStyles.Use(this);
        }

        public Tone Tone { get; private set; }

        public void Set(string text, Tone tone)
        {
            text = text ?? "";
            if (this.text != text) this.text = text;
            if (tone == Tone) return;
            RemoveFromClassList(ToneClass(Tone));
            Tone = tone;
            AddToClassList(ToneClass(tone));
        }

        public static string ToneClass(Tone tone)
        {
            switch (tone)
            {
                case Tone.Good: return "tone-good";
                case Tone.Sun: return "tone-sun";
                case Tone.Bad: return "tone-bad";
                case Tone.Info: return "tone-info";
                default: return "tone-neutral";
            }
        }
    }
}
