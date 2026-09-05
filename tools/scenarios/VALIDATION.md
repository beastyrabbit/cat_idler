# Acceptance evidence

The executable suite and Unity EditMode tests share `AcceptanceScenarios.cs`. Results below came from the .NET executable on the development Mac. Unity Editor and packaged UI execution belong to the parent migration checks.

Observed failures before simulation corrections:

| Scenario | Observed failure | Correction verified by scenario |
| --- | --- | --- |
| Emergency staffed water | A thirsty worker stayed trapped while carrying its fetched water. | Physical shrine consumption restored thirst and preserved remaining cargo. |
| Station capacity | A completed Clothier had no physical reserve for its capacity study. | Public construction created the container, then a public research purchase increased capacity. |
| Targeted scout | An ore-targeted scout failed to return nearby ore knowledge. | The nearby resource became permanent knowledge after shrine return. |
| Selected quarry | A Gem request produced no Gem. | The selected source depleted into exact Gem cargo and storage. |
| Smithy and Woodworking workflow | Their output studies changed neither output nor material use for exact items. | Both now reduce finite ingredient use while retaining one whole item identity. |
| Officer vacancy | Removing the Forester left an automatically staffed processor running. | Removing the officer releases automatic station workers and jobs while preserving the idle reserve. |
| Communal enclosure | The 30-cat communal blueprint used the personal village radius. | Communal radius 9 and personal radius 6 both pass. |
| Death and extinction | Zero-health cats recovered health before the death check. | Death returns carried scalar goods and exact equipment; extinction restores a valid founding colony atomically. |
| Repeating and paused queues | A repeating head starved later recipes, and pause left active work running. | Both recipes complete in sequence; paused work retains its state and resumes. |
| Exact equipment on rail | Rail transport could not carry exact items. | The same item ID, material, quality, and condition reach the destination. |
| Interrupted expansion | Taking control of the worker prevented the perimeter job from resuming. | The pending job resumes, preserves completed segments, and opens the final gate. |
| Communal hunter return | A path-search limit accepted departure but rejected the detour home around the larger enclosure. | The seed-7 fresh campaign retains every founding cat for 48 hours with reciprocal travel. |
| Personal founding access | Generated water and mountains isolated the personal gate from finite food and water sources. | A narrow exterior founding footpath connects the gate to those sources; the seed-41 shared/personal campaign survives 48 hours. |
| Shipping cancellation | Cancelling a loaded vessel spilled its cargo at sea. | The vessel returns physically to port, retains cargo while storage is full, and releases ownership after unloading. A dead driver's cargo remains at its actual position until bridge construction and hauling recover it. |

Earlier source-reviewed interruption findings were already corrected when their executable scenarios first ran. Their passing tests cover busy-worker ownership, direct-control route release, item-only pile removal, farm replacement, preserves, medicines/brew, equipment combat effects, accounting reachability, zone decisions, and bounded direct movement.

A complete noncampaign run passed all 536 cases in 8.58 seconds. The 34 regression cases also passed separately in 1.68 seconds. The first final campaign matrix exposed four failures from two travel defects listed above. After their corrections, all nine campaign twins passed in 170.87 seconds. Every original founding cat survived, and no colony reset.

Three shipping regressions pass in the final noncampaign run. The full-source case fills and clears the port using a second cat's public deposit/pickup actions with finite goods established before play. The death case applies lethal health as its explicit fault injection, then uses public bridge construction and hauling to recover the stranded cargo. The campaign result predates the final transport-only correction. Those campaigns create no transport routes; they were not repeated after that correction.

Campaigns compare all public world fields between hourly and minute-partitioned twins, rounded to nine decimal places for doubles, and validate item locations, claims, and beds every hour. The founding-survival assertion now retains original cat IDs so arriving cats cannot mask a founding death. The established campaign uses one staffed repeating wood-processing station with finite initial supplies; it does not establish that a complete mature economy runs indefinitely.

All 108 recipes run through physical input carrying, station work, and output delivery. All 487 research nodes are publicly purchased in a dependency traversal. Differential effects cover 75 building, 238 service, and 64 resource studies. Capacity modifiers use the authoritative capacity projection, with separate physical construction and full-storage checks; this is narrower than a complete hauling campaign for every capacity study. Exact-item output studies conserve one whole item and improve ingredient efficiency rather than emitting fractional equipment. No test calls a private simulation method or a live AI provider.

`latest-results.txt` and `campaign-results.txt` contain executable results. They are overwritten only by explicit local verification commands. Read each final `RESULT` line and the named failures before describing a run as successful.
