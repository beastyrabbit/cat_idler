using System.Linq;
using IdleCatForest.Presentation;
using NUnit.Framework;

namespace IdleCatForest.Tests
{
    public sealed class ForestFrameTimingTests
    {
        [Test]
        public void PhasesBeforeGameUpdateRemainWithTheirFrameAndFollowingInterval()
        {
            var history = new FrameTimingHistory(true);
            history.Add(40, FramePhase.View, 3);
            history.Add(40, FramePhase.UI, 2);
            history.BeginFrame(40, 16);
            history.Add(41, FramePhase.View, 7);
            history.BeginFrame(41, 33);
            var sample = history.Snapshot().Single();
            Assert.That(sample.frameIndex, Is.EqualTo(40));
            Assert.That(sample.followingFrameIndex, Is.EqualTo(41));
            Assert.That(sample.followingIntervalMilliseconds, Is.EqualTo(33));
            Assert.That(Phase(sample, FramePhase.View).milliseconds, Is.EqualTo(3));
            Assert.That(Phase(sample, FramePhase.UI).milliseconds, Is.EqualTo(2));
            history.BeginFrame(42, 17);
            Assert.That(Phase(history.Snapshot().Last(), FramePhase.View).milliseconds, Is.EqualTo(7));
        }

        [Test]
        public void MultipleSimulationStepsAggregateCallsAndCpuTimeWithinOneFrame()
        {
            var history = new FrameTimingHistory(true);
            history.BeginFrame(10, 16);
            history.Add(10, FramePhase.Simulation, .5);
            history.Add(10, FramePhase.Simulation, 1.5);
            history.Add(10, FramePhase.Simulation, 2);
            history.Add(10, FramePhase.Save, 6);
            history.BeginFrame(11, 34);
            var sample = history.Snapshot().Single();
            Assert.That(Phase(sample, FramePhase.Simulation).calls, Is.EqualTo(3));
            Assert.That(Phase(sample, FramePhase.Simulation).milliseconds, Is.EqualTo(4));
            Assert.That(Phase(sample, FramePhase.Save).milliseconds, Is.EqualTo(6));
        }

        [Test]
        public void SnapshotExcludesUnfinishedCurrentFrame()
        {
            var history = new FrameTimingHistory(true);
            history.BeginFrame(20, 16);
            history.Add(20, FramePhase.Evidence, 8);
            Assert.That(history.Snapshot(), Is.Empty);
            history.BeginFrame(21, 25);
            history.Add(21, FramePhase.Evidence, 99);
            Assert.That(history.Snapshot().Select(s => s.frameIndex), Is.EqualTo(new[] { 20 }));
            Assert.That(Phase(history.Snapshot().Single(), FramePhase.Evidence).milliseconds, Is.EqualTo(8));
        }

        [Test]
        public void BoundedHistoryWrapDropsOldFramesWithoutReusingTheirPhases()
        {
            var history = new FrameTimingHistory(true, 2);
            history.BeginFrame(1, 16);
            history.Add(1, FramePhase.Save, 12);
            for (int frame = 2; frame <= 5; frame++)
            {
                history.BeginFrame(frame, frame * 10);
                history.Add(frame, FramePhase.Animate, frame);
            }
            var samples = history.Snapshot();
            Assert.That(samples.Select(s => s.frameIndex), Is.EqualTo(new[] { 3, 4 }));
            Assert.That(samples.Select(s => s.followingIntervalMilliseconds), Is.EqualTo(new[] { 40d, 50d }));
            Assert.That(samples.All(s => Phase(s, FramePhase.Save).calls == 0), Is.True);
            Assert.That(samples.Select(s => Phase(s, FramePhase.Animate).milliseconds), Is.EqualTo(new[] { 3d, 4d }));
        }

        [Test]
        public void DisabledRecorderRetainsNoFrames()
        {
            var history = new FrameTimingHistory(false);
            history.BeginFrame(1, 16);
            history.Add(1, FramePhase.Simulation, 2);
            history.BeginFrame(2, 33);
            Assert.That(history.Enabled, Is.False);
            Assert.That(history.Snapshot(), Is.Empty);
        }

        private static FramePhaseTiming Phase(FrameTimingSample sample, FramePhase phase)
            => sample.phases.Single(p => p.phase == phase.ToString());
    }
}
