using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Advances one cat's job by <paramref name="dt"/> and watches it: progress refreshes
        /// <see cref="Job.LastProgressAt"/> and clears a blocked reason the moment the block ends; a job whose
        /// route stays blocked for <c>JobWatchdogSeconds</c> gives up its source.
        /// </summary>
        private void TickJob(Village v, Cat c, Job j, double dt)
        {
            // A cat back from a need trip, newly assigned or freshly started has made no stale progress.
            if (j.LastProgressAt < 0 || !ShowsJobGoal(c, j))
                j.LastProgressAt = TimeSeconds;
            var before = new JobMark(v, c, j);
            string prior = j.BlockedReason;
            bool dispatched = TickJobSteps(v, c, j, dt);
            if (j.Completed || c.JobId != j.Id)
                return;
            bool progressed = !before.Same(new JobMark(v, c, j));
            if (dispatched)
            {
                // Phase handlers re-assert a reason that still holds; a waiting tick keeps the old one.
                if (!progressed && j.BlockedReason == "")
                    j.BlockedReason = prior;
                else if (progressed && j.BlockedReason == "" && prior != "" && c.BlockedReason == prior)
                    c.BlockedReason = "";
            }
            if (progressed)
            {
                j.LastProgressAt = TimeSeconds;
                return;
            }
            if (TimeSeconds - j.LastProgressAt + 1e-9 >= Pace.JobWatchdogSeconds && RouteBlocked(c, j))
                WatchdogRelease(v, c, j);
        }

        /// <summary>
        /// The shared prologue (territory check, planning gate, production pause, suspended cargo), then each
        /// phase handler in order. A handler that returns true ended the tick; false lets the next phase run in
        /// the same tick. True when the phase handlers ran.
        /// </summary>
        private bool TickJobSteps(Village v, Cat c, Job j, double dt)
        {
            if (!CanConstructJob(v, j))
            {
                j.BlockedReason = c.BlockedReason = Blocked.ForeignTerritory;
                return false;
            }
            if (j.BlockedReason == Blocked.ForeignTerritory)
                j.BlockedReason = c.BlockedReason = "";
            bool planning = TimeSeconds + 1e-9 >= j.NextPlanningAt;
            if (!planning && (j.BlockedReason == Blocked.ExpansionFootprintBlocked || j.BlockedReason == Blocked.InfrastructureFootprintBlocked || j.BlockedReason == Blocked.BuildingEntranceDisconnected))
                return false;
            if (planning && JobReplanBlocked(v, c, j))
                return false;
            if (JobProductionPaused(v, c, j))
                return false;
            if (JobResumeSuspendedCargo(v, c, j, dt))
                return false;
            c.Goal = j.Kind + GoalKey.Separator + j.Phase;
            // Every reason left here belongs to a phase handler; the handler sets it again while it holds.
            j.BlockedReason = "";
            if (j.Phase == JobPhase.ItemFetch)
            {
                TickJobItemFetch(v, c, j, dt);
                return true;
            }
            if (j.Kind == JobKind.Scout)
            {
                TickJobScout(v, c, j, planning, dt);
                return true;
            }
            if (j.Kind == JobKind.Build)
            {
                TickJobBuild(v, c, j, planning, dt);
                return true;
            }
            if (j.Phase == JobPhase.Fetch && TickJobFetch(v, c, j, dt))
                return true;
            if (j.Phase == JobPhase.InputDelivery && TickJobInputDelivery(v, c, j, dt))
                return true;
            if (j.Phase == JobPhase.Travel && TickJobTravel(c, j, dt))
                return true;
            if (j.Phase == JobPhase.Working && TickJobWorking(v, c, j, planning, dt))
                return true;
            if (j.Phase == JobPhase.OutputDelivery)
                TickJobOutputDelivery(v, c, j, dt);
            return true;
        }

        /// <summary>Re-checks footprints once per planning interval; true when the job is blocked this tick.</summary>
        private bool JobReplanBlocked(Village v, Cat c, Job j)
        {
            j.NextPlanningAt = Math.Floor(TimeSeconds) + Pace.JobPlanningIntervalSeconds;
            if (j.Kind == JobKind.Expand && j.OriginalKind != JobKind.ExpandVillage)
            {
                if (ExpansionFootprintBlocked(v, v.Radius + 2, j.Path))
                {
                    j.BlockedReason = c.BlockedReason = Blocked.ExpansionFootprintBlocked;
                    return true;
                }
                if (j.BlockedReason == Blocked.ExpansionFootprintBlocked)
                    j.BlockedReason = c.BlockedReason = "";
            }
            if (j.Kind == JobKind.Road || j.Kind == JobKind.Rail)
            {
                if (j.Path.Any(p => !FreeInfrastructureFootprint(v, j.Kind, p)))
                {
                    j.BlockedReason = c.BlockedReason = Blocked.InfrastructureFootprintBlocked;
                    return true;
                }
                if (j.BlockedReason == Blocked.InfrastructureFootprintBlocked)
                    j.BlockedReason = c.BlockedReason = "";
            }
            if (j.Kind == JobKind.Build)
            {
                var scaffold = v.Buildings.Find(b => b.Id == j.TargetId);
                if (scaffold != null && !ConnectedEntrance(v, scaffold))
                {
                    j.BlockedReason = c.BlockedReason = Blocked.BuildingEntranceDisconnected;
                    return true;
                }
                if (j.BlockedReason == Blocked.BuildingEntranceDisconnected)
                    j.BlockedReason = c.BlockedReason = "";
            }
            return false;
        }

        /// <summary>True when the job's station or slot is paused, which holds production before delivery.</summary>
        private bool JobProductionPaused(Village v, Cat c, Job j)
        {
            if (j.Kind != JobKind.Production || j.Phase == JobPhase.OutputDelivery)
                return false;
            var station = v.Buildings.Find(b => b.Id == j.TargetId);
            var slot = station?.Slots.Find(s => s.CatId == c.Id);
            if (station?.Paused == true || slot?.Paused == true)
            {
                j.BlockedReason = Blocked.QueuePaused;
                c.Goal = GoalKey.ProductionPaused;
                return true;
            }
            return false;
        }

        /// <summary>Collects cargo spilled when the job was interrupted; true while a pickup is under way.</summary>
        private bool JobResumeSuspendedCargo(Village v, Cat c, Job j, double dt)
        {
            if (j.SuspendedCargoPileId == "")
                return false;
            if (c.Cargo.Count == 0 && Reservations.Any(r => r.OwnerId == j.Id + ":resume"))
            {
                PickupClaim(v, c, j.Id + ":resume", j, dt);
                return true;
            }
            if (!Reservations.Any(r => r.OwnerId == j.Id + ":resume"))
                j.SuspendedCargoPileId = "";
            return false;
        }

        /// <summary>True while the cat's goal still names this job, so time since its last progress counts.</summary>
        private static bool ShowsJobGoal(Cat c, Job j) =>
            c.Goal.Length > j.Kind.Length && c.Goal.StartsWith(j.Kind, StringComparison.Ordinal) && string.CompareOrdinal(c.Goal, j.Kind.Length, GoalKey.Separator, 0, GoalKey.Separator.Length) == 0;

        private static bool RouteBlocked(Cat c, Job j) => c.BlockedReason == Blocked.BlockedRoute || j.BlockedReason == Blocked.BlockedRoute || j.BlockedReason == Blocked.RouteBlocked;

        /// <summary>Jobs the watchdog cancels outright; construction and station work keep their worker and only drop claims.</summary>
        private static bool WatchdogCancels(string kind) =>
            GathersAtSource(kind) || kind == JobKind.Scout || kind == JobKind.Train || kind == JobKind.Haul || kind == JobKind.Offering;

        private static bool GathersAtSource(string kind) =>
            kind == JobKind.Hunt || kind == JobKind.Water || kind == JobKind.Logs || kind == JobKind.Quarry || kind == JobKind.Fibre || kind == JobKind.Fish || kind == JobKind.Replant;

        /// <summary>
        /// A job whose route stayed blocked: the unreachable claim piles and work tile are excluded for
        /// <c>SourceExclusionSeconds</c>; gathering, hauling and errands release the cat, while construction and
        /// station jobs keep their worker, drop the unreachable claims and plan again.
        /// </summary>
        private void WatchdogRelease(Village v, Cat c, Job j)
        {
            var piles = new List<string>();
            foreach (var claim in Reservations)
            {
                if (claim.OwnerId != j.Id && claim.OwnerId != j.Id + ":resume" && !(j.Kind == JobKind.Build && claim.OwnerId == j.TargetId))
                    continue;
                if (piles.Contains(claim.PileId))
                    continue;
                var pile = v.Stockpiles.Find(s => s.Id == claim.PileId);
                if (pile != null && Path(c.Position, pile.Position, v) == null)
                    piles.Add(pile.Id);
            }
            foreach (var pile in piles)
                ExcludeSource(v, PileKey(pile));
            if ((j.Phase == JobPhase.Travel || j.Phase == JobPhase.Working) && GathersAtSource(j.Kind))
                ExcludeSource(v, TileKey(j.Position));
            if (j.Kind == JobKind.Haul && j.Phase == JobPhase.Fetch && j.SourceId != "")
                ExcludeSource(v, PileKey(j.SourceId));
            if (WatchdogCancels(j.Kind))
            {
                CancelWork(v, c);
                c.BlockedReason = "";
                return;
            }
            // Only production and construction claim their bill again when a claim goes; other kinds keep theirs.
            if (j.Kind == JobKind.Production || j.Kind == JobKind.Build)
                Reservations.RemoveAll(r => piles.Contains(r.PileId) && (r.OwnerId == j.Id || j.Kind == JobKind.Build && r.OwnerId == j.TargetId));
            if (j.Kind == JobKind.Production && j.Phase == JobPhase.Fetch && c.Cargo.Count == 0 && !Reservations.Any(r => r.OwnerId == j.Id))
                j.Phase = JobPhase.Working;
            c.Path.Clear();
            j.LastProgressAt = TimeSeconds;
        }

        private static string PileKey(string pileId) => "pile:" + pileId;
        private static string TileKey(Int2 p) => "tile:" + p.X.ToString(CultureInfo.InvariantCulture) + "," + p.Z.ToString(CultureInfo.InvariantCulture) + "," + p.Level.ToString(CultureInfo.InvariantCulture);

        private void ExcludeSource(Village v, string key)
        {
            v.SourceExclusions.RemoveAll(e => e.Until <= TimeSeconds || e.Key == key);
            v.SourceExclusions.Add(new SourceExclusion { Key = key, Until = TimeSeconds + Pace.SourceExclusionSeconds });
        }

        private bool SourceExcluded(Village v, string key)
        {
            foreach (var e in v.SourceExclusions)
                if (e.Key == key && e.Until > TimeSeconds)
                    return true;
            return false;
        }

        private bool TileExcluded(Village v, Int2 p) => v.SourceExclusions.Count > 0 && SourceExcluded(v, TileKey(p));
        private bool PileExcluded(Village v, string pileId) => v.SourceExclusions.Count > 0 && SourceExcluded(v, PileKey(pileId));

        /// <summary>What a job's tick can change: position, phase, work, cargo and outputs. Equal marks mean no progress.</summary>
        private readonly struct JobMark
        {
            private readonly Int2 position;
            private readonly double x, z, progress, cargo, local, building;
            private readonly string phase;
            private readonly int pathIndex, items, notes;

            public JobMark(Village v, Cat c, Job j)
            {
                position = c.Position;
                x = c.X;
                z = c.Z;
                progress = j.Progress;
                phase = j.Phase;
                pathIndex = j.PathIndex;
                items = j.ItemIds.Count;
                notes = c.ScoutNotes.Count;
                cargo = Sum(c.Cargo);
                local = Sum(j.Local);
                building = 0;
                if (j.Kind == JobKind.Build)
                    foreach (var b in v.Buildings)
                        if (b.Id == j.TargetId)
                        {
                            building = b.Progress + Sum(b.Inputs);
                            break;
                        }
            }

            private static double Sum(List<Stack> goods)
            {
                double total = 0;
                foreach (var s in goods)
                    total += s.Amount;
                return total;
            }

            public bool Same(JobMark other) =>
                position.Equals(other.position) && x == other.x && z == other.z && progress == other.progress && cargo == other.cargo && local == other.local &&
                building == other.building && phase == other.phase && pathIndex == other.pathIndex && items == other.items && notes == other.notes;
        }
    }
}
