using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The element helpers every section builds with. They keep the legacy USS class names.</summary>
    public static class UiKit
    {
        /// <summary>The root a section returns from <see cref="ISection.Build"/>, named <c>section-&lt;id&gt;</c>.</summary>
        public static VisualElement SectionRoot(string sectionId)
        {
            var root = new VisualElement { name = UiNames.Section(sectionId) };
            root.style.flexShrink = 0;
            return root;
        }

        public static T Named<T>(this T element, string name) where T : VisualElement
        {
            element.name = name;
            return element;
        }

        public static VisualElement Element(VisualElement parent, string classes)
        {
            var e = new VisualElement();
            foreach (var cls in classes.Split(' ')) e.AddToClassList(cls);
            parent.Add(e);
            return e;
        }

        public static Label Label(VisualElement parent, string text, string cls)
        {
            var e = new Label(text);
            e.AddToClassList(cls);
            parent.Add(e);
            return e;
        }

        public static Label Text(VisualElement parent, string text, string cls = "body") => Label(parent, text, cls);

        public static Button Button(VisualElement parent, string text, Action click, string tooltip = "")
        {
            var b = new Button(click) { text = text, tooltip = tooltip };
            parent.Add(b);
            return b;
        }

        public static TextField Field(VisualElement parent, string label, string value)
        {
            var f = new TextField(label) { value = value };
            parent.Add(f);
            return f;
        }

        public static DoubleField Number(VisualElement parent, string label, double value, Action<double> set)
        {
            var f = new DoubleField(label) { value = value };
            f.RegisterValueChangedCallback(e => set(e.newValue));
            parent.Add(f);
            return f;
        }

        /// <summary>A dropdown that reports its initial value through <paramref name="changed"/> while it is built.</summary>
        public static DropdownField Choice(VisualElement parent, string label, IEnumerable<string> options, string value, Action<string> changed)
        {
            var list = options.ToList();
            if (list.Count == 0) list.Add("None available");
            int index = Mathf.Max(0, list.IndexOf(value));
            var f = new DropdownField(label, list, index);
            changed(list[index]);
            f.RegisterValueChangedCallback(e => changed(e.newValue));
            parent.Add(f);
            return f;
        }

        /// <summary>A 0-100 bar refreshed every frame while its section is open.</summary>
        public static ProgressBar Meter(UiContext ctx, VisualElement parent, string name, Func<double> read)
        {
            var bar = new ProgressBar { lowValue = 0, highValue = 100 };
            parent.Add(bar);
            void Refresh()
            {
                double value = read();
                bar.value = (float)value;
                bar.title = name + " " + value.ToString("0");
            }
            Refresh();
            ctx.Live(Refresh);
            return bar;
        }

        /// <summary>The live work progress bar, named <c>live-work-progress</c>.</summary>
        public static ProgressBar WorkMeter(UiContext ctx, VisualElement parent, Func<double> progress, Func<double> required)
        {
            var bar = new ProgressBar { name = "live-work-progress", lowValue = 0 };
            parent.Add(bar);
            void Refresh()
            {
                double value = progress();
                double total = Math.Max(1, required());
                bar.highValue = (float)total;
                bar.value = (float)value;
                bar.title = "Work " + value.ToString("0.0") + " / " + total.ToString("0");
            }
            Refresh();
            ctx.Live(Refresh);
            return bar;
        }

        public static VisualElement Card(VisualElement parent, string title)
        {
            var card = Element(parent, "card");
            Label(card, title, "subtitle");
            return card;
        }

        public static VisualElement Separator(VisualElement parent) => Element(parent, "separator");
    }
}
