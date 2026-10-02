using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Discrete player intents. <see cref="KeyMap"/> decides which key raises which intent and
    /// <see cref="InputRouter"/> dispatches them. Intents raised in the same frame dispatch in
    /// declaration order, which keeps today's key order (Escape, Tab, zoom, Home, levels).
    /// </summary>
    public enum InputIntent
    {
        Escape,
        ToggleControl,
        TogglePause,
        Speed1,
        Speed2,
        Speed3,
        FocusSelected,
        ZoomIn,
        ZoomOut,
        Home,
        LevelDown,
        LevelUp,
        RotateLeft,
        RotateRight,
        Interact,
        RotatePlacement,
        PhotoMode,
        ToggleFullscreen,
    }

    /// <summary>
    /// Handler priorities: higher runs first, ties run in registration order. The Escape order is
    /// therefore modal, popover, placement, leave control, close drawer, deselect, open pause.
    /// </summary>
    public static class InputPriority
    {
        /// <summary>Shell dialogs, the settings page and the end card.</summary>
        public const int Modal = 100;
        public const int Popover = 90;
        public const int Placement = 80;
        /// <summary>Leave direct control.</summary>
        public const int Control = 70;
        public const int Drawer = 60;
        /// <summary>Deselect.</summary>
        public const int Selection = 50;
        /// <summary>Camera and picking.</summary>
        public const int World = 40;
        /// <summary>Open the pause menu.</summary>
        public const int Fallback = 0;
    }

    /// <summary>
    /// Pointer gestures over the world. None is dispatched while the pointer is over UI when the
    /// gesture starts; a press that moves more than <see cref="KeyMap.DragThresholdPixels"/> becomes
    /// a drag and produces no click.
    /// </summary>
    public enum PointerAction
    {
        PrimaryClick,
        SecondaryClick,
        PrimaryDragStart,
        PrimaryDrag,
        PrimaryDragEnd,
        /// <summary>
        /// The primary button went down over the world this frame. Today's world selection and
        /// placement react on the press, so W1 binds them here; a later owner may move them to
        /// <see cref="PrimaryClick"/>.
        /// </summary>
        PrimaryPress,
    }

    /// <summary>Pointer state handed to a pointer handler, in screen pixels.</summary>
    public readonly struct PointerInfo
    {
        public readonly Vector2 Position;
        public readonly Vector2 Delta;
        public readonly Vector2 PressPosition;
        public readonly bool ShiftHeld;
        public readonly bool CommandHeld;

        public PointerInfo(Vector2 position, Vector2 delta, Vector2 pressPosition, bool shiftHeld, bool commandHeld)
        {
            Position = position;
            Delta = delta;
            PressPosition = pressPosition;
            ShiftHeld = shiftHeld;
            CommandHeld = commandHeld;
        }
    }
}
