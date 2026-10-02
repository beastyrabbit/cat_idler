using System.Runtime.CompilerServices;

// PlayMode tests drive the view's internal accessors (ForceReconcile, UpdateCamera, TickInput, ...)
// instead of reaching private members through reflection.
[assembly: InternalsVisibleTo("IdleCatForest.PlayModeTests")]
