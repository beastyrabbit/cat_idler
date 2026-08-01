# Thread Q&A and Direct-Intent Audit

**Audited thread:** Orca/Codex rollout beginning 2026-07-22 14:52 UTC

**Audit authority:** user answers and direct messages through the final Plan 2 approval

**Stored plans:** [Plan 1](../leader-ai-overhaul/final-hole-hunting-content-plan.md) and
[Plan 2](../leader-ai-overhaul/final-integrated-overhaul-plan.md)

**Implementation boards:** [main board](../leader-ai-overhaul/BOARD.md),
[merge board](BOARD.md), and [Plan 2 board](bug-gui-design-BOARD.md)

## Purpose and authority

The final plan blocks are exact approved snapshots, but they are not the only source of intent.
The user's selected answers, free-form notes attached to answers, and direct messages also define
how the game should feel. An answer is not disposable background merely because a later plan
compresses it into one bullet.

This audit found 59 question rounds containing 139 prompts:

- 138 prompts have answers.
- One prompt was abandoned without an answer and immediately re-asked as a clearer retry; the
  retry is authoritative.
- Later answers supersede earlier answers only where this document records the change.
- A later answer may refine an earlier idea without deleting its motivation. For example, the
  Shrine becomes The Hole, but the endless progression pressure and believable good/bad Leader
  choices remain.

Implementation may not close from this document alone. Every retained answer must reach behavior,
report-safe protocol, persistence where relevant, player-visible UI or world presentation, and
acceptance evidence through its destination cards.

## Gaps found and repaired in the boards

| Gap | Thread intent that was at risk | Board repair |
|---|---|---|
| QAG.01 | The target feel is a state-of-the-art strategy-game brain inside an idle god-sim: a colony can be left for a month, continue expanding, and still fail for understandable leadership or world reasons. | LAI.69–70 now retain the 30-game-day fresh/established campaign targets and require growth/progression, not survival-only idling. |
| QAG.02 | The interface should be informative but not a generic full dashboard or a spammy event stream. The player should understand work, reasons, and blockers without omniscient truth. | LAI.54, LAI.66–69 now require quiet event aggregation, bounded repetition, drill-down detail, and the authored parchment/wood/pixel strategy-game language. |
| QAG.03 | Officer rooms/tools add effective expertise; specialist officers make “keep X in stock” policies possible and send typed needs such as workshop/space requests to the Leader. | LAI.55, LAI.63, and LAI.67 now explicitly retain institution bonuses, standing-order capability, and officer-to-Leader request flow. |
| QAG.04 | The four specialized Divine Boosts retain their researched duration/economy progression after Favor is removed; the later 15-minute Inspiration action is additive, not a replacement. | LAI.44, LAI.61, LAI.64, and LAI.67 now require the existing boost manifest, one-hour base duration, researched duration/cost curve, economy reductions, Void payment, and separate Inspiration semantics. |
| QAG.05 | Scholars were intended as later progression rather than a founding shortcut. | LAI.44 and LAI.58 now require manifest-owned Research Hut/scholar pacing and prove that the free Leader lane does not erase physical God-lane preparation. |
| QAG.06 | The two source branches contain valuable dirty code, tests, assets, layouts, and explanations even though their hot roots cannot be merged safely. | The [source-transfer manifest](source-transfer-manifest.md) freezes both inputs and LAI.35, LAI.53, and LAI.70 require a per-file semantic disposition and parity evidence. |

## Recorded later refinements and supersessions

