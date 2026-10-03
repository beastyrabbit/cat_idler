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
        /// <summary>False for keys that are mapped but not dispatched. Every demo key is live since W2-18.</summary>
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

    /// <summary>One typed character that raises an intent, whatever physical key produced it.</summary>
    public readonly struct TextBinding
    {
        public readonly char Character;
        public readonly InputIntent Intent;
        /// <summary>A key that types the same character; while it is held its own key row already raised the intent.</summary>
        public readonly Key PhysicalTwin;

        public TextBinding(char character, InputIntent intent, Key physicalTwin = Key.None)
        {
            Character = character;
            Intent = intent;
            PhysicalTwin = physicalTwin;
        }
    }

    /// <summary>The single key table. Physical keys follow the Input System's US layout names.</summary>
    public static class KeyMap
    {
        /// <summary>A primary press that moves further than this becomes a drag and produces no click.</summary>
        public const float DragThresholdPixels = 6;

        /// <summary>Screen pixels of pan per normalized unit of two-finger trackpad scroll.</summary>
        public const float TrackpadPanPixels = 24;

        /// <summary>Scroll events closer together than this belong to one gesture (wheel spin or trackpad swipe).</summary>
        public const float ScrollGestureGapSeconds = .3f;

        public static readonly KeyBinding[] Intents =
        {
            new KeyBinding(Key.Escape, InputIntent.Escape),
            new KeyBinding(Key.Tab, InputIntent.ToggleControl),
            new KeyBinding(Key.Space, InputIntent.TogglePause),
            new KeyBinding(Key.Digit1, InputIntent.Speed1),
            new KeyBinding(Key.Digit2, InputIntent.Speed2),
            new KeyBinding(Key.Digit3, InputIntent.Speed3),
            new KeyBinding(Key.F, InputIntent.FocusSelected),
            // + = and - come from the typed character (Text below), so they follow the keyboard layout.
            new KeyBinding(Key.NumpadPlus, InputIntent.ZoomIn),
            new KeyBinding(Key.NumpadMinus, InputIntent.ZoomOut),
            new KeyBinding(Key.Home, InputIntent.Home),
            new KeyBinding(Key.H, InputIntent.Home),
            new KeyBinding(Key.PageDown, InputIntent.LevelDown),
            new KeyBinding(Key.LeftBracket, InputIntent.LevelDown),
            new KeyBinding(Key.PageUp, InputIntent.LevelUp),
            new KeyBinding(Key.RightBracket, InputIntent.LevelUp),
            new KeyBinding(Key.Q, InputIntent.RotateLeft),
            new KeyBinding(Key.E, InputIntent.RotateRight, KeyContext.Management),
            new KeyBinding(Key.E, InputIntent.Interact, KeyContext.Control),
            new KeyBinding(Key.R, InputIntent.RotatePlacement),
            new KeyBinding(Key.P, InputIntent.PhotoMode),
            new KeyBinding(Key.Enter, InputIntent.ToggleFullscreen, modifiers: KeyModifiers.Command),
        };

        /// <summary>
        /// Intents raised by typed characters (<c>Keyboard.onTextInput</c>), so "+" works on every layout
        /// (on German QWERTZ it is not the US Equals key). Typed characters repeat while the key is held.
        /// </summary>
        public static readonly TextBinding[] Text =
        {
            new TextBinding('+', InputIntent.ZoomIn, Key.NumpadPlus),
            new TextBinding('=', InputIntent.ZoomIn),
            new TextBinding('-', InputIntent.ZoomOut, Key.NumpadMinus),
        };

        // Continuous movement (MoveAxis): pans the management camera, steps the controlled cat.
        public static readonly Key[] MoveRight = { Key.D, Key.RightArrow };
        public static readonly Key[] MoveLeft = { Key.A, Key.LeftArrow };
        public static readonly Key[] MoveUp = { Key.W, Key.UpArrow };
        public static readonly Key[] MoveDown = { Key.S, Key.DownArrow };
    }
}
