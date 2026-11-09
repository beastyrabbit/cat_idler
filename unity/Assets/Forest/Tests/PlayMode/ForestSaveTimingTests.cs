using System;
using System.IO;
using System.Linq;
using System.Reflection;
using IdleCatForest.Authority;
using IdleCatForest.Presentation;
using NUnit.Framework;
using UnityEngine;

namespace IdleCatForest.Tests
{
    public sealed class ForestSaveTimingTests
    {
        [Test]
        public void OptInSaveHistoryBoundsRowsWithoutLosingChronologyOrFrames()
        {
            var history = new SaveTimingHistory(true, 2);
            history.Add(40, Sample(true, "None", 4));
            history.Add(50, Sample(false, "MoveReplace", 7));
            history.Add(50, Sample(true, "None", 9));
            var rows = history.Snapshot();
            Assert.That(rows.Length, Is.EqualTo(2), "enabled history did not retain the bounded save rows");
            Assert.That(rows.Select(row => row.sequence), Is.EqualTo(new long[] { 2, 3 }));
            Assert.That(rows.Select(row => row.frameIndex), Is.EqualTo(new[] { 50, 50 }));
            Assert.That(rows.Select(row => row.success), Is.EqualTo(new[] { false, true }));
            Assert.That(rows[0].failedStage, Is.EqualTo("MoveReplace"));
            Assert.That(rows[0].stages.Single().milliseconds, Is.EqualTo(7));
            Assert.That(rows[1].stages.Single().milliseconds, Is.EqualTo(9));
        }

        [Test]
        public void DisabledSaveHistoryRetainsNoRows()
        {
            Assert.That(SaveTimingHistory.Requested("", new[] { "--forest-profile-saves" }), Is.False);
            Assert.That(SaveTimingHistory.Requested("evidence", Array.Empty<string>()), Is.False);
            Assert.That(SaveTimingHistory.Requested("evidence", new[] { "--forest-profile-frames" }), Is.False);
            Assert.That(SaveTimingHistory.Requested("evidence", new[] { "--forest-profile-saves" }), Is.True);
            var history = new SaveTimingHistory(false);
            history.Add(12, Sample(true, "None", 5));
            Assert.That(history.Enabled, Is.False);
            Assert.That(history.Snapshot(), Is.Empty);
        }

        [Test]
        public void OptInGameSaveCapturesCompletedAuthorityStagesAndExportsThem()
        {
            var directory = Path.Combine(Path.GetTempPath(), "forest-save-timing-ui-" + Guid.NewGuid().ToString("N"));
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            var go = new GameObject("Save timing test");
            try
            {
                var game = go.AddComponent<ForestGame>();
                typeof(ForestGame).GetField("local", BindingFlags.Instance | BindingFlags.NonPublic)
                    .SetValue(game, LocalAuthority.CreateNew(Path.Combine(directory, "world.json"), 41));
                typeof(ForestGame).GetProperty("SaveTimings").SetValue(game, new SaveTimingHistory(true));
                game.Save();
                var sample = game.SaveTimings.Snapshot().Single();
                Assert.That(sample.success, Is.True);
                Assert.That(sample.frameIndex, Is.EqualTo(Time.frameCount));
                Assert.That(sample.payloadBytes, Is.GreaterThan(0));
                Assert.That(sample.envelopeBytes, Is.GreaterThan(sample.payloadBytes));
                Assert.That(sample.stages.Length, Is.EqualTo(10));
                Assert.That(sample.stages.All(stage => stage.attempted && stage.succeeded), Is.True);
                var json = JsonUtility.ToJson(game.MeasurePerformance());
                Assert.That(json, Does.Contain("\"saveTimings\""));
                Assert.That(json, Does.Contain("\"PayloadEncode\""));
                Assert.That(json, Does.Not.Contain("world.json"));
                Assert.That(json, Does.Not.Contain("PlayerId"));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(go);
                if (Directory.Exists(directory)) Directory.Delete(directory, true);
            }
        }

        private static SaveTimingSample Sample(bool success, string failedStage, double milliseconds)
            => new SaveTimingSample { success = success, failedStage = failedStage,
                stages = new[] { new SaveStageSample { stage = "MoveReplace", attempted = true,
                    succeeded = success, milliseconds = milliseconds } } };
    }
}
