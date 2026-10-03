using System;
using System.IO;
using System.Threading;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using UnityEngine;
using Stopwatch = System.Diagnostics.Stopwatch;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The presentation owns connections, never simulation rules or resource credits.
    /// This file holds the lifecycle, frame pacing and the guarded local step loop. Persistence, remote
    /// play, telemetry and player actions live in the sibling <c>ForestGame.*.cs</c> partial files.
    /// Runs before the view and the HUD so that both read the state of the current frame.
    /// </summary>
    [DefaultExecutionOrder(-100)]
    public sealed partial class ForestGame : MonoBehaviour
    {
        /// <summary>Frame cap while the window has focus (vsync is off, so this is the cap).</summary>
        public const int FocusedFrameRate = 60;
        /// <summary>Frame cap while the player's window is in the background; the simulation keeps running.</summary>
        public const int BackgroundFrameRate = 10;
        /// <summary>Wall-clock milliseconds of simulation steps one frame may spend before the backlog is dropped.</summary>
        public const double DefaultStepBudgetMilliseconds = 6;
        private const int MaxStepsPerFrame = 40;
        private const double MaxFrameSeconds = .25;
        private const double BehindHoldSeconds = 1;

        public static ForestGame Instance { get; private set; }
        public World CurrentWorld => remote != null ? RemoteFrame?.World : local?.World;
        public Village Selected => CurrentWorld?.Villages.Find(v => v.Id == SelectedVillageId);
        public string SelectedVillageId => remote != null ? RemoteFrame?.SelectedVillageId ?? "" : local?.SelectedVillageId ?? "";
        public string PlayerId => remote != null ? remote.Credential?.PlayerId ?? "" : local?.Credential?.PlayerId ?? "";
        public bool IsRemote => remote != null;
        public string Status { get; private set; } = "Opening forest…";
        public double TickMilliseconds { get; private set; }

        /// <summary>
        /// Grows when authoritative state changed: once per frame in which a local step ran, on every
        /// successful action, on a fault, and when a new remote frame arrives.
        /// </summary>
        public int Revision { get; private set; }

        /// <summary>The local simulation speed, always one of <see cref="GameSpeeds"/>.</summary>
        public float Speed { get; private set; } = 1;

        /// <summary>
        /// True after a step or a local action threw, until the next step succeeds. While true the forest is
        /// paused and the faulted world is never autosaved.
        /// </summary>
        public bool Faulted { get; private set; }

        /// <summary>The <see cref="SaveFailure"/> code of a refused open (for the recovery card), or empty.</summary>
        public string OpenFailureCode { get; private set; } = "";

        /// <summary>How many step or action faults this session has seen.</summary>
        public int FaultCount { get; private set; }

        /// <summary>
        /// True for a moment after a frame could not run every step it owed within its step budget; the
        /// backlog was dropped, so the forest ran slower than the selected speed.
        /// </summary>
        public bool SimulationBehind => Time.realtimeSinceStartupAsDouble < behindUntil;

        /// <summary>Raised at most once per frame after <see cref="Revision"/> changed.</summary>
        public event Action Changed;
        public ForestView View { get; private set; }
        public ForestUI UI { get; private set; }

        /// <summary>Test seam: runs after every local step; a throw simulates a failure in the middle of a tick.</summary>
        internal Action<World> StepProbe;
        /// <summary>Wall-clock budget for the steps of one frame; the first step of a frame always runs.</summary>
        internal double StepBudgetMilliseconds = DefaultStepBudgetMilliseconds;

        private LocalAuthority local;
        private WorldClient remote;
        private WorldFrame remoteFrame;
        private long remoteSequence;
        private CancellationTokenSource cancellation;
        private double accumulated;
        private double behindUntil;
        private bool ready;
        private bool throttleInBackground;
        private int changedRevision;

        /// <summary>The remote frame captured at the start of this frame, so one frame never mixes two worlds.</summary>
        private WorldFrame RemoteFrame => remoteFrame ?? remote?.LatestFrame;

        private void Awake()
        {
            if (Instance != null) { Destroy(gameObject); return; }
            Instance = this;
            // With vsync on, Unity ignores targetFrameRate and ProMotion displays render at 120 Hz.
            QualitySettings.vSyncCount = 0;
            Application.targetFrameRate = FocusedFrameRate;
            throttleInBackground = !Application.isEditor && !Application.isBatchMode
                && Array.IndexOf(Environment.GetCommandLineArgs(), "--forest-capture-script") < 0;
            cancellation = new CancellationTokenSource();
            View = gameObject.AddComponent<ForestView>();
            UI = gameObject.AddComponent<ForestUI>();
        }

        private void Start()
        {
            try
            {
                var args = Environment.GetCommandLineArgs();
                string Argument(string key, string fallback) { int i = Array.IndexOf(args, key); return i >= 0 && i + 1 < args.Length ? args[i + 1] : fallback; }
                SavePath = Argument("--forest-save", InitialSavePath.Length > 0 ? InitialSavePath : DefaultSavePath());
                // A new single-player world is the named demo colony on the demo seed unless a seed is given (W2-INT-c).
                int seed = int.TryParse(Argument("--forest-seed", ""), out int n) ? n : IdleCatForest.Simulation.World.DemoSeed;
                evidenceDirectory = Argument("--forest-evidence", "");
                string saveRoot = Path.GetDirectoryName(Path.GetFullPath(SavePath)) ?? "";
                ForestLog.Begin(Path.Combine(saveRoot, "logs"), saveRoot, seed);
                local = InitialClassicWorld ? LocalAuthority.OpenClassic(SavePath, seed) : new LocalAuthority(SavePath, seed);
                Status = "Local shared world · saved on this Mac";
                ready = true;
                Revision++;
                View.FocusVillage();
                var address = Argument("--forest-server", "");
                if (!string.IsNullOrEmpty(address)) _ = Connect(address);
            }
            catch (Exception e)
            {
                // The reason (often with an absolute path) goes to the log only, never to the screen.
                ForestLog.Exception(e, "opening the saved world");
                OpenFailureCode = (e as SaveException)?.Code ?? "";
                Status = "Could not open the saved world.";
            }
        }

        private void Update() => RunFrame(Time.unscaledDeltaTime);

        /// <summary>One frame of the game loop for <paramref name="realDeltaSeconds"/> of wall-clock time.</summary>
        internal void RunFrame(double realDeltaSeconds)
        {
            if (!ready) return;
            if (remote != null) CaptureRemoteFrame();
            if (CurrentWorld == null) return;
            TelemetryFrame(realDeltaSeconds);
            double stepped = 0;
            if (remote != null) Status = remote.Status;
            else stepped = StepLocal(realDeltaSeconds);
            var persistence = Stopwatch.StartNew();
            try { PersistenceTick(stepped); }
            catch (Exception e) { ForestLog.Exception(e, "persistence tick"); }
            RecordPersistenceTiming(persistence.Elapsed.TotalMilliseconds);
            RaiseChanged();
        }

        private void CaptureRemoteFrame()
        {
            remoteFrame = remote.LatestFrame;
            long sequence = remoteFrame?.Sequence ?? 0;
            if (sequence == remoteSequence) return;
            remoteSequence = sequence;
            Revision++;
        }

        /// <summary>
        /// Advances the local world by the fixed steps owed this frame, within the step budget, and returns the
        /// simulated seconds to count for autosave. A throwing step faults the forest and counts nothing.
        /// </summary>
        private double StepLocal(double realDeltaSeconds)
        {
            accumulated += Math.Min(Math.Max(0, realDeltaSeconds), MaxFrameSeconds) * Speed;
            int count = 0;
            double stepped = 0;
            bool cut = false;
            var budget = Stopwatch.StartNew();
            // Direct-control walking runs in real time: the simulation divides it by the selected speed (W2-06).
            local.World.ControlTimeScale = Speed;
            try
            {
                while (accumulated + 1e-9 >= World.SimulationStepSeconds)
                {
                    if (count >= MaxStepsPerFrame || (count > 0 && budget.Elapsed.TotalMilliseconds >= StepBudgetMilliseconds)) { cut = true; break; }
                    count++;
                    var before = local.World.TimeSeconds;
                    var step = Stopwatch.StartNew();
                    local.Advance(World.SimulationStepSeconds);
                    StepProbe?.Invoke(local.World);
                    RecordStepTiming(before, local.World.TimeSeconds, step);
                    accumulated = Math.Max(0, accumulated - World.SimulationStepSeconds);
                    stepped += World.SimulationStepSeconds;
                    Faulted = false;
                }
            }
            catch (Exception e)
            {
                Fault(e, "simulation step at " + local.World.TimeSeconds.ToString("0.00", System.Globalization.CultureInfo.InvariantCulture) + " s");
                return 0;
            }
            if (cut)
            {
                // Drop the backlog instead of chasing it: the forest runs slower rather than spiralling.
                accumulated = Math.Min(accumulated, World.SimulationStepSeconds);
                behindUntil = Time.realtimeSinceStartupAsDouble + BehindHoldSeconds;
            }
            if (count > 0) { Revision++; LogIsolatedFaults(); }
            if (StepMilliseconds.Count > 0) TickMilliseconds = StepMilliseconds[StepMilliseconds.Count - 1];
            return stepped;
        }

        /// <summary>
        /// A step or a local action threw: log it once, pause, restore the last good world when the authority
        /// can, and stay faulted (no autosave) until a step succeeds.
        /// </summary>
        private void Fault(Exception e, string context, bool publish = true)
        {
            ForestLog.Exception(e, context);
            FaultCount++;
            Faulted = true;
            Speed = GameSpeeds.Paused;
            accumulated = 0;
            try
            {
                if (local != null && local.RestoreLastGood()) ForestLog.Warning("restored the last good world after: " + context);
            }
            catch (Exception restore) { ForestLog.Exception(restore, "restoring the last good world"); }
            Revision++;
            if (publish) Publish(new GameAction { Kind = "Simulation" }, ActionResult.Fail(FeedbackCode.SimulationFault, ""));
        }

        /// <summary>The simulation isolates in-tick faults and keeps running; each new one is logged once here.</summary>
        private void LogIsolatedFaults()
        {
            var faults = local.World.IsolatedFaults();
            if (faults.Count < loggedFaults) loggedFaults = 0; // A restored world starts a new list.
            for (; loggedFaults < faults.Count; loggedFaults++) ForestLog.Warning("isolated simulation fault: " + faults[loggedFaults]);
        }
        private int loggedFaults;

        private void RaiseChanged()
        {
            if (Revision == changedRevision) return;
            changedRevision = Revision;
            Changed?.Invoke();
        }

        /// <summary>Selects a local speed, snapped to the allowed set (pause, 1x, 2x, 4x; 8x and 16x for dev).</summary>
        public void SetSpeed(float speed)
        {
            if (IsRemote)
            {
                Publish(new GameAction { Kind = "SetSpeed" }, ActionResult.Fail(FeedbackCode.ServerControlsTime, "The server controls time in a shared world."));
                return;
            }
            Speed = GameSpeeds.Clamp(speed);
            if (Speed == GameSpeeds.Paused) accumulated = 0;
        }

        private void OnApplicationFocus(bool focus)
        {
            if (throttleInBackground) Application.targetFrameRate = focus ? FocusedFrameRate : BackgroundFrameRate;
            if (!focus) SaveIfChanged();
        }

        private void OnDestroy()
        {
            cancellation?.Cancel(); remote?.Dispose(); local?.Dispose(); cancellation?.Dispose();
            if (Instance == this) { Instance = null; ForestLog.End(); }
        }
    }
}
