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
            yield return new Scenario("seam.random_int_uses_high_bits", RandomIntHighBits);
            yield return new Scenario("seam.action_result_codes", ActionResultCodes);
            yield return new Scenario("seam.event_seq_increases_and_continues", EventSeq);
            yield return new Scenario("seam.pacing_world_classic_values", PacingWorldClassic);
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
            Require(Pacing.Classic.Profile == "classic" && Pacing.Classic.DayScale == 1 && Pacing.Demo.DayScale == 60, "Core pacing defaults changed");
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
            // Demo worlds read DemoContent's copies (W2-10); demo_content.* checks their numbers.
            w.PacingProfile = Pacing.DemoProfile;
            foreach (var recipe in Catalog.Recipes)
                Require(w.RecipeOf(recipe.Id)?.Id == recipe.Id && !ReferenceEquals(w.RecipeOf(recipe.Id), recipe), "Demo RecipeOf does not return DemoContent's copy of " + recipe.Id);
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

        private static void RandomIntHighBits()
        {
            var a = World.Create(4242); var b = World.Create(4242);
            uint before = a.RandomState;
            int r = a.RandomInt(4);
            Require(r == (int)(((ulong)b.Random() * 4u) >> 32), "RandomInt does not use the high bits");
            Require(a.RandomState == unchecked(before * 1664525u + 1013904223u), "RandomInt must consume exactly one draw");
            var c = World.Create(4242);
            var seen = Enumerable.Range(0, 64).Select(_ => c.RandomInt(4)).ToArray();
            Require(Enumerable.Range(0, 4).All(d => seen.Contains(d)), "RandomInt(4) never reached every direction");
            bool threw = false; try { c.RandomInt(0); } catch (ArgumentOutOfRangeException) { threw = true; }
            Require(threw, "RandomInt(0) was accepted");
        }

        private static void ActionResultCodes()
        {
            Require(ActionResult.Ok("x").Code == "" && ActionResult.Fail("m").Code == "" && ActionResult.Fail("m").Args == null, "Legacy helpers must keep empty codes");
            var f = ActionResult.Fail(ResultCode.MissingMaterial, "msg", ("resource", "planks"));
            Require(!f.Success && f.Code == "missing_material" && f.Error == "msg" && f.Args["resource"] == "planks", "Coded failure lost its code or args");
            var o = ActionResult.Ok(ResultCode.Planned, "den-9");
            Require(o.Success && o.Code == "planned" && o.EntityId == "den-9" && o.Args == null, "Coded success lost its code or entity");
        }

        private static void EventSeq()
        {
            var w = World.Create(4242);
            var ctx = new PlayerContext { PlayerId = "p", VillageId = "communal" };
            var v = w.Villages[0]; v.ResearchPoints = 1e12;
            for (int i = 0; i < 3; i++)
            {
                var open = Catalog.Research.First(s => s.Prerequisites.All(q => v.Research.Contains(q)) && !v.Research.Contains(s.Id));
                w.Apply(ctx, new GameAction { Kind = "ResearchNode", NodeId = open.Id });
            }
            Require(v.Events.Select(e => e.Seq).SequenceEqual(new long[] { 1, 2, 3 }) && w.NextEventSeq == 4, "Events are not numbered 1..n");
            // An old save: no sequence fields at all, one partially numbered log.
            w.NextEventSeq = 0; foreach (var e in v.Events) e.Seq = 0; v.Events[0].Seq = 7;
            var next = Catalog.Research.First(s => s.Prerequisites.All(q => v.Research.Contains(q)) && !v.Research.Contains(s.Id));
            w.Apply(ctx, new GameAction { Kind = "ResearchNode", NodeId = next.Id });
            Require(v.Events.Last().Seq == 8 && w.NextEventSeq == 9, "Sequence did not continue above the highest seen");
            Require(SavedMembers(typeof(World)).Contains(nameof(World.NextEventSeq)) && SavedMembers(typeof(Event)).Contains(nameof(Event.Seq)), "Sequence fields are not saved");
        }

        private static void PacingWorldClassic()
        {
            var p = Pacing.Classic;
            Require(p.CatBaseSpeed == 1.6 && p.RoadSpeedFactor == 1.75 && p.DirtSpeedFactor == 1.05 && p.MountainSpeedFactor == 0.4 && p.ForestSpeedFactor == 0.85 && p.OpenGroundSpeedFactor == 1 && p.WadingSpeedFactor == .55 && p.PathWearToDirt == 50, "Classic world pacing changed");
        }
    }
}
