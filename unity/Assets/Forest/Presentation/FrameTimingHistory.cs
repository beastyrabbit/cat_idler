using System;

namespace IdleCatForest.Presentation
{
    public enum FramePhase { Game, Simulation, Save, Evidence, View, Reconcile, Animate, InputCamera, UI, Rebuild, Live, Terrain }

    [Serializable]
    public struct FramePhaseTiming
    {
        public string phase;
        public int calls;
        public double milliseconds;
    }

    [Serializable]
    public sealed class FrameTimingSample
    {
        public int frameIndex, followingFrameIndex;
        public double followingIntervalMilliseconds;
        public FramePhaseTiming[] phases;
    }

    /// <summary>CPU scopes from one Unity frame, paired only after the following interval arrives.</summary>
    public sealed class FrameTimingHistory
    {
        private static readonly string[] PhaseNames = Enum.GetNames(typeof(FramePhase));
        private readonly FrameTimingSample[] completed;
        private FrameTimingSample current, pending;
        private int next, count;
        public bool Enabled { get; }
        public FrameTimingHistory(bool enabled, int capacity = 3600)
        {
            if (capacity < 1) throw new ArgumentOutOfRangeException(nameof(capacity));
            Enabled = enabled;
            completed = enabled ? new FrameTimingSample[capacity] : Array.Empty<FrameTimingSample>();
        }

        public void BeginFrame(int frameIndex, double elapsedIntervalMilliseconds)
        {
            if (!Enabled || EnsureFrame(frameIndex) == null) return;
            if (pending != null && pending.frameIndex == frameIndex - 1)
            {
                pending.followingFrameIndex = frameIndex;
                pending.followingIntervalMilliseconds = elapsedIntervalMilliseconds;
                completed[next] = pending;
                next = (next + 1) % completed.Length;
                count = Math.Min(count + 1, completed.Length);
            }
            pending = null;
        }

        public void Add(int frameIndex, FramePhase phase, double elapsedMilliseconds)
        {
            if (!Enabled) return;
            var sample = EnsureFrame(frameIndex);
            if (sample == null) return;
            int index = (int)phase;
            sample.phases[index].calls++;
            sample.phases[index].milliseconds += elapsedMilliseconds;
        }

        private FrameTimingSample EnsureFrame(int frameIndex)
        {
            if (current != null && frameIndex <= current.frameIndex)
                return frameIndex == current.frameIndex ? current : null;
            pending = current;
            current = new FrameTimingSample { frameIndex = frameIndex, phases = new FramePhaseTiming[PhaseNames.Length] };
            for (int i = 0; i < PhaseNames.Length; i++) current.phases[i].phase = PhaseNames[i];
            return current;
        }

        public FrameTimingSample[] Snapshot()
        {
            if (count == 0) return Array.Empty<FrameTimingSample>();
            var samples = new FrameTimingSample[count];
            int first = (next - count + completed.Length) % completed.Length;
            for (int i = 0; i < count; i++) samples[i] = completed[(first + i) % completed.Length];
            return samples;
        }
    }
}
