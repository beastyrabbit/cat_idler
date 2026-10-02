using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    /// <summary>Cross-cutting seams that later waves compile against; they must not change behaviour.</summary>
    public static class SeamScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("seam.save_contract_pacing_profile", SaveContract);
            yield return new Scenario("seam.save_without_pacing_profile_loads_classic", SaveWithoutProfile);
            yield return new Scenario("seam.pacing_for_demo", PacingForDemo);
            yield return new Scenario("seam.pacing_for_unknown_is_classic", PacingForUnknown);
            yield return new Scenario("seam.recipe_of_matches_catalog", RecipeOfMatchesCatalog);
            yield return new Scenario("seam.gain_skill_adds_like_add", GainSkillAdds);
        }

        private static void Require(bool value, string message)
        {
            if (!value) throw new InvalidOperationException(message);
        }

        // Saves are Newtonsoft JSON with the default contract: public instance fields and public
        // readable properties. The simulation assemblies cannot reference the serializer, so this
        // checks that contract on World directly; the server tests round-trip the real WireJson.
        private static string[] SavedMembers(Type type) =>
            type.GetFields(BindingFlags.Public | BindingFlags.Instance).Where(f => !Attribute.IsDefined(f, typeof(NonSerializedAttribute))).Select(f => f.Name)
                .Concat(type.GetProperties(BindingFlags.Public | BindingFlags.Instance).Where(p => p.GetGetMethod() != null && p.GetIndexParameters().Length == 0).Select(p => p.Name))
                .ToArray();

        private static void SaveContract()
        {
            var saved = SavedMembers(typeof(World));
            Require(saved.Contains(nameof(World.PacingProfile)), "Save JSON lacks PacingProfile");
            Require(!saved.Contains("Pace"), "Derived Pace leaked into save JSON");
            Require(typeof(World).GetField(nameof(World.PacingProfile)).FieldType == typeof(string), "PacingProfile is not saved as a plain string");
            var leaked = typeof(World).GetFields(BindingFlags.Public | BindingFlags.Instance).Where(f => f.FieldType == typeof(Pacing)).Select(f => f.Name)
                .Concat(typeof(World).GetProperties(BindingFlags.Public | BindingFlags.Instance).Where(p => p.PropertyType == typeof(Pacing)).Select(p => p.Name)).ToArray();
            Require(leaked.Length == 0, "Pacing tables would be written into saves: " + string.Join(",", leaked));

            var created = World.Create(4242);
            Require(created.PacingProfile == Pacing.ClassicProfile && created.CurrentPacing() == Pacing.Classic, "New worlds are not classic by default");
            // Loading copies the saved string back; the profile object is derived, never stored.
            foreach (var profile in new[] { Pacing.ClassicProfile, Pacing.DemoProfile })
            {
                var loaded = new World { PacingProfile = new string(profile.ToCharArray()) };
                Require(loaded.PacingProfile == profile && loaded.CurrentPacing() == Pacing.For(profile), "Reloaded world lost pacing profile " + profile);
            }
        }

        private static void SaveWithoutProfile()
        {
            // A save written before the field existed populates nothing, so the initialiser decides.
            var loaded = new World();
            Require(loaded.PacingProfile == "classic", "Old save did not default to the classic profile");
            Require(loaded.CurrentPacing() == Pacing.Classic, "Old save does not pace as classic");
        }

        private static void PacingForDemo()
        {
            Require(Pacing.For("demo").Profile == "demo", "Demo profile lookup returned " + Pacing.For("demo").Profile);
            Require(Pacing.For("demo") == Pacing.Demo && Pacing.Demo != Pacing.Classic, "Demo profile is not its own table");
            Require(Pacing.Classic.Profile == "classic" && Pacing.Classic.DayScale == 1 && Pacing.Demo.DayScale == 1, "Core pacing defaults changed");
        }

        private static void PacingForUnknown()
        {
            foreach (var profile in new[] { "x", "classic", "", null, "Demo", " demo" })
                Require(Pacing.For(profile) == Pacing.Classic, "Unknown profile '" + profile + "' is not classic");
        }

        private static void RecipeOfMatchesCatalog()
        {
            var w = World.Create(4242);
            Require(Catalog.Recipes.Count > 0, "Catalog has no recipes");
            foreach (var recipe in Catalog.Recipes)
                Require(ReferenceEquals(w.RecipeOf(recipe.Id), Catalog.Recipe(recipe.Id)), "RecipeOf differs from the catalog for " + recipe.Id);
            Require(w.RecipeOf("no_such_recipe") == null, "RecipeOf invented an unknown recipe");
            w.PacingProfile = Pacing.DemoProfile;
            foreach (var recipe in Catalog.Recipes)
                Require(ReferenceEquals(w.RecipeOf(recipe.Id), Catalog.Recipe(recipe.Id)), "RecipeOf overrides " + recipe.Id + " before demo content exists");
        }

        private static void GainSkillAdds()
        {
            var w = World.Create(4242);
            var cat = w.Villages[0].Cats[0];
            var expected = new List<Stack>(cat.Skills.Select(s => new Stack(s.Resource, s.Amount)));
            w.GainSkill(cat, "hunting", 0.25);
            World.Add(expected, "hunting", 0.25);
            w.GainSkill(cat, "hunting", 1.5);
            World.Add(expected, "hunting", 1.5);
            Require(cat.Skills.Count == expected.Count && cat.Skills.Zip(expected, (a, b) => a.Resource == b.Resource && a.Amount == b.Amount).All(same => same), "GainSkill differs from Add");
            Require(World.Amount(cat.Skills, "hunting") == World.Amount(expected, "hunting"), "GainSkill lost skill amount");
        }
    }
}
