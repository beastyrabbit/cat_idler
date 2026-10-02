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
        private string evidenceDirectory = "";
        private float nextEvidence;
        private bool capturing;

        /// <summary>Per-frame telemetry: evidence capture hotkeys, frame timing and the periodic performance file.</summary>
        private void TelemetryFrame()
        {
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
        }

        /// <summary>Records one simulation step that advanced the clock; whole-second crossings count as economy ticks.</summary>
        private void RecordStepTiming(double before, double after, System.Diagnostics.Stopwatch step)
        {
            if (after <= before) return;
            StepMilliseconds.Add(step.Elapsed.TotalMilliseconds);
            if (StepMilliseconds.Count > 3600) StepMilliseconds.RemoveAt(0);
            if (Math.Floor(after) > Math.Floor(before))
            { EconomyMilliseconds.Add(step.Elapsed.TotalMilliseconds); if (EconomyMilliseconds.Count > 3600) EconomyMilliseconds.RemoveAt(0); }
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
    }
}
