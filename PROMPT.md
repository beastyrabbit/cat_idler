# Objective

Run a continuous, generative feature development session for this cat colony idle game.
Each cycle: invent a new feature, implement it with TDD, create a PR, then cycle back
and invent the next one. Keep going until manually stopped.

There is no completion marker. This loop runs until you are stopped.

## Session Contract

Each cycle follows this flow:

1. **Discover** — Read the codebase and memories to understand what already exists.
2. **Invent** — Come up with a small, user-visible feature that doesn't exist yet.
3. **Scope** — Keep it to 1-6 files, pure functions preferred.
4. **Branch** — Create a `feature/<name>` branch from `origin/main`.
5. **Implement** — TDD: write failing tests first, then make them pass.
6. **Validate** — Pass all quality gates.
7. **Evidence** — Capture screenshots for UI changes.
8. **PR** — Create a PR with German explanation and evidence.
9. **Reset** — Return to `main` and start the next cycle.

## Creative Feature Discovery

An idle game always has room for more features. Never conclude that there's
"nothing left to build." Look for opportunities in these categories:

### Quick Wins (TODOs in the codebase)

- Search for `TODO`, `FIXME`, `HACK` comments
- Check `convex/encounters.ts` — colony defense calculation
- Check `convex/breeding.ts` — fertility blessing modifier
- Any incomplete stubs or placeholder logic

### Newspaper Sections

The UI is a broadsheet newspaper. New sections are always welcome:

- Weather forecasts affecting gameplay
- Cat personality profiles from trait data
- Resource market trends with indicators
- Classified ads for available jobs
- Obituaries for fallen cats
- Birth announcements
- Editorial opinions based on colony status

### Game Mechanics

Pure functions in `lib/game/` — easy to test, safe to add:

- Weather system affecting resource rates
- Cat aging milestones and life stage events
- Resource trend analysis (moving averages)
- Cat relationship/friendship tracking
- Territory and exploration mechanics
- Seasonal cycles affecting the colony
- Trading between colonies
- Cat mood/happiness system
- Predator threat levels
- Food variety and diet preferences

### UI Polish

Small visual improvements to the newspaper:

- Edition counter (Vol. X, No. Y)
- Colony age in human-readable format
- Status indicators and trend arrows
- Historical statistics section
- Colony achievements/milestones display

### Event System

Extend the existing event log:

- Crisis events (famine, storms, predator waves)
- Milestone celebrations
- Cat achievement events
- Seasonal festival events
- Discovery events from exploration

## Scope Constraints Per Feature

- 1-6 files changed maximum
- No schema migrations or new database tables
- No broad refactors or page overhauls
- Prefer pure functions in `lib/game/` with unit tests
- UI changes go in existing newspaper sections
- If a feature grows beyond scope, trim it down
- Do not change unrelated files

## Branch and Git Rules

- Always branch from `origin/main`
- Use `feature/<kebab-case>` naming
- No destructive git commands
- No force push
- Return to `main` between features

## Required Gates (per feature)

- `bun run lint`
- `bun run typecheck`
- `bun run test`
- `bun run test:coverage`

## Screenshot Evidence Contract

For features with UI changes, capture at least 2 screenshots.
Primary upload: `curl -F'file=@<path>' https://0x0.st`
Fallback: `docs/pr-assets/<branch>/`

## PR Contract (per feature)

PR body must include:

1. German explanation of what the feature does.
2. Test/check commands run and results.
3. Screenshot links (if applicable).
4. Known limitations.

## Failure Recovery

- Quality gate fails: fix and retry (up to 2 retries, 3 total attempts).
- Still failing after 3 attempts: skip the feature, record reason, move on.
- Feature too complex during planning: scope down or skip.
- 3 consecutive skips: pick something deliberately simpler next cycle.
- Git conflicts: reset to main and start fresh.

## Task/Memory Discipline

Use Ralph runtime tools to persist state across iterations:

```bash
# Tasks
ralph tools task add "Feature: <title>" --priority <N>
ralph tools task ready --format json
ralph tools task close <id>

# Memories (critical for cross-cycle awareness)
ralph tools memory add --type decision "Feature plan: <title>"
ralph tools memory add --type context "Completed: <title> — PR #N"
ralph tools memory add --type context "Skipped: <title> — reason"
ralph tools memory add --type fix "<non-trivial fix detail>"
ralph tools memory prime --budget 2000
```

## Skill Loading Contract

Hats load skills as needed:

- `feature-scope-small` — planner uses for scoping
- `tdd-implement-slice` — coder uses for implementation
- `qa-evidence-pack` — qa_checker uses for evidence
- `pr-german-template` — pr_creator uses for PR body