| Topic | Earlier answer | Later authoritative resolution | What remains from the earlier intent |
|---|---|---|---|
| Shrine economy | Endless Shrine, Favor-funded weekly research and boosts | Full rename to The Hole; physical feeds produce Void Insight; ordinary work produces Research Notes; free Leader research is a separate lane | Endless progression pressure, visible physical supply work, good/bad offering choices, player boosts, and report-limited knowledge |
| Officer knowledge | Specialists were described as unlocking exact information | The explicit report ladder gives progressively tighter estimates; regeneration stays hidden through level 3 and becomes bounded at levels 4–5 | Higher expertise materially improves planning and reduces unnoticed shortages; neither God nor AI gets an oracle |
| Catalog count | An intermediate answer selected exactly 556 studies | Later answers require every material/content capability plus the branch graph, so the manifest total is derived rather than hard-coded | All content has one canonical unlock and nothing is dropped merely to preserve an obsolete number |
| Research control | Leader originally bought a weekly study from the same point economy | Leader research is free, instant, guaranteed, cadence-limited, finite-first, and independent from the physical player/God queue | A neglected or poorly directed player queue cannot permanently stall village progression |
| Research collision | Both lanes might choose the same study | Leader normally avoids active God work; emergency need or exact mistake bands may duplicate it; God currency is refunded but labor is lost | Leaders can make bounded believable mistakes without routinely wasting work |
| Trade consent | Early answer allowed autonomous trade for friendly/allied villages | Personal villages expose Alliance/Neutral/Enemy; Alliance and Neutral currently trade identically; Enemy excludes/rejects; global village is Neutral | Trade remains autonomous, physical, report-safe, and relationship-gated without implying defense or migration |
| Save handling | Early implementation had semantic migration work | Pre-production reset removes obsolete gameplay state; Plan 2 preserves only unrelated authentication/identity metadata required by the reset contract | No dual schema or compatibility gameplay path survives |
| Item catalog prompt | First all-items question received no answer | The immediate retry selected all existing and new items now | Stable-ID catalog coverage is universal |
| Divine interaction | Existing specialized boosts had researched durations | Inspiration was later added as a free +10%/15-minute per-player action | Inspiration and the four paid specialized boosts are distinct systems |

## Complete question-ID coverage

Each question ID appears exactly once below. The row records the latest meaning of the selected
answer and its user note, including later refinements.

### Original Leader-AI overhaul — 21 prompts

| Question IDs | Retained answer, motivation, and destination |
|---|---|
| `automation_authority`, `officer_progression`, `officer_appointment` | Start with a capable but inefficient Leader using rough reports. Specialists improve domain AI, reports, standing orders, and scaling; office rooms/tools add effective levels; officers request dependencies from the Leader; the Leader establishes posts and may appoint poorly. LAI.10, LAI.16, LAI.55, LAI.57, LAI.63, LAI.67. |
| `overhaul_capability`, `unattended_failure_model`, `survival_acceptance` | Full unattended progression for month-away idle play, with understandable world and leadership failure. More officers and expertise reduce avoidable collapse. Preserve at least 85% fresh and 97% established 30-game-day targets across deterministic seeds. LAI.32, LAI.63, LAI.69–70. |
| `decision_visibility`, `player_override_semantics` | Show a restrained work/event view with useful reasons and queue context, not a giant dashboard or spam stream. Player edits are temporary nudges; later standing orders provide explicit persistent policy. LAI.28, LAI.54, LAI.66–69. |
| `expertise_effect`, `leader_strategy_variation`, `trait_scope`, `trait_behavior_strength`, `trait_model` | Expertise improves information/control and lowers mistakes. Attributes are centered on the species baseline of 10 before inheritance/mutation; axes plus named traits affect every cat. Roughly 80% remain soft/ordinary while rare 95/5-style extremes create strong personalities and refusal without disabling self-preservation. Traits exist at birth and may be acquired; stress and life events matter. LAI.4–6, LAI.15–16, LAI.55, LAI.57, LAI.63, LAI.67. |
| `injury_depth`, `injury_recovery` | Compact individual-part anatomy, persistent loss, work/movement eligibility, prosthetics, and accommodation rather than full restoration. LAI.6–7, LAI.55, LAI.63, LAI.67. |
| `ai_rollout` | Direct single-path replacement, not shadow mode or optional dual governance; later clean-reset decisions strengthen this. LAI.23, LAI.52, LAI.63, LAI.70. |
| `auto_trade_scope` | Initially broad autonomous trade for friendly/allied villages; later stance answers refine this to Alliance/Neutral allowed and Enemy rejected, with physical barter only. LAI.22, LAI.62–64. |
| `shrine_demand_model`, `god_knowledge_view` | Preserve endless progression opportunities and the same report-safe knowledge for God and leadership. Later Hole/currency answers replace the Shrine name and economy, not the pressure or secrecy. LAI.41, LAI.45, LAI.47, LAI.61, LAI.63–67. |
| `research_gate`, `god_boost_duration` | Scholar physical work prepares/discounts studies and belongs later in progression. Retain four player boosts with a one-hour base, research-expanded duration/cost choices, and economy research; later pay with Void and keep Inspiration separate. LAI.44, LAI.58, LAI.61, LAI.64, LAI.67. |

### Plan 1 / `the-shrine-upgrade` integration — 34 prompts

