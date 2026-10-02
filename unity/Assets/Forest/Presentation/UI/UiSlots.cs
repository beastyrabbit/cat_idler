namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The mount slots ForestUI creates. Each is a full-size, non-picking overlay on the HUD root, so a
    /// mounted element positions itself exactly as if it were a direct child of the root.
    /// </summary>
    public static class UiSlots
    {
        public const string Chip = "slot-chip";
        public const string Seal = "slot-seal";
        public const string Pills = "slot-pills";
        public const string Dock = "slot-dock";
        public const string Hint = "slot-hint";
        public const string Drawer = "slot-drawer";
        public const string Card = "slot-card";
        public const string Toasts = "slot-toasts";
        public const string Mode = "slot-mode";
        public const string Reins = "slot-reins";
        public const string Objective = "slot-objective";
        public const string Modal = "slot-modal";

        public static readonly string[] All = { Chip, Seal, Pills, Dock, Hint, Drawer, Card, Toasts, Mode, Reins, Objective, Modal };
    }
}
