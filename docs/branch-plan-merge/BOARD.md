# Branch Plan Merge Board

This is the dedicated append-only board for reconciling design and planning work from multiple
branches before implementation. It does not replace the Leader-AI implementation board. Its job is
to preserve every branch's intent, expose conflicts, visualize the combined system, and produce one
decision-complete implementation plan.

## Non-negotiable merge rules

- Archive each branch plan before reconciliation.
- Never make an archived plan smaller.
- Treat every answered question, attached user note, and direct design message as authority unless a
  later answer explicitly supersedes it; use the [thread Q&A audit](thread-qa-audit.md).
- Treat explanations, examples, motivations, visual ideas, documentation requests, and extension
  guidance as requirements unless the user explicitly rejects them.
- Do not resolve a conflict by silently selecting whichever branch has newer code.
- Record every conflict as keep, replace, combine, or supersede-with-reason.
- Preserve branch-specific tests, assets, content tables, and domain invariants as evidence.
- Freeze dirty/source-only branch state and require a justified per-file disposition through the
  [source-transfer manifest](source-transfer-manifest.md); “not merged” never means “not inspected.”
- Hot roots are reconciled structurally only after the unified design is approved.
- No implementation begins while a high-impact product conflict remains undecided.
- All visual requirements must appear in the written plan and visual-spec inventory.
- This project has no production deployment; compatibility work exists only when explicitly chosen.
- Editing may be parallel, but heavy tests/builds/browser runs are serialized.

## Status flow

`todo → inventory → compared → decision → unified → accepted → implemented`

## Branch inventory

| Branch / plan | Status | Stored authority | Notes |
|---|---|---|---|
| `feature-new-leader-ai` + `the-shrine-upgrade` | accepted | [Final Hole/Hunting/content plan](../leader-ai-overhaul/final-hole-hunting-content-plan.md), [Q&A audit](thread-qa-audit.md), and [source-transfer manifest](source-transfer-manifest.md) | First integration snapshot stored 2026-07-24. The source freeze records 13 modified + 69 untracked files, including 53 assets, and routes Leader reports, The Hole, Hunting, typed food, universal quality, catalogs, art, docs, and QA semantically. |
| `bug-gui-design` | accepted | [Final integrated overhaul plan](../leader-ai-overhaul/final-integrated-overhaul-plan.md), [dedicated implementation board](bug-gui-design-BOARD.md), [Q&A audit](thread-qa-audit.md), and [source-transfer manifest](source-transfer-manifest.md) | Second integration snapshot stored 2026-07-25. Source head `748db74`; four source-only commits, 26 committed paths, and 20 dirty paths are frozen. Integration is semantic because both worktrees touch divergent hot roots and the source predates the report-limited Leader/Hole architecture. |
| Future branch | todo | Pending | Add a row before reading or reconciling a later branch. |

## Merge cards

