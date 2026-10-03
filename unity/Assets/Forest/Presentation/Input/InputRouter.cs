using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>How a scroll gesture is read: wheel steps zoom, two-finger trackpad motion pans.</summary>
    public enum ScrollStyle
    {
        /// <summary>Decide per gesture: horizontal or fractional motion is a trackpad, whole vertical steps are a wheel.</summary>
        Auto,
        /// <summary>Every scroll zooms (a mouse wheel that reports fractional steps).</summary>
        WheelZooms,
        /// <summary>Every scroll pans; Command or Control with scroll zooms.</summary>
        TrackpadPans,
    }

    /// <summary>
    /// The one place that reads keyboard and mouse devices and decides what every key and button
    /// means. <see cref="Tick"/> runs once per frame (ForestView calls it at the top of Update): it
    /// refreshes the continuous state, then dispatches intents and pointer actions to the bound
    /// handlers from the highest priority down until one returns true.
    /// </summary>
    /// <remarks>
    /// Mac trackpads: two-finger scroll arrives as fractional or horizontal mouse scroll and pans the
    /// map (<see cref="PanDelta"/>); Command (or Control) plus scroll zooms. Unity's Input System does not
    /// report pinch (magnify) gestures on macOS, so pinch cannot zoom; the Command+scroll path and the
    /// +/- keys cover it.
    /// </remarks>
    public sealed class InputRouter : IDisposable
    {
        /// <summary>The router of the running view (created in ForestView.Awake).</summary>
        public static InputRouter Current { get; set; }

        /// <summary>The player's scroll preference (Auto until the settings page offers a choice).</summary>
        public static ScrollStyle ScrollStyle { get; set; } = ScrollStyle.Auto;

        // Continuous state, refreshed by Tick.
        /// <summary>False when no mouse or keyboard is connected; nothing else is read or dispatched then.</summary>
        public bool DevicesPresent { get; private set; }
        /// <summary>Held movement keys (WASD and arrows): x right, y forward, each -1, 0 or 1.</summary>
        public Vector2 MoveAxis { get; private set; }
        /// <summary>Like <see cref="MoveAxis"/>, but also counts a key tapped within this frame.</summary>
        public Vector2 MoveStepAxis { get; private set; }
        /// <summary>A movement key went down this frame.</summary>
        public bool MovePressed { get; private set; }
        public Vector2 PointerPosition { get; private set; }
        public Vector2 PointerDelta { get; private set; }
        /// <summary>Zoom steps this frame (positive zooms in): wheel steps, or Command+scroll. Zero over UI.</summary>
        public float ScrollZoom { get; private set; }
        /// <summary>
        /// Map pan this frame in screen pixels, in the direction the content should move: pointer motion of a
        /// right or middle drag that started over the world (latched until release, even across UI) plus
        /// two-finger trackpad scroll over the world. Left drags arrive as pointer actions instead.
        /// </summary>
        public Vector2 PanDelta { get; private set; }
        /// <summary>The current scroll gesture was read as a trackpad (pan) rather than a wheel (zoom).</summary>
        public bool TrackpadScrolling { get; private set; }
        public bool PrimaryHeld { get; private set; }
        public bool SecondaryHeld { get; private set; }
        public bool MiddleHeld { get; private set; }
        public bool ShiftHeld { get; private set; }
        /// <summary>Command on macOS, Control elsewhere.</summary>
        public bool CommandHeld { get; private set; }
        /// <summary>A primary press that started over the world has moved past the drag threshold and is still held.</summary>
        public bool PrimaryDragging => primaryTracked && primaryDragging;
        /// <summary>Any key, button, scroll or movement happened this frame (used to skip camera intros).</summary>
        public bool AnyInput { get; private set; }

        // Set by their owners before Tick.
        /// <summary>The pointer is over an interactive panel; world pointer actions do not start.</summary>
        public bool PointerOverUi { get; set; }
        /// <summary>A text field has focus; only <see cref="InputIntent.Escape"/> is dispatched.</summary>
        public bool TextFieldFocused { get; set; }
        /// <summary>The view is in direct control of a cat; decides what E means.</summary>
        public bool ControlMode { get; set; }

        private static readonly int IntentCount = Enum.GetValues(typeof(InputIntent)).Length;
        private static readonly int PointerCount = Enum.GetValues(typeof(PointerAction)).Length;
        private readonly Binding[][] intentHandlers = new Binding[IntentCount][];
        private readonly Binding[][] pointerHandlers = new Binding[PointerCount][];
        private readonly bool[] raised = new bool[IntentCount];
        private readonly List<char> typed = new List<char>();
        private Keyboard textSource;
        private int sequence;
        private bool primaryTracked, primaryDragging, secondaryTracked, secondaryMoved, middleTracked;
        private Vector2 primaryPress, secondaryPress;
        private float lastScrollTime = float.NegativeInfinity;

        public IDisposable Bind(InputIntent intent, int priority, Func<bool> handler)
        {
            if (handler == null) throw new ArgumentNullException(nameof(handler));
            return Add(intentHandlers, (int)intent, new Binding(priority, sequence++, handler, null));
        }

        public IDisposable BindPointer(PointerAction action, int priority, Func<PointerInfo, bool> handler)
        {
            if (handler == null) throw new ArgumentNullException(nameof(handler));
            return Add(pointerHandlers, (int)action, new Binding(priority, sequence++, null, handler));
        }

        /// <summary>Runs the intent's handlers as if its key was pressed; true when one handled it.</summary>
        public bool Dispatch(InputIntent intent)
        {
            var handlers = intentHandlers[(int)intent];
            if (handlers == null) return false;
            foreach (var binding in handlers)
            {
                if (binding.Removed) continue;
                try { if (binding.Intent()) return true; }
                catch (Exception error) { return Failed(error); }
            }
            return false;
        }

        /// <summary>Runs the pointer action's handlers; true when one handled it.</summary>
        public bool DispatchPointer(PointerAction action, PointerInfo pointer)
        {
            var handlers = pointerHandlers[(int)action];
            if (handlers == null) return false;
            foreach (var binding in handlers)
            {
                if (binding.Removed) continue;
                try { if (binding.Pointer(pointer)) return true; }
                catch (Exception error) { return Failed(error); }
            }
            return false;
        }

        /// <summary>
        /// Reads one scroll sample as a trackpad (true) or a wheel (false). A gesture keeps the reading of
        /// its first sample, so a fast wheel spin that reports fractional steps never turns into a pan;
        /// horizontal motion always means a trackpad.
        /// </summary>
        public static bool ReadsAsTrackpad(Vector2 scroll, bool gestureContinues, bool gestureWasTrackpad, ScrollStyle style)
        {
            if (style == ScrollStyle.WheelZooms) return false;
            if (style == ScrollStyle.TrackpadPans) return true;
            if (Mathf.Abs(scroll.x) > 1e-4f) return true;
            if (gestureContinues) return gestureWasTrackpad;
            return Mathf.Abs(scroll.y - Mathf.Round(scroll.y)) > .01f;
        }

        public void Tick()
        {
            var mouse = Mouse.current;
            var keyboard = Keyboard.current;
            DevicesPresent = mouse != null && keyboard != null;
            Listen(keyboard);
            if (!DevicesPresent)
            {
                ClearContinuous();
                primaryTracked = primaryDragging = secondaryTracked = middleTracked = false;
                typed.Clear();
                return;
            }
            PointerPosition = mouse.position.ReadValue();
            PointerDelta = mouse.delta.ReadValue();
            PrimaryHeld = mouse.leftButton.isPressed;
            SecondaryHeld = mouse.rightButton.isPressed;
            MiddleHeld = mouse.middleButton.isPressed;
            ShiftHeld = keyboard.shiftKey.isPressed;
            bool mac = Application.platform == RuntimePlatform.OSXPlayer || Application.platform == RuntimePlatform.OSXEditor;
            CommandHeld = mac ? keyboard.leftMetaKey.isPressed || keyboard.rightMetaKey.isPressed : keyboard.ctrlKey.isPressed;
            var scroll = mouse.scroll.ReadValue();
            AnyInput = keyboard.anyKey.wasPressedThisFrame || mouse.leftButton.wasPressedThisFrame || mouse.rightButton.wasPressedThisFrame
                || mouse.middleButton.wasPressedThisFrame || scroll != Vector2.zero || typed.Count > 0;
            if (TextFieldFocused)
            {
                MoveAxis = MoveStepAxis = PanDelta = Vector2.zero;
                MovePressed = TrackpadScrolling = false; ScrollZoom = 0;
                primaryTracked = primaryDragging = secondaryTracked = middleTracked = false;
                typed.Clear();
                if (Raised(keyboard, InputIntent.Escape)) Dispatch(InputIntent.Escape);
                return;
            }
            MoveAxis = new Vector2(Held(keyboard, KeyMap.MoveRight, false) - Held(keyboard, KeyMap.MoveLeft, false), Held(keyboard, KeyMap.MoveUp, false) - Held(keyboard, KeyMap.MoveDown, false));
            MoveStepAxis = new Vector2(Held(keyboard, KeyMap.MoveRight, true) - Held(keyboard, KeyMap.MoveLeft, true), Held(keyboard, KeyMap.MoveUp, true) - Held(keyboard, KeyMap.MoveDown, true));
            MovePressed = Pressed(keyboard, KeyMap.MoveRight) || Pressed(keyboard, KeyMap.MoveLeft) || Pressed(keyboard, KeyMap.MoveUp) || Pressed(keyboard, KeyMap.MoveDown);
            AnyInput |= MoveAxis != Vector2.zero;
            PanDelta = Vector2.zero;
            ReadScroll(scroll, CommandHeld || keyboard.ctrlKey.isPressed);
            ReadPan(mouse);

            Array.Clear(raised, 0, raised.Length);
            foreach (var key in KeyMap.Intents)
                if (Applies(key) && keyboard[key.Key].wasPressedThisFrame) raised[(int)key.Intent] = true;
            if (!CommandHeld)
                foreach (char c in typed)
                    foreach (var text in KeyMap.Text)
                        if (text.Character == c && (text.PhysicalTwin == Key.None || !keyboard[text.PhysicalTwin].isPressed)) raised[(int)text.Intent] = true;
            typed.Clear();
            for (int i = 0; i < raised.Length; i++)
                if (raised[i]) Dispatch((InputIntent)i);
            TrackPointer(mouse);
        }

        /// <summary>Stops listening for typed characters.</summary>
        public void Dispose()
        {
            Listen(null);
            typed.Clear();
        }

        private void Listen(Keyboard keyboard)
        {
            if (ReferenceEquals(textSource, keyboard)) return;
            if (textSource != null) textSource.onTextInput -= OnText;
            textSource = keyboard;
            if (textSource != null) textSource.onTextInput += OnText;
        }

        private void OnText(char c) { if (typed.Count < 64) typed.Add(c); }

        private void ReadScroll(Vector2 scroll, bool zoomModifier)
        {
            ScrollZoom = 0;
            if (scroll == Vector2.zero) return;
            float now = Time.unscaledTime;
            TrackpadScrolling = ReadsAsTrackpad(scroll, now - lastScrollTime < KeyMap.ScrollGestureGapSeconds, TrackpadScrolling, ScrollStyle);
            lastScrollTime = now;
            // Scrolling over a panel scrolls the panel, never the map.
            if (PointerOverUi) return;
            if (!TrackpadScrolling || zoomModifier) ScrollZoom = scroll.y;
            else PanDelta += -scroll * KeyMap.TrackpadPanPixels;
        }

        private void ReadPan(Mouse mouse)
        {
            // Right and middle drags pan only when they start over the world, and keep panning until release.
            if (mouse.rightButton.wasPressedThisFrame && !PointerOverUi) { secondaryTracked = true; secondaryMoved = false; secondaryPress = PointerPosition; }
            if (mouse.middleButton.wasPressedThisFrame && !PointerOverUi) middleTracked = true;
            if (!mouse.middleButton.isPressed) middleTracked = false;
            if (secondaryTracked && SecondaryHeld || middleTracked) PanDelta += PointerDelta;
        }

        private void TrackPointer(Mouse mouse)
        {
            var primary = mouse.leftButton;
            if (primary.wasPressedThisFrame && !PointerOverUi)
            {
                primaryTracked = true; primaryDragging = false; primaryPress = PointerPosition;
                DispatchPointer(PointerAction.PrimaryPress, Pointer(primaryPress));
            }
            if (primaryTracked)
            {
                // A gesture that started over the world keeps its drag events until release.
                if (!primaryDragging && (PointerPosition - primaryPress).magnitude > KeyMap.DragThresholdPixels)
                {
                    primaryDragging = true;
                    DispatchPointer(PointerAction.PrimaryDragStart, Pointer(primaryPress));
                }
                else if (primaryDragging && primary.isPressed) DispatchPointer(PointerAction.PrimaryDrag, Pointer(primaryPress));
                if (!primary.isPressed)
                {
                    primaryTracked = false;
                    if (primaryDragging) DispatchPointer(PointerAction.PrimaryDragEnd, Pointer(primaryPress));
                    else if (!PointerOverUi) DispatchPointer(PointerAction.PrimaryClick, Pointer(primaryPress));
                    primaryDragging = false;
                }
            }
            var secondary = mouse.rightButton;
            if (secondaryTracked)
            {
                if ((PointerPosition - secondaryPress).magnitude > KeyMap.DragThresholdPixels) secondaryMoved = true;
                if (!secondary.isPressed)
                {
                    secondaryTracked = false;
                    if (!secondaryMoved && !PointerOverUi) DispatchPointer(PointerAction.SecondaryClick, Pointer(secondaryPress));
                }
            }
        }

        private PointerInfo Pointer(Vector2 press) => new PointerInfo(PointerPosition, PointerDelta, press, ShiftHeld, CommandHeld);

        private bool Applies(KeyBinding key)
        {
            if (!key.Live) return false;
            if (key.Context == KeyContext.Control && !ControlMode || key.Context == KeyContext.Management && ControlMode) return false;
            if ((key.Modifiers & KeyModifiers.Command) != 0 && !CommandHeld) return false;
            return (key.Modifiers & KeyModifiers.Shift) == 0 || ShiftHeld;
        }

        private bool Raised(Keyboard keyboard, InputIntent intent)
        {
            foreach (var key in KeyMap.Intents)
                if (key.Intent == intent && Applies(key) && keyboard[key.Key].wasPressedThisFrame) return true;
            return false;
        }

        private static int Held(Keyboard keyboard, Key[] keys, bool includeTaps)
        {
            foreach (var key in keys)
                if (keyboard[key].isPressed || includeTaps && keyboard[key].wasPressedThisFrame) return 1;
            return 0;
        }

        private static bool Pressed(Keyboard keyboard, Key[] keys)
        {
            foreach (var key in keys)
                if (keyboard[key].wasPressedThisFrame) return true;
            return false;
        }

        private void ClearContinuous()
        {
            MoveAxis = MoveStepAxis = PointerDelta = PanDelta = Vector2.zero;
            MovePressed = PrimaryHeld = SecondaryHeld = MiddleHeld = ShiftHeld = CommandHeld = TrackpadScrolling = AnyInput = false;
            ScrollZoom = 0;
        }

        private static bool Failed(Exception error)
        {
            // One failing handler must not stop the frame's remaining input; it counts as handled
            // so a lower priority does not act on the same key twice.
            Debug.LogException(error);
            return true;
        }

        private static IDisposable Add(Binding[][] table, int index, Binding binding)
        {
            var list = new List<Binding>(table[index] ?? Array.Empty<Binding>()) { binding };
            list.Sort((a, b) => a.Priority != b.Priority ? b.Priority.CompareTo(a.Priority) : a.Sequence.CompareTo(b.Sequence));
            table[index] = list.ToArray();
            binding.Table = table; binding.Index = index;
            return binding;
        }

        private static void Remove(Binding binding)
        {
            var table = binding.Table; var current = table[binding.Index];
            if (current == null) return;
            table[binding.Index] = Array.FindAll(current, b => !ReferenceEquals(b, binding));
        }

        private sealed class Binding : IDisposable
        {
            public readonly int Priority, Sequence;
            public readonly Func<bool> Intent;
            public readonly Func<PointerInfo, bool> Pointer;
            public Binding[][] Table;
            public int Index;
            public bool Removed;

            public Binding(int priority, int sequence, Func<bool> intent, Func<PointerInfo, bool> pointer)
            {
                Priority = priority; Sequence = sequence; Intent = intent; Pointer = pointer;
            }

            public void Dispose()
            {
                if (Removed) return;
                Removed = true;
                Remove(this);
            }
        }
    }
}