| Question IDs | Retained answer, motivation, and destination |
|---|---|
| `shrine_model`, `branch_scope`, `save_migration`, `ai_ownership`, `divine_boost_currency` | Import every coherent Hole/Hunting/art idea, fully rename Shrine to Hole, use the new Leader AI as sole owner, reset obsolete pre-production state, and pay specialized boosts with Void Insight. LAI.35, LAI.41, LAI.45–52. |
| `preparation_model`, `catalog_model`, `all_material_unlock_scope`, `fresh_colony_unlock_bootstrap` | Scholar preparation is labor-only and grants a 25% Notes discount. Preserve all useful branch and new studies, but derive the final total after every material/content unlock. Logs, Stone, Water, Apples, hand-fishing, and basic handling bootstrap survival; Planks and later processing require research. LAI.36, LAI.38, LAI.43–44, LAI.51, LAI.58. |
| `species_values`, `research_tool_meaning`, `rare_material_crafting`, `augmentation_slots`, `material_processing_unlock` | Threat-scaled materials have multiple curated uses in cloth, weapons, furniture, tools, microscopes, fixtures, and one typed augmentation slot. Materials may be collected before unlock but not processed/installed until their canonical study. LAI.36–37, LAI.42–44, LAI.49–51. |
| `monster_scope`, `encounter_mix`, `drop_identity`, `rare_drop_quality`, `creature_roster` | Lock twenty creatures: ten normal and ten mystic across a transition mix, including dragons; use unique named drops plus shared Meat/Bone/Hide in sensible quantities; quality scales by encounter; registry remains extensible. LAI.42–43, LAI.49–51. |
| `apple_model`, `typed_food_model`, `fishing_workshop`, `founding_food_sources` | Remove aggregate Food. Guarantee physical Water, Apple, and fish sources; Apples regrow slowly; founding fishing is hard until Rod/Hut improvements; typed lot identity and quality extend beyond food. LAI.37–40, LAI.46–51. |
| `general_item_catalog_scope`, `general_item_catalog_scope_retry` | The first prompt was unanswered; its immediate retry requires the stable-ID catalog to cover every existing and new item now because there is no production compatibility constraint. LAI.36, LAI.43, LAI.47–52. |
| `cooking_station_model`, `recipe_unlock_formula`, `meal_quality_storage`, `universal_quality_scope` | Cooking uses a new 3×3 Cookhouse; capabilities unlock curated ingredient bundles, not per-recipe studies; raw-to-Feast complexity must monotonically improve hunger/value; every gathered and produced physical stock carries quality. LAI.37–39, LAI.44, LAI.46–51. |
| `visualization_pack_depth`, `creature_visual_presence`, `lair_level_visibility`, `hole_internal_identity`, `item_visual_composition` | Full written visual specification and extension guidance are mandatory. Creatures appear as portraits only when a Lair is inspected; ten Lair sprites show public ten-level bands while exact level is report-gated. Use only Hole identity. Item silhouette+material changes visually; quality/augmentation initially live in detail/badges with extension points. LAI.35, LAI.41–43, LAI.47, LAI.49–52. |

### Plan 2 / `bug-gui-design` integration — 84 prompts

