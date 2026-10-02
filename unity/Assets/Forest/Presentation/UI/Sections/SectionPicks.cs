using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The choices the legacy sections share across panels, exactly as before the split (a cat picked in
    /// one panel stays picked in the next). Sections that move to their own state stop using it.
    /// </summary>
    internal sealed class SectionPicks
    {
        public const string ChooseAvailableCat = "Choose available cat";

        public string Query = "";
        public string SelectedResource = "food";
        public string SelectedRecipe = "";
        public string SelectedCat = "";
        public string SelectedPile = "";
        public string SelectedOther = "";
        public string SelectedMode = "rail";
        public double Amount = 5;
        public double OtherAmount = 5;
        public string ResearchCategory = "All categories";
        public bool ResearchMap;
        public string ServerAddress = "ws://127.0.0.1:8788/ws";

        /// <summary>The shared cat dropdown. Without <paramref name="automatic"/> it preselects the first cat.</summary>
        public DropdownField CatChoice(UiContext ctx, VisualElement parent, bool automatic = false)
        {
            var cats = ctx.Village.Cats.Where(c => c.Alive).ToList();
            if (!cats.Any(c => c.Id == SelectedCat)) SelectedCat = automatic ? "" : cats.FirstOrDefault()?.Id ?? "";
            var names = cats.Select(c => c.Name + " · " + c.Id).ToList();
            if (automatic) names.Insert(0, ChooseAvailableCat);
            string current = SelectedCat == "" ? names.FirstOrDefault() : names.Find(n => n.EndsWith(" · " + SelectedCat));
            var field = UiKit.Choice(parent, "Cat", names, current, x => SelectedCat = x == ChooseAvailableCat ? "" : cats.Find(c => x.EndsWith(" · " + c.Id))?.Id ?? "");
            field.name = "choice-cat";
            return field;
        }

        public DropdownField ResourceChoice(VisualElement parent)
        {
            var field = UiKit.Choice(parent, "Resource", Catalog.Resources.Where(r => r != "blessings"), SelectedResource, x => SelectedResource = x);
            field.name = "choice-resource";
            return field;
        }

        public DoubleField AmountField(VisualElement parent, string label = "Amount")
        {
            var field = UiKit.Number(parent, label, Amount, x => Amount = x);
            field.name = "field-amount";
            return field;
        }
    }
}
