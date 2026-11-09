using System;

namespace IdleCatForest.Presentation
{
    [Serializable]
    public sealed class SaveStageSample
    {
        public string stage;
        public double milliseconds;
        public bool attempted, succeeded;
    }

    [Serializable]
    public sealed class SaveTimingSample
    {
        public long sequence, payloadBytes, envelopeBytes;
        public int frameIndex;
        public bool success;
        public string failedStage;
        public double totalMilliseconds, residualMilliseconds;
        public SaveStageSample[] stages;
    }

    public sealed class SaveTimingHistory
    {
        private readonly SaveTimingSample[] samples;
        private int next, count;
        private long sequence;
        public bool Enabled { get; }
        public SaveTimingHistory(bool enabled, int capacity = 128)
        {
            if (capacity < 1) throw new ArgumentOutOfRangeException(nameof(capacity));
            Enabled = enabled;
            samples = enabled ? new SaveTimingSample[capacity] : Array.Empty<SaveTimingSample>();
        }

        public static bool Requested(string evidenceDirectory, string[] arguments)
            => !string.IsNullOrEmpty(evidenceDirectory) && Array.IndexOf(arguments, "--forest-profile-saves") >= 0;

        public void Add(int frameIndex, SaveTimingSample sample)
        {
            if (!Enabled || sample == null) return;
            sample.sequence = ++sequence;
            sample.frameIndex = frameIndex;
            samples[next] = sample;
            next = (next + 1) % samples.Length;
            count = Math.Min(count + 1, samples.Length);
        }

        public SaveTimingSample[] Snapshot()
        {
            if (count == 0) return Array.Empty<SaveTimingSample>();
            var result = new SaveTimingSample[count];
            int first = (next - count + samples.Length) % samples.Length;
            for (int i = 0; i < count; i++) result[i] = samples[(first + i) % samples.Length];
            return result;
        }
    }
}
