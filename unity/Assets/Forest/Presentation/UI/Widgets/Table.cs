using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Rows keyed by entity id, reconciled in place: a row is made once per key and kept while its key
    /// lasts, so hover, focus, open dropdowns and transitions survive every refresh. <see cref="Refresh"/>
    /// never clears; it binds rows, moves only rows whose position changed and removes rows whose key
    /// disappeared (through <see cref="Leave"/>, so they can fade). A row whose value is a struct, a string
    /// or another <see cref="IEquatable{T}"/> and equals the last bound value is not bound again; reference
    /// rows (a Cat, say) are bound on every refresh, and their bind should set text only on change
    /// (<see cref="UiKit.SetText"/>).
    /// </summary>
    public class Table<T> : VisualElement
    {
        public const string UssClass = "table";
        public const string RowClass = "table__row";

        private static readonly bool ComparesByValue = typeof(T).IsValueType || typeof(IEquatable<T>).IsAssignableFrom(typeof(T));

        private readonly Func<IEnumerable<(string key, T row)>> rows;
        private readonly Func<T, VisualElement> make;
        private readonly Action<VisualElement, T> bind;
        private readonly Dictionary<string, Entry> entries = new Dictionary<string, Entry>();
        private readonly HashSet<VisualElement> leaving = new HashSet<VisualElement>();
        private readonly List<string> order = new List<string>();

        private sealed class Entry
        {
            public VisualElement Element;
            public T Value;
            public bool Bound;
        }

        public Table(Func<IEnumerable<(string key, T row)>> rows, Func<T, VisualElement> make, Action<VisualElement, T> bind)
        {
            this.rows = rows;
            this.make = make;
            this.bind = bind;
            AddToClassList(UssClass);
            Refresh();
        }

        /// <summary>
        /// How a row whose key disappeared leaves: called with the element and a callback that removes it.
        /// The default removes it at once.
        /// </summary>
        public Action<VisualElement, Action> Leave { get; set; }

        /// <summary>Live rows (leaving rows excluded).</summary>
        public int Count => entries.Count;

        /// <summary>The keys of the live rows in display order.</summary>
        public IReadOnlyList<string> Keys => order;

        /// <summary>Rows made since construction; a refresh with the same keys makes none.</summary>
        public int MadeCount { get; private set; }

        /// <summary>Calls to the bind delegate since construction.</summary>
        public int BindCount { get; private set; }

        public VisualElement RowOf(string key) => key != null && entries.TryGetValue(key, out var entry) ? entry.Element : null;

        public void Refresh()
        {
            var seen = new HashSet<string>();
            order.Clear();
            int child = 0;
            foreach (var (key, row) in rows())
            {
                if (key == null || !seen.Add(key)) continue;
                if (!entries.TryGetValue(key, out var entry))
                {
                    entry = new Entry { Element = make(row) };
                    entry.Element.AddToClassList(RowClass);
                    entries[key] = entry;
                    MadeCount++;
                }
                if (!entry.Bound || !ComparesByValue || !EqualityComparer<T>.Default.Equals(entry.Value, row))
                {
                    bind?.Invoke(entry.Element, row);
                    BindCount++;
                    entry.Value = row;
                    entry.Bound = true;
                }
                // Leaving rows keep their place while they fade; live rows go around them.
                while (child < childCount && leaving.Contains(ElementAt(child))) child++;
                if (child >= childCount || ElementAt(child) != entry.Element) Insert(Math.Min(child, childCount), entry.Element);
                child++;
                order.Add(key);
            }
            List<string> stale = null;
            foreach (var pair in entries)
                if (!seen.Contains(pair.Key))
                    (stale ?? (stale = new List<string>())).Add(pair.Key);
            if (stale == null) return;
            foreach (var key in stale)
            {
                var element = entries[key].Element;
                entries.Remove(key);
                if (Leave == null)
                {
                    element.RemoveFromHierarchy();
                    continue;
                }
                leaving.Add(element);
                Leave(element, () =>
                {
                    leaving.Remove(element);
                    element.RemoveFromHierarchy();
                });
            }
        }
    }
}
