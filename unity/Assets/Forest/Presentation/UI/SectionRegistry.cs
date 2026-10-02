using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Presentation
{
    /// <summary>The ordered drawer sections with their navigation group and demo visibility.</summary>
    public sealed class SectionRegistry
    {
        /// <summary>Today's navigation order (ForestUI.Sections).</summary>
        public static readonly string[] NavigationOrder = { "Village", "Cats", "Build", "Work", "Stores", "Research", "Officers", "Shrine", "World", "Defense", "Trade", "Routes", "Events" };

        public sealed class Entry
        {
            public Entry(ISection section, bool navigable, bool visibleInDemo = true)
            {
                Section = section;
                Navigable = navigable;
                VisibleInDemo = visibleInDemo;
            }

            public ISection Section { get; }

            /// <summary>Listed in the navigation (Inspect and Overview are reached another way today).</summary>
            public bool Navigable { get; }

            public bool VisibleInDemo { get; set; }
        }

        private readonly List<Entry> entries = new List<Entry>();

        public IReadOnlyList<Entry> Entries => entries;

        /// <summary>Navigable, demo-visible sections in order.</summary>
        public IEnumerable<ISection> Navigation => entries.Where(e => e.Navigable && e.VisibleInDemo).Select(e => e.Section);

        public void Add(ISection section, bool navigable = true, bool visibleInDemo = true)
        {
            if (section == null) throw new ArgumentNullException(nameof(section));
            if (Find(section.Id) != null) throw new ArgumentException("Section " + section.Id + " is already registered.");
            entries.Add(new Entry(section, navigable, visibleInDemo));
        }

        public ISection Find(string id)
        {
            foreach (var entry in entries)
                if (entry.Section.Id == id) return entry.Section;
            return null;
        }

        public Entry EntryOf(string id) => entries.Find(e => e.Section.Id == id);

        public static SectionRegistry CreateDefault()
        {
            var registry = new SectionRegistry();
            registry.Add(new VillageSection());
            registry.Add(new CatsSection());
            registry.Add(new BuildSection());
            registry.Add(new WorkSection());
            registry.Add(new StoresSection());
            registry.Add(new ResearchSection());
            registry.Add(new OfficersSection());
            registry.Add(new ShrineSection());
            registry.Add(new WorldSection());
            registry.Add(new DefenseSection());
            registry.Add(new TradeSection());
            registry.Add(new RoutesSection());
            registry.Add(new EventsSection());
            registry.Add(new InspectSection(), navigable: false);
            registry.Add(new OverviewSection(), navigable: false);
            return registry;
        }
    }
}
