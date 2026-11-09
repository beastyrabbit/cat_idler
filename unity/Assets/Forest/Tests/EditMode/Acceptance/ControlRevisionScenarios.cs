using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ControlRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.second_control_needs_fractional_eat", () => FractionalServing("eat", "food"));
            yield return new Scenario("regression.second_control_needs_fractional_drink", () => FractionalServing("drink", "water"));
            yield return new Scenario("regression.second_control_needs_closed_den_entrance", ClosedDenEntrance);
            yield return new Scenario("regression.second_control_needs_fractional_rest", FractionalRest);
            yield return new Scenario("regression.second_control_needs_rest_rejects_avoid_zone", () => RestAvoidZone(false));
            yield return new Scenario("regression.second_control_needs_rest_rechecks_avoid_zone", () => RestAvoidZone(true));
            yield return new Scenario("regression.second_control_offering_requires_physical_reach", FractionalOffering);
        }

        private static World Fixture(out Village village, out Cat cat)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            foreach (var resident in village.Cats)
            {
                resident.ControlledBy = "fixture-held";
                resident.ControlLeaseUntil = 10000000;
            }
            cat = village.Cats[0];
            cat.ControlledBy = "";
            cat.Hunger = cat.Thirst = 20;
            cat.Rest = 50;
            Act(world, village, new GameAction { Kind = "EnterCatControl", CatId = cat.Id });
            return world;
        }

        private static PlayerContext Context(Village village) => new PlayerContext { PlayerId = "control-revision", VillageId = village.Id };
        private static ActionResult Need(World world, Village village, Cat cat, string mode) => world.Apply(Context(village), new GameAction { Kind = "InteractCat", CatId = cat.Id, Mode = mode });
        private static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(Context(village), action);
            Check(result.Success, action.Kind + " failed: " + result.Error);
        }
        private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        private static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 0.00001, message + " expected=" + expected + " actual=" + actual);
        private static void Place(World world, Cat cat, Int2 at)
        {
            cat.Position = at;
            cat.X = at.X;
            cat.Z = at.Z;
            cat.Y = world.WalkHeight(at);
            cat.HasHeight = true;
            cat.Path.Clear();
        }

        private static void FractionalServing(string mode, string resource)
        {
            var world = Fixture(out var village, out var cat);
            Place(world, cat, new Int2(village.Center.X + 1, village.Center.Z));
            cat.Cargo.Add(new Stack(resource, 2));
            Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(0, 1) });
            world.Step(0.05);
            Check(cat.Position.Equals(new Int2(village.Center.X + 1, village.Center.Z)) && cat.Z > village.Center.Z, "Fixture must be partway away from the shrine's dining reach");
            double hunger = cat.Hunger, thirst = cat.Thirst;
            var path = cat.Path.ToArray();
            Check(!Need(world, village, cat, mode).Success, "A snapped adjacent tile allowed a carried serving beyond physical dining reach");
            Near(World.Amount(cat.Cargo, resource), 2, "Rejected serving consumed finite cargo");
            Near(cat.Hunger, hunger, "Rejected serving restored hunger");
            Near(cat.Thirst, thirst, "Rejected serving restored thirst");
            Check(cat.Path.SequenceEqual(path), "Rejected serving changed the player's movement command");

            Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(-1, 0) });
            world.Step(2);
            Check(cat.Position.Equals(village.Center), "Cat must physically retrace and reach the dining destination");
            double before = mode == "eat" ? cat.Hunger : cat.Thirst;
            Check(Need(world, village, cat, mode).Success, "A serving remained unavailable after physically reaching dining");
            Near(World.Amount(cat.Cargo, resource), 1, "Accepted serving must consume exactly one finite unit");
            Near(mode == "eat" ? cat.Hunger : cat.Thirst, before + 50, "Accepted serving restored the wrong need amount");
        }

        private static void ClosedDenEntrance()
        {
            var world = Fixture(out var village, out var cat);
            var den = village.Buildings.Single(b => b.Id == cat.BedId);
            var outside = new[] { new Int2(den.Position.X - 1, den.Position.Z), new Int2(den.Position.X, den.Position.Z - 1) }
                .First(p => world.Walkable(village, p) && !world.Crossable(p, den.Position));
            Place(world, cat, outside);
            double rest = cat.Rest;
            Check(!Need(world, village, cat, "rest").Success, "The closed side of a Den allowed resting through its wall");
            Near(cat.Rest, rest, "Rejected rest changed the cat's need");
            Check(cat.Goal == "player_control", "Rejected rest changed direct-control intent");

            var path = world.Path(cat.Position, den.Position, village);
            Check(path != null && path.Contains(den.Entrance), "The reserved bed must have a real route through its entrance");
            foreach (var next in path)
            {
                Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(next.X - cat.Position.X, next.Z - cat.Position.Z) });
                world.Step(2);
                Check(cat.Position.Equals(next), "Manual movement did not reach the next legal Den route tile");
            }
            Check(Need(world, village, cat, "rest").Success, "The same reserved bed was unavailable after entering through its door");
            rest = cat.Rest;
            world.Step(1);
            Check(cat.Rest > rest && cat.Goal == "player_rest", "Physical arrival must permit normal reserved-bed recovery");
        }

        private static void FractionalOffering()
        {
            var world = Fixture(out var village, out var cat);
            Place(world, cat, new Int2(village.Center.X + 1, village.Center.Z));
            cat.Cargo.Add(new Stack("food", 5));
            Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(0, 1) });
            world.Step(0.05);
            Check(cat.Position.Equals(new Int2(village.Center.X + 1, village.Center.Z)) && cat.Z > village.Center.Z, "Fixture must be physically beyond the shrine's offering reach");
            double blessings = village.Blessings;
            var path = cat.Path.ToArray();
            var offering = new GameAction { Kind = "InteractCat", CatId = cat.Id };
            Check(!world.Apply(Context(village), offering).Success, "A snapped adjacent tile allowed an offering beyond physical shrine reach");
            Near(World.Amount(cat.Cargo, "food"), 5, "Rejected offering consumed finite goods");
            Near(village.Blessings, blessings, "Rejected offering credited blessings");
            Check(cat.Path.SequenceEqual(path), "Rejected offering changed the player's movement command");

            Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(-1, 0) });
            world.Step(2);
            Check(cat.Position.Equals(village.Center), "Cat must physically retrace and reach the shrine before offering");
            Check(world.Apply(Context(village), offering).Success, "The finite offering remained unavailable at the reached shrine");
            Near(World.Amount(cat.Cargo, "food"), 0, "Accepted offering must consume its finite food");
            Near(village.Blessings, blessings + 1, "Accepted offering credited the wrong blessings");
            world.Apply(Context(village), offering);
            Near(village.Blessings, blessings + 1, "Repeated empty offering credited the same goods twice");
        }

        private static void FractionalRest()
        {
            var world = Fixture(out var village, out var cat);
            var den = village.Buildings.Single(b => b.Id == cat.BedId);
            Place(world, cat, new Int2(den.Position.X + 1, den.Position.Z));
            Act(world, village, new GameAction { Kind = "MoveCat", CatId = cat.Id, Position = new Int2(0, 1) });
            world.Step(0.05);
            Check(cat.Position.Equals(new Int2(den.Position.X + 1, den.Position.Z)) && cat.Z > den.Position.Z, "Fixture must be moving along an interior edge beyond the reserved bed's reach");
            double rest = cat.Rest;
            var path = cat.Path.ToArray();
            Check(!Need(world, village, cat, "rest").Success, "An unfinished edge beyond bed reach allowed immediate rest");
            Near(cat.Rest, rest, "Rejected fractional rest changed the need");
            Check(cat.Path.SequenceEqual(path) && cat.Goal == "player_control", "Rejected fractional rest cancelled player movement");
        }

        private static void RestAvoidZone(bool alreadyResting)
        {
            var world = Fixture(out var village, out var cat);
            var den = village.Buildings.Single(b => b.Id == cat.BedId);
            Place(world, cat, den.Position);
            if (alreadyResting)
                Check(Need(world, village, cat, "rest").Success, "Accessible reserved bed must permit rest before designation");
            var zone = world.Apply(Context(village), new GameAction { Kind = "CreateZone", Resource = "avoid", Position = den.Position, End = den.Position });
            Check(zone.Success, "Public avoid designation must be permitted on the bed away from its entrance: " + zone.Error);
            Check(!world.Walkable(village, den.Position), "Avoid designation must block authoritative access");
            double rest = cat.Rest;
            if (alreadyResting)
            {
                world.Step(1);
                Check(cat.Rest <= rest && cat.Goal == "player_control", "Ongoing player rest ignored a newly forbidden bed");
            }
            else
            {
                Check(!Need(world, village, cat, "rest").Success, "A forbidden reserved bed still accepted manual rest");
                Near(cat.Rest, rest, "Rejected forbidden-bed rest changed the need");
            }
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = zone.EntityId });
            Check(Need(world, village, cat, "rest").Success, "Removing the same access block must allow rest without replacing the cat or bed");
            rest = cat.Rest;
            world.Step(1);
            Check(cat.Rest > rest, "Reopened bed must restore the same cat's rest");
        }
    }
}
