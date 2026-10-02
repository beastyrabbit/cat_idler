using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The view's bindings on the input router. Devices are read only by Input/InputRouter.cs;
    /// this file reacts to intents, pointer actions and the router's continuous state.
    /// </summary>
    public sealed partial class ForestView
    {
        private InputRouter router;
        private readonly List<IDisposable> inputBindings = new List<IDisposable>();
        private float nextInput;

        private void CreateInput()
        {
            router = new InputRouter();
            InputRouter.Current = router;
            inputBindings.Add(router.Bind(InputIntent.Escape, InputPriority.Control, LeaveControlOnEscape));
            inputBindings.Add(router.Bind(InputIntent.Escape, InputPriority.Drawer, CloseDrawerOnEscape));
            inputBindings.Add(router.Bind(InputIntent.ToggleControl, InputPriority.Control, ToggleControl));
            inputBindings.Add(router.Bind(InputIntent.Interact, InputPriority.Control, InteractBesideControlledCat));
            inputBindings.Add(router.Bind(InputIntent.ZoomIn, InputPriority.World, () => Management(() => ZoomBy(1))));
            inputBindings.Add(router.Bind(InputIntent.ZoomOut, InputPriority.World, () => Management(() => ZoomBy(-1))));
            inputBindings.Add(router.Bind(InputIntent.Home, InputPriority.World, () => Management(FocusVillage)));
            inputBindings.Add(router.Bind(InputIntent.LevelDown, InputPriority.World, () => Management(() => ViewLevel(RenderedLevel - 1))));
            inputBindings.Add(router.Bind(InputIntent.LevelUp, InputPriority.World, () => Management(() => ViewLevel(RenderedLevel + 1))));
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryPress, InputPriority.Placement, PlaceAtPointer));
            inputBindings.Add(router.BindPointer(PointerAction.PrimaryPress, InputPriority.World, PickAtPointer));
        }

        private void ReleaseInput()
        {
            foreach (var binding in inputBindings) binding.Dispose();
            inputBindings.Clear();
            if (InputRouter.Current == router) InputRouter.Current = null;
            router = null;
        }

        /// <summary>Hands the router this frame's UI and control state, then lets it read devices and dispatch.</summary>
        internal void TickInput()
        {
            if (router == null) return;
            router.ControlMode = DirectControl;
            router.PointerOverUi = Game.UI != null && Game.UI.PointerOverPanel;
            router.TextFieldFocused = Game.UI != null && Game.UI.TextInputFocused;
            router.Tick();
        }

        private bool LeaveControlOnEscape()
        {
            if (!WorldReady || router.TextFieldFocused || !DirectControl) return false;
            LeaveCat();
            return true;
        }

        private bool CloseDrawerOnEscape()
        {
            if (!WorldReady || router.TextFieldFocused) return false;
            Game.UI?.ClosePanel();
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

        private bool PickAtPointer(PointerInfo pointer)
        {
            if (!WorldReady) return false;
            CursorTile = TileUnder(pointer.Position);
            PickAt(CursorTile);
            return true;
        }

        /// <summary>Per-frame movement, look, pan and wheel zoom from the router's continuous state.</summary>
        internal void ApplyContinuousInput()
        {
            if (router == null || !router.DevicesPresent) return;
            CursorTile = TileUnder(router.PointerPosition);
            if (router.TextFieldFocused) return;
            bool onUI = router.PointerOverUi;
            var controlled = ControlledCat;
            if (controlled != null)
            {
                if (!onUI && router.SecondaryHeld) yaw += router.PointerDelta.x * .2f;
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
            var pan = Vector3.ClampMagnitude(Camera.transform.right * router.MoveAxis.x + forward * router.MoveAxis.y, 1);
            focus += pan * Time.unscaledDeltaTime * zoom * 1.1f;
            if (onUI) return;
            float steps = router.ScrollZoom;
            if (steps != 0)
            {
                var cursor = router.PointerPosition; var anchor = GroundPoint(cursor);
                ZoomBy(steps);
                focus += anchor - GroundPoint(cursor);
                ApplyManagementCamera();
            }
            if (router.MiddleHeld || router.SecondaryHeld)
            {
                var cursor = router.PointerPosition;
                focus += GroundPoint(cursor - router.PanDelta) - GroundPoint(cursor);
            }
        }
    }
}
