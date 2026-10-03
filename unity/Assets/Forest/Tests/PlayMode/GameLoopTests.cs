using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Net.Sockets;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace IdleCatForest.Tests
{
    /// <summary>
    /// The game loop under failure and load: a throwing step or action pauses the forest, flags it as
    /// faulted, never saves the faulted world and recovers on the next good step; speeds snap to D18;
    /// the step loop keeps to its budget; every action publishes structured feedback.
    /// </summary>
    public class GameLoopTests
    {
        private ForestGame game;
        private string directory;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-game-loop-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated game loop test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            // The test drives frames itself so that the stepped time is exact.
            game.enabled = false;
            game.StepBudgetMilliseconds = double.PositiveInfinity;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [Test]
        public void ANewLocalWorldIsTheNamedDemoColony()
        {
            var world = game.CurrentWorld;
            Assert.That(world.PacingProfile, Is.EqualTo(Pacing.DemoProfile), "New single-player worlds use demo pacing.");
            Assert.That(world.GenerationVersion, Is.EqualTo(2), "New single-player worlds are founded by CreateColony.");
            Assert.That(game.Selected.Name, Is.EqualTo(World.DefaultColonyName));
            Assert.That(game.Selected.Cats.Count(c => c.Alive), Is.GreaterThanOrEqualTo(30));
            Assert.That(world.Validate(), Is.Empty);
        }

        [Test]
        public void InjectedStepExceptionPausesFlagsFaultedSkipsTheSaveAndTheNextGoodStepClearsIt()
        {
            game.SetSpeed(8);
            game.RunFrame(.25);
            game.RunFrame(.25);
            Assert.That(game.Faulted, Is.False);
            byte[] saved = File.ReadAllBytes(game.SavePath);
            int changes = 0, errors = ForestLog.ErrorCount;
            game.Changed += () => changes++;
            int probes = 0;
            // Thirty good steps (1.5 simulated seconds) run in the faulting frame before the throw, so a loop
            // that kept counting them would reach the 5 s autosave with the half-applied world.
            game.StepProbe = world => { if (++probes == 31) throw new InvalidOperationException("Injected step failure"); };
            LogAssert.Expect(LogType.Exception, new Regex("Injected step failure"));

            game.RunFrame(.25);

            Assert.That(probes, Is.EqualTo(31));
            Assert.That(game.Faulted, Is.True, "A throwing step must flag the forest as faulted.");
            Assert.That(game.Speed, Is.EqualTo(0), "A throwing step must pause the forest.");
            Assert.That(changes, Is.EqualTo(1), "The fault is a state change the HUD must hear about.");
            Assert.That(File.ReadAllBytes(game.SavePath), Is.EqualTo(saved), "The faulted world must not be saved.");
            Assert.That(game.LastFeedback.Code, Is.EqualTo(FeedbackCode.SimulationFault));
            Assert.That(game.LastFeedback.Success, Is.False);
            Assert.That(game.LastAction, Does.Not.Contain("Injected"), "Exception text never reaches player text.");
            Assert.That(ForestLog.ErrorCount, Is.EqualTo(errors + 1));
            Assert.That(ForestLog.SessionLogPath, Does.StartWith(directory), "Isolated worlds keep their logs next to their save.");
            Assert.That(File.ReadAllText(ForestLog.SessionLogPath), Does.Contain("Injected step failure").And.Contain("[forest] session"));

            for (int i = 0; i < 5; i++) game.RunFrame(.25);
            Assert.That(probes, Is.EqualTo(31), "A paused, faulted forest must not keep stepping.");
            Assert.That(game.Faulted, Is.True, "Only a successful step clears the fault.");
            Assert.That(changes, Is.EqualTo(1));
            Assert.That(File.ReadAllBytes(game.SavePath), Is.EqualTo(saved), "The faulted world must not be saved while paused.");

            game.StepProbe = null;
            game.SetSpeed(1);
            game.RunFrame(.1);
            Assert.That(game.Faulted, Is.False, "The next good step clears the fault.");
            Assert.That(changes, Is.EqualTo(2));
            Assert.That(game.FaultCount, Is.EqualTo(1));
        }

        [Test]
        public void ThrowingLocalActionFaultsLikeAThrowingStep()
        {
            game.SetSpeed(2);
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.ApplyProbe = action => throw new InvalidOperationException("Injected action failure");
            LogAssert.Expect(LogType.Exception, new Regex("Injected action failure"));

            var result = game.Send(new GameAction { Kind = "AssignWorker", CatId = cat.Id }).Result;

            Assert.That(result.Success, Is.False);
            Assert.That(result.Code, Is.EqualTo(FeedbackCode.SimulationFault));
            Assert.That(result.Error, Does.Not.Contain("Injected"));
            Assert.That(game.Faulted, Is.True);
            Assert.That(game.Speed, Is.EqualTo(0));
            Assert.That(game.LastFeedback.Kind, Is.EqualTo("AssignWorker"));
            Assert.That(game.LastFeedback.Code, Is.EqualTo(FeedbackCode.SimulationFault));
            Assert.That(game.InFlight, Is.Empty, "A failed action must not stay in flight.");

            game.ApplyProbe = null;
            game.SetSpeed(1);
            game.RunFrame(.1);
            Assert.That(game.Faulted, Is.False);
        }

        [Test]
        public void SpeedsSnapToTheD18Set()
        {
            game.SetSpeed(5); Assert.That(game.Speed, Is.EqualTo(4f));
            game.SetSpeed(3); Assert.That(game.Speed, Is.EqualTo(2f), "Ties go to the slower speed.");
            game.SetSpeed(.3f); Assert.That(game.Speed, Is.EqualTo(1f), "A positive request never pauses.");
            game.SetSpeed(-2); Assert.That(game.Speed, Is.EqualTo(0f));
            game.SetSpeed(1000); Assert.That(game.Speed, Is.EqualTo(GameSpeeds.Clamp(1000)));
            Assert.That(GameSpeeds.Clamp(1000, false), Is.EqualTo(4f), "Players top out at 4x.");
            Assert.That(GameSpeeds.Clamp(1000, true), Is.EqualTo(16f), "Dev speeds top out at 16x.");
        }

        [Test]
        public void FramePacingCapsAt60WithoutVsync()
        {
            Assert.That(QualitySettings.vSyncCount, Is.EqualTo(0), "Vsync overrides the frame cap on ProMotion displays.");
            Assert.That(Application.targetFrameRate, Is.EqualTo(ForestGame.FocusedFrameRate));
            Assert.That(ForestGame.FocusedFrameRate, Is.EqualTo(60));
        }

        [Test]
        public void StepBudgetRunsOneStepAndDropsTheBacklog()
        {
            Assert.That(game.SimulationBehind, Is.False);
            game.StepBudgetMilliseconds = 0;
            game.SetSpeed(4);
            double before = game.CurrentWorld.TimeSeconds;
            game.RunFrame(.25);
            Assert.That(game.CurrentWorld.TimeSeconds - before, Is.EqualTo(World.SimulationStepSeconds).Within(1e-9), "The first step of a frame always runs.");
            Assert.That(game.SimulationBehind, Is.True);
            before = game.CurrentWorld.TimeSeconds;
            game.RunFrame(0);
            Assert.That(game.CurrentWorld.TimeSeconds - before, Is.LessThanOrEqualTo(World.SimulationStepSeconds + 1e-9), "The dropped backlog must not be chased.");
        }

        [Test]
        public void RevisionAndChangedFireOncePerSteppingFrame()
        {
            int changes = 0;
            game.Changed += () => changes++;
            int revision = game.Revision;
            game.SetSpeed(4);
            game.RunFrame(.25);
            Assert.That(game.Revision, Is.EqualTo(revision + 1), "Twenty steps in one frame are one change.");
            Assert.That(changes, Is.EqualTo(1));
            game.SetSpeed(0);
            game.RunFrame(.25);
            game.RunFrame(.25);
            Assert.That(changes, Is.EqualTo(1), "A paused frame changes nothing.");
        }

        [Test]
        public void ActionsPublishStructuredFeedback()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            int posted = 0;
            game.FeedbackPosted += feedback => posted++;

            var ok = game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result;
            Assert.That(ok.Success, Is.True);
            var success = game.LastFeedback;
            Assert.That(success.Success, Is.True);
            Assert.That(success.Kind, Is.EqualTo("AssignWorker"));
            Assert.That(success.Code, Is.Not.Empty);
            Assert.That(success.Sequence, Is.EqualTo(1));
            Assert.That(success.At, Is.GreaterThan(0));
            Assert.That(posted, Is.EqualTo(1));

            var refused = game.Send(new GameAction { Kind = "ResearchNode", NodeId = "no_such_study" }).Result;
            Assert.That(refused.Success, Is.False);
            var failure = game.LastFeedback;
            Assert.That(failure.Success, Is.False);
            Assert.That(failure.Kind, Is.EqualTo("ResearchNode"));
            Assert.That(failure.Sequence, Is.EqualTo(2));
            Assert.That(failure.Code.Length + failure.Message.Length, Is.GreaterThan(0), "A refusal carries a code or the simulation's reason.");

            // Continuous control traffic publishes nothing: moves (also into a fence) and heartbeats.
            game.Send(new GameAction { Kind = "MoveCat", CatId = cat.Id, Direction = 1 }).Wait();
            Assert.That(game.LastFeedback, Is.SameAs(failure));
            Assert.That(Feedback.IsSilent("KeepCatControl", true), Is.True);
            Assert.That(Feedback.IsSilent("AttackCreature", true), Is.True);
            Assert.That(Feedback.IsSilent("KeepCatControl", false), Is.False, "A lost control lease must still be reported.");
            Assert.That(posted, Is.EqualTo(2));
        }

        [Test]
        public void DuplicateActionsInFlightAreRefused()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            var action = new GameAction { Kind = "AssignWorker", CatId = cat.Id };
            var before = game.LastFeedback;
            game.InFlight.Add(Feedback.FlightKey(action));
            var duplicate = game.Send(action).Result;
            Assert.That(duplicate.Success, Is.False);
            Assert.That(duplicate.Code, Is.EqualTo(FeedbackCode.AlreadySent));
            Assert.That(game.LastFeedback, Is.SameAs(before), "The pending original reports; the duplicate stays quiet.");
            Assert.That(Feedback.FlightKey(action), Is.Not.EqualTo(Feedback.FlightKey(new GameAction { Kind = "AssignWorker", CatId = cat.Id + "x" })));
            game.InFlight.Clear();
            Assert.That(game.Send(action).Result.Code, Is.Not.EqualTo(FeedbackCode.AlreadySent));
        }

        [Test]
        public void ExceptionsMapToStableCodes()
        {
            Assert.That(Feedback.CodeFor(new TimeoutException("x")), Is.EqualTo(FeedbackCode.Timeout));
            Assert.That(Feedback.CodeFor(new TaskCanceledException("A task was canceled.")), Is.EqualTo(FeedbackCode.Timeout));
            Assert.That(Feedback.CodeFor(new IOException("/Users/someone/secret/path")), Is.EqualTo(FeedbackCode.IoError));
            Assert.That(Feedback.CodeFor(new SocketException()), Is.EqualTo(FeedbackCode.Network));
            Assert.That(Feedback.CodeFor(new AggregateException(new IOException("x"))), Is.EqualTo(FeedbackCode.IoError));
            Assert.That(Feedback.CodeFor(new UriFormatException("x")), Is.EqualTo(FeedbackCode.InvalidAddress));
            Assert.That(Feedback.CodeFor(new InvalidOperationException("x")), Is.EqualTo(FeedbackCode.Unexpected));
            Assert.That(Feedback.CodeFor(new InvalidOperationException("x", new TimeoutException())), Is.EqualTo(FeedbackCode.Timeout));
        }

        [Test]
        public void LogDeduplicatesByTypeAndStack()
        {
            string a = ForestLog.Signature("IOException: one", "at A()\nat B()", LogType.Exception);
            string b = ForestLog.Signature("IOException: two", "at A()\nat B()", LogType.Exception);
            string c = ForestLog.Signature("IOException: one", "at C()", LogType.Exception);
            string d = ForestLog.Signature("TimeoutException: one", "at A()\nat B()", LogType.Exception);
            Assert.That(a, Is.EqualTo(b), "The same failure with another message is one entry.");
            Assert.That(a, Is.Not.EqualTo(c));
            Assert.That(a, Is.Not.EqualTo(d));
        }
    }
}
