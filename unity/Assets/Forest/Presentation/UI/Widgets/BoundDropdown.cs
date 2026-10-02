using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A dropdown over typed items. Unlike <see cref="UiKit.Choice"/> it never calls
    /// <c>changed</c> while it is built, only when the player picks another item.
    /// </summary>
    public class BoundDropdown<T> : DropdownField
    {
        private readonly IList<T> items;

        public BoundDropdown(IList<T> items, Func<T, string> label, T selected, Action<T> changed)
        {
            this.items = items ?? new List<T>();
            choices = this.items.Select(label).ToList();
            int index = this.items.IndexOf(selected);
            if (index < 0 && this.items.Count > 0) index = 0;
            if (index >= 0) SetValueWithoutNotify(choices[index]);
            this.RegisterValueChangedCallback(_ =>
            {
                int picked = this.index;
                if (picked >= 0 && picked < this.items.Count) changed?.Invoke(this.items[picked]);
            });
        }

        public T Selected
        {
            get
            {
                int picked = index;
                return picked >= 0 && picked < items.Count ? items[picked] : default;
            }
        }
    }
}
