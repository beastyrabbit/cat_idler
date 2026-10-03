using System;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// HUD moments that audio and FX react to (W3-17 subscribes for sound). Raised by the widgets and the
    /// attention and toast chrome:
    /// <list type="bullet">
    /// <item><see cref="Clicked"/>: an <see cref="ActionButton"/> was pressed (also the first press of a destructive one).</item>
    /// <item><see cref="Confirmed"/> / <see cref="Failed"/>: an ActionButton's action succeeded or was refused.</item>
    /// <item><see cref="Opened"/> / <see cref="Closed"/>: the attention popover opened or closed.</item>
    /// <item><see cref="Attention"/>: a new urgent attention item or an alert toast appeared (the seal pulses).</item>
    /// <item><see cref="Flourish"/>: a notable toast appeared; the argument is the event kind
    /// (<c>research</c>, <c>birth</c>, <c>raid_repelled</c>, ...), so audio can pick a payoff chime.</item>
    /// </list>
    /// Handlers run on the main thread; a throwing handler is logged and never breaks the HUD.
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

        internal static void RaiseClicked() => Raise(Clicked);
        internal static void RaiseConfirmed() => Raise(Confirmed);
        internal static void RaiseFailed() => Raise(Failed);
        internal static void RaiseOpened() => Raise(Opened);
        internal static void RaiseClosed() => Raise(Closed);
        internal static void RaiseAttention() => Raise(Attention);

        internal static void RaiseFlourish(string kind)
        {
            var handlers = Flourish;
            if (handlers == null) return;
            foreach (Action<string> handler in handlers.GetInvocationList())
            {
                try { handler(kind ?? ""); }
                catch (Exception e) { UnityEngine.Debug.LogException(e); }
            }
        }

        private static void Raise(Action handlers)
        {
            if (handlers == null) return;
            foreach (Action handler in handlers.GetInvocationList())
            {
                try { handler(); }
                catch (Exception e) { UnityEngine.Debug.LogException(e); }
            }
        }
    }
}
