using System;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// HUD moments that audio and FX react to. Declared in Wave 1 and raised nowhere yet; W2-16
    /// raises them from the widgets.
    /// </summary>
    public static class UiEvents
    {
        public static event Action Clicked;
        public static event Action Confirmed;
        public static event Action Failed;
        public static event Action Opened;
        public static event Action Closed;
        public static event Action Attention;
        public static event Action<string> Flourish;

        internal static void RaiseClicked() => Clicked?.Invoke();
        internal static void RaiseConfirmed() => Confirmed?.Invoke();
        internal static void RaiseFailed() => Failed?.Invoke();
        internal static void RaiseOpened() => Opened?.Invoke();
        internal static void RaiseClosed() => Closed?.Invoke();
        internal static void RaiseAttention() => Attention?.Invoke();
        internal static void RaiseFlourish(string kind) => Flourish?.Invoke(kind);
    }
}
