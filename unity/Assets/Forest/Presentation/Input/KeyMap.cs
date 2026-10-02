using System;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>Which control mode a key binding applies in (the router's <c>ControlMode</c>).</summary>
    public enum KeyContext { Any, Management, Control }

    /// <summary>Modifiers a key binding requires. A binding without modifiers ignores them.</summary>
    [Flags]
    public enum KeyModifiers { None = 0, Command = 1, Shift = 2 }

    /// <summary>One row of the key table.</summary>
    public readonly struct KeyBinding
    {
        public readonly Key Key;
        public readonly InputIntent Intent;
        public readonly KeyContext Context;
        public readonly KeyModifiers Modifiers;
        /// <summary>
        /// False for keys that are mapped but not dispatched yet. W1 maps the demo's new keys
        /// without changing what the player can do; their owner turns them live.
        /// </summary>
        public readonly bool Live;

        public KeyBinding(Key key, InputIntent intent, KeyContext context = KeyContext.Any, KeyModifiers modifiers = KeyModifiers.None, bool live = true)
        {
            Key = key;
            Intent = intent;
            Context = context;
            Modifiers = modifiers;
            Live = live;
        }
    }

    /// <summary>The single key table. Physical keys follow the Input System's US layout names.</summary>
    public static class KeyMap
    {
        /// <summary>A primary press that moves further than this becomes a drag and produces no click.</summary>
        public const float DragThresholdPixels = 6;

        public static readonly KeyBinding[] Intents =
        {
            new KeyBinding(Key.Escape, InputIntent.Escape),
            new KeyBinding(Key.Tab, InputIntent.ToggleControl),
            new KeyBinding(Key.Space, InputIntent.TogglePause, live: false),
            new KeyBinding(Key.Digit1, InputIntent.Speed1, live: false),
            new KeyBinding(Key.Digit2, InputIntent.Speed2, live: false),
            new KeyBinding(Key.Digit3, InputIntent.Speed3, live: false),
            new KeyBinding(Key.F, InputIntent.FocusSelected, live: false),
            new KeyBinding(Key.Equals, InputIntent.ZoomIn),
            new KeyBinding(Key.NumpadPlus, InputIntent.ZoomIn),
            new KeyBinding(Key.Minus, InputIntent.ZoomOut),
            new KeyBinding(Key.NumpadMinus, InputIntent.ZoomOut),
            new KeyBinding(Key.Home, InputIntent.Home),
            new KeyBinding(Key.H, InputIntent.Home, live: false),
            new KeyBinding(Key.PageDown, InputIntent.LevelDown),
            new KeyBinding(Key.LeftBracket, InputIntent.LevelDown, live: false),
            new KeyBinding(Key.PageUp, InputIntent.LevelUp),
            new KeyBinding(Key.RightBracket, InputIntent.LevelUp, live: false),
            new KeyBinding(Key.Q, InputIntent.RotateLeft, live: false),
            new KeyBinding(Key.E, InputIntent.RotateRight, KeyContext.Management, live: false),
            new KeyBinding(Key.E, InputIntent.Interact, KeyContext.Control),
            new KeyBinding(Key.R, InputIntent.RotatePlacement, live: false),
            new KeyBinding(Key.P, InputIntent.PhotoMode, live: false),
            new KeyBinding(Key.Enter, InputIntent.ToggleFullscreen, modifiers: KeyModifiers.Command, live: false),
        };

        // Continuous movement (MoveAxis): pans the management camera, steps the controlled cat.
        public static readonly Key[] MoveRight = { Key.D, Key.RightArrow };
        public static readonly Key[] MoveLeft = { Key.A, Key.LeftArrow };
        public static readonly Key[] MoveUp = { Key.W, Key.UpArrow };
        public static readonly Key[] MoveDown = { Key.S, Key.DownArrow };
    }
}
