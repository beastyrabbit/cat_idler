using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public enum Tone { Neutral, Good, Sun, Bad, Info }

    /// <summary>A short status word with a tone. Wave 1 stub: a label with <c>state-chip</c> and <c>tone-*</c> classes.</summary>
    public class StateChip : Label
    {
        public StateChip(string text, Tone tone)
        {
            AddToClassList("state-chip");
            Set(text, tone);
        }

        public Tone Tone { get; private set; }

        public void Set(string text, Tone tone)
        {
            RemoveFromClassList(ToneClass(Tone));
            this.text = text;
            Tone = tone;
            AddToClassList(ToneClass(tone));
        }

        private static string ToneClass(Tone tone) => "tone-" + tone.ToString().ToLowerInvariant();
    }
}
