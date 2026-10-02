using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Rows keyed by entity id. Wave 1 stub with a naive keyed reconciliation: rows are made once per
    /// key, bound on every <see cref="Refresh"/>, reordered and removed when their key disappears.
    /// </summary>
    public class Table<T> : VisualElement
    {
        private readonly Func<IEnumerable<(string key, T row)>> rows;
        private readonly Func<T, VisualElement> make;
        private readonly Action<VisualElement, T> bind;
        private readonly Dictionary<string, VisualElement> elements = new Dictionary<string, VisualElement>();

        public Table(Func<IEnumerable<(string key, T row)>> rows, Func<T, VisualElement> make, Action<VisualElement, T> bind)
        {
            this.rows = rows;
            this.make = make;
            this.bind = bind;
            Refresh();
        }

        public void Refresh()
        {
            var seen = new HashSet<string>();
            int index = 0;
            foreach (var (key, row) in rows())
            {
                if (!seen.Add(key)) continue;
                if (!elements.TryGetValue(key, out var element))
                {
                    element = make(row);
                    elements[key] = element;
                }
                bind?.Invoke(element, row);
                if (element.parent != this || IndexOf(element) != index) Insert(index, element);
                index++;
            }
            var stale = new List<string>();
            foreach (var pair in elements)
                if (!seen.Contains(pair.Key)) stale.Add(pair.Key);
            foreach (var key in stale)
            {
                elements[key].RemoveFromHierarchy();
                elements.Remove(key);
            }
        }
    }
}
