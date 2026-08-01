# Cats, workforce, injuries, and prosthetics

`cat-sim` owns all cat identity and capability state. Protocol/UI receive report-safe summaries and
breakdowns, never a second capability calculation. All traits, incidents, fitting, repair, and wear
are deterministic and persistent.

## Innate attributes and inheritance

Every cat has eight innate attributes on a 1–20 scale: Attack, Defense, Hunting, Medicine,
Cleaning, Building, Leadership, and Vision. Newly generated adults center on 10. Existing 0–100
save values migrate exactly once with:

`new_attribute = clamp(round(1 + old_attribute × 19 / 100), 1, 20)`

Newborn inheritance uses the rounded parental midpoint plus a deterministic mutation from −2
through +2, then clamps to 1–20. A missing parent contributes the species baseline of 10. Learned
labor skills and per-office experience remain separate from innate attributes.

## Personality

The eight continuous axes are:

- Cautious ↔ Bold
- Leisurely ↔ Diligent
- Traditional ↔ Curious
- Content ↔ Ambitious
- Self-reliant ↔ Communal
- Solitary ↔ Gregarious
- Self-sufficient ↔ Mercantile
- Skeptical ↔ Devout

The deterministic population distribution is 80% subtle, 15% pronounced, and 5% extreme.
Magnitudes change relevant weights by 5%, 15%, and 30% respectively. Axes affect only their
documented concerns:

- Cautious/Bold changes threat and injury cost.
- Leisurely/Diligent changes long-shift stress and refusal.
- Traditional/Curious changes unfamiliar-technology preference.
- Content/Ambitious changes expansion pressure.
- Self-reliant/Communal changes independent/shared-project affinity.
- Solitary/Gregarious changes quiet/social rest and team affinity.
- Self-sufficient/Mercantile changes import and fair-trade preference.
- Skeptical/Devout changes willingness to prioritize optional Hole dependencies, divine-aid work,
  and report-safe progression proposals after survival/defense/committed work. It never changes
  Hole value, food permissions, hidden knowledge, or player-only authority.

The same model influences Leaders, officers, workers, barter, Hole choices, and research. It
changes scores and willingness, not knowledge, physical eligibility, or authoritative truth.

## Stress, willingness, and refusal

Stress persists from 0 through 100:

| Stress | Behavior |
|---:|---|
| 0–59 | Normal willingness |
| 60–79 | Reduced suitability for stressful or risky work |
| 80–94 | May deterministically refuse non-emergency work and must be rematched |
| 95–100 | Stops optional work and seeks rest, safety, or treatment |

Exact changes are:

- +1 per two hours worked beyond eight hours in a rolling game-day.
- +10 for a minor injury.
- +25 for a severe injury.
- +35 for a missing-part incident.
- +15 for raid defeat.
- −2 per safe resting hour.
- −1 additional per compatible social-rest hour for Gregarious cats.

Self-preservation—hunger, thirst, sleep, evacuation, and immediate treatment—always overrides
refusal. Leaders and officers are not immune. Pregnant or injured cats reject high-risk work when a
safer eligible worker exists.

For stress 80–94, non-emergency refusal uses an exact base probability of
`(stress − 60) × 100` basis points: 2,000 at stress 80 through 3,400 at stress 94. The comparison
uses an explicit deterministic 0–9,999 bucket keyed by world seed, colony, cat, task, and assignment
occurrence, so input order, retry batching, and unrelated RNG draws cannot change the choice.
Personality, acquired traits, pregnancy/injury risk, and task emergency status remain separate
fixed-point willingness inputs; they do not reveal hidden truth or silently change this base band.
Stress 95–100 always refuses optional work, while self-preservation still overrides refusal.

A refusing carrier first deposits cargo at its pinned endpoint or nearest safe owned stockpile. A
station worker completes an atomic recipe step whose inputs were already consumed, then leaves. If
no willing eligible cat exists, the intent becomes `Blocked(NoWillingWorker)`; no invisible forced
assignment, lost cargo, repeated input consumption, or permanently busy cat is allowed.

## Acquired traits

- `Traumatized`: severe/missing injury or raid defeat; +25% combat-risk stress.
- `Battle-Hardened`: five combat deployments without a severe outcome; −25% combat stress and
  replaces `Traumatized`.
- `Caregiver`: 100 completed treatment hours; +10% medicine effectiveness.
- `Burned Out`: stress at least 90 for 24 hours; −25% non-emergency willingness until 72 hours
  continuously below 40.
- `Prosthetic-Adapted`: 72 productive hours with a prosthetic; +10 percentage points restoration,
  still under the 90% cap.

Opposed traits are mutually exclusive. Temporary grief, fear, pain, exhaustion, hunger, and thirst
remain conditions rather than rewriting identity.

