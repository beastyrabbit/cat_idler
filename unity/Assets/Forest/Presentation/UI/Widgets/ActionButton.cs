using System;
using System.Threading.Tasks;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// A button that sends one action. Wave 1 stub: it runs the action on click and remembers the
    /// result. W2-16 adds the pending state, inline result and inline confirmation.
    /// </summary>
    public class ActionButton : Button
    {
        private readonly Func<Task<ActionResult>> run;

        public ActionButton(string label, Func<Task<ActionResult>> run, string confirmLabel = null, bool destructive = false)
        {
            text = label;
            this.run = run;
            ConfirmLabel = confirmLabel;
            Destructive = destructive;
            clicked += OnClicked;
        }

        public string ConfirmLabel { get; }
        public bool Destructive { get; }
        public bool Pending { get; private set; }
        public ActionResult LastResult { get; private set; }

        private async void OnClicked()
        {
            if (run == null || Pending) return;
            Pending = true;
            try
            {
                LastResult = await run();
            }
            catch (Exception e)
            {
                Debug.LogException(e);
            }
            finally
            {
                Pending = false;
            }
        }
    }
}