| ID | Card | Status | Depends on | Required output | Acceptance |
|---|---|---|---|---|---|
| BPM.0 | Preserve first integration plan | accepted | — | Immutable self-contained plan snapshot and link from this board | The stored file is byte-identical to the approved thread plan after removing only `<proposed_plan>` transport tags; its 45-row exact register maps all 15 sections and the uncompressed LAI.35–LAI.52 cards. |
| BPM.1 | Inventory second branch | accepted | BPM.0 | Branch/worktree identity, Git base, commits and dirty files, plan/design docs, code domains, tests, assets, screenshots, unfinished work, and active workers/processes | Recorded in the dedicated board; source worktree was read-only during inventory. |
| BPM.1A | Identify and protect both source states for semantic transfer | accepted | BPM.1 | Exact heads/merge base, commit inventory, modified/untracked path lists, asset counts, drift hashes, no-clean/remove rule, authorized backup prerequisite, domain routes, and required per-file receipt format | [Source-transfer manifest](source-transfer-manifest.md) identifies 82 Shrine working files, 26 committed Bug paths, and 20 dirty Bug paths without modifying either source; hashes detect drift but do not pretend to be backups. |
| BPM.2 | Extract second-branch requirements | accepted | BPM.1 | Append-only requirement register separating product behavior, simulation rules, AI, UI, visuals, content, documentation, extension guidance, persistence, and testing | GUI-R01–GUI-R26 summarize the branch plus Q&A-only acceptance; the exact P2 register and LAI.53–LAI.70 checklists preserve every final-plan obligation and rationale. |
| BPM.2A | Audit the complete planning Q&A and direct user notes | accepted | BPM.0, BPM.2 | Every question ID, selected answer, attached note, direct design message, later correction, retained motivation, destination card, and uncovered gap | [Thread Q&A audit](thread-qa-audit.md) accounts for all 139 prompts, the one immediate retry, later supersessions, direct inputs, and six repaired board gaps. |
| BPM.3 | Produce visual branch map | accepted | BPM.1, BPM.2 | Architecture diagram, subsystem ownership map, data flow, state machines, footprints, panel wireframes, asset/state matrix, and dependency graph for the second branch | Plan architecture, research flow, wireframes, implementation DAG, and GUI-V01–GUI-V12 stored. |
| BPM.4 | Build cross-branch conflict matrix | accepted | BPM.2, BPM.3 | Field-by-field comparison against the stored first plan: identical, additive, compatible variation, direct conflict, or obsolete compatibility assumption | GUI-C01–GUI-C12 record the high-impact differences, including Research UI, new institutions, reset scope, and shell/root replacement. |
| BPM.5 | Resolve product decisions | accepted | BPM.4 | User-approved decisions for every high-impact conflict plus explicit defaults for minor implementation choices | Final plan locks semantic integration, authority, two research lanes, physical construction, barter, and clean reset. |
| BPM.6 | Define unified public interfaces | accepted | BPM.5 | Stable IDs, catalogs, types, actions, reports/redaction, version lanes, persistence/reset, content/asset references, and extension templates | Exact Plan 2 sections 9 and 11, plus Plan 1 sections 3, 10, and 12. |
| BPM.7 | Define unified AI and spatial behavior | accepted | BPM.5, BPM.6 | Leader/officer ownership, beliefs/reports, goal dependencies, domain commands, physical tasks, exact sites/footprints/routes, reservations, failure/recovery, and player nudges | Exact Plan 2 sections 2–7 and Plan 1 sections 5–9. |
| BPM.8 | Define unified visual and documentation pack | accepted | BPM.5, BPM.6 | Final diagrams, wireframes, sprite/icon/portrait/state inventory, accessibility text, root-doc updates, and add-anything contributor guides | Exact Plan 2 sections 8 and 11, Plan 1 sections 2, 10–12, and GUI-V01–GUI-V12. |
| BPM.9 | Define serialized verification | accepted | BPM.6–BPM.8 | Focused, restart, campaign, protocol, persistence, rendering, Playwright, visible-browser, diagnostics, performance, and liveness acceptance | Exact Plan 2 section 12 and Plan 1 section 14: quick feature checks, one heavy process, late integration campaign, one Playwright worker, visible browser, no live AI. |
| BPM.10 | Publish complete replacement plan | accepted | BPM.5–BPM.9, BPM.2A | One decision-complete package containing both immutable plans plus all retained Q&A/direct-note intent and recorded supersessions | Both stored files are byte-identical to their final approved thread plans after removing only transport tags; the additive Q&A audit preserves intent that the final prose compressed. |
| BPM.11 | Translate unified plan into implementation board | accepted | BPM.10, BPM.1A, BPM.2A | Additive cards, owners, dependencies, Q&A/source receipts, red/green/QA evidence, hot-root ownership, test-slot rules, and final cutover gate | Dedicated board and main-board LAI.53–LAI.70 wave include the Q&A and per-file semantic-transfer gates. |
| BPM.12 | Implement and verify unified plan | dev | BPM.11 | Code, content, assets, docs, source-derived tests, per-file transfer receipts, browser evidence, and legacy deletion/reset required by both plans and the Q&A audit | In progress. No card may claim source functionality solely because a branch was not merged; it must inspect and disposition the relevant source paths. |

## Requirement register template

Use one row per requirement. Never combine unrelated requirements merely to shorten the board.

| Requirement ID | Source branch/file | Exact intent | Why it exists | Behavior/UI/visual/doc impact | Conflicts | Unified destination | Status |
|---|---|---|---|---|---|---|---|
| `REQ-example` | branch + file/section | Requirement in implementable language | User or design motivation | Affected surfaces | IDs or none | Unified plan section/card | inventory |

## Conflict matrix template

| Conflict ID | Topic | First stored plan | Second branch | User-visible consequence | Recommended resolution | Decision | Cards |
|---|---|---|---|---|---|---|---|
| `CON-example` | Domain | Existing rule | Incoming rule | What changes for the player/simulation | Keep/combine/replace with reason | pending | pending |

## Visual inventory template

| Visual ID | Feature/state | Required views | Data source | Player visibility | Assets needed | Accessibility/text | Acceptance screenshot |
|---|---|---|---|---|---|---|---|
| `VIS-example` | Feature | World, panel, state sheet, diagram | Authoritative/report-safe fields | Exact/bounded/hidden | Existing/new | Label and description | Named checkpoint |

## Operational notes

- Restore the Orca runtime before claiming worker orchestration.
- Every orchestrated worker must have task/dispatch provenance and visible completion.
- Use at most three independent editing workers plus the coordinator.
- Only the coordinator grants the single heavy test/browser slot.
- Local commands use `CARGO_BUILD_JOBS=1`, `taskset -c 0-3`, one Rust test thread, and one
  Playwright worker.
- During planning, exploration is read-only. Branch code and plans are not edited until the unified
  plan is accepted.

## Second integration decision — 2026-07-25

The accepted second plan is
[final-integrated-overhaul-plan.md](../leader-ai-overhaul/final-integrated-overhaul-plan.md).
Its complete card, requirement, conflict, and visual register is
[bug-gui-design-BOARD.md](bug-gui-design-BOARD.md).

The locked integration strategy is **not** a Git merge. Both source and target worktrees have
uncommitted changes in overlapping hot roots, and `bug-gui-design` predates the report-limited
Leader/Hole cutover. Bounded source modules, tests, layouts, assets, and design intent are adapted
card by card into `feature-new-leader-ai`; hot roots are reconciled once by an explicit integration
owner.
