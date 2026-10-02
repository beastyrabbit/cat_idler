using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Advances one cat's job by <paramref name="dt"/>. The shared prologue (territory check, planning
        /// gate, production pause, suspended cargo) runs first; then each phase handler runs in order. A
        /// handler that returns true ended the tick; false lets the next phase run in the same tick.
        /// </summary>
        private void TickJob(Village v, Cat c, Job j, double dt)
        {
            if (!CanConstructJob(v, j))
            {
                j.BlockedReason = c.BlockedReason = Blocked.ForeignTerritory;
                return;
            }
            if (j.BlockedReason == Blocked.ForeignTerritory)
                j.BlockedReason = c.BlockedReason = "";
            bool planning = TimeSeconds + 1e-9 >= j.NextPlanningAt;
            if (!planning && (j.BlockedReason == Blocked.ExpansionFootprintBlocked || j.BlockedReason == Blocked.InfrastructureFootprintBlocked || j.BlockedReason == Blocked.BuildingEntranceDisconnected))
                return;
            if (planning && JobReplanBlocked(v, c, j))
                return;
            if (JobProductionPaused(v, c, j))
                return;
            if (JobResumeSuspendedCargo(v, c, j, dt))
                return;
            c.Goal = j.Kind + " · " + j.Phase;
            if (j.Phase == JobPhase.ItemFetch)
            {
                TickJobItemFetch(c, j, dt);
                return;
            }
            if (j.Kind == JobKind.Scout)
            {
                TickJobScout(v, c, j, planning, dt);
                return;
            }
            if (j.Kind == JobKind.Build)
            {
                TickJobBuild(v, c, j, planning, dt);
                return;
            }
            if (j.Phase == JobPhase.Fetch && TickJobFetch(v, c, j, dt))
                return;
            if (j.Phase == JobPhase.InputDelivery && TickJobInputDelivery(v, c, j, dt))
                return;
            if (j.Phase == JobPhase.Travel && TickJobTravel(c, j, dt))
                return;
            if (j.Phase == JobPhase.Working && TickJobWorking(v, c, j, planning, dt))
                return;
            if (j.Phase == JobPhase.OutputDelivery)
                TickJobOutputDelivery(v, c, j, dt);
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
                c.Goal = "production paused";
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
    }
}
