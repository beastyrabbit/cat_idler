using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>The presentation owns connections, never simulation rules or resource credits.</summary>
    public sealed class ForestGame : MonoBehaviour
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
        public string SavePath { get; private set; } = "";
        public string InitialSavePath = "";
        public readonly System.Collections.Generic.List<double> FrameMilliseconds = new System.Collections.Generic.List<double>();
        public readonly System.Collections.Generic.List<double> StepMilliseconds = new System.Collections.Generic.List<double>();
        public readonly System.Collections.Generic.List<double> EconomyMilliseconds = new System.Collections.Generic.List<double>();
        public ForestView View { get; private set; }
        public ForestUI UI { get; private set; }
        private LocalAuthority local;
        private WorldClient remote;
        private CancellationTokenSource cancellation;
        private double accumulated, sinceSave;
        private bool ready, connecting;
        private string evidenceDirectory = "";
        private float nextEvidence;
        private bool capturing;

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
                SavePath = Argument("--forest-save", InitialSavePath.Length > 0 ? InitialSavePath : Path.Combine(Application.persistentDataPath, "unity", "world-v1.json"));
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
            if (evidenceDirectory != "" && !capturing)
            {
                if (Keyboard.current?.f9Key.wasPressedThisFrame == true) StartCoroutine(CaptureEvidence(1));
                if (Keyboard.current?.f10Key.wasPressedThisFrame == true) StartCoroutine(CaptureEvidence(40));
            }
            FrameMilliseconds.Add(Time.unscaledDeltaTime * 1000);
            if (FrameMilliseconds.Count > 3600) FrameMilliseconds.RemoveAt(0);
            if (evidenceDirectory != "" && Time.unscaledTime >= nextEvidence)
            {
                nextEvidence = Time.unscaledTime + 10;
                Directory.CreateDirectory(evidenceDirectory);
                File.WriteAllText(Path.Combine(evidenceDirectory, "performance.json"), JsonUtility.ToJson(MeasurePerformance(), true));
            }
            if (remote != null) { Status = remote.Status; return; }
            accumulated += Math.Min(Time.unscaledDeltaTime, .25) * Speed;
            int count = 0;
            while (accumulated + 1e-9 >= World.SimulationStepSeconds && count < 40)
            {
                count++;
                var before = local.World.TimeSeconds;
                var step = System.Diagnostics.Stopwatch.StartNew();
                local.Advance(World.SimulationStepSeconds);
                if (local.World.TimeSeconds > before)
                {
                    StepMilliseconds.Add(step.Elapsed.TotalMilliseconds);
                    if (StepMilliseconds.Count > 3600) StepMilliseconds.RemoveAt(0);
                    if (Math.Floor(local.World.TimeSeconds) > Math.Floor(before))
                    { EconomyMilliseconds.Add(step.Elapsed.TotalMilliseconds); if (EconomyMilliseconds.Count > 3600) EconomyMilliseconds.RemoveAt(0); }
                }
                accumulated = Math.Max(0, accumulated - World.SimulationStepSeconds);
                sinceSave += World.SimulationStepSeconds;
                Revision++;
            }
            if (StepMilliseconds.Count > 0) TickMilliseconds = StepMilliseconds[StepMilliseconds.Count - 1];
            if (sinceSave >= 5) { Save(); sinceSave = 0; }
        }

        public void SetSpeed(float speed)
        {
            if (IsRemote) { LastAction = "The server controls time in a shared world."; return; }
            Speed = Mathf.Clamp(speed, 0, 8);
        }

        // Explicit opt-in captures the composited game, including UI, for review evidence.
        private IEnumerator CaptureEvidence(int frames)
        {
            capturing = true;
            string target = Path.Combine(evidenceDirectory, "capture-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss-fff"));
            Directory.CreateDirectory(target);
            for (int i = 0; i < frames; i++)
            {
                yield return new WaitForEndOfFrame();
                ScreenCapture.CaptureScreenshot(Path.Combine(target, i.ToString("D3") + ".png"));
                if (frames > 1) yield return new WaitForSecondsRealtime(.2f);
            }
            capturing = false;
        }

        public PerformanceSample MeasurePerformance()
        {
            double Percentile(System.Collections.Generic.List<double> samples, double q)
            { var a = samples.OrderBy(x => x).ToArray(); return a.Length == 0 ? 0 : a[Math.Min(a.Length - 1, (int)Math.Floor(q * a.Length))]; }
            return new PerformanceSample
            {
                population = Selected?.Cats.Count(c => c.Alive) ?? 0,
                buildings = Selected?.Buildings.Count ?? 0,
                workingCats = Selected?.Cats.Count(c => c.Alive && c.JobId != "") ?? 0,
                reservations = CurrentWorld?.Reservations.Count ?? 0,
                frames = FrameMilliseconds.Count,
                steps = StepMilliseconds.Count,
                economyTicks = EconomyMilliseconds.Count,
                frameP50 = Percentile(FrameMilliseconds, .5),
                frameP95 = Percentile(FrameMilliseconds, .95),
                stepP50 = Percentile(StepMilliseconds, .5),
                stepP95 = Percentile(StepMilliseconds, .95),
                economyP50 = Percentile(EconomyMilliseconds, .5),
                economyP95 = Percentile(EconomyMilliseconds, .95),
                width = Screen.width,
                height = Screen.height,
                processor = SystemInfo.processorType,
                graphics = SystemInfo.graphicsDeviceName,
                editor = Application.isEditor,
                remote = IsRemote,
                directControl = View.DirectControl,
                speed = Speed,
                simulationStepSeconds = World.SimulationStepSeconds
            };
        }

        [Serializable]
        public class PerformanceSample
        {
            public int population, buildings, workingCats, reservations, frames, steps, economyTicks, width, height;
            public double frameP50, frameP95, stepP50, stepP95, economyP50, economyP95, speed, simulationStepSeconds;
            public string processor, graphics; public bool editor, remote, directControl;
        }

        public async Task<ActionResult> Send(GameAction action)
        {
            if (!ready) return ActionResult.Fail("World is still opening.");
            try
            {
                ActionResult result = remote != null ? await remote.SendAsync(action) : local.Apply(action);
                if (!result.Success || action.Kind != "KeepCatControl")
                    LastAction = result.Success ? Describe(action.Kind) + " accepted" : result.Error;
                if (result.Success) Revision++;
                return result;
            }
            catch (Exception e) { LastAction = "Action failed: " + e.Message; return ActionResult.Fail(LastAction); }
        }

        public void Act(GameAction action) { _ = Send(action); }

        public async Task Connect(string address)
        {
            if (connecting) return;
            connecting = true;
            WorldClient candidate = null;
            try
            {
                var uri = new Uri(address);
                if (uri.Scheme != "ws" && uri.Scheme != "wss") throw new ArgumentException("Use a ws:// or wss:// server address.");
                Status = "Connecting to shared world…";
                var credentialPath = Path.Combine(Application.persistentDataPath, "unity", "servers", SafeServerKey(uri) + ".json");
                candidate = new WorldClient(uri.ToString(), credentialPath);
                await candidate.ConnectAsync(cancellation.Token);
                Save();
                remote?.Dispose(); remote = candidate; candidate = null;
                Revision++; View.FocusVillage();
            }
            catch (Exception e) { LastAction = "Connection failed: " + e.Message; Status = "Local world retained"; candidate?.Dispose(); }
            finally { connecting = false; }
        }

        public void Disconnect()
        {
            remote?.Dispose(); remote = null; Status = "Local shared world · saved on this Mac"; Revision++; View.FocusVillage();
        }

        public void Save()
        {
            if (local == null) return;
            try { local.Save(); }
            catch (Exception e) { Status = "Save failed: " + e.Message; Debug.LogError(Status); }
        }

        private void OnApplicationPause(bool pause) { if (pause) Save(); }
        private void OnApplicationQuit() { Save(); }
        private void OnDestroy()
        {
            cancellation?.Cancel(); remote?.Dispose(); local?.Dispose(); cancellation?.Dispose();
            if (Instance == this) Instance = null;
        }

        private static string SafeServerKey(Uri uri)
        {
            using var hash = System.Security.Cryptography.SHA256.Create();
            return BitConverter.ToString(hash.ComputeHash(System.Text.Encoding.UTF8.GetBytes(uri.GetLeftPart(UriPartial.Authority)))).Replace("-", "").ToLowerInvariant();
        }

        public static string Describe(string text)
        {
            if (string.IsNullOrEmpty(text)) return "";
            return System.Text.RegularExpressions.Regex.Replace(text.Replace('_', ' '), "(?<=[a-z])(?=[A-Z])", " ");
        }
    }
}
