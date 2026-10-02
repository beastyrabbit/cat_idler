using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A dropdown over typed items. Unlike <see cref="UiKit.Choice"/> it never calls <c>changed</c> while it
    /// is built or refreshed, only when the player picks another item. <see cref="SetItems"/> replaces the
    /// items in place and keeps the player's pick while it still exists, so a refresh never resets it or
    /// closes an open popup's owner.
    /// </summary>
    public class BoundDropdown<T> : DropdownField
    {
        public const string BoundUssClass = "bound-dropdown";

        private readonly Func<T, string> labelOf;
        private readonly Action<T> changed;
        private List<T> items;

        public BoundDropdown(IList<T> items, Func<T, string> label, T selected, Action<T> changed)
        {
            labelOf = label ?? (item => item?.ToString() ?? "");
            this.changed = changed;
            this.items = items?.ToList() ?? new List<T>();
            AddToClassList(BoundUssClass);
            choices = this.items.Select(labelOf).ToList();
            int index = this.items.IndexOf(selected);
            if (index < 0 && this.items.Count > 0) index = 0;
            if (index >= 0) SetValueWithoutNotify(choices[index]);
            this.RegisterValueChangedCallback(_ =>
            {
                int picked = this.index;
                if (picked >= 0 && picked < this.items.Count) this.changed?.Invoke(this.items[picked]);
            });
            WidgetStyles.Use(this);
        }

        public IReadOnlyList<T> Items => items;

        public T Selected
        {
            get
            {
                int picked = index;
                return picked >= 0 && picked < items.Count ? items[picked] : default;
            }
        }

        /// <summary>Selects an item without calling <c>changed</c>; ignored when the item is not listed.</summary>
        public void SetSelectedWithoutNotify(T item)
        {
            int at = items.IndexOf(item);
            if (at >= 0 && index != at) SetValueWithoutNotify(choices[at]);
        }

        /// <summary>
        /// Replaces the items without calling <c>changed</c>. The current pick stays selected when it is still
        /// listed; otherwise the first item is shown. Labels are rebuilt only when they differ.
        /// </summary>
        public void SetItems(IList<T> newItems)
        {
            var keep = Selected;
            bool hadPick = index >= 0 && index < items.Count;
            items = newItems?.ToList() ?? new List<T>();
            var labels = items.Select(labelOf).ToList();
            if (!labels.SequenceEqual(choices)) choices = labels;
            int at = hadPick ? items.IndexOf(keep) : -1;
            if (at < 0 && items.Count > 0) at = 0;
            if (at >= 0)
            {
                if (value != labels[at]) SetValueWithoutNotify(labels[at]);
            }
            else if (!string.IsNullOrEmpty(value)) SetValueWithoutNotify("");
        }
    }
}
