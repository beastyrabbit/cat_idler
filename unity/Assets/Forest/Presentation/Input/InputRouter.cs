using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The one place that reads keyboard and mouse devices and decides what every key and button
    /// means. <see cref="Tick"/> runs once per frame (ForestView calls it at the top of Update): it
    /// refreshes the continuous state, then dispatches intents and pointer actions to the bound
    /// handlers from the highest priority down until one returns true.
    /// </summary>
    public sealed class InputRouter
    {
        /// <summary>The router of the running view (created in ForestView.Awake).</summary>
        public static InputRouter Current { get; set; }

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
        /// <summary>Normalized wheel steps this frame (positive zooms in).</summary>
        public float ScrollZoom { get; private set; }
        /// <summary>Pointer motion in pixels while a pan button (right or middle) is held.</summary>
        public Vector2 PanDelta { get; private set; }
        public bool PrimaryHeld { get; private set; }
        public bool SecondaryHeld { get; private set; }
        public bool MiddleHeld { get; private set; }
        public bool ShiftHeld { get; private set; }
        /// <summary>Command on macOS, Control elsewhere.</summary>
        public bool CommandHeld { get; private set; }

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
        private int sequence;
        private bool primaryTracked, primaryDragging, secondaryTracked, secondaryMoved;
        private Vector2 primaryPress, secondaryPress;

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

        public void Tick()
        {
            var mouse = Mouse.current;
            var keyboard = Keyboard.current;
            DevicesPresent = mouse != null && keyboard != null;
            if (!DevicesPresent)
            {
                ClearContinuous();
                primaryTracked = primaryDragging = secondaryTracked = false;
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
            if (TextFieldFocused)
            {
                MoveAxis = MoveStepAxis = PanDelta = Vector2.zero;
                MovePressed = false; ScrollZoom = 0;
                primaryTracked = primaryDragging = secondaryTracked = false;
                if (Raised(keyboard, InputIntent.Escape)) Dispatch(InputIntent.Escape);
                return;
            }
            MoveAxis = new Vector2(Held(keyboard, KeyMap.MoveRight, false) - Held(keyboard, KeyMap.MoveLeft, false), Held(keyboard, KeyMap.MoveUp, false) - Held(keyboard, KeyMap.MoveDown, false));
            MoveStepAxis = new Vector2(Held(keyboard, KeyMap.MoveRight, true) - Held(keyboard, KeyMap.MoveLeft, true), Held(keyboard, KeyMap.MoveUp, true) - Held(keyboard, KeyMap.MoveDown, true));
            MovePressed = Pressed(keyboard, KeyMap.MoveRight) || Pressed(keyboard, KeyMap.MoveLeft) || Pressed(keyboard, KeyMap.MoveUp) || Pressed(keyboard, KeyMap.MoveDown);
            ScrollZoom = mouse.scroll.ReadValue().y;
            PanDelta = SecondaryHeld || MiddleHeld ? PointerDelta : Vector2.zero;

            Array.Clear(raised, 0, raised.Length);
            foreach (var key in KeyMap.Intents)
                if (Applies(key) && keyboard[key.Key].wasPressedThisFrame) raised[(int)key.Intent] = true;
            for (int i = 0; i < raised.Length; i++)
                if (raised[i]) Dispatch((InputIntent)i);
            TrackPointer(mouse);
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
            if (secondary.wasPressedThisFrame && !PointerOverUi) { secondaryTracked = true; secondaryMoved = false; secondaryPress = PointerPosition; }
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
            MovePressed = PrimaryHeld = SecondaryHeld = MiddleHeld = ShiftHeld = CommandHeld = false;
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
