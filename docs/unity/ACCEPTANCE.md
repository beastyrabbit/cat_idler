# Unity migration acceptance

Status: implementation in progress. No release readiness is claimed by this checklist.
Baseline: current `main`, `8c5ea0f2d0871a1f12dfdafd831e6d4a78d40cec`.
The current source has 53 public action variants, including development controls,
25 building types, 108 station recipes, 487 studies, seven specialist officers and
19 labors. Older prose claiming 52 actions or 104 recipes is stale.

| Required capability | Acceptance evidence | Status |
| --- | --- | --- |
| Reproducible native game | Fresh dependency resolve, compile, open, Play mode and Apple Silicon build; documented commands | Open |
| Living cats | Physical finite meals/water and sleep, preferences/skills, injury, aging, breeding bed reservations, migrants, death and atomic colony recovery | Open |
| Manual and automated work | Founding Leader safety floor; all seven office prerequisites and vacancies; normal UI for every manual category | Open |
| Physical economy | Sources, cargo, finite pile capacity, source/destination claims, per-cat workplace slots, all 108 queued recipes, full input-to-delivery chains | Open |
| Scarcity and interruption | Contended inputs/beds, cancelled/reassigned jobs, death, blocked route, full destination and direct-control handoffs conserve goods and release claims | Open |
| Construction and territory | Exact placement, delivered construction materials, roads/bridges, expansion, exterior farms and growth/harvest | Open |
| Knowledge and progression | Provisional scouting returned to shrine, all 487 reachable studies with functioning effects, blessings, research labor and daily Leader choice | Open |
| Equipment, defense and trade | Exact equipment identities/wear/repair, finite visiting trader, raids/warriors, rail/shipping, physical inter-village caravan | Open |
| Persistent shared authority | Communal and personal villages, two player identities, foreign denial and confidential reports, reconnection and real network trade | Open |
| Saves and migration | Atomic save/reload/server restart retain jobs, cargo, reservations, identities and progression; read-only legacy import with unchanged source and no replay | Open |
| 3D management and cat control | Recognizable Blender-authored forest, visible movement/carry/work/needs, readable controls, existing cat enters third-person control and returns to AI | Open |
| Normal UI operation | Editor and packaged app: inspect, build, queues, research, walk/interact, return to management; state assertions accompany inspected frames | Open |
| Extended operation | Fresh and established multiple-seed campaigns; distinguish designed loss from starvation/deadlock; no invented inputs | Open |
| Measured performance | Founding and expanded populations, real tick/frame timings, workloads/machine/limits recorded | Open |
| Delivery | Complete outgoing diff/security scan, independent candidate review, one PR with evidence; no merge/deploy | Open |

The C# model and Unity scene are replacements for the legacy implementation. The
legacy entry points remain until the replacement gates pass. A catalog count or
successful compile alone does not close a gameplay row.

Intentional design decisions and their tests belong in `GAMEPLAY_ACCEPTANCE.md`.
Save compatibility belongs in `PERSISTENCE.md`. Asset provenance and reproduction
commands belong in `source-art/README.md`. Record observed verification results
and remaining failures here before delivery.