The former Shrine-derived `Devoted` and scholar-Insight-derived `Seasoned Scholar` traits are
retired with their authorities and receive no automatic replacement or semantic save conversion.
Future Hole/research traits must be added through the stable-ID/acquired-trait extension recipe with
declared physical completion receipts and must not grant currency, report clearance, or player-only
authority.

Capability modifiers apply in this exact order: anatomy, prosthetic restoration, innate attribute,
personality, acquired trait, stress, then active divine boost. Equipment, labor skill, office
experience, and operational building/tool support feed the task-fit calculation at their documented
layer; the UI must show the breakdown rather than one unexplained total.

## Colony-wide workforce matching

Every ready approved task contributes explicit slots to one deterministic maximum-weight bipartite
matching/min-cost-flow pass. Candidate fit includes innate/learned suitability, office expertise,
travel time, anatomy/prosthetic capability, stress/refusal, personality, continuity/churn, urgency,
strategic importance, team/slot capacity, and reserved tools/equipment.

Scores are fixed-point integers. Equal scores resolve by task ID, then cat ID, independent of input
collection order. Recompute on planning cadence, emergency, death, refusal, injury/recovery,
task completion/block/site loss, route/destination change, and accepted player nudge/order.
Ordinary preemption requires at least 15% improvement; emergencies, invalid routes, and incapacity
bypass that floor.

Matching and reservation are one transaction: a cat is not marked busy until the objective, work
slot, delivery capacity, route, tools, and cargo/resource claims are valid. Refusal or invalidation
releases the worker and invalid claims atomically, then rematches.

## Anatomy and injury incidents

Tracked parts are four sided paws, two sided eyes, and the tail. Part states and natural function
are Healthy 100%, Minor 85%, Severe 50%, and Missing 0%.

Incident probability is evaluated once per completed hazardous work unit:

| Work unit | Incident probability |
|---|---:|
| Scout | 1.5% |
| Hunt | 1.0% |
| Quarry | 0.8% |
| Logging | 0.5% |
| Construction | 0.3% |
| Raid victory | 5% |
| Raid defeat | 20% |

Conditional outcome probabilities are Minor 70%, Severe 20%, Missing 8%, and Fatal 2%. The injury
RNG is a dedicated LCG fork, keyed by stable incident identity so batching and unrelated draws do
not change results. A non-fatal incident selects one eligible body part deterministically.

Paw function averages four paws and affects movement and physical labor. Eye function averages two
eyes and affects vision, scouting, hunting, and ranged combat. Tail function supplies 10% of
balance-sensitive movement and combat performance. Severe injury blocks unsuitable hazardous work
until treated. Minor injury requires 12 effective treatment-work hours and severe injury requires
48. Missing parts never regrow. Treatment is a physical, consent-aware task at a reachable care
site. Effective work uses the existing treatment productivity/skill pipeline, with `Caregiver` and
`Restorative Grace` applied in the global modifier order; no second healing-rate system or scalar
remote heal may bypass route, worker, medicine, patient, or refusal state.

## Prosthetic lifecycle

Wooden prosthetics restore 50% and metal prosthetics 75% of missing function. Rehabilitation adds
2 percentage points per stage; `Prosthetic-Adapted` adds 10 points. Total restoration is capped at
90%.

Every item has a stable ID and exact part/side. Fitting requires the correct unequipped item, a
capable medic/fitter, patient consent, and a reachable treatment or Workshop site. Refusal consumes
nothing. The transaction atomically moves the unique item from inventory to the anatomy slot.

- Wooden durability: 360 affected work-hours.
- Metal durability: 1,080 affected work-hours.
- Wear accrues only from affected work and is batching-invariant.
- At zero durability the item is broken and the part returns to missing functionality.
- Repair is a physical Workshop task using the canonical Workshop objective and finite inputs.
- Repair restores durability; it never mints a replacement item.
- A fitted item returns to owned inventory during recoverable death handling.
- Prosthetics may move through physical trade only when not fitted/reserved.

Fitting, refusal, cancellation, repair, death, trade, and restart must conserve exactly one item ID.
No branch may duplicate, delete, or equip the same item twice. Persist anatomy, fitted item IDs,
condition, durability, adaptation progress, treatment state, and reservations; restart reconstructs
and revalidates the active physical task before continuing.

## UI and tests

Cat care UI shows innate attributes, learned skills/office experience, personality, stress and
recovery, refusal reason, acquired traits, every body part, injury/function, fitted prosthetic,
durability, restoration cap, treatment/fitting/repair status, and bounded eligibility reasons.

Required evidence includes exact migration/inheritance clamps, the deterministic 80/15/5
distribution, axis-isolation fixtures, stress boundary/recovery tests, known greedy matching
counterexamples, collection-order twins, incident matrices, batch-equivalent wear, and one-item
conservation across every fitting/death/trade/repair/restart transition.

