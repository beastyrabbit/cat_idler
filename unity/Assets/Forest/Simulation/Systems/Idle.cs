using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Every idle activity, in the order the picker falls back through them.</summary>
        private static readonly string[] IdleActivities = { GoalKey.IdleNap, GoalKey.IdleGroom, GoalKey.IdleSocial, GoalKey.IdlePatrol, GoalKey.IdleShrine, GoalKey.IdlePlay };

        // PACING-HANDOFF IdleWalkLimitSeconds: an idle cat that has not reached its spot after this long picks again (45).
        private const double IdleWalkLimitSeconds = 45;
        // PACING-HANDOFF IdleTileShare: idle cats that may stand on one tile (2).
        private const int IdleTileShare = 2;

        public static bool IsIdleActivity(string goal) => Array.IndexOf(IdleActivities, goal) >= 0;

        /// <summary>
        /// A cat with no job, station or need lives its own life: it picks a weighted idle activity
        /// (nap near its den, groom, sit with another idle cat, patrol the fence, visit the shrine, play),
        /// walks around every footprint to a free spot and lingers there for the activity's dwell.
        /// </summary>
        private void TickIdle(Village v, Cat c, double dt, bool planning)
        {
            bool active = IsIdleActivity(c.Goal) && c.HasIdleSpot;
            if (active && c.IdleUntil >= 0 && TimeSeconds + 1e-9 >= c.IdleUntil)
                active = false;
            else if (active && c.IdleUntil < 0 && TimeSeconds - c.IdleStartedAt > IdleWalkLimitSeconds)
                active = false;
            else if (active && planning && !IdleSpotOpen(v, c.IdleSpot))
                active = false;
            if (!active)
            {
                // A cat that just became idle first finishes the walk it is on.
                if (!IsIdleActivity(c.Goal) && c.Path.Count > 0)
                {
                    Move(c, c.Path[c.Path.Count - 1], dt);
                    return;
                }
                // New choices happen on the cat's planning second; until then it stays where it is.
                if (!planning)
                    return;
                if (!PickIdleActivity(v, c))
                {
                    c.Goal = GoalKey.Idle;
                    c.HasIdleSpot = false;
                    return;
                }
            }
            if (WalkOpen(v, c, c.IdleSpot, dt))
            {
                if (c.IdleUntil < 0)
                    c.IdleUntil = TimeSeconds + IdleDwell(c);
            }
            else if (c.BlockedReason == Blocked.BlockedRoute)
                c.IdleStartedAt = TimeSeconds - IdleWalkLimitSeconds - 1;
        }

        /// <summary>Chooses the next idle activity and its spot. False when nothing nearby is free.</summary>
        private bool PickIdleActivity(Village v, Cat c)
        {
            bool kitten = c.AgeHours < Pace.KittenUntilHours;
            bool night = DayClock.IsNight(this);
            var weights = new double[IdleActivities.Length];
            for (int i = 0; i < IdleActivities.Length; i++)
            {
                string activity = IdleActivities[i];
                double weight = activity == GoalKey.IdleNap ? (kitten ? 2 : 3) * (night ? 3 : 1)
                    : activity == GoalKey.IdleGroom ? (kitten ? 1 : 3)
                    : activity == GoalKey.IdleSocial ? (kitten ? 2 : 3)
                    : activity == GoalKey.IdlePatrol ? (kitten ? 0 : 2)
                    : activity == GoalKey.IdleShrine ? (kitten ? 1 : 2)
                    : (kitten ? 5 : 1);
                // Variety: the activity just finished is half as likely to follow itself.
                weights[i] = activity == c.Goal ? weight / 2 : weight;
            }
            string tick = TickIndex.ToString(CultureInfo.InvariantCulture);
            double roll = Unit(c.Id + ":idle:" + tick) * weights.Sum();
            int first = weights.Length - 1;
            for (int i = 0; i < weights.Length; i++)
            {
                roll -= weights[i];
                if (roll < 0)
                {
                    first = i;
                    break;
                }
            }
            var crowd = IdleCrowd(v, c);
            for (int k = 0; k < IdleActivities.Length; k++)
            {
                int index = (first + k) % IdleActivities.Length;
                if (weights[index] <= 0)
                    continue;
                string activity = IdleActivities[index];
                var open = IdleCandidates(v, c, activity).Distinct().Where(p => IdleSpotOpen(v, p) && (!crowd.TryGetValue(p, out int n) || n < IdleTileShare)).ToList();
                if (open.Count == 0)
                    continue;
                // Try a few of the open spots, starting from a seeded pick, until one is reachable.
                int start = (int)(Unit(c.Id + ":spot:" + activity + ":" + tick) * open.Count);
                for (int attempt = 0; attempt < Math.Min(3, open.Count); attempt++)
                {
                    var spot = open[(start + attempt) % open.Count];
                    if (!RouteTo(v, c, spot))
                        continue;
                    c.Goal = activity;
                    c.IdleSpot = spot;
                    c.HasIdleSpot = true;
                    c.IdleStartedAt = TimeSeconds;
                    c.IdleUntil = -1;
                    return true;
                }
            }
            return false;
        }

        /// <summary>Where each other cat stands or is heading for its idle activity or need.</summary>
        private Dictionary<Int2, int> IdleCrowd(Village v, Cat c)
        {
            var crowd = new Dictionary<Int2, int>();
            foreach (var o in v.Cats)
            {
                if (o == c || !o.Alive)
                    continue;
                Int2 at;
                if (IsIdleActivity(o.Goal) && o.HasIdleSpot)
                    at = o.IdleSpot;
                else if (BusyWithNeed(o) && o.HasNeedSpot)
                    at = o.NeedSpot;
                else
                    continue;
                crowd[at] = crowd.TryGetValue(at, out int n) ? n + 1 : 1;
            }
            return crowd;
        }

        /// <summary>Inside the fence, walkable, dry and clear of every footprint and doorway.</summary>
        private bool IdleSpotOpen(Village v, Int2 p)
        {
            if (p.Level != 0 || Math.Abs(p.X - v.Center.X) >= v.Radius || Math.Abs(p.Z - v.Center.Z) >= v.Radius)
                return false;
            if (Footprints(v).Contains(p) || Entrances(v).Contains(p) || !Walkable(v, p))
                return false;
            var tile = GetTile(p);
            return tile != null && !tile.Water;
        }

        /// <summary>Candidate spots for one activity; the radii scale with the village.</summary>
        private IEnumerable<Int2> IdleCandidates(Village v, Cat c, string activity)
        {
            switch (activity)
            {
                case GoalKey.IdleNap:
                    {
                        // In the sun near home: a ring around the cat's den, or around the shrine without a bed.
                        var home = v.Buildings.Find(b => b.Id == c.BedId && b.Completed) ?? v.Buildings.Find(b => b.Kind == "shrine");
                        if (home == null)
                            return Around(c.Position, 1, 2);
                        int reach = Math.Clamp(v.Radius / 3, 2, 4);
                        return AroundRect(home.Position, home.Width, home.Depth, 1, reach);
                    }
                case GoalKey.IdleGroom:
                    return Around(c.Position, 0, Math.Clamp(v.Radius / 4, 1, 3));
                case GoalKey.IdleSocial:
                    {
                        var partner = IdlePartner(v, c, false);
                        return partner == null ? Enumerable.Empty<Int2>() : Around(IdleAnchor(partner), 1, 1);
                    }
                case GoalKey.IdlePatrol:
                    {
                        // Just inside the fence; the seeded pick spreads patrols around the whole ring.
                        int ring = v.Radius - 1;
                        return AroundRect(new Int2(v.Center.X, v.Center.Z), 1, 1, ring, ring);
                    }
                case GoalKey.IdleShrine:
                    {
                        var shrine = v.Buildings.Find(b => b.Kind == "shrine");
                        return shrine == null ? Enumerable.Empty<Int2>() : AroundRect(shrine.Position, shrine.Width, shrine.Depth, 1, 1);
                    }
                default:
                    {
                        // Play: chase another kitten when one is idle nearby, else pounce about.
                        var partner = c.AgeHours < Pace.KittenUntilHours ? IdlePartner(v, c, true) : null;
                        int reach = Math.Clamp(v.Radius / 3, 2, 4);
                        return partner != null ? Around(IdleAnchor(partner), 1, 1) : Around(c.Position, 2, reach);
                    }
            }
        }

        private static Int2 IdleAnchor(Cat c) => IsIdleActivity(c.Goal) && c.HasIdleSpot ? c.IdleSpot : c.Position;

        /// <summary>Another idle cat to sit with (or, for a kitten, another kitten), seeded by this cat's hash.</summary>
        private Cat IdlePartner(Village v, Cat c, bool kittensOnly)
        {
            int reach = Math.Max(4, v.Radius * 2);
            var partners = v.Cats.Where(o => o != c && o.Alive && o.Position.Level == 0 && IsIdleActivity(o.Goal) && o.HasIdleSpot && Int2.Distance(IdleAnchor(o), c.Position) <= reach && (!kittensOnly || o.AgeHours < Pace.KittenUntilHours)).OrderBy(o => o.Id, StringComparer.Ordinal).ToList();
            if (partners.Count == 0)
                return null;
            return partners[(int)(Unit(c.Id + ":partner:" + TickIndex.ToString(CultureInfo.InvariantCulture)) * partners.Count)];
        }

        /// <summary>Tiles whose Chebyshev distance from a point lies in [inner, outer].</summary>
        private static IEnumerable<Int2> Around(Int2 at, int inner, int outer) => AroundRect(at, 1, 1, inner, outer);

        /// <summary>Tiles whose Chebyshev distance from a rectangle lies in [inner, outer].</summary>
        private static IEnumerable<Int2> AroundRect(Int2 at, int width, int depth, int inner, int outer)
        {
            for (int x = at.X - outer; x < at.X + width + outer; x++)
                for (int z = at.Z - outer; z < at.Z + depth + outer; z++)
                {
                    int dx = x < at.X ? at.X - x : x >= at.X + width ? x - (at.X + width - 1) : 0;
                    int dz = z < at.Z ? at.Z - z : z >= at.Z + depth ? z - (at.Z + depth - 1) : 0;
                    int d = Math.Max(dx, dz);
                    if (d >= inner && d <= outer)
                        yield return new Int2(x, z, at.Level);
                }
        }

        /// <summary>How long the cat lingers at its spot: a share of IdleDwellMin..Max that suits the activity.</summary>
        private double IdleDwell(Cat c)
        {
            (double lo, double hi) = c.Goal == GoalKey.IdleNap ? (0.5, 1.0)
                : c.Goal == GoalKey.IdleGroom ? (0.2, 0.6)
                : c.Goal == GoalKey.IdleSocial ? (0.3, 0.8)
                : c.Goal == GoalKey.IdlePatrol ? (0.0, 0.3)
                : c.Goal == GoalKey.IdleShrine ? (0.2, 0.6)
                : (0.0, 0.25);
            double share = lo + (hi - lo) * Unit(c.Id + ":idle-dwell:" + TickIndex.ToString(CultureInfo.InvariantCulture));
            return Pace.IdleDwellMin + (Pace.IdleDwellMax - Pace.IdleDwellMin) * share;
        }
    }
}
