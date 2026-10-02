using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Walks to a carried item's source; the item then travels as output.</summary>
        private void TickJobItemFetch(Cat c, Job j, double dt)
        {
            if (!Move(c, j.Position, dt))
                return;
            j.Phase = JobPhase.OutputDelivery;
            c.Path.Clear();
        }

        /// <summary>Observes around the scout, follows the survey path and returns notes to the village centre.</summary>
        private void TickJobScout(Village v, Cat c, Job j, bool planning, double dt)
        {
            if (planning || !j.HasObservedPosition || !j.ObservedPosition.Equals(c.Position))
            {
                j.HasObservedPosition = true;
                j.ObservedPosition = c.Position;
                for (int z = -Pace.ScoutSightRadius; z <= Pace.ScoutSightRadius; z++)
                    for (int x = -Pace.ScoutSightRadius; x <= Pace.ScoutSightRadius; x++)
                    {
                        var p = new Int2(c.Position.X + x, c.Position.Z + z);
                        var observed = TileAt(p);
                        if (!c.ScoutNotes.Contains(p))
                            c.ScoutNotes.Add(p);
                        if (j.Resource != "" && j.Phase != JobPhase.Return && ((j.Resource == "water" && observed.Water) || SourceAmount(observed, j.Resource) > 0))
                        {
                            j.Phase = JobPhase.Return;
                            j.TargetId = p.ToString();
                            c.Path.Clear();
                        }
                    }
            }
            if (j.Phase == JobPhase.Return)
            {
                if (Move(c, v.Center, dt))
                {
                    foreach (var p in c.ScoutNotes)
                        if (!v.Known.Contains(p))
                            v.Known.Add(p);
                    foreach (var other in Villages.Where(other => other.Id != v.Id && c.ScoutNotes.Any(p => Int2.Distance(p, other.Center) <= other.Radius)))
                    {
                        if (!v.Contacts.Contains(other.Id))
                            v.Contacts.Add(other.Id);
                        if (!other.Contacts.Contains(v.Id))
                            other.Contacts.Add(v.Id);
                    }
                    Note(v, "discovery", c.ScoutNotes.Count + " notes delivered to shrine", c.Id);
                    c.ScoutNotes.Clear();
                    Finish(v, c, j);
                }
                return;
            }
            if (Move(c, j.Position, dt))
            {
                if (j.Path.Count > 0 && j.PathIndex + 1 < j.Path.Count)
                {
                    j.PathIndex++;
                    j.Position = j.Path[j.PathIndex];
                    c.Path.Clear();
                }
                else
                {
                    j.Phase = JobPhase.Return;
                    c.Path.Clear();
                }
            }
            if (TimeSeconds - j.StartedAt > Pace.ScoutMaxSeconds)
            {
                j.Phase = JobPhase.Return;
                c.Path.Clear();
            }
        }

        /// <summary>Picks up reserved inputs; true when the tick ends here, false to continue with input delivery.</summary>
        private bool TickJobFetch(Village v, Cat c, Job j, double dt)
        {
            if (c.Cargo.Count > 0)
            {
                j.Phase = JobPhase.InputDelivery;
                c.Path.Clear();
                return true;
            }
            if (Reservations.Any(r => r.OwnerId == j.Id))
            {
                PickupClaim(v, c, j.Id, j, dt);
                return true;
            }
            j.Phase = JobPhase.InputDelivery;
            return false;
        }

        /// <summary>Walks to the work position; true while still walking, false once work can start this tick.</summary>
        private bool TickJobTravel(Cat c, Job j, double dt)
        {
            if (!Move(c, j.Position, dt))
                return true;
            j.Phase = JobPhase.Working;
            return false;
        }
    }
}
