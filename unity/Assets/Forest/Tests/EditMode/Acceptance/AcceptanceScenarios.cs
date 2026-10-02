// Which file owns which case-name prefix (every name below is under regression. unless shown otherwise):
//   DungeonScenarios.cs          dungeons_*
//   SpatialScenarios.cs          world_levels_*
//   WorldGenerationScenarios.cs  world_generation_*
//   SeamScenarios.cs             seam.*
//   CoreScenarios.cs             continuous_* except continuous_storage_*
//   NeedsScenarios.cs            cargo_critical_need_*, preserved_food_*, emergency_staffed_water, medicine_and_brew_*
//   PopulationScenarios.cs       review_housing_skips_disconnected_*, pregnancy_beds_*, death_preserves_*, extinction_*,
//                                unhoused_migrant_*
//   HaulingScenarios.cs          continuous_storage_*, busy_cat_*, item_only_stockpile_*, exact_haul_*, review_scalar_haul_*,
//                                review_armor_*
//   ConstructionScenarios.cs     layout_*, review_expansion_*, review_fishing_placement_*, buildable_founding_*,
//                                communal_and_personal_enclosures, expansion_interruption_*
//   ProductionScenarios.cs       recipe.*, chain.*, production_expertise_*, accountant_*, mountain_research_*,
//                                station_capacity_*, repeat_queue_*, pause_preserves_*, officer_vacancy_*, selected_quarry_*,
//                                farm_worker_replacement
//   EffectScenarios.cs           catalog.*, building_effect.*, service_effect.*, resource_effect.*, legacy_upgrade.*,
//                                legacy_effect.*, resource_capacity_is_resource_specific
//   ControlScenarios.cs          direct_control_*, review_scaffold_preserves_*, review_new_building_preserves_*,
//                                review_new_job_preserves_*, review_resume_*, gather_and_avoid_zones, resource_targeted_scout
//   TransportScenarios.cs        capability.*, rail_transfers_*, review_rail_*, review_transport_*, shipping_*
//   GovernanceScenarios.cs       territory_*
//   DefenseScenarios.cs          undefended_raid_*, functional_weapon_*, review_land_mover_*
//   CampaignScenarios.cs         campaign.* (explicit in Unity, excluded by the runner unless --include-campaigns)
//   ScenarioKit.cs               no cases: Scenario plus the shared fixtures and assertions
// A new file names its cases <file prefix in snake_case>.<case> (job_robustness.* in JobRobustnessScenarios.cs) and
// needs no registration: any static *Scenarios class with a public static Cases() is discovered below.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;

namespace IdleCatForest.Acceptance
{
    /// <summary>The single entry point the .NET runner and Unity's AcceptanceTests enumerate.</summary>
    public static class AcceptanceScenarios
    {
        // Files that existed before discovery, kept first and in their old order.
        static readonly Type[] Registered = { typeof(DungeonScenarios), typeof(SpatialScenarios), typeof(WorldGenerationScenarios), typeof(SeamScenarios) };

        public static IEnumerable<Scenario> Cases()
        {
            foreach (var source in Registered.Concat(Discovered()))
            {
                var cases = (Func<IEnumerable<Scenario>>)CasesMethod(source).CreateDelegate(typeof(Func<IEnumerable<Scenario>>));
                foreach (var scenario in cases())
                    yield return scenario;
            }
        }

        public static void Run(string name) => Cases().Single(c => c.Name == name).Run();

        static IEnumerable<Type> Discovered() =>
            typeof(AcceptanceScenarios).Assembly.GetTypes()
                .Where(t => t.IsClass && t.IsAbstract && t.IsSealed && t.Name.EndsWith("Scenarios", StringComparison.Ordinal))
                .Where(t => t != typeof(AcceptanceScenarios) && !Registered.Contains(t) && CasesMethod(t) != null)
                .OrderBy(t => t.Name, StringComparer.Ordinal)
                .ThenBy(t => t.FullName, StringComparer.Ordinal);

        static MethodInfo CasesMethod(Type type)
        {
            var method = type.GetMethod("Cases", BindingFlags.Public | BindingFlags.Static, null, Type.EmptyTypes, null);
            return method != null && method.ReturnType == typeof(IEnumerable<Scenario>) ? method : null;
        }
    }
}
