using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    public sealed class WorkshopSupplyInput
    {
        public string Resource = "";
        public double Required, Reported;
    }

    public sealed class WorkshopSupplyReport
    {
        public string RecipeId = "";
        public List<WorkshopSupplyInput> Inputs = new List<WorkshopSupplyInput>();
        public int CountedPiles, UncountedPiles;
        public double OldestCountedAt = -1, NewestCountedAt = -1;
    }

    public partial class World
    {
        /// <summary>Detached recipe bill and historical storage reports, without live stock or route information.</summary>
        public WorkshopSupplyReport WorkshopSupply(Village village, Building workshop, string recipeId)
        {
            var report = new WorkshopSupplyReport();
            var recipe = Catalog.Recipe(recipeId);
            if (village == null || workshop == null || recipe == null || recipe.Building != workshop.Kind)
                return report;

            report.RecipeId = recipe.Id;
            foreach (var input in recipe.Inputs)
                report.Inputs.Add(new WorkshopSupplyInput { Resource = input.Resource, Required = RecipeInput(village, recipe, input.Amount) });
            foreach (var pile in village.Stockpiles)
            {
                if (pile.Kind != "storage")
                    continue;
                if (pile.CountedAt < 0)
                {
                    report.UncountedPiles++;
                    continue;
                }

                if (report.CountedPiles == 0 || pile.CountedAt < report.OldestCountedAt)
                    report.OldestCountedAt = pile.CountedAt;
                if (report.CountedPiles == 0 || pile.CountedAt > report.NewestCountedAt)
                    report.NewestCountedAt = pile.CountedAt;
                report.CountedPiles++;
                foreach (var input in report.Inputs)
                    input.Reported += Amount(pile.Report, input.Resource);
            }
            return report;
        }
    }
}
