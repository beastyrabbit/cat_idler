using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// The attention model behind the seal and popover: one case per rule, plus purity, ordering, stable
    /// ids and the per-section counts the dock badges read.
    /// </summary>
    public static class AttentionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("attention.calm_colony_needs_nothing", CalmColony);
            yield return new Scenario("attention.founding_lists_useful_first_steps", FoundingFirstSteps);
            yield return new Scenario("attention.research_hut_without_scholar", ResearchHutWithoutScholar);
            yield return new Scenario("attention.idle_cats", IdleCats);
            yield return new Scenario("attention.study_affordable", StudyAffordable);
            yield return new Scenario("attention.workplace_blocked", WorkplaceBlocked);
            yield return new Scenario("attention.workplace_vacant", WorkplaceVacant);
            yield return new Scenario("attention.need_without_reachable_fix", NeedWithoutFix);
            yield return new Scenario("attention.cats_without_bed", CatsWithoutBed);
            yield return new Scenario("attention.raid_active", RaidActive);
            yield return new Scenario("attention.raid_warning", RaidWarning);
            yield return new Scenario("attention.explorer_in_danger", ExplorerInDanger);
            yield return new Scenario("attention.stale_counts", StaleCounts);
            yield return new Scenario("attention.order_and_section_counts", OrderAndCounts);
            yield return new Scenario("attention.pure_and_stable", PureAndStable);
        }

        /// <summary>The rule fixture with nothing to do: one cat held by its fixture building, no research to spend.</summary>
        private static World Calm(out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            v.ResearchPoints = 0;
            return w;
        }

        private static List<AttentionItem> Items(World w, Village v) => Attention.Compute(w, v).ToList();

        private static AttentionItem Only(World w, Village v, string rule)
        {
            var matches = Items(w, v).Where(i => i.Id == rule || i.Id.StartsWith(rule + ":", StringComparison.Ordinal)).ToList();
            Check(matches.Count == 1, "Expected one '" + rule + "' item, found " + matches.Count + " in [" + Describe(w, v) + "]");
            return matches[0];
        }

        private static bool Has(World w, Village v, string rule) =>
            Items(w, v).Any(i => i.Id == rule || i.Id.StartsWith(rule + ":", StringComparison.Ordinal));

        private static string Describe(World w, Village v) => string.Join(", ", Items(w, v).Select(i => i.Id + "/" + i.Severity));

        /// <summary>Releases <paramref name="count"/> fixture-held cats so they are free, idle adults.</summary>
        private static List<Cat> Free(Village v, int count)
        {
            var freed = v.Cats.Where(x => x.ControlledBy == "fixture-held").Take(count).ToList();
            foreach (var cat in freed)
            {
                cat.ControlledBy = "";
                cat.ControlLeaseUntil = 0;
                cat.BuildingId = "";
                cat.JobId = "";
            }
            Check(freed.Count == count, "The fixture has too few cats to free " + count);
            return freed;
        }

        /// <summary>Perform the item's own action exactly as the popover would.</summary>
        private static ActionResult Perform(World w, Village v, AttentionItem item)
        {
            var action = new GameAction { Kind = item.ActionKind, CatId = item.Arg(AttentionArg.Cat), BuildingId = item.Arg(AttentionArg.Building), NodeId = item.Arg(AttentionArg.Study), Name = item.Arg(AttentionArg.Job) };
            return Act(w, v, action);
        }

        private static void CalmColony()
        {
            var w = Calm(out var v, out _);
            Check(Items(w, v).Count == 0, "A calm colony still needs attention: " + Describe(w, v));
            Check(Attention.CountBySection(w, v).Count == 0, "A calm colony still has section badges");
        }

        private static void FoundingFirstSteps()
        {
            // A brand-new colony is the stranger's first screen: the seal must offer a few useful steps,
            // led by staffing the research hut, and nothing alarming.
            var w = World.Create(41);
            var v = w.Villages[0];
            var items = Items(w, v);
            Check(items.Count >= 2 && items.Count <= 4, "A new colony should list two to four first steps: " + Describe(w, v));
            Check(items.All(i => i.Severity != AttentionSeverity.Critical), "A new colony shows an alarm: " + Describe(w, v));
            Check(items[0].Id.StartsWith(AttentionRule.ResearchUnstaffed + ":", StringComparison.Ordinal), "Staffing the research hut is not the first step: " + Describe(w, v));
            Check(items.All(i => i.TitleCode.StartsWith("attention.", StringComparison.Ordinal) && i.Section != ""), "Items lack a title code or section");
        }

        private static void ResearchHutWithoutScholar()
        {
            var w = Calm(out var v, out _);
            var hut = Station(w, v, "research_hut");
            Check(!Has(w, v, AttentionRule.ResearchUnstaffed), "An unstaffed hut with no free cat offers nothing to do yet still showed");
            var free = Free(v, 2);
            free[1].Skills.Add(new Stack("research", 4));
            var item = Only(w, v, AttentionRule.ResearchUnstaffed);
            Check(item.Id == AttentionRule.ResearchUnstaffed + ":" + hut.Id && item.Anchor == hut.Id, "The item does not point at the hut");
            Check(item.Severity == AttentionSeverity.Warning && item.Section == AttentionSection.Work, "Unstaffed hut has the wrong severity or section");
            Check(item.ActionKind == "AssignWorker" && item.Arg(AttentionArg.Building) == hut.Id, "The hut item does not offer to assign a worker");
            Check(item.Arg(AttentionArg.Cat) == free[1].Id, "The offered scholar is not the free cat with the most research skill");
            Perform(w, v, item);
            Check(!Has(w, v, AttentionRule.ResearchUnstaffed), "Assigning the scholar did not clear the item");
        }

        private static void IdleCats()
        {
            var w = Calm(out var v, out _);
            Free(v, 2);
            Check(!Has(w, v, AttentionRule.IdleCats), "Two idle cats are normal and must not nag");
            var freed = Free(v, 3);
            var item = Only(w, v, AttentionRule.IdleCats);
            Check(item.Arg(AttentionArg.Count) == "5", "Idle count is " + item.Arg(AttentionArg.Count) + ", expected 5");
            Check(item.Severity == AttentionSeverity.Info && item.Section == AttentionSection.Work && item.ActionKind == AttentionAction.OpenSection, "Idle cats item has the wrong shape");
            Check(v.Cats.Any(x => x.Id == item.Anchor && x.JobId == "" && x.BuildingId == ""), "The idle item does not point at an idle cat");
            // Cats on a need trip, kittens and the controlled cat are not idle labour.
            foreach (var cat in freed)
                cat.Goal = GoalKey.NeedDrink;
            Check(!Has(w, v, AttentionRule.IdleCats), "Cats on need trips counted as idle");
        }

        private static void StudyAffordable()
        {
            var w = Calm(out var v, out _);
            var cheapest = Catalog.Research.Where(s => !v.Research.Contains(s.Id) && s.Prerequisites.All(v.Research.Contains)).OrderBy(s => s.Cost).First();
            v.ResearchPoints = cheapest.Cost - 0.01;
            Check(!Has(w, v, AttentionRule.StudyAffordable), "A study shows as affordable before the points are there");
            v.ResearchPoints = cheapest.Cost;
            var item = Only(w, v, AttentionRule.StudyAffordable);
            Check(item.ActionKind == "ResearchNode" && Catalog.Study(item.Arg(AttentionArg.Study)) != null, "The study item does not offer a study");
            Check(Catalog.Study(item.Arg(AttentionArg.Study)).Cost <= v.ResearchPoints, "The offered study is not affordable");
            Check(item.Section == AttentionSection.Research && item.Severity == AttentionSeverity.Notice, "Study item has the wrong section or severity");
            Perform(w, v, item);
            Check(v.Research.Contains(item.Arg(AttentionArg.Study)), "Performing the study item did not study it");
            Check(!Has(w, v, AttentionRule.StudyAffordable), "The item stayed after the points were spent");
        }

        private static void WorkplaceBlocked()
        {
            var w = Calm(out var v, out _);
            var station = Station(w, v, "woodworking");
            var worker = Free(v, 1)[0];
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = worker.Id, BuildingId = station.Id });
            Check(!Has(w, v, AttentionRule.WorkplaceBlocked), "A staffed workplace without a block reported one");
            station.Slots[0].BlockedReason = Blocked.MissingRecipeInput;
            var item = Only(w, v, AttentionRule.WorkplaceBlocked);
            Check(item.Id == AttentionRule.WorkplaceBlocked + ":" + station.Id && item.Anchor == station.Id, "Blocked item does not point at the workplace");
            Check(item.CauseCode == Blocked.MissingRecipeInput && item.Severity == AttentionSeverity.Warning, "Blocked item lost the cause code");
            station.Slots[0].BlockedReason = "";
            station.BlockedReason = Blocked.OutputStorageFullOrUnreachable;
            Check(Only(w, v, AttentionRule.WorkplaceBlocked).CauseCode == Blocked.OutputStorageFullOrUnreachable, "A building-level block was missed");
            station.Paused = true;
            Check(!Has(w, v, AttentionRule.WorkplaceBlocked), "A workplace the player paused nags as blocked");
        }

        private static void WorkplaceVacant()
        {
            var w = Calm(out var v, out _);
            var station = Station(w, v, "woodworking");
            Check(Catalog.Recipes.Any(r => r.Building == "woodworking" && Catalog.RecipeAvailable(v, r)), "Fixture assumption: woodworking has a founding recipe");
            Check(!Has(w, v, AttentionRule.WorkplaceVacant), "A vacant workplace with nobody free to staff it still nagged");
            var free = Free(v, 1);
            var item = Only(w, v, AttentionRule.WorkplaceVacant);
            Check(item.Anchor == station.Id && item.ActionKind == "AssignWorker" && item.Arg(AttentionArg.Cat) == free[0].Id, "Vacant item does not offer the free cat for the workplace");
            Check(item.Severity == AttentionSeverity.Info && item.Arg(AttentionArg.Count) == "1", "Vacant item has the wrong severity or count");
            // A building with no available work (all recipes locked) is not a vacancy worth staffing.
            v.Buildings.Remove(station);
            string locked = Catalog.Buildings.First(k => Catalog.Recipes.Any(r => r.Building == k) && !Catalog.Recipes.Any(r => r.Building == k && Catalog.RecipeAvailable(v, r)));
            Station(w, v, locked);
            Check(!Has(w, v, AttentionRule.WorkplaceVacant), "A " + locked + " with every recipe locked was shown as a vacancy");
        }

        private static void NeedWithoutFix()
        {
            var w = Calm(out var v, out var c);
            c.Thirst = 20;
            Check(!Has(w, v, AttentionRule.NeedThirst), "A thirsty cat with water in storage needs no attention");
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water");
            var item = Only(w, v, AttentionRule.NeedThirst);
            Check(item.Severity == AttentionSeverity.Critical && item.Anchor == c.Id && item.Arg(AttentionArg.Count) == "1", "Thirst item has the wrong shape");
            Check(item.ActionKind == "RequestJob" && item.Arg(AttentionArg.Job) == JobKind.Water, "Thirst item does not offer a water run");
            c.Thirst = 31;
            Check(!Has(w, v, AttentionRule.NeedThirst), "A need at 31 is not under 30");
            c.Hunger = 10;
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "food");
            var hunger = Only(w, v, AttentionRule.NeedHunger);
            Check(hunger.Arg(AttentionArg.Job) == JobKind.Hunt, "Hunger item does not offer a hunt");
            c.Hunger = 100;
            // A supply that exists but cannot be reached is no fix either.
            World.Add(v.Stockpiles[0].Goods, "water", 50);
            c.Thirst = 5;
            c.BlockedReason = Blocked.BlockedRoute;
            Check(Only(w, v, AttentionRule.NeedThirst).CauseCode == Blocked.BlockedRoute, "An unreachable supply was treated as a fix");
            c.Alive = false;
            Check(!Has(w, v, AttentionRule.NeedThirst), "A dead cat still needs water");
        }

        private static void CatsWithoutBed()
        {
            var w = Calm(out var v, out var c);
            c.BedId = "";
            var item = Only(w, v, AttentionRule.NoBed);
            Check(item.Severity == AttentionSeverity.Warning && item.Anchor == c.Id && item.Arg(AttentionArg.Count) == "1", "No-bed item has the wrong shape");
            Check(item.Section == AttentionSection.Build && item.ActionKind == "RequestJob" && item.Arg(AttentionArg.Job) == JobKind.BuildHouse, "No-bed item does not offer to plan a den");
            var site = new Building { Id = w.Id("den"), Kind = "den", Position = new Int2(6, 6) };
            v.Buildings.Add(site);
            var building = Only(w, v, AttentionRule.NoBed);
            Check(building.Severity == AttentionSeverity.Info && building.Anchor == site.Id && building.ActionKind == "", "A den under construction should calm the no-bed item and point at the site");
            c.Migration = "arriving";
            c.BedId = "";
            v.Buildings.Remove(site);
            Check(!Has(w, v, AttentionRule.NoBed), "An arriving wanderer counted as homeless before it joined");
        }

        private static void RaidActive()
        {
            var w = Calm(out var v, out _);
            var raid = new Raid { Id = w.Id("raid"), Position = new Int2(0, 5) };
            v.Raids.Add(raid);
            var items = Items(w, v);
            Check(items[0].Id == AttentionRule.Raid + ":" + raid.Id && items[0].Severity == AttentionSeverity.Critical, "An active raid is not the first, critical item: " + Describe(w, v));
            Check(items[0].ActionKind == "DefendRaid" && items[0].Section == AttentionSection.Defense && items[0].Anchor == raid.Id, "Raid item has the wrong action, section or anchor");
            v.Raids.Clear();
            Check(!Has(w, v, AttentionRule.Raid), "A finished raid still shows");
        }

        private static void RaidWarning()
        {
            var w = Calm(out var v, out _);
            w.TimeSeconds = 1000;
            v.Events.Add(new Event { Time = 990, Kind = EventKind.RaidWarning, Text = "Raiders spotted", Seq = 1 });
            var item = Only(w, v, AttentionRule.RaidWarning);
            Check(item.Severity == AttentionSeverity.Warning && item.Section == AttentionSection.Defense, "Raid warning has the wrong severity or section");
            v.Raids.Add(new Raid { Id = w.Id("raid") });
            Check(!Has(w, v, AttentionRule.RaidWarning) && Has(w, v, AttentionRule.Raid), "The warning should give way to the raid itself");
            v.Raids.Clear();
            v.Events.Add(new Event { Time = 995, Kind = EventKind.RaidRepelled, Text = "Raid repelled", Seq = 2 });
            Check(!Has(w, v, AttentionRule.RaidWarning), "A repelled raid still shows its warning");
            v.Events.RemoveAt(v.Events.Count - 1);
            w.TimeSeconds = 990 + w.CurrentPacing().RaidWarningSeconds + 61;
            Check(!Has(w, v, AttentionRule.RaidWarning), "A stale warning never expires");
        }

        private static void ExplorerInDanger()
        {
            var w = Calm(out var v, out var c);
            c.DungeonId = "fixture-dungeon";
            c.DungeonPhase = "exploring";
            Check(!Has(w, v, AttentionRule.ExplorerDanger), "A healthy explorer is not in danger");
            c.Health = 45;
            var warning = Only(w, v, AttentionRule.ExplorerDanger);
            Check(warning.Severity == AttentionSeverity.Warning && warning.Anchor == c.Id && warning.Id == AttentionRule.ExplorerDanger + ":" + c.Id, "A hurt explorer is not a warning on that cat");
            Check(warning.ActionKind == "RecallExplorer" && warning.Arg(AttentionArg.Cat) == c.Id && warning.Section == AttentionSection.World, "Explorer item does not offer a recall");
            c.Health = 30;
            Check(Only(w, v, AttentionRule.ExplorerDanger).Severity == AttentionSeverity.Critical, "A badly hurt explorer is not critical");
            c.DungeonPhase = "returning";
            Check(Only(w, v, AttentionRule.ExplorerDanger).ActionKind == "", "A recall was offered to an explorer already heading home");
            c.Health = 90;
            c.Thirst = 20;
            Check(Has(w, v, AttentionRule.ExplorerDanger), "A parched explorer is in danger too");
        }

        private static void StaleCounts()
        {
            var w = Calm(out var v, out _);
            Check(!Has(w, v, AttentionRule.StaleCounts), "Uncounted stores without an accounting tent offer nothing to do");
            var tent = Station(w, v, "accounting_tent");
            var free = Free(v, 1);
            var item = Only(w, v, AttentionRule.StaleCounts);
            Check(item.Anchor == tent.Id && item.ActionKind == "AssignWorker" && item.Arg(AttentionArg.Cat) == free[0].Id, "Stale counts do not offer a counter for the tent");
            Check(item.Section == AttentionSection.Stores && item.Severity == AttentionSeverity.Info, "Stale counts item has the wrong shape");
            w.TimeSeconds = 100000;
            v.Stockpiles[0].CountedAt = w.TimeSeconds - 60;
            Check(!Has(w, v, AttentionRule.StaleCounts), "A fresh count is not stale");
            v.Stockpiles[0].CountedAt = w.TimeSeconds - 5 * 3600;
            Check(Has(w, v, AttentionRule.StaleCounts), "A five-hour-old count is stale");
        }

        private static void OrderAndCounts()
        {
            var w = Calm(out var v, out var c);
            Free(v, 4);
            c.BedId = "";
            v.ResearchPoints = 1000000;
            v.Raids.Add(new Raid { Id = w.Id("raid") });
            var items = Items(w, v);
            for (int i = 1; i < items.Count; i++)
                Check(items[i - 1].Severity >= items[i].Severity, "Items are not ordered by severity: " + Describe(w, v));
            Check(items.Select(i => i.Id).Distinct().Count() == items.Count, "Duplicate item ids: " + Describe(w, v));
            var counts = Attention.CountBySection(w, v);
            Check(counts.Values.Sum() == items.Count, "Section counts do not add up to the items");
            foreach (var group in items.GroupBy(i => i.Section))
                Check(counts.TryGetValue(group.Key, out int n) && n == group.Count(), "Section " + group.Key + " count is wrong");
        }

        private static void PureAndStable()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            v.Cats[3].Thirst = 10;
            v.Stockpiles.ForEach(p => p.Goods.RemoveAll(s => s.Resource == "water"));
            string before = Fingerprint(w);
            var first = Attention.Compute(w, v).Select(i => i.Id + "|" + i.Severity + "|" + i.Anchor + "|" + string.Join(",", i.Args)).ToList();
            var second = Attention.Compute(w, v).Select(i => i.Id + "|" + i.Severity + "|" + i.Anchor + "|" + string.Join(",", i.Args)).ToList();
            Check(Fingerprint(w) == before, "Computing attention changed the world");
            Check(first.SequenceEqual(second), "Two computations of the same world differ");
            Check(Attention.Compute(null, v).Count == 0 && Attention.Compute(w, null).Count == 0, "Missing world or village must give no items");
        }
    }
}
