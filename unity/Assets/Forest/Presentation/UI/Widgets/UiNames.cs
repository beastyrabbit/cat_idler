namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Stable element names. Tests and tools find controls by these names, never by label text:
    /// <c>section-&lt;id&gt;</c>, <c>nav-&lt;id&gt;</c>, <c>act-&lt;verb&gt;-&lt;entity id&gt;</c>, <c>card-&lt;kind&gt;</c>,
    /// <c>seal</c> and <c>placement-&lt;kind&gt;</c>.
    /// </summary>
    public static class UiNames
    {
        public const string Seal = "seal";
        public const string Drawer = "drawer";
        public const string DrawerScroll = "drawer-scroll";
        public const string DrawerBack = "act-back-drawer";
        public const string DrawerClose = "act-close-drawer";
        public const string WorkProgress = "live-work-progress";

        public static string Section(string id) => "section-" + Key(id);

        public static string Nav(string id) => "nav-" + Key(id);

        public static string Card(string kind) => "card-" + Key(kind);

        public static string Placement(string kind) => "placement-" + kind;

        /// <summary><c>act-&lt;verb&gt;</c> followed by each id part, joined with '-'.</summary>
        public static string Act(string verb, params string[] ids)
        {
            string name = "act-" + verb;
            foreach (var id in ids) name += "-" + id;
            return name;
        }

        private static string Key(string id) => (id ?? "").ToLowerInvariant();
    }
}
