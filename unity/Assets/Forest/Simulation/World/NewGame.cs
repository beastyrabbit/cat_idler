using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>New-game setup: the single-player colony a player names in the New forest dialog.</summary>
    public partial class World
    {
        public const string DefaultColonyName = "Mosslight";
        public const int MaxColonyNameLength = 48;
        /// <summary>Chebyshev radius of the natural ring a version 2 founding reveals around its centre.</summary>
        public const int ColonyRevealRadius = 34;
        private const int ColonyRadius = 9;
        // Candidate centres lie within this many tiles of the origin, on even coordinates.
        private const int SiteSearchRadius = 64;
        private const int SiteSampleStep = 2;
        private const int SiteTrials = 64;
        // Forest tiles with trees the reveal must hold after founding clears its square and trail.
        private const int SiteMinimumTrees = 300;

        /// <summary>
        /// Founds a new single-player colony on generation version 2. The colony keeps the internal
        /// communal id so every existing system treats it as the founding colony; only its name is
        /// the player's. A blank name falls back to <see cref="DefaultColonyName"/>. The site is the
        /// best scored centre near the origin (river or lake 3 to 8 tiles outside the fence, forest,
        /// highland and a rock landmark in view), so the same seed always yields the same colony.
        /// </summary>
        public static World CreateColony(int seed, string colonyName, string pacingProfile, long nowUnixMs)
        {
            string name = string.IsNullOrWhiteSpace(colonyName) ? DefaultColonyName : colonyName.Trim();
            if (name.Length > MaxColonyNameLength)
                throw new ArgumentOutOfRangeException(nameof(colonyName), "Colony name requires 1 to " + MaxColonyNameLength + " characters");
            var w = new World
            {
                Seed = unchecked((uint)seed),
                RandomState = unchecked((uint)seed),
                EpochUnixMs = nowUnixMs,
                GenerationVersion = 2,
                PacingProfile = pacingProfile == Pacing.DemoProfile ? Pacing.DemoProfile : Pacing.ClassicProfile,
            };
            var center = w.ChooseColonySite(out double datum);
            w.SurfaceDatum = datum;
            w.Villages.Add(w.Found("communal", name, "", center, true));
            w.EnsureVillageDungeon(w.Villages[0]);
            return w;
        }

        /// <summary>Summed samples of the unfounded landscape on a regular grid, queried by square windows.</summary>
        private sealed class SiteSamples
        {
            public readonly int Span, Count;
            public readonly double[,] Water, Trees, Landmarks, Highland, DryHeight, Dry;
            public SiteSamples(int span)
            {
                Span = span;
                Count = span * 2 / SiteSampleStep + 1;
                Water = new double[Count + 1, Count + 1];
                Trees = new double[Count + 1, Count + 1];
                Landmarks = new double[Count + 1, Count + 1];
                Highland = new double[Count + 1, Count + 1];
                DryHeight = new double[Count + 1, Count + 1];
                Dry = new double[Count + 1, Count + 1];
            }
            public static void Accumulate(double[,] sums)
            {
                int n = sums.GetLength(0);
                for (int i = 1; i < n; i++)
                    for (int j = 1; j < n; j++)
                        sums[i, j] += sums[i - 1, j] + sums[i, j - 1] - sums[i - 1, j - 1];
            }
            /// <summary>Sum over samples whose coordinates lie within <paramref name="half"/> tiles (Chebyshev) of the point.</summary>
            public double Sum(double[,] sums, int x, int z, int half)
            {
                int Low(int v) => Math.Max(0, (int)Math.Ceiling((v - half + Span) / (double)SiteSampleStep));
                int High(int v) => Math.Min(Count - 1, (int)Math.Floor((v + half + Span) / (double)SiteSampleStep));
                int i0 = Low(x), i1 = High(x), j0 = Low(z), j1 = High(z);
                if (i1 < i0 || j1 < j0)
                    return 0;
                return sums[i1 + 1, j1 + 1] - sums[i0, j1 + 1] - sums[i1 + 1, j0] + sums[i0, j0];
            }
        }

        /// <summary>
        /// Deterministic scored site search near the origin. A coarse sample grid ranks every even centre
        /// within <see cref="SiteSearchRadius"/>; the best candidates are then generated exactly (with their
        /// datum and plateau) until one meets every first-look guarantee. The datum is the local mean height
        /// of the dry ground the plateau and its shoulder cover.
        /// </summary>
        private Int2 ChooseColonySite(out double datum)
        {
            int reveal = ColonyRevealRadius, radius = ColonyRadius, grade = FoundingCoreRadius(radius) + ShoulderMinimum;
            var s = new SiteSamples(SiteSearchRadius + reveal + SiteSampleStep);
            for (int i = 0; i < s.Count; i++)
                for (int j = 0; j < s.Count; j++)
                {
                    var t = GenerateSurfaceTile(new Int2(-s.Span + i * SiteSampleStep, -s.Span + j * SiteSampleStep));
                    s.Water[i + 1, j + 1] = t.Water ? 1 : 0;
                    s.Trees[i + 1, j + 1] = t.Resource == "logs" && t.Biome.Contains("forest") ? 1 : 0;
                    s.Landmarks[i + 1, j + 1] = t.Mountain && (t.Landform == "outcrop" || t.Landform == "ridge") ? 1 : 0;
                    s.Highland[i + 1, j + 1] = t.Biome == "highland" ? 1 : 0;
                    s.DryHeight[i + 1, j + 1] = t.Water ? 0 : t.Elevation;
                    s.Dry[i + 1, j + 1] = t.Water ? 0 : 1;
                }
            foreach (var sums in new[] { s.Water, s.Trees, s.Landmarks, s.Highland, s.DryHeight, s.Dry })
                SiteSamples.Accumulate(sums);
            double area = SiteSampleStep * SiteSampleStep;
            var candidates = new List<(Int2 Center, double Score)>();
            for (int cz = -SiteSearchRadius; cz <= SiteSearchRadius; cz += SiteSampleStep)
                for (int cx = -SiteSearchRadius; cx <= SiteSearchRadius; cx += SiteSampleStep)
                {
                    if (s.Sum(s.Water, cx, cz, radius + 2) > 0 || s.Sum(s.Water, cx, cz, radius + 7) == 0 || s.Sum(s.Water, cx, cz + radius + 7, 2) > 0)
                        continue;
                    // Rock never stands inside the graded shoulder, so only landmarks beyond it count.
                    double landmarks = s.Sum(s.Landmarks, cx, cz, reveal) - s.Sum(s.Landmarks, cx, cz, grade);
                    if (landmarks <= 0)
                        continue;
                    double trees = s.Sum(s.Trees, cx, cz, reveal) * area;
                    double highland = s.Sum(s.Highland, cx, cz, reveal) * area;
                    bool closeWater = s.Sum(s.Water, cx, cz, radius + 5) > 0;
                    double score = trees + 40 * Math.Min(landmarks, 3) + 0.3 * Math.Min(highland, 400) + (closeWater ? 30 : 0) - 1.5 * (Math.Abs(cx) + Math.Abs(cz));
                    candidates.Add((new Int2(cx, cz), score));
                }
            double Datum(Int2 c)
            {
                double dry = s.Sum(s.Dry, c.X, c.Z, grade);
                return dry > 0 ? QuantizeHeight(s.Sum(s.DryHeight, c.X, c.Z, grade) / dry) : 0;
            }
            var ranked = candidates.OrderByDescending(c => c.Score).ThenBy(c => Math.Abs(c.Center.X) + Math.Abs(c.Center.Z)).ThenBy(c => c.Center.Z).ThenBy(c => c.Center.X).Select(c => c.Center).ToList();
            foreach (var center in ranked.Take(SiteTrials))
                if (SiteMeetsFirstLook(center, Datum(center)))
                {
                    datum = Datum(center);
                    return center;
                }
            // No candidate passed every exact check: keep the best coarse site, or the classic origin.
            var fallback = ranked.Count > 0 ? ranked[0] : new Int2(0, 0);
            datum = Datum(fallback);
            return fallback;
        }

        /// <summary>Generates the founded reveal exactly (datum and plateau included) and checks the first-look guarantees.</summary>
        private bool SiteMeetsFirstLook(Int2 center, double datum)
        {
            int radius = ColonyRadius, reveal = ColonyRevealRadius;
            var trial = new World { Seed = Seed, GenerationVersion = GenerationVersion, SurfaceDatum = datum };
            int core = FoundingCoreRadius(radius);
            trial.SurfacePlateaus.Add(new SurfacePlateau { VillageId = "site", Center = center, CoreRadius = core, Shoulder = trial.GradeShoulder(center, core) });
            var entrance = new Int2(center.X, center.Z + radius + 7);
            int trees = 0, nearestWater = int.MaxValue, size = reveal * 2 + 1;
            bool landmark = false;
            // Walk height of each revealed tile, NaN where a cat cannot stand (rock, deep water, the trail).
            var walk = new double[size, size];
            for (int z = -reveal; z <= reveal; z++)
                for (int x = -reveal; x <= reveal; x++)
                {
                    var p = new Int2(center.X + x, center.Z + z);
                    var t = trial.GenerateSurfaceTile(p);
                    int ring = Math.Max(Math.Abs(x), Math.Abs(z));
                    bool standable = !t.Mountain && (!t.Water || t.WaterDepth <= WadingDepth) && ring > radius;
                    if (t.Water)
                    {
                        nearestWater = Math.Min(nearestWater, ring);
                        if (Math.Max(Math.Abs(p.X - entrance.X), Math.Abs(p.Z - entrance.Z)) <= 2)
                            return false;
                    }
                    // The founding clears its square and footpath; the Root Hollow trail and stair clear a few more.
                    bool stair = z == radius + 7 && x > 0 && x <= 4;
                    bool trail = x == 0 && z > radius && z <= radius + 7 || stair;
                    // EnsureVillageDungeon lays its trail at the plateau height, so the ground beside it must
                    // stay within a climbable step or the entrance stands on a pedestal.
                    bool besideTrail = !trail && z > radius && (Math.Abs(x) == 1 && z <= radius + 7 || x == 0 && z == radius + 8);
                    if (besideTrail && !t.Water && !t.Mountain && Math.Abs(t.Elevation) > 0.6)
                        return false;
                    if (ring >= radius + 2 && !trail && t.Resource == "logs" && t.Amount > 0 && t.Biome.Contains("forest"))
                        trees++;
                    landmark |= t.Mountain && (t.Landform == "outcrop" || t.Landform == "ridge");
                    walk[x + reveal, z + reveal] = standable && !trail ? t.Elevation : double.NaN;
                }
            if (nearestWater < radius + 3 || nearestWater > radius + 8 || trees < SiteMinimumTrees || !landmark)
                return false;
            // Grading and relief never leave a step a cat cannot climb between two standable tiles.
            for (int i = 0; i < size; i++)
                for (int j = 0; j < size; j++)
                {
                    double here = walk[i, j];
                    if (double.IsNaN(here))
                        continue;
                    if (i + 1 < size && Math.Abs(here - walk[i + 1, j]) > ClimbableStep || j + 1 < size && Math.Abs(here - walk[i, j + 1]) > ClimbableStep)
                        return false;
                }
            return true;
        }
    }
}
