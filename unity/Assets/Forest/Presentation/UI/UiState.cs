using System;
using System.Collections.Generic;

namespace IdleCatForest.Presentation
{
    public enum UiMode { None, Placement, Control }

    /// <summary>
    /// The HUD's global state: open drawer section, mode, selected subject and popover. It also owns the
    /// Escape order of the HUD (GUIDE §6: popover → mode → drawer → deselect), bound on the input router
    /// at the frozen priorities (Popover, Placement, Drawer, Selection). Leaving direct control (Control)
    /// and modals belong to their owners.
    /// </summary>
    public sealed class UiState
    {
        /// <summary>Esc blurs a focused text field before anything else, even a modal's Esc.</summary>
        public const int TextFieldEscapePriority = InputPriority.Modal + 10;

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

        /// <summary>Set by ForestUI: closes the drawer.</summary>
        internal Action CloseDrawer;
        /// <summary>Set by ForestUI: cancels the active placement (and restores the drawer it hid).</summary>
        internal Action CancelPlacement;
        /// <summary>Set by ForestUI: clears the world selection.</summary>
        internal Action Deselect;
        /// <summary>Set by ForestUI: blurs a focused text field.</summary>
        internal Action BlurTextField;
        /// <summary>Set by ForestUI: true while a text field has keyboard focus.</summary>
        internal Func<bool> TextFieldFocused = () => false;

        /// <summary>
        /// One Escape press through the HUD's own levels in order; true when something was closed. The
        /// router does the same with priorities; this is the same order for callers without a router.
        /// </summary>
        public bool HandleEscape() => EscapeTextField() || EscapePopover() || EscapePlacement() || EscapeDrawer() || EscapeSelection();

        internal bool EscapeTextField()
        {
            if (!TextFieldFocused()) return false;
            BlurTextField?.Invoke();
            return true;
        }

        internal bool EscapePopover()
        {
            if (!PopoverOpen) return false;
            PopoverOpen = false;
            return true;
        }

        internal bool EscapePlacement()
        {
            if (Mode != UiMode.Placement || CancelPlacement == null) return false;
            CancelPlacement();
            return true;
        }

        internal bool EscapeDrawer()
        {
            if (OpenSection == "" || CloseDrawer == null) return false;
            CloseDrawer();
            return true;
        }

        internal bool EscapeSelection()
        {
            if (Selected.Kind == SubjectKind.None || Deselect == null) return false;
            Deselect();
            return true;
        }

        /// <summary>Binds the HUD's Escape levels on a router. Dispose the result to unbind.</summary>
        public IDisposable Bind(InputRouter router)
        {
            if (router == null) throw new ArgumentNullException(nameof(router));
            var bindings = new Bindings();
            bindings.Add(router.Bind(InputIntent.Escape, TextFieldEscapePriority, EscapeTextField));
            bindings.Add(router.Bind(InputIntent.Escape, InputPriority.Popover, EscapePopover));
            bindings.Add(router.Bind(InputIntent.Escape, InputPriority.Placement, EscapePlacement));
            bindings.Add(router.Bind(InputIntent.Escape, InputPriority.Drawer, EscapeDrawer));
            bindings.Add(router.Bind(InputIntent.Escape, InputPriority.Selection, EscapeSelection));
            return bindings;
        }

        private void Set(ref string field, string value)
        {
            if (field == value) return;
            field = value;
            Changed?.Invoke();
        }

        /// <summary>A group of router bindings released together.</summary>
        internal sealed class Bindings : IDisposable
        {
            private readonly List<IDisposable> items = new List<IDisposable>();

            public void Add(IDisposable binding) => items.Add(binding);

            public void Dispose()
            {
                foreach (var item in items) item.Dispose();
                items.Clear();
            }
        }
    }
}
