using System;

namespace IdleCatForest.Presentation
{
    public enum UiMode { None, Placement, Control }

    /// <summary>The HUD's global state: open drawer section, mode, selected subject and popover.</summary>
    public sealed class UiState
    {
        private string openSection = "";
        private UiMode mode;
        private Subject selected;
        private bool popoverOpen;

        public event Action Changed;

        /// <summary>The open drawer section id ("" when the drawer is closed).</summary>
        public string OpenSection
        {
            get => openSection;
            set => Set(ref openSection, value ?? "");
        }

        public UiMode Mode
        {
            get => mode;
            set
            {
                if (mode == value) return;
                mode = value;
                Changed?.Invoke();
            }
        }

        public Subject Selected
        {
            get => selected;
            set
            {
                if (selected == value) return;
                selected = value;
                Changed?.Invoke();
            }
        }

        public bool PopoverOpen
        {
            get => popoverOpen;
            set
            {
                if (popoverOpen == value) return;
                popoverOpen = value;
                Changed?.Invoke();
            }
        }

        /// <summary>Set by ForestUI: closes the drawer and cancels placement.</summary>
        internal Action CloseDrawer;

        /// <summary>
        /// Escape handling. Today it closes the drawer (which also cancels a placement) and reports
        /// whether anything was open.
        /// </summary>
        public bool HandleEscape()
        {
            bool open = OpenSection != "" || Mode == UiMode.Placement;
            if (open) CloseDrawer?.Invoke();
            return open;
        }

        private void Set(ref string field, string value)
        {
            if (field == value) return;
            field = value;
            Changed?.Invoke();
        }
    }
}
