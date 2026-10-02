using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The view's bindings on the input router. Devices are read only by Input/InputRouter.cs;
    /// this file reacts to intents, pointer actions and the router's continuous state.
    /// World selection clicks are bound by the selection owner (PrimaryClick at Selection priority);
    /// this file binds the camera, placement clicks, direct control and drag panning.
    /// </summary>
    public sealed partial class ForestView
    {
        private InputRouter router;
        private readonly List<IDisposable> inputBindings = new List<IDisposable>();
        private float nextInput;
        private bool dragPanning;
        private float dragPlaneHeight;
        private Vector3 dragAnchor;
        private VisualElement tabGuard;

        private void CreateInput()
        {
            router = new InputRouter();
            InputRouter.Current = router;
            inputBindings.Add(router.Bind(InputIntent.Escape, InputPriority.Control, LeaveControlOnEscape));
            inputBindings.Add(router.Bind(InputIntent.ToggleControl, InputPriority.Control, ToggleControl));
            inputBindings.Add(router.Bind(InputIntent.Interact, InputPriority.Control, InteractBesideControlledCat));
            inputBindings.Add(router.Bind(InputIntent.ZoomIn, InputPriority.World, () => Management(() => ZoomBy(1))));
            inputBindings.Add(router.Bind(InputIntent.ZoomOut, InputPriority.World, () => Management(() => ZoomBy(-1))));
            inputBindings.Add(router.Bind(InputIntent.Home, InputPriority.World, () => Management(GoHome)));
            inputBindings.Add(router.Bind(InputIntent.LevelDown, InputPriority.World, () => Management(() => ViewLevel(RenderedLevel - 1))));
            inputBindings.Add(router.Bind(InputIntent.LevelUp, InputPriority.World, () => Management(() => ViewLevel(RenderedLevel + 1))));
            inputBindings.Add(router.Bind(InputIntent.RotateLeft, InputPriority.World, () => Management(() => RotateView(-1))));
            inputBindings.Add(router.Bind(InputIntent.RotateRight, InputPriority.World, () => Management(() => RotateView(1))));
            inputBindings.Add(router.Bind(InputIntent.FocusSelected, InputPriority.World, FocusSelected));
            // Placement places on release without a drag, so a left drag pans even while placing.
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryClick, InputPriority.Placement, PlaceAtPointer));
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryDragStart, InputPriority.World, StartDragPan));
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryDrag, InputPriority.World, DragPan));
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryDragEnd, InputPriority.World, EndDragPan));
        }

        private void ReleaseInput()
        {
            foreach (var binding in inputBindings) binding.Dispose();
            inputBindings.Clear();
            if (tabGuard != null)
            {
                tabGuard.UnregisterCallback<KeyDownEvent>(SwallowTab, TrickleDown.TrickleDown);
                tabGuard.UnregisterCallback<NavigationMoveEvent>(SwallowTabNavigation, TrickleDown.TrickleDown);
                tabGuard = null;
            }
            router?.Dispose();
            if (InputRouter.Current == router) InputRouter.Current = null;
            router = null;
        }

        /// <summary>Hands the router this frame's UI and control state, then lets it read devices and dispatch.</summary>
        internal void TickInput()
        {
            if (router == null) return;
            GuardTabFocus();
            router.ControlMode = DirectControl;
            router.PointerOverUi = Game.UI != null && Game.UI.PointerOverPanel;
            router.TextFieldFocused = Game.UI != null && Game.UI.TextInputFocused;
            router.Tick();
            if (router.AnyInput) EndIntro();
        }

        /// <summary>
        /// Tab belongs to direct control: stop UI Toolkit from also moving keyboard focus across the HUD's
        /// buttons. Text fields keep Tab (the router dispatches only Escape while one has focus).
        /// </summary>
        private void GuardTabFocus()
        {
            if (tabGuard != null) return;
            var root = GetComponent<UIDocument>()?.rootVisualElement?.panel?.visualTree;
            if (root == null) return;
            tabGuard = root;
            tabGuard.RegisterCallback<KeyDownEvent>(SwallowTab, TrickleDown.TrickleDown);
            tabGuard.RegisterCallback<NavigationMoveEvent>(SwallowTabNavigation, TrickleDown.TrickleDown);
        }

        private void SwallowTab(KeyDownEvent e)
        {
            if (e.keyCode != KeyCode.Tab && e.character != '\t' || router != null && router.TextFieldFocused) return;
            e.StopImmediatePropagation();
            tabGuard?.focusController?.IgnoreEvent(e);
        }

        private void SwallowTabNavigation(NavigationMoveEvent e)
        {
            if (e.direction != NavigationMoveEvent.Direction.Next && e.direction != NavigationMoveEvent.Direction.Previous) return;
            if (router != null && router.TextFieldFocused) return;
            e.StopImmediatePropagation();
            tabGuard?.focusController?.IgnoreEvent(e);
        }

        private bool LeaveControlOnEscape()
        {
            if (!WorldReady || router.TextFieldFocused || !DirectControl) return false;
            LeaveCat();
            return true;
        }

        private bool ToggleControl()
        {
            if (!WorldReady) return false;
            if (DirectControl) LeaveCat(); else EnterSelectedCat();
            return true;
        }

        private bool InteractBesideControlledCat()
        {
            if (!WorldReady) return false;
            var controlled = ControlledCat;
            if (controlled == null) return false;
            var creature = Game.CurrentWorld.Creatures.Where(e => e.Health > 0 && e.Position.Level == controlled.Position.Level && Int2.Distance(e.Position, controlled.Position) <= 1).OrderBy(e => Int2.Distance(e.Position, controlled.Position)).FirstOrDefault();
            if (creature != null) { Game.Act(new GameAction { Kind = "AttackCreature", CatId = controlled.Id, TargetId = creature.Id }); return true; }
            var pile = Game.Selected.Stockpiles.Where(p => Int2.Distance(p.Position, controlled.Position) <= 1).OrderBy(p => Int2.Distance(p.Position, controlled.Position)).FirstOrDefault();
            Game.Act(new GameAction { Kind = "InteractCat", CatId = controlled.Id, TargetId = pile?.Id ?? "", Resource = pile?.Report.FirstOrDefault(s => s.Amount > 0)?.Resource ?? "food", Amount = 1 });
            return true;
        }

        /// <summary>Runs a management-camera key action; keys do nothing to the camera in direct control.</summary>
        private bool Management(Action action)
        {
            if (!WorldReady || ControlledCat != null) return false;
            action();
            return true;
        }

        private bool PlaceAtPointer(PointerInfo pointer)
        {
            if (!WorldReady || Game.UI == null || !Game.UI.HasPlacement) return false;
            CursorTile = TileUnder(pointer.Position);
            TileClicked?.Invoke(CursorTile);
            return true;
        }

        private bool StartDragPan(PointerInfo pointer)
        {
            if (!WorldReady || DirectControl) return false;
            // Pan against a fixed plane at the height grabbed, so hills and water do not change the rate.
            dragPlaneHeight = GroundPoint(pointer.PressPosition).y;
            dragAnchor = PlanePoint(pointer.PressPosition, dragPlaneHeight);
            dragPanning = true;
            DragPan(pointer);
            return true;
        }

        private bool DragPan(PointerInfo pointer)
        {
            if (!dragPanning) return false;
            if (!WorldReady || DirectControl) { dragPanning = false; return false; }
            PanBy(dragAnchor - PlanePoint(pointer.Position, dragPlaneHeight));
            return true;
        }

        private bool EndDragPan(PointerInfo pointer)
        {
            if (!dragPanning) return false;
            DragPan(pointer);
            dragPanning = false;
            return true;
        }

        /// <summary>Per-frame movement, look, pan and wheel zoom from the router's continuous state.</summary>
        internal void ApplyContinuousInput()
        {
            if (router == null || !router.DevicesPresent) return;
            CursorTile = TileUnder(router.PointerPosition);
            if (router.TextFieldFocused) return;
            var controlled = ControlledCat;
            if (controlled != null)
            {
                if (!router.PointerOverUi && router.SecondaryHeld) yaw += router.PointerDelta.x * .2f;
                if (Time.unscaledTime >= nextInput || router.MovePressed)
                {
                    int x = (int)router.MoveStepAxis.x, z = (int)router.MoveStepAxis.y;
                    if (x != 0 || z != 0)
                    {
                        var direction = Quaternion.Euler(0, yaw, 0) * new Vector3(x, 0, z);
                        var step = Mathf.Abs(direction.x) > Mathf.Abs(direction.z) ? new Int2(Math.Sign(direction.x), 0) : new Int2(0, Math.Sign(direction.z));
                        Game.Act(new GameAction { Kind = "MoveCat", CatId = controlled.Id, Position = step });
                    }
                    nextInput = Time.unscaledTime + .1f;
                }
                return;
            }
            var forward = Vector3.ProjectOnPlane(Camera.transform.forward, Vector3.up).normalized;
            var keys = Vector3.ClampMagnitude(Camera.transform.right * router.MoveAxis.x + forward * router.MoveAxis.y, 1);
            if (keys != Vector3.zero) PanBy(keys * Mathf.Min(Time.unscaledDeltaTime, .1f) * zoom * KeyPanSpeed);
            // Right/middle drags and two-finger trackpad scroll arrive as PanDelta; the router already
            // ignores gestures that started over UI and keeps a world drag latched across panels.
            if (router.PanDelta != Vector2.zero)
            {
                var cursor = router.PointerPosition; float height = shownFocus.y;
                PanBy(PlanePoint(cursor - router.PanDelta, height) - PlanePoint(cursor, height));
            }
            if (router.ScrollZoom != 0) ZoomAround(router.ScrollZoom, router.PointerPosition);
        }
    }
}
