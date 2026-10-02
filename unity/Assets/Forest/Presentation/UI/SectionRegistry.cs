using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Authority;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The ordered drawer sections with their dock group, dock icon and demo visibility. The dock follows
    /// GUIDE §5.4: Overview · Cats · Village | Build · Work · Stores | Research · Officers · Shrine |
    /// World · Defense · Trade · Routes | Events. Inspection is not a section: the tethered card is the
    /// only inspector (W2-14).
    /// </summary>
    public sealed class SectionRegistry
    {
        /// <summary>The dock order (ForestUI.Sections).</summary>
        public static readonly string[] NavigationOrder = { "Overview", "Cats", "Village", "Build", "Work", "Stores", "Research", "Officers", "Shrine", "World", "Defense", "Trade", "Routes", "Events" };

        public const string ColonyGroup = "Colony", MakingGroup = "Making", GrowthGroup = "Growth", WorldGroup = "World", LogGroup = "Log";

        /// <summary>The dock group of each section; a separator is drawn between groups.</summary>
        private static readonly Dictionary<string, string> Groups = new Dictionary<string, string>
        {
            { "Overview", ColonyGroup }, { "Cats", ColonyGroup }, { "Village", ColonyGroup },
            { "Build", MakingGroup }, { "Work", MakingGroup }, { "Stores", MakingGroup },
            { "Research", GrowthGroup }, { "Officers", GrowthGroup }, { "Shrine", GrowthGroup },
            { "World", WorldGroup }, { "Defense", WorldGroup }, { "Trade", WorldGroup }, { "Routes", WorldGroup },
            { "Events", LogGroup },
        };

        public sealed class Entry
        {
            public Entry(ISection section, bool navigable, bool visibleInDemo = true)
            {
                Section = section;
                Navigable = navigable;
                VisibleInDemo = visibleInDemo;
            }

            public ISection Section { get; }
            public string Id => Section.Id;

            /// <summary>Listed in the dock.</summary>
            public bool Navigable { get; }

            public bool VisibleInDemo { get; set; }

            /// <summary>The dock group (GUIDE §5.4), independent of the section's own legacy group label.</summary>
            public string Group => Groups.TryGetValue(Section.Id, out var group) ? group : string.IsNullOrEmpty(Section.Group) ? LogGroup : Section.Group;

            /// <summary>The glyph name under Resources/UI/Icons (from docs/ui-redesign/mockups/icons.js).</summary>
            public string Icon => string.IsNullOrEmpty(Section.Icon) ? Section.Id.ToLowerInvariant() : Section.Icon;

            /// <summary>Shown in the dock: navigable, visible in the demo build and allowed by DemoScope.</summary>
            public bool InDock => Navigable && VisibleInDemo && DemoScope.IsSectionVisible(Section.Id);
        }

        private readonly List<Entry> entries = new List<Entry>();

        public IReadOnlyList<Entry> Entries => entries;

        /// <summary>Dock entries in order.</summary>
        public IEnumerable<Entry> Dock => entries.Where(e => e.InDock);

        /// <summary>Navigable, demo-visible sections in order.</summary>
        public IEnumerable<ISection> Navigation => Dock.Select(e => e.Section);

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
            registry.Add(new OverviewSection());
            registry.Add(new CatsSection());
            registry.Add(new VillageSection());
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
            return registry;
        }
    }
}
