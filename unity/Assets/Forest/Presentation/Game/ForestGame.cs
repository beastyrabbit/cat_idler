using System;
using System.Threading;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The presentation owns connections, never simulation rules or resource credits.
    /// This file holds the lifecycle and the local step loop. Persistence, remote play, telemetry and
    /// player actions live in the sibling <c>ForestGame.*.cs</c> partial files.
    /// </summary>
    public sealed partial class ForestGame : MonoBehaviour
    {
        public static ForestGame Instance { get; private set; }
        public World CurrentWorld => remote != null ? remote.LatestWorld : local?.World;
        public Village Selected => CurrentWorld?.Villages.Find(v => v.Id == SelectedVillageId);
        public string SelectedVillageId => remote != null ? remote.SelectedVillageId : local?.SelectedVillageId ?? "";
        public string PlayerId => remote != null ? remote.Credential?.PlayerId ?? "" : local?.Credential?.PlayerId ?? "";
        public bool IsRemote => remote != null;
        public string Status { get; private set; } = "Opening forest…";
        public string LastAction { get; private set; } = "";
        public double TickMilliseconds { get; private set; }
        public int Revision { get; private set; }
        public float Speed { get; private set; } = 1;
        /// <summary>Set when the local simulation cannot advance. Stub: always false until W2-12.</summary>
        public bool Faulted { get; private set; }
        /// <summary>Raised at most once per frame after <see cref="Revision"/> or the remote frame sequence changed.</summary>
        public event Action Changed;
        public ForestView View { get; private set; }
        public ForestUI UI { get; private set; }
        private LocalAuthority local;
        private WorldClient remote;
        private CancellationTokenSource cancellation;
        private double accumulated;
        private bool ready;
        private int changedRevision;
        private long changedSequence;

        private void Awake()
        {
            if (Instance != null) { Destroy(gameObject); return; }
            Instance = this;
            Application.targetFrameRate = 60;
            QualitySettings.vSyncCount = 1;
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
                int seed = int.TryParse(Argument("--forest-seed", "4242"), out int n) ? n : 4242;
                evidenceDirectory = Argument("--forest-evidence", "");
                local = new LocalAuthority(SavePath, seed);
                Status = "Local shared world · saved on this Mac";
                ready = true;
                Revision++;
                View.FocusVillage();
                var address = Argument("--forest-server", "");
                if (!string.IsNullOrEmpty(address)) _ = Connect(address);
            }
            catch (Exception e) { Status = "Could not open the saved world: " + e.Message; Debug.LogError(Status); }
        }

        private void Update()
        {
            if (!ready || CurrentWorld == null) return;
            TelemetryFrame();
            double stepped = 0;
            if (remote != null) Status = remote.Status;
            else stepped = StepLocal();
            PersistenceTick(stepped);
            RaiseChanged();
        }

        /// <summary>Advances the local world by the fixed steps owed this frame and returns the simulated seconds stepped.</summary>
        private double StepLocal()
        {
            accumulated += Math.Min(Time.unscaledDeltaTime, .25) * Speed;
            int count = 0;
            double stepped = 0;
            while (accumulated + 1e-9 >= World.SimulationStepSeconds && count < 40)
            {
                count++;
                var before = local.World.TimeSeconds;
                var step = System.Diagnostics.Stopwatch.StartNew();
                local.Advance(World.SimulationStepSeconds);
                RecordStepTiming(before, local.World.TimeSeconds, step);
                accumulated = Math.Max(0, accumulated - World.SimulationStepSeconds);
                stepped += World.SimulationStepSeconds;
                Revision++;
            }
            if (StepMilliseconds.Count > 0) TickMilliseconds = StepMilliseconds[StepMilliseconds.Count - 1];
            return stepped;
        }

        private void RaiseChanged()
        {
            long sequence = remote?.LatestFrame?.Sequence ?? 0;
            if (Revision == changedRevision && sequence == changedSequence) return;
            changedRevision = Revision;
            changedSequence = sequence;
            Changed?.Invoke();
        }

        public void SetSpeed(float speed)
        {
            if (IsRemote) { LastAction = "The server controls time in a shared world."; return; }
            Speed = Mathf.Clamp(speed, 0, 8);
        }

        private void OnDestroy()
        {
            cancellation?.Cancel(); remote?.Dispose(); local?.Dispose(); cancellation?.Dispose();
            if (Instance == this) Instance = null;
        }
    }
}
