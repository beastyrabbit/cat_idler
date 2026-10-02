using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static string JobLabor(string kind) => kind == JobKind.Water ? "fetch_water" : kind == JobKind.Logs || kind == JobKind.Replant ? "woodcut" : kind == JobKind.Fibre ? "forage" : kind == JobKind.Fish ? "fishing" : kind == JobKind.Offering ? "ritual" : kind == JobKind.Expand || kind == JobKind.Road || kind == JobKind.Rail || kind == JobKind.Bridge || kind == JobKind.Dock ? "build" : kind;
        private bool FreeWorker(Village v, Cat c) => c != null && !IsDungeonExplorer(c) && c.Position.Level == 0 && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "" && !v.Routes.Any(r => r.CatId == c.Id);
        private Cat AvailableCat(Village v, string labor) => v.Cats.Where(c => c.Alive && c.AgeHours >= Pace.WorkerMinAgeHours && FreeWorker(v, c) && c.Migration != "arriving" && c.Migration != "departing")
            .OrderByDescending(c => (c.Preferences.Contains(labor) ? Pace.AssignPreferenceScore : 0) + (c.Boosted ? Pace.AssignBoostScore : 0) + Amount(c.Skills, labor)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
        private void StartJob(Village v, Cat c, Job j)
        {
            if (c.JobId != "" || v.Jobs.Any(job => !job.Completed && job.CatId == c.Id))
                throw new InvalidOperationException("Cat already owns active work");
            if (c.Cargo.Count > 0)
                Spill(v, c.Position, c.Cargo);
            if (j.Id == "")
                j.Id = Id("job");
            j.CatId = c.Id;
            j.Origin = c.Position;
            j.StartedAt = TimeSeconds;
            c.JobId = j.Id;
            c.Goal = j.Kind;
            c.Path.Clear();
            v.Jobs.Add(j);
        }
        private ActionResult CreateInputJob(Village v, Cat c, string kind, Int2 target, string resource, double amount, double work, string targetId)
        {
            c = c ?? AvailableCat(v, JobLabor(kind));
            if (!FreeWorker(v, c))
                return ActionResult.Fail("No available worker");
            string id = Id("job");
            if (!Reserve(v, id, resource, amount))
                return ActionResult.Fail("Required finite input unavailable");
            StartJob(v, c, new Job { Id = id, Kind = kind, TargetId = targetId, Position = target, Resource = resource, Amount = amount, RequiredWork = work, Phase = JobPhase.Fetch, Manual = true });
            return ActionResult.Ok(id);
        }
    }
}
