using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult Request(Village v, string kind, Cat c, bool manual, string resource = "")
        {
            kind = Snake(kind);
            switch (kind)
            {
                case JobKind.SupplyFood:
                case JobKind.LeaderPlanHunt:
                case JobKind.HuntExpedition:
                    kind = JobKind.Hunt;
                    break;
                case JobKind.SupplyWater:
                case JobKind.FetchWater:
                    kind = JobKind.Water;
                    break;
                case JobKind.GatherLogs:
                    kind = JobKind.Logs;
                    break;
                case JobKind.ForageFibre:
                    kind = JobKind.Fibre;
                    break;
                case JobKind.ReplantTree:
                    kind = JobKind.Replant;
                    break;
                case JobKind.Explore:
                    kind = JobKind.Scout;
                    break;
                case JobKind.TrainWarrior:
                    kind = JobKind.Train;
                    break;
                case JobKind.ExpandVillage:
                    kind = JobKind.Expand;
                    break;
                case JobKind.LeaderPlanHouse:
                case JobKind.BuildHouse:
                    for (int z = -Pace.AutoDenSearchRadius; z < Pace.AutoDenSearchRadius; z++)
                        for (int x = -Pace.AutoDenSearchRadius; x < Pace.AutoDenSearchRadius; x++)
                        {
                            var p = new Int2(v.Center.X + x, v.Center.Z + z);
                            if (FreeSite(v, p, 2, 2) && FreeBuildingEntrance(v, p).HasValue)
                                return Plan(v, "den", p, c);
                        }
                    return ActionResult.Fail("No clear Den footprint");
                case JobKind.Ritual:
                    return CreateInputJob(v, c, JobKind.Offering, v.Center, "herbs", Pace.OfferingHerbs, Pace.OfferingWork, "");
                case JobKind.CarryOffering:
                case JobKind.PerformOffering:
                    return ActionResult.Fail("Use selectable OfferResource to create a physical offering");
            }
            if (!new[] { JobKind.Hunt, JobKind.Water, JobKind.Logs, JobKind.Quarry, JobKind.Fibre, JobKind.Fish, JobKind.Scout, JobKind.Train, JobKind.Expand, JobKind.Replant }.Contains(kind))
                return ActionResult.Fail("Unknown physical job: " + kind);
            c = c ?? AvailableCat(v, JobLabor(kind));
            if (!FreeWorker(v, c))
                return NoWorker(c, "No available worker");
            if (kind == JobKind.Train)
            {
                var barracks = v.Buildings.Find(b => b.Kind == "barracks" && b.Completed);
                if (barracks == null)
                    return ActionResult.Fail("Completed Barracks required");
                StartJob(v, c, new Job { Kind = kind, Position = barracks.Position, RequiredWork = Pace.TrainWork, Manual = manual });
                return ActionResult.Ok(c.JobId);
            }
            if (kind == JobKind.Expand)
                return RequestExpansion(v, c);
            if (kind == JobKind.Scout)
                return RequestScout(v, c, manual, resource);
            return RequestGather(v, c, kind, manual, resource);
        }
        private ActionResult RequestExpansion(Village v, Cat c)
        {
            var pending = v.Jobs.Find(j => j.Kind == JobKind.Expand && !j.Completed);
            if (pending != null)
            {
                if (pending.CatId != "")
                    return ActionResult.Fail("Expansion already active");
                if (!CanConstructJob(v, pending))
                    return ActionResult.Fail("Foreign territory blocks expansion");
                if (pending.OriginalKind != JobKind.ExpandVillage && ExpansionFootprintBlocked(v, v.Radius + 2, pending.Path))
                    return ActionResult.Fail("Existing footprint or entrance blocks expansion");
                if (c.Cargo.Count > 0)
                    Spill(v, c.Position, c.Cargo);
                pending.CatId = c.Id;
                c.JobId = pending.Id;
                c.Path.Clear();
                return ActionResult.Ok(pending.Id);
            }
            int radius = v.Radius + 2;
            if (!CanModifyTerrain(v.Id, new Int2(v.Center.X - radius, v.Center.Z - radius), radius * 2 + 1, radius * 2 + 1))
                return ActionResult.Fail("Foreign territory blocks expansion");
            if (ExpansionFootprintBlocked(v, radius))
                return ActionResult.Fail("Existing footprint or entrance blocks expansion");
            var perimeter = new List<Int2>();
            for (int x = -radius; x <= radius; x++)
            {
                perimeter.Add(new Int2(v.Center.X + x, v.Center.Z - radius));
                if (x != 0)
                    perimeter.Add(new Int2(v.Center.X + x, v.Center.Z + radius));
            }
            for (int z = -radius + 1; z < radius; z++)
            {
                perimeter.Add(new Int2(v.Center.X - radius, v.Center.Z + z));
                perimeter.Add(new Int2(v.Center.X + radius, v.Center.Z + z));
            }
            perimeter.RemoveAll(p => FarmExterior(v, p, radius) || LayoutGate(v, p, radius));
            if (perimeter.Any(p => !v.Known.Contains(p) || !Walkable(p)))
                return ActionResult.Fail("Survey a dry reachable outer perimeter first");
            var result = CreateInputJob(v, c, JobKind.Expand, v.Center, "materials", perimeter.Count, Pace.ExpansionWork, "");
            if (result.Success)
                v.Jobs.Find(j => j.Id == result.EntityId).Path = perimeter;
            return result;
        }
        private ActionResult RequestScout(Village v, Cat c, bool manual, string resource)
        {
            if (resource != "" && resource != "explore")
            {
                var survey = new List<Int2>();
                for (int surveyRadius = Pace.ScoutSurveyRingStep; surveyRadius <= Pace.ScoutSurveyMaxRadius; surveyRadius += Pace.ScoutSurveyRingStep)
                    foreach (var d in new[] { new Int2(surveyRadius, 0), new Int2(surveyRadius, surveyRadius), new Int2(0, surveyRadius), new Int2(-surveyRadius, surveyRadius), new Int2(-surveyRadius, 0), new Int2(-surveyRadius, -surveyRadius), new Int2(0, -surveyRadius), new Int2(surveyRadius, -surveyRadius) })
                        survey.Add(new Int2(c.Position.X + d.X, c.Position.Z + d.Z));
                int first = survey.FindIndex(p => Path(c.Position, p, v) != null);
                if (first < 0)
                    return ActionResult.Fail("No reachable survey heading");
                StartJob(v, c, new Job { Kind = JobKind.Scout, Resource = resource == "wood" ? "logs" : resource, Position = survey[first], Path = survey, PathIndex = first, RequiredWork = Pace.ScoutWork, Manual = manual });
                return ActionResult.Ok(c.JobId);
            }
            // High-bit draws: Random() % n repeats with a short period, so scouts only went north and south.
            int direction = RandomInt(4), radius = Pace.ScoutBaseRadius + v.Known.Count / Pace.ScoutKnownTilesPerRadius + RandomInt(Pace.ScoutRadiusJitter);
            var offsets = new[] { new Int2(radius, 0), new Int2(0, radius), new Int2(-radius, 0), new Int2(0, -radius) };
            var target = new Int2(v.Center.X + offsets[direction].X, v.Center.Z + offsets[direction].Z);
            for (int attempt = 0; attempt < 4 && Path(c.Position, target, v) == null; attempt++)
            {
                direction = (direction + 1) % 4;
                target = new Int2(v.Center.X + offsets[direction].X, v.Center.Z + offsets[direction].Z);
            }
            if (Path(c.Position, target, v) == null)
                return ActionResult.Fail("No reachable survey route");
            StartJob(v, c, new Job { Kind = JobKind.Scout, Resource = resource, Position = target, RequiredWork = Pace.ScoutWork, Manual = manual });
            return ActionResult.Ok(c.JobId);
        }
        private ActionResult RequestGather(Village v, Cat c, string kind, bool manual, string resource)
        {
            string wanted = kind == JobKind.Hunt ? "food" : kind == JobKind.Quarry ? (resource == "" ? "stone" : resource) : kind;
            Tile source = null;
            bool busy = false;
            if (kind == JobKind.Fish)
            {
                var spot = v.Stockpiles.Find(s => s.Kind == "fishing" && Neighbors(s.Position).Select(TileAt).Any(t => t.Water && FishStock(t) >= 1));
                if (spot == null)
                    return ActionResult.Fail("Designate a shoreline workplace beside replenished Fish habitat");
                source = TileAt(spot.Position);
                wanted = "fish";
            }
            else if (kind == JobKind.Water)
            {
                // Each carrier takes its own water edge, so the leader's crew is not limited to one.
                var edges = v.Known.Select(TileAt).Where(t => t.Water).SelectMany(t => Neighbors(t.Position)).Distinct().Where(p => Walkable(v, p) && !TileExcluded(v, p)).ToList();
                var free = edges.Where(p => !SourceClaimed(v, kind, p)).ToList();
                busy = free.Count == 0 && edges.Count > 0;
                source = free.Where(p => Path(c.Position, p, v) != null).Select(TileAt).FirstOrDefault();
            }
            else if (kind == JobKind.Replant)
                source = v.Known.Select(TileAt).Where(t => t.Resource == "logs" && t.Amount <= 0 && t.RegrowAt < 0 && !TileExcluded(v, t.Position)).OrderBy(t => Int2.Distance(t.Position, c.Position)).FirstOrDefault(t => Path(c.Position, t.Position, v) != null);
            else
            {
                var grounds = v.Known.Select(TileAt).Where(t => SourceAmount(t, wanted) > 0 && Walkable(v, t.Position) && !TileExcluded(v, t.Position)).ToList();
                var free = grounds.Where(t => !SourceClaimed(v, kind, t.Position)).ToList();
                busy = free.Count == 0 && grounds.Count > 0;
                source = free.OrderByDescending(t => v.Stockpiles.Any(s => s.Kind == "zone_gather" && Contains(s.Position, s.Width, s.Depth, t.Position))).ThenBy(t => Int2.Distance(c.Position, t.Position)).FirstOrDefault(t => Path(c.Position, t.Position, v) != null);
            }
            if (source == null && busy)
                return ActionResult.Fail(ResultCode.AllGroundsBusy, kind == JobKind.Hunt ? "Every known hunting ground is already being hunted" : "Every known " + wanted + " source is already being worked", ("resource", wanted));
            if (source == null)
                return ActionResult.Fail("No known reachable " + wanted + " source; scout first");
            if (SourceClaimed(v, kind, source.Position))
                return ActionResult.Fail(ResultCode.AllGroundsBusy, "Source work position claimed", ("resource", wanted));
            StartJob(v, c, new Job { Kind = kind, Resource = wanted, Position = source.Position, RequiredWork = kind == JobKind.Hunt ? Pace.HuntWork : Pace.GatherWork, Manual = manual });
            return ActionResult.Ok(c.JobId);
        }
        private static bool SourceClaimed(Village v, string kind, Int2 at) => v.Jobs.Any(j => !j.Completed && j.Kind == kind && j.Position.Equals(at));
        private IEnumerable<Int2> Neighbors(Int2 p)
        {
            yield return new Int2(p.X, p.Z + 1, p.Level);
            yield return new Int2(p.X + 1, p.Z, p.Level);
            yield return new Int2(p.X, p.Z - 1, p.Level);
            yield return new Int2(p.X - 1, p.Z, p.Level);
        }
    }
}
