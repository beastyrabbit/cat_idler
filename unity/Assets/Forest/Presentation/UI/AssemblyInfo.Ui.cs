using System.Runtime.CompilerServices;

// PlayMode tests drive the HUD through its internal API (Tick, Placement, Drawer) instead of reflection.
[assembly: InternalsVisibleTo("IdleCatForest.PlayModeTests")]
