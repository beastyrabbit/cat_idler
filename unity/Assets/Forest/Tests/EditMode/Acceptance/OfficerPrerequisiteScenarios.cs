using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class OfficerPrerequisiteScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.officer_prerequisite_names_missing_study_and_building", () => Prerequisites(false, false));
            yield return new Scenario("regression.officer_prerequisite_names_only_missing_building", () => Prerequisites(true, false));
            yield return new Scenario("regression.officer_prerequisite_names_only_missing_study", () => Prerequisites(false, true));
            yield return new Scenario("regression.officer_prerequisite_completed_requirements_allow_appointment", () => Prerequisites(true, true));
        }

        private static void Prerequisites(bool study, bool completed)
        {
            var world = World.Create(41);
            var village = world.Villages[0];
            var cat = village.Cats.First(c => c.Alive);
            village.Research.RemoveAll(id => id == "sawmill");
            village.Buildings.RemoveAll(b => b.Kind == "sawmill");
            if (study) village.Research.Add("sawmill");
            village.Buildings.Add(new Building { Id = world.Id("office-prerequisite"), Kind = "sawmill", Position = new Int2(3, 3), Completed = completed });
            var result = world.Apply(new PlayerContext { PlayerId = "officer-prerequisite-review", VillageId = village.Id }, new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = cat.Id });
            if (result.Success != (study && completed)) throw new InvalidOperationException("Appointment must require both the study and a completed building.");
            if (result.Success)
            {
                if (village.Officers.Single(o => o.Role == "forester").CatId != cat.Id) throw new InvalidOperationException("Appointment changed the selected cat.");
                return;
            }
            if (result.Error.Contains("research study Sawmill", StringComparison.Ordinal) != !study || result.Error.Contains("completed building sawmill", StringComparison.Ordinal) != !completed)
                throw new InvalidOperationException("Refusal must name only missing requirements and distinguish research study from completed building: " + result.Error);
            if (village.Officers.Any(o => o.Role == "forester")) throw new InvalidOperationException("Refused appointment changed the office.");
        }
    }
}
