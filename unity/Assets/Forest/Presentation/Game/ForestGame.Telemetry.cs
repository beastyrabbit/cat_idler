using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>Frame and step timing, the performance probe and opt-in evidence captures.</summary>
    public sealed partial class ForestGame
    {
        public readonly List<double> FrameMilliseconds = new List<double>();
        public readonly List<double> StepMilliseconds = new List<double>();
        public readonly List<double> EconomyMilliseconds = new List<double>();
        /// <summary>Managed bytes allocated per frame (an estimate from the managed heap size; frames with a collection are skipped).</summary>
        public readonly List<double> AllocatedBytesPerFrame = new List<double>();
        /// <summary>Main-thread milliseconds of the persistence tick on frames where it did real work (a save).</summary>
        public readonly List<double> PersistenceTickMilliseconds = new List<double>();
        /// <summary>Frames slower than 33 ms (below 30 fps) since the session started.</summary>
        public int FramesOver33Milliseconds { get; private set; }
        private const int TelemetryWindow = 3600;
        private const double SaveWorkThresholdMilliseconds = .5;
        private long lastManagedBytes = -1;
        private int lastCollections = -1;
        private string evidenceDirectory = "";
        private float nextEvidence;
        private bool capturing;

        /// <summary>Per-frame telemetry: evidence capture hotkeys, frame timing and the periodic performance file.</summary>
        private void TelemetryFrame(double realDeltaSeconds)
        {
            if (evidenceDirectory != "" && !capturing)
            {
                if (Keyboard.current?.f9Key.wasPressedThisFrame == true) StartCoroutine(CaptureEvidence(1));
                if (Keyboard.current?.f10Key.wasPressedThisFrame == true) StartCoroutine(CaptureEvidence(40));
            }
            double frame = realDeltaSeconds * 1000;
            Window(FrameMilliseconds, frame);
            if (frame > 33) FramesOver33Milliseconds++;
            RecordAllocation();
            if (evidenceDirectory != "" && Time.unscaledTime >= nextEvidence)
            {
                nextEvidence = Time.unscaledTime + 10;
                try
                {
                    Directory.CreateDirectory(evidenceDirectory);
                    File.WriteAllText(Path.Combine(evidenceDirectory, "performance.json"), JsonUtility.ToJson(MeasurePerformance(), true));
                }
                catch (Exception e) { ForestLog.Exception(e, "writing performance evidence"); }
            }
        }

        private static void Window(List<double> samples, double value)
        {
            samples.Add(value);
            if (samples.Count > TelemetryWindow) samples.RemoveAt(0);
        }

        // No ProfilerRecorder: the managed heap delta between frames, skipping frames in which a collection ran.
        private void RecordAllocation()
        {
            long bytes = GC.GetTotalMemory(false);
            int collections = GC.CollectionCount(0);
            if (lastManagedBytes >= 0 && collections == lastCollections && bytes >= lastManagedBytes)
                Window(AllocatedBytesPerFrame, bytes - lastManagedBytes);
            lastManagedBytes = bytes;
            lastCollections = collections;
        }

        private void RecordPersistenceTiming(double milliseconds)
        {
            if (milliseconds >= SaveWorkThresholdMilliseconds) Window(PersistenceTickMilliseconds, milliseconds);
        }

        /// <summary>Records one simulation step that advanced the clock; whole-second crossings count as economy ticks.</summary>
        private void RecordStepTiming(double before, double after, System.Diagnostics.Stopwatch step)
        {
            if (after <= before) return;
            Window(StepMilliseconds, step.Elapsed.TotalMilliseconds);
            if (Math.Floor(after) > Math.Floor(before)) Window(EconomyMilliseconds, step.Elapsed.TotalMilliseconds);
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
            double Percentile(List<double> samples, double q)
            { var a = samples.OrderBy(x => x).ToArray(); return a.Length == 0 ? 0 : a[Math.Min(a.Length - 1, (int)Math.Floor(q * a.Length))]; }
            double Max(List<double> samples) => samples.Count == 0 ? 0 : samples.Max();
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
                frameP99 = Percentile(FrameMilliseconds, .99),
                frameMax = Max(FrameMilliseconds),
                framesOver33ms = FrameMilliseconds.Count(f => f > 33),
                stepP50 = Percentile(StepMilliseconds, .5),
                stepP95 = Percentile(StepMilliseconds, .95),
                stepMax = Max(StepMilliseconds),
                saves = PersistenceTickMilliseconds.Count,
                saveMsLast = PersistenceTickMilliseconds.Count == 0 ? 0 : PersistenceTickMilliseconds[PersistenceTickMilliseconds.Count - 1],
                saveMsMax = Max(PersistenceTickMilliseconds),
                saveMs = SaveMilliseconds,
                saveMainThreadMs = SaveMainThreadMilliseconds,
                allocatedBytesP50 = Percentile(AllocatedBytesPerFrame, .5),
                allocatedBytesP95 = Percentile(AllocatedBytesPerFrame, .95),
                gcCollections = GC.CollectionCount(0),
                errors = ForestLog.ErrorCount,
                faults = FaultCount,
                faulted = Faulted,
                simulationBehind = SimulationBehind,
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
            /// <summary>Hitches: p99 and worst frame, and frames over 33 ms, all in the sample window.</summary>
            public double frameP99, frameMax; public int framesOver33ms;
            public double stepMax;
            /// <summary>Main-thread milliseconds of the persistence tick on frames that saved.</summary>
            public int saves; public double saveMsLast, saveMsMax;
            /// <summary>Last successful save in total (encode plus background write) and its main-thread part.</summary>
            public double saveMs, saveMainThreadMs;
            /// <summary>Estimated managed bytes allocated per frame; gen-0 collections since startup.</summary>
            public double allocatedBytesP50, allocatedBytesP95; public int gcCollections;
            public int errors, faults; public bool faulted, simulationBehind;
        }
    }
}