| Question IDs | Retained answer, motivation, and destination |
|---|---|
| `research_integration`, `research_catalog_policy`, `repeatable_research`, `automatic_research_queue_authority`, `research_queue_funding`, `research_preparation_labor`, `leader_research_cost_model`, `research_lane_concurrency`, `research_duplicate_target_resolution`, `free_leader_research_cadence`, `leader_infinite_research_policy`, `physical_building_upgrade_authority` | One integrated graph keeps all useful branch and new capabilities plus all repeatables. God queue funding freezes at the front and uses physical work/preparation. The independent Leader lane is free, instant, cadence-limited, finite-first, and chooses permits while the Leader chooses the actual building/upgrade. Collisions refund currency only; labor loss remains. LAI.44, LAI.58–59, LAI.63–67, LAI.70. |
| `navigation_architecture`, `ui_language_policy`, `viewport_scope`, `council_tab_structure` | Ship English now, native desktop and WASM from 1024×768 through 4K, exactly five primary screens, and six Council tabs. LAI.54, LAI.66–70. |
| `world_reset_policy`, `manual_vs_ai_control`, `research_nudge_behavior`, `direct_action_exceptions`, `spatial_nudge_precision`, `god_kick_scope`, `god_officer_appointment_authority` | Test reset is signed/two-step and production-rejected. AI owns routine village decisions; God directly controls research/powers/consent, may expel only from an owned village, adds bounded election backing, and nudges domains but never exact locations. Leader appoints officers. LAI.57–65, LAI.67, LAI.70. |
| `divine_construction_aid`, `inspire_cats_design`, `free_click_scale`, `void_miracle_tier`, `global_aid_call`, `click_value_formula`, `construction_miracle_time`, `emergency_void_provision` | Preserve the intended “illusion of control”: ordinary clicks are deliberately tiny, use 100 Log clicks and canonical-value scaling, and create bound physical aid without replacing colony work. Inspiration stacks across different global players. One-Void repeatable miracles supply double feed-value construction inputs and remove 10% original time. Food/water rescue exists; real players may coordinate outside the game, but there is no in-game global-help mechanic. LAI.61, LAI.64–65, LAI.67, LAI.69–70. |
| `emergency_aid_form`, `divine_rescue_drop_location`, `divine_rescue_click_cost`, `divine_rescue_active_cap`, `double_population_divine_supply`, `leader_food_permission_states`, `god_food_policy_control`, `divine_supply_names`, `divine_click_input_policy` | Non-expiring Divine Ration/Water appears on the Hole apron, fully restores one need, is hauled urgently, defaults to Reserve, and has no stock cap. Ordinary meter creates one; one Void creates twice the living population. Food is Allowed/Reserve/Forbidden under Leader control; God nudges conservation only. Accept discrete input through batched/rate-limited wire actions. LAI.61, LAI.64–67, LAI.70. |
| `construction_phase_depth`, `scaffold_timber_rule`, `construction_phase_scope`, `construction_phase_time_split`, `construction_fitout_inputs` | All buildings and upgrades use scaffold/structure/fit-out at 20/60/20, with tiered Wood versus Lumber/Planks and catalog-owned physical finishing inputs. At least scaffold and partial structure require distinct authored sprites; the final plan also requires fit-out presentation. LAI.59–60, LAI.63–70. |
| `charisma_model`, `leadership_xp_sources`, `expanded_innate_stats`, `skill_taxonomy_depth`, `skill_progression_shape`, `ambient_cleaning_learning`, `trait_job_refusal_profile`, `refused_job_emergency_override` | Add innate/growing Charisma and Intelligence plus detailed learned work/office skills. All productive work grants its skill; offices cross-train related skills. Level caps at 100 while Mastery continues for legacy/teaching. Tiny trait-guided cleaning gains let an unskilled cat discover a path. Explicit affinities, anatomy, and Refused status gate work; Refused is never overridden. LAI.55–57, LAI.63–67, LAI.70. |
| `leader_ballot_model`, `global_god_vote`, `cat_ballot_scoring`, `relational_analytical_axis`, `election_candidate_slate`, `election_vote_formula` | Cats elect among the top five by civic merit. Charisma, Intelligence, Governance/Leadership, service, age-earned experience, and traits matter; the inherited Relational↔Analytical axis strongly changes voter weighting. Each eligible player contributes one replaceable +10 block. LAI.55, LAI.57, LAI.64, LAI.66–70. |
| `family_skill_transfer`, `mentorship_scope`, `post_cap_mastery_effect`, `family_profession_origin`, `family_business_attachment`, `family_trait_inheritance`, `occupational_surname_rule`, `tradition_uptake_variation`, `skill_based_work_matching` | Families become professional dynasties through small birth seeds, continued parent/mentor teaching, Mastery legacy, emergent repeated work, named enterprises, earned occupational surnames, aptitude/lore/tradition, and controlled variation. Urgency wins first; within it, enterprise/fit/skill/traits choose workers. LAI.55–56, LAI.63–70. |
| `paired_family_identity`, `household_housing_model`, `expulsion_family_behavior`, `cat_partnership_authority`, `elder_lodge_benefits`, `family_teaching_priority` | Partnerships are autonomous and compatibility-based; dual lineages remain visible. Housing progresses through Dens, Family Homes, and Elder Lodges. Expulsion can target an individual or household with dependent cleanup. Parents teach after three real tasks; mentors teach before cleaning. Elder Lodges add protection, mentoring, and a bounded longevity benefit. LAI.56–57, LAI.60, LAI.63–70. |
| `container_capacity_model`, `workshop_input_storage_location` | Containers have visible typed internal lots, not abstract capacity. Workshop inputs occupy a real adjacent stockpile zone outside the full 3×3 footprint. LAI.59–60, LAI.63–70. |
| `personal_trade_consent_mode`, `personal_diplomacy_states`, `bilateral_stance_resolution`, `diplomacy_stance_effects`, `global_village_stance`, `alliance_trade_only_meaning`, `incoming_enemy_trade_policy`, `distance_trade_profit_model` | Personal village radio states are Alliance/Neutral/Enemy. Alliance and Neutral currently mean the same trade permission; global is Neutral. Enemy destinations are excluded and incoming Enemy trade rejects before dispatch. No defense/migration yet. Trade is moneyless material barter; the Leader chooses possible-now versus better-later routes using offerings, need, value, distance, time, risk, capacity, and opportunity. LAI.62–70. |