## LAI.30 cat care UI contract

`LAI.30_CAT_CARE_UI_CONTRACT` is the post-cutover client contract for report-safe per-cat care
panels. The Bevy client must install one `CatCarePanelPlugin` with a `CatCarePanelRoot` keyed by
`CatCareStableCatId` and filtered through `CatCareSelectedColonyFilter`. It renders only the
authorized cat projection from the snapshot; `CatCareReportSafeProjectionOnly` forbids client-side
recalculation from hidden truth, private world state, or another colony's data.

The identity and capability section shows `MigratedInnateAttributeBreakdown` for the eight
post-migration innate attributes, `LearnedSkillAndOfficeExperienceBreakdown` for learned labor and
office experience, `PersonalityAxisBreakdown` for all eight personality axes, and
`AcquiredTraitBadgeList` for active acquired traits. It never collapses those inputs into an
unexplained total and never invents missing report fields.

Stress and work readiness use `CatStressRecoveryMeter`, `CatRefusalStateBadge`,
`CatWillingnessReasonList`, `CatCareBoundedEligibilityReason`, `CatCareTypedBlockReason`,
`CatCareNoHiddenTruthWillingnessRecompute`, `CatCareNoHiddenRegenerationProjection`, and
`CatCareSelfPreservationOverrideBadge`. Reasons are typed, bounded, and based on the authorized
projection. Hidden regeneration, exact unavailable source quantities, private plans, and private
beliefs cannot appear in labels, tooltips, logs, or inspector text.

Anatomy uses `CatAnatomyPanel` and `FourPawTwoEyeTailAnatomyGrid`. Required body-part labels are
`LeftFrontPawStateLabel`, `RightFrontPawStateLabel`, `LeftRearPawStateLabel`,
`RightRearPawStateLabel`, `LeftEyeStateLabel`, `RightEyeStateLabel`, and `TailStateLabel`.
`CatInjuryTreatmentState` and `TreatmentHoursRemainingLabel` report injury severity, treatment
state, consent/refusal status, effective remaining work, and bounded block reasons.

Prosthetics use `CatProstheticPanel` with `FittedProstheticStableItemId`,
`FittedProstheticSideLabel`, `FittedProstheticTypeLabel`,
`FittedProstheticRestorationPercent`, `FittedProstheticDurabilityHours`,
`FittedProstheticWearProgress`, `ProstheticAdaptationProgress`, and
`ProstheticRestorationCapLabel`. The UI displays the persisted item ID and never synthesizes a
replacement identity; fitting, removal, repair, cancellation, death recovery, trade, and restart
must conserve exactly one item/cargo identity.

Active care work uses `ActiveCareTaskReferenceList`, `CareTaskSiteRefLabel`,
`CareTaskCargoReferenceLabel`, `CareTaskTreatmentPatientRef`, `CareTaskFitterOrMedicRef`,
`CareTaskWorkshopRepairRef`, `CareItemCargoIdentityConservationGuard`, and
`CatCareMultiColonyPrivacyGuard`. Care task references are report-safe links to authoritative
task/site/cargo snapshots, not client-derived movement targets or hidden reservation data.

Controls are accessible and action-backed. `CareTreatmentActionButton`,
`CareConsentActionButton`, `CareRefusalAcknowledgeButton`, `ProstheticFitActionButton`,
`ProstheticRemoveActionButton`, and `ProstheticRepairActionButton` send
`build_cat_care_action_envelope` payloads carrying `AuthenticatedPlayerIdentity`,
`ExpectedCatCareVersion`, and `StableIdempotencyId`. Disabled controls expose
`CatCareControlDisabledReason`; accepted, rejected, stale, and duplicate responses surface through
`CatCareTypedFeedbackToast`, `CatCareActionConflictRefresh`,
`CatCareVersionMismatchRefreshHandler`, `PreserveSelectedCatAfterRefresh`,
`PreserveCareDraftAfterRefresh`, `DuplicateCareReplayUsesOriginalResult`, and
`RemovedCatSelectionClearsSafely`.

Playwright and visible-browser acceptance use stable labels and IDs:
`ACCESSIBLE_CAT_CARE_PANEL_LABEL`, `CAT_CARE_PANEL_TEST_ID_PREFIX`,
`CAT_CARE_BODY_PART_TEST_ID_PREFIX`, `CAT_CARE_CONTROL_TEST_ID_PREFIX`,
`CAT_CARE_TASK_REF_TEST_ID_PREFIX`, and `PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST`. Browser checkpoints
are `VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL`,
`VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC`, and
`VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY`. Every checkpoint records selected colony,
cat ID, action ID, expected care version, idempotency ID, relevant task/site/cargo/prosthetic item
IDs, accessibility tree, screenshot, console/network state, and stale-refresh outcome.
