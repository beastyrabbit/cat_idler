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
| Construction with carried Food | Assigning a scaffold after direct control destroyed the cat's unrelated Food cargo. | The eight carried Food remain in a physical spill before material collection; completed construction consumes only its bill, with Food reduced only by ordinary spoilage. |
| Construction with surplus Planks | The scaffold consumed four Planks beyond its material bill. | Surplus carried Planks remain recoverable, and only the required Planks and Blocks leave the conserved totals. |
| Armor production storage | Finished Armor could not reach a stockpile that explicitly accepted Armor. | Production delivers the same exact item to the accepting pile, preserving condition and consuming only its Metal bill. |
| Unequipping Armor | Unequip rejected reachable Armor-only storage. | The same item returns to that pile with its condition and maximum condition unchanged, without scalar copies. |

Earlier source-reviewed interruption findings were already corrected when their executable scenarios first ran. Their passing tests cover busy-worker ownership, direct-control route release, item-only pile removal, farm replacement, preserves, medicines/brew, equipment combat effects, accounting reachability, zone decisions, and bounded direct movement.

A complete final noncampaign run passed all 540 cases in 8.1122 seconds. The 38 regression cases also passed separately in 1.4725 seconds. All nine campaign twins passed in 161.2604 seconds after the conservation, authority and terrain changes. Every original founding cat survived, and no colony reset. This campaign run supersedes the earlier result that preceded the shipping correction.

The four review regressions for carried Food, surplus Planks, Armor production and unequipping first failed together, then passed after their fixes. Existing assertions were preserved. The construction cases follow public control, pickup, reassignment, material delivery and completion, checking the exact bill and recoverable surplus. The Armor cases check accepting storage, item identity, condition and the absence of duplicate scalar goods.

Three shipping regressions pass in the final noncampaign run. The full-source case fills and clears the port using a second cat's public deposit/pickup actions with finite goods established before play. The death case applies lethal health as its explicit fault injection, then uses public bridge construction and hauling to recover the stranded cargo. The final campaign rerun includes all these corrections, although its workloads create no transport routes.

Campaigns compare all public world fields between hourly and minute-partitioned twins, rounded to nine decimal places for doubles, and validate item locations, claims, and beds every hour. The founding-survival assertion now retains original cat IDs so arriving cats cannot mask a founding death. The established campaign uses one staffed repeating wood-processing station with finite initial supplies; it does not establish that a complete mature economy runs indefinitely.

All 108 recipes run through physical input carrying, station work, and output delivery. All 487 research nodes are publicly purchased in a dependency traversal. Differential effects cover 75 building, 238 service, and 64 resource studies. Capacity modifiers use the authoritative capacity projection, with separate physical construction and full-storage checks; this is narrower than a complete hauling campaign for every capacity study. Exact-item output studies conserve one whole item and improve ingredient efficiency rather than emitting fractional equipment. No test calls a private simulation method or a live AI provider.

`latest-results.txt`, `campaign-results.txt` and `regression-results.txt` contain the final executable results copied from their local verification logs. Command and exit-status wrappers are omitted; every scenario result and final `RESULT` line is retained. Read those counts and any named failures before describing a run as successful.
