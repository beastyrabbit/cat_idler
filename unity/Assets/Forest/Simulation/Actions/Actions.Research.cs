using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // PACING-HANDOFF BlessingStudyCostFactor: blessing price of a study as a multiple of its research cost
        // (classic 1; demo 3, so blessings stop being the cheap path to the core studies).
        private const double BlessingStudyCostFactorDemo = 3;
        // PACING-HANDOFF HurryStudySeconds: seconds of current research one blessing buys (300 in every profile).
        private const double HurryStudySeconds = 300;
        // Studies a blessing can buy: the original first entries of the research catalog.
        private const int BlessableStudies = 24;

        private double BlessingStudyCostFactor => Pace.Profile == Pacing.DemoProfile ? BlessingStudyCostFactorDemo : 1;

        /// <summary>The blessing price of a study, or +infinity when blessings cannot buy it.</summary>
        public double BlessingStudyCost(Study node) => node != null && Catalog.Research.IndexOf(node) is int index && index >= 0 && index < BlessableStudies ? node.Cost * BlessingStudyCostFactor : double.PositiveInfinity;

        private ActionResult HandleResearchNode(Village village, GameAction action, string kind) => Research(village, action.NodeId, kind == "unlocknode");
        private ActionResult Research(Village v, string id, bool blessings)
        {
            var node = Catalog.Study(id);
            if (node == null)
                return ActionResult.Fail(ResultCode.UnknownStudy, "Unknown study", (ResultArg.Study, id ?? ""));
            if (v.Research.Contains(id))
                return ActionResult.Fail(ResultCode.AlreadyResearched, "Already researched", (ResultArg.Study, id));
            string missing = node.Prerequisites.FirstOrDefault(p => !v.Research.Contains(p));
            if (missing != null)
                return ActionResult.Fail(ResultCode.MissingPrerequisite, "Missing prerequisites", (ResultArg.Study, missing), (ResultArg.Target, id));
            double cost = blessings ? BlessingStudyCost(node) : node.Cost;
            if (double.IsPositiveInfinity(cost))
                return ActionResult.Fail(ResultCode.NotBlessable, "Blessings purchase original studies only", (ResultArg.Study, id));
            double have = blessings ? v.Blessings : v.ResearchPoints;
            if (have < cost)
                return ActionResult.Fail(ResultCode.CannotAfford, "Insufficient " + (blessings ? "blessings" : "research points"), (ResultArg.Resource, blessings ? "blessings" : "research"), (ResultArg.Need, ActionResult.Number(cost)), (ResultArg.Have, ActionResult.Number(have)), (ResultArg.Study, id));
            if (blessings)
                v.Blessings -= cost;
            else
                v.ResearchPoints -= cost;
            v.Research.Add(id);
            Note(v, EventKind.Research, node.Name, id);
            return ActionResult.Ok(ResultCode.Studied, id, (ResultArg.Resource, blessings ? "blessings" : "research"), (ResultArg.Amount, ActionResult.Number(cost)));
        }

        /// <summary>
        /// Spends whole blessings (Amount, at least one) to add <see cref="HurryStudySeconds"/> of the current
        /// research rate per blessing to the village's research points.
        /// </summary>
        private ActionResult HandleHurryStudy(Village v, GameAction a)
        {
            double rate = StudyRate(v, v.Buildings);
            if (rate <= 0)
                return ActionResult.Fail(ResultCode.NoActiveResearch, "Nobody is studying right now", (ResultArg.Building, "research_hut"));
            double blessings = Math.Max(1, Math.Floor(a.Amount));
            if (v.Blessings < blessings)
                return ActionResult.Fail(ResultCode.CannotAfford, "Insufficient blessings", (ResultArg.Resource, "blessings"), (ResultArg.Need, ActionResult.Number(blessings)), (ResultArg.Have, ActionResult.Number(v.Blessings)));
            double seconds = HurryStudySeconds * blessings;
            double points = seconds * rate;
            v.Blessings -= blessings;
            v.ResearchPoints += points;
            return ActionResult.Ok(ResultCode.Hurried, v.Id, (ResultArg.Blessings, ActionResult.Number(blessings)), (ResultArg.Seconds, ActionResult.Number(seconds)), (ResultArg.Points, ActionResult.Number(points)));
        }
    }
}
