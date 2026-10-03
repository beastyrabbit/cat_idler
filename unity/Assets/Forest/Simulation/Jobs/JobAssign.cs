using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static readonly string[] NeedCargo = { "water", "brew", "food", "fish", "preserves", "medicine", "herbs" };

        private static string JobLabor(string kind) => kind == JobKind.Water ? "fetch_water" : kind == JobKind.Logs || kind == JobKind.Replant ? "woodcut" : kind == JobKind.Fibre ? "forage" : kind == JobKind.Fish ? "fishing" : kind == JobKind.Offering ? "ritual" : kind == JobKind.Expand || kind == JobKind.Road || kind == JobKind.Rail || kind == JobKind.Bridge || kind == JobKind.Dock ? "build" : kind;

        /// <summary>
        /// True when the cat may be given work at all: an adult, not arriving or leaving, not on a need trip and
        /// not carrying the serving it is about to eat or drink. Manual orders and automatic staffing both ask.
        /// </summary>
        public bool CanTakeOrders(Cat c) => OrderRefusal(c) == "";

        /// <summary>Why the cat cannot take orders, as a result code; "" when it can.</summary>
        private string OrderRefusal(Cat c)
        {
            if (c == null || !c.Alive)
                return ResultCode.UnknownTarget;
            if (c.AgeHours < Pace.WorkerMinAgeHours)
                return ResultCode.Kitten;
            if (c.Migration == "arriving" || c.Migration == "departing")
                return ResultCode.Migrating;
            if (BusyWithNeed(c) || CarryingNeedCargo(c))
                return ResultCode.BusyWithNeed;
            return "";
        }

        /// <summary>A cat without work that keeps food, water or medicine because it needs it (see <c>TickNeeds</c>).</summary>
        private bool CarryingNeedCargo(Cat c) =>
            c.JobId == "" && c.Cargo.Count > 0 && c.Cargo.Any(s => NeedCargo.Contains(s.Resource)) && (c.Thirst < Pace.CargoKeepThirst || c.Hunger < Pace.CargoKeepHunger || c.Health < Pace.CargoKeepHealth);

        private bool FreeWorker(Village v, Cat c) => c != null && CanTakeOrders(c) && !IsDungeonExplorer(c) && c.Position.Level == 0 && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.RalliedTo == "" && !v.Routes.Any(r => r.CatId == c.Id);
        private Cat AvailableCat(Village v, string labor) => v.Cats.Where(c => c.Alive && FreeWorker(v, c))
            .OrderByDescending(c => (c.Preferences.Contains(labor) ? Pace.AssignPreferenceScore : 0) + (c.Boosted ? Pace.AssignBoostScore : 0) + Amount(c.Skills, labor)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();

        /// <summary>The failure for a missing or unfit worker: the cat's refusal code when it cannot take orders.</summary>
        private ActionResult NoWorker(Cat c, string message)
        {
            string refusal = c == null ? "" : OrderRefusal(c);
            return refusal == "" ? ActionResult.Fail(message) : ActionResult.Fail(refusal, message);
        }

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
            AutoEquipTool(v, c, j);
        }
        private ActionResult CreateInputJob(Village v, Cat c, string kind, Int2 target, string resource, double amount, double work, string targetId)
        {
            c = c ?? AvailableCat(v, JobLabor(kind));
            if (!FreeWorker(v, c))
                return NoWorker(c, "No available worker");
            string id = Id("job");
            if (!ReserveNearest(v, id, resource, amount, c.Position))
                return ActionResult.Fail(ResultCode.MissingMaterial, "Required finite input unavailable", ("resource", resource));
            StartJob(v, c, new Job { Id = id, Kind = kind, TargetId = targetId, Position = target, Resource = resource, Amount = amount, RequiredWork = work, Phase = JobPhase.Fetch, Manual = true });
            return ActionResult.Ok(id);
        }
    }
}
