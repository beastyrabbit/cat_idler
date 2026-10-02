using System;
using System.Threading.Tasks;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public enum ActionButtonState { Idle, Confirming, Pending, Confirmed, Failed }

    /// <summary>
    /// A button that sends one action and shows its result in place (GUIDE section 6): while the action
    /// runs it is disabled and marked pending, so a double click never sends twice; afterwards its label
    /// reads the confirmation or the humanised refusal for <see cref="ResultSeconds"/>, then returns.
    /// A destructive button first asks inline ("Remove? Yes · No"). Raises <see cref="UiEvents"/>
    /// Clicked, Confirmed and Failed for audio and FX.
    /// </summary>
    public class ActionButton : Button
    {
        public const string UssClass = "act";
        public const string PrimaryClass = "act--primary";
        public const string DestructiveClass = "act--destructive";
        public const string PendingClass = "act--pending";
        public const string ConfirmedClass = "act--confirmed";
        public const string FailedClass = "act--failed";
        public const string ConfirmingClass = "act--confirming";
        public const long ResultMilliseconds = 2000;
        public const long ConfirmTimeoutMilliseconds = 4000;

        private readonly Func<Task<ActionResult>> run;
        private readonly VisualElement confirmRow;
        private readonly Label question;
        private string label;
        private IVisualElementScheduledItem revert;
        private int swallowFrame = -1;

        public ActionButton(string label, Func<Task<ActionResult>> run, string confirmLabel = null, bool destructive = false)
        {
            this.label = label ?? "";
            text = this.label;
            this.run = run;
            ConfirmLabel = confirmLabel;
            Destructive = destructive;
            AddToClassList(UssClass);
            EnableInClassList(DestructiveClass, destructive);
            clicked += Press;

            confirmRow = new VisualElement { name = "act-confirm-row", pickingMode = PickingMode.Ignore };
            confirmRow.AddToClassList("act-confirm");
            question = new Label { pickingMode = PickingMode.Ignore };
            question.AddToClassList("act-confirm__question");
            var yes = new Button(ConfirmYes) { text = HudCopy.Yes, name = "act-confirm-yes" };
            yes.AddToClassList("act-confirm__yes");
            var no = new Button(ConfirmNo) { text = HudCopy.No, name = "act-confirm-no" };
            no.AddToClassList("act-confirm__no");
            confirmRow.Add(question);
            confirmRow.Add(yes);
            confirmRow.Add(no);
            WidgetStyles.Use(this);
        }

        /// <summary>The question a destructive button asks first; "Remove?" when null.</summary>
        public string ConfirmLabel { get; }
        public bool Destructive { get; }
        public bool Pending => State == ActionButtonState.Pending;
        public ActionResult LastResult { get; private set; }
        public ActionButtonState State { get; private set; }

        /// <summary>The resting label. Changing it while a result shows takes effect when the result clears.</summary>
        public string Label
        {
            get => label;
            set
            {
                label = value ?? "";
                if (State == ActionButtonState.Idle && text != label) text = label;
            }
        }

        /// <summary>The GameAction kind the button sends; picks <see cref="UiText.Confirm"/> when no <see cref="ConfirmedLabel"/> is set.</summary>
        public string ActionKind { get; set; } = "";

        /// <summary>The label shown after success, reusing the verb ("Sent", "Den planned"); "" keeps the resting label.</summary>
        public string ConfirmedLabel { get; set; }

        /// <summary>The one sun-coloured action in its group.</summary>
        public bool Primary
        {
            get => ClassListContains(PrimaryClass);
            set => EnableInClassList(PrimaryClass, value);
        }

        /// <summary>Raised once per finished action, after the label shows its result.</summary>
        public event Action<ActionResult> Completed;

        /// <summary>The click path: asks first when destructive, otherwise sends.</summary>
        internal void Press()
        {
            if (swallowFrame == Time.frameCount) return;
            if (run == null || State == ActionButtonState.Pending || State == ActionButtonState.Confirming) return;
            UiEvents.RaiseClicked();
            if (Destructive) BeginConfirm();
            else Send();
        }

        internal void ConfirmYes()
        {
            if (State != ActionButtonState.Confirming) return;
            SwallowParentClick();
            EndConfirm();
            Send();
        }

        internal void ConfirmNo()
        {
            if (State != ActionButtonState.Confirming) return;
            SwallowParentClick();
            EndConfirm();
            Rest();
        }

        private void BeginConfirm()
        {
            CancelRevert();
            State = ActionButtonState.Confirming;
            question.text = string.IsNullOrEmpty(ConfirmLabel) ? HudCopy.RemoveQuestion : ConfirmLabel;
            text = "";
            AddToClassList(ConfirmingClass);
            Add(confirmRow);
            revert = schedule.Execute(ConfirmNoQuietly).StartingIn(ConfirmTimeoutMilliseconds);
        }

        private void ConfirmNoQuietly()
        {
            if (State != ActionButtonState.Confirming) return;
            EndConfirm();
            Rest();
        }

        private void EndConfirm()
        {
            CancelRevert();
            confirmRow.RemoveFromHierarchy();
            RemoveFromClassList(ConfirmingClass);
        }

        // A click on Yes or No can bubble to this button's own clickable in the same frame; ignore it.
        private void SwallowParentClick() => swallowFrame = Time.frameCount;

        private async void Send()
        {
            State = ActionButtonState.Pending;
            AddToClassList(PendingClass);
            SetEnabled(false);
            ActionResult result;
            try
            {
                result = await run() ?? ActionResult.Fail(HudCopy.Failed);
            }
            catch (Exception e)
            {
                Debug.LogException(e);
                result = ActionResult.Fail(HudCopy.Failed);
            }
            SetEnabled(true);
            RemoveFromClassList(PendingClass);
            ShowResult(result);
        }

        /// <summary>Shows a result in the label for <see cref="ResultMilliseconds"/> and raises the matching events.</summary>
        internal void ShowResult(ActionResult result)
        {
            CancelRevert();
            LastResult = result;
            bool ok = result != null && result.Success;
            State = ok ? ActionButtonState.Confirmed : ActionButtonState.Failed;
            string shown = ok ? SuccessText() : FailureText(result);
            text = string.IsNullOrEmpty(shown) ? label : shown;
            EnableInClassList(ConfirmedClass, ok);
            EnableInClassList(FailedClass, !ok);
            if (ok) UiEvents.RaiseConfirmed();
            else UiEvents.RaiseFailed();
            revert = schedule.Execute(Rest).StartingIn(ResultMilliseconds);
            Completed?.Invoke(result);
        }

        /// <summary>Back to the resting label.</summary>
        internal void Rest()
        {
            CancelRevert();
            State = ActionButtonState.Idle;
            RemoveFromClassList(ConfirmedClass);
            RemoveFromClassList(FailedClass);
            if (text != label) text = label;
        }

        private void CancelRevert()
        {
            revert?.Pause();
            revert = null;
        }

        private string SuccessText()
        {
            if (ConfirmedLabel != null) return ConfirmedLabel;
            return string.IsNullOrEmpty(ActionKind) ? HudCopy.Done : UiText.Confirm(ActionKind);
        }

        /// <summary>The refusal in the player's words: the coded reason when there is one, otherwise the authority's message.</summary>
        public static string FailureText(ActionResult result)
        {
            if (result == null) return HudCopy.Failed;
            if (!string.IsNullOrEmpty(result.Code)) return UiText.Reason(result.Code);
            string error = UiText.ActionError(result);
            return string.IsNullOrEmpty(error) ? HudCopy.Failed : error;
        }
    }
}