## Direct user inputs outside the question widgets

| Direct input | Required interpretation | Destination |
|---|---|---|
| State-of-the-art Age-of-Empires/StarCraft-like AI | The planner must sequence goals and dependencies, not merely assign labor reactively. Its personality comes from cats/reports rather than omniscient scripted cheats. | LAI.15, LAI.45, LAI.63, LAI.69–70 |
| Gods must not see regeneration before officers can | Server-side report projection is shared by God and Leader; no client-only hiding. | LAI.47, LAI.63–67, LAI.70 |
| Endless resource pressure drives the game; good Leaders choose value, weak Leaders may waste food or forget | The Hole remains endlessly eligible, mistakes remain legal and visible, and resulting shortages create real recovery work. | LAI.41, LAI.45–46, LAI.61, LAI.63, LAI.70 |
| Write every explanation in detail; explanations outside AI are implementation requirements | Plans, boards, diagrams, UI, documentation, art, and examples are normative, not optional commentary. | LAI.35, LAI.51–53, LAI.68–70 |
| Only add to plans; never shrink them | Exact plan files remain immutable; later integration is additive and every supersession needs a recorded reason. | BPM.0–BPM.12 including BPM.1A/BPM.2A, LAI.35, LAI.53, LAI.70 |
| Hunt/Water tasks must be on real Lair/water tiles; Workshop is the whole 3×3 | No arbitrary/fallback markers; complete footprints, roles, route, cargo, and endpoints are authoritative. | LAI.46, LAI.60, LAI.63, LAI.68, LAI.70 |
| Document how to add any future workshop or other content | Contributor recipes cover data, behavior, authority, physicality, protocol, persistence, visuals, tests, and rollback. | LAI.51, LAI.69–70 |
| Use the browser for later acceptance | One-worker Playwright against real services is followed by an independently visible browser audit. | LAI.51–52, LAI.69–70 |
| Do not run many tests in parallel; add diagnostics if long tests are unclear | One heavy process at a time; quick focused checks only after bounded features; bounded diagnostics and heartbeats precede campaigns. | LAI.51–52, LAI.69–70 |
| Integrate `the-shrine-upgrade` design despite its ignorance of the new Leader AI | Import domain behavior, tests, art, and knowledge semantically; the new Leader/report authority remains the root. | LAI.35–52 and source-transfer manifest |
| Finalize and visualize Plan 1 before implementation | The exact Plan 1, visual inventory, wireframes, diagrams, assets, and board mapping are implementation gates. | LAI.35, LAI.49–52 |
| Integrate `bug-gui-design` and preserve full notes, not only selected choices | The complete answer notes and branch design/code inventory remain traceable through this audit and the Plan 2 board. | LAI.53–70 and source-transfer manifest |
| Add timed scaffold/material build phases | Physical three-stage construction applies to new buildings and upgrades with real cargo and authored states. | LAI.59–60, LAI.68–70 |
| End-game families become cooks and other specialized businesses | Multi-generation skill, teaching, enterprises, surnames, housing, and work matching create visible professional dynasties. | LAI.55–56, LAI.63, LAI.66–70 |
| Leader should avoid God research already in progress except urgent need or an “oopsie” | Two lanes share completion state; exact avoidance, emergency, mistake, refund, and lost-labor behavior is mandatory. | LAI.58, LAI.63–67, LAI.70 |
| Do not spend the project on repeated testing; integrate broadly, then run the heavy campaign | Workers use one quick focused check after a complete feature; the coordinator serializes final integration, campaign, browser, and visible-browser evidence. | LAI.51–52, LAI.69–70 |
| Concern about merging the two branches | A textual Git merge is deliberately rejected because both branches diverge across dirty hot roots. Functionality is transferred through the per-file semantic receipt process, not ignored. | BPM.1A, BPM.12, LAI.35, LAI.53, LAI.70 |

## Final acceptance rule

LAI.70 cannot be accepted until:

1. every grouped question row above has linked implementation and evidence;
2. every direct-input row has a maintained destination;
3. every recorded supersession is reflected consistently in simulation, protocol, persistence,
   UI, and documentation;
4. every source file in the source-transfer manifest has an explicit disposition;
5. the quiet strategy-game feel, month-away growth behavior, report-limited knowledge, physical
   task truth, family stories, and bounded God influence are demonstrated in real play rather than
   inferred from type definitions.
