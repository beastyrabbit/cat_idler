using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace IdleCatForest.Authority
{
    public enum SaveStage
    {
        None, LockWait, Validation, PayloadEncode, PayloadUtf8Hash, EnvelopeEncode,
        TempSetupProtection, EnvelopeUtf8Write, FlushClose, MoveReplace, DirectorySync
    }

    public readonly struct SaveStageTiming
    {
        public SaveStage Stage { get; }
        public double Milliseconds { get; }
        public bool Attempted { get; }
        public bool Succeeded { get; }

        internal SaveStageTiming(SaveStage stage, double milliseconds, bool succeeded)
        { Stage = stage; Milliseconds = milliseconds; Attempted = true; Succeeded = succeeded; }
    }

    /// <summary>A call-scoped save measurement containing no state or filesystem names.</summary>
    public sealed class SaveTiming
    {
        private readonly SaveStageTiming[] stages;
        private readonly long started;
        private long stageStarted, measuredTicks;
        private int count, active = -1;
        private static readonly double TickMilliseconds = 1000.0 / Stopwatch.Frequency;
        public IReadOnlyList<SaveStageTiming> Stages { get; }
        public double TotalMilliseconds { get; private set; }
        public double ResidualMilliseconds { get; private set; }
        public bool Success { get; private set; }
        public SaveStage FailedStage { get; private set; }
        public long PayloadBytes { get; internal set; }
        public long EnvelopeBytes { get; internal set; }

        private SaveTiming(long started)
        {
            this.started = started;
            stages = new SaveStageTiming[10];
            Stages = Array.AsReadOnly(stages);
        }
        internal static SaveTiming Start() => new SaveTiming(Stopwatch.GetTimestamp());
        internal void Begin(SaveStage stage)
        {
            active = count++;
            stages[active] = new SaveStageTiming(stage, 0, false);
            stageStarted = Stopwatch.GetTimestamp();
        }
        internal void EndStage(bool succeeded)
        {
            if (active < 0) return;
            var ticks = Stopwatch.GetTimestamp() - stageStarted;
            var stage = stages[active].Stage;
            stages[active] = new SaveStageTiming(stage, ticks * TickMilliseconds, succeeded);
            measuredTicks += ticks;
            if (!succeeded) FailedStage = stage;
            active = -1;
        }
        internal void Complete(bool success)
        {
            EndStage(success);
            var ticks = Stopwatch.GetTimestamp() - started;
            TotalMilliseconds = ticks * TickMilliseconds;
            ResidualMilliseconds = (ticks - measuredTicks) * TickMilliseconds;
            Success = success;
        }
    }
}
