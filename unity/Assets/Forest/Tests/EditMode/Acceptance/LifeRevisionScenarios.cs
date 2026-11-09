using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class LifeRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.life_reserved_beds_migrant_after_one_birth", () => ReservedBeds(1));
            yield return new Scenario("regression.life_reserved_beds_migrant_after_four_births", () => ReservedBeds(4));
            yield return new Scenario("regression.life_hourly_birth_fills_last_reserved_bed", HourlyBirthCapacity);
        }

        private static void Require(bool condition, string message)
        {
            if (!condition)
                throw new InvalidOperationException(message);
        }

        private static World Fixture(double at, out Village v)
        {
            var w = World.Create(41);
            v = w.Villages[0];
            w.TimeSeconds = at - World.SimulationStepSeconds;
            v.LastMigration = at;
            v.NextElection = at + 1000000;
            v.Trader.NextAt = at + 1000000;
            foreach (var cat in v.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = at + 1000000;
            }
            foreach (var pile in v.Stockpiles)
                pile.Goods.Clear();
            World.Add(v.Stockpiles[0].Goods, "food", 1000);
            World.Add(v.Stockpiles[0].Goods, "water", 1000);
            return w;
        }

        private static Building SpareDen(World w, Village v)
        {
            var den = new Building { Id = w.Id("den"), Kind = "den", Completed = true, Position = new Int2(-4, 1) };
            v.Buildings.Add(den);
            return den;
        }

        private static void CheckBeds(World w, Village v)
        {
            int capacity = v.Buildings.Count(b => b.Kind == "den" && b.Completed) * w.DenCapacity(v);
            int occupants = v.Cats.Count(c => c.Alive && c.BedId != "");
            int reservations = v.Cats.Count(c => c.Alive && c.PregnantUntil > 0);
            Require(occupants + reservations <= capacity, "Births and remaining pregnancies overbooked permanent beds");
            Require(w.Validate().Count == 0, string.Join("; ", w.Validate()));
        }

        private static void ReservedBeds(int birthsBeforeMigrant)
        {
            var w = Fixture(60, out var v);
            SpareDen(w, v);
            var mothers = v.Cats.Take(5).ToArray();
            foreach (var mother in mothers)
                mother.PregnantUntil = 59;
            var migrant = new Cat
            {
                Id = w.Id("arrival"),
                Name = "Probationer",
                VillageId = v.Id,
                Position = v.Center,
                Migration = "probationary",
                ProbationUntil = 100000,
                ControlledBy = "fixture-held",
                ControlLeaseUntil = 1000000
            };
            // Allocation must be independent of where an arrival appears among due mothers.
            v.Cats.Insert(birthsBeforeMigrant, migrant);
            CheckBeds(w, v);

            w.Step(World.SimulationStepSeconds);
            Require(v.Cats.Count(c => c.Alive && c.AgeHours < 1) == 5, "A migrant took a reserved bed and postponed a due birth");
            Require(mothers.All(c => c.PregnantUntil < 0), "A due mother retained an already funded pregnancy");
            Require(migrant.BedId == "" && migrant.Migration == "probationary", "A migrant consumed a bed claimed by a future kitten");
            CheckBeds(w, v);

            // A genuinely released permanent bed should still admit the same waiting migrant.
            v.Cats.First(c => c.AgeHours >= 24 && !mothers.Contains(c) && c != migrant).Health = 0;
            w.TimeSeconds = 120 - World.SimulationStepSeconds;
            w.SimulationTimeSeconds = w.TimeSeconds;
            w.Step(World.SimulationStepSeconds);
            Require(migrant.Alive && migrant.BedId != "" && migrant.Migration == "resident" && migrant.ProbationUntil < 0, "A waiting migrant did not receive the next genuinely free bed");
            Require(v.Cats.Count(c => c.Alive && c.AgeHours < 1) == 5, "Later migrant admission changed the five kitten identities");
            CheckBeds(w, v);
        }

        private static void HourlyBirthCapacity()
        {
            const double at = 36 * 3600;
            var w = Fixture(at, out var v);
            var den = SpareDen(w, v);
            for (int i = 0; i < 4; i++)
                v.Cats.Add(new Cat
                {
                    Id = w.Id("resident"),
                    Name = "Resident",
                    VillageId = v.Id,
                    Position = v.Center,
                    BedId = den.Id,
                    ControlledBy = "fixture-held",
                    ControlLeaseUntil = at + 1000000
                });
            v.Cats[0].PregnantUntil = at - 1;
            // Seed a successful conception opportunity after the newborn's stats/inheritance rolls.
            w.RandomState = 3802;
            CheckBeds(w, v);

            w.Step(World.SimulationStepSeconds);
            Require(v.Cats.Count(c => c.Alive && c.AgeHours < 1) == 1, "The final reserved bed did not receive its kitten");
            Require(v.Cats.All(c => c.PregnantUntil < 0), "Hourly breeding reserved a bed already occupied by the new kitten");
            CheckBeds(w, v);
        }
    }
}
