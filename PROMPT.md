# Objective
Run one autonomous feature loop for this repository and stop only after creating a pull request.

Completion marker (exact literal): `PR_CREATED_LOOP_COMPLETE`

## Mandatory Outcome
1. Implement one small, user-visible feature slice for this idle game.
2. Add tests for the feature.
3. Pass required quality gates.
4. Capture screenshots for the feature.
5. Upload screenshots to `0x0.st` (fallback to committed repo paths if upload fails).
6. Create a PR to `main` with a German explanation of the feature and screenshot evidence.
7. Emit completion marker after PR URL exists.

## Scope Constraints
- Keep scope intentionally small.
- No broad refactors or page overhauls.
- Do not change unrelated files.
- Prefer existing architecture and patterns.

## Branch and Git Rules
- Start from `origin/main`.
- Use a dedicated feature branch for this run.
- No destructive git commands.
- No force push.

## Required Gates (must all pass before PR)
- `bun run lint`
- `bun run typecheck`
- `bun run test`
- `bun run test:coverage`

## Screenshot Evidence Contract
Capture at least 2 screenshots that clearly show the new behavior.

Primary upload path:
```bash
curl -F'file=@<path>' https://0x0.st
```

Fallback:
- Store screenshot files under `docs/pr-assets/<branch>/`
- Reference these repo paths in PR body if 0x0 upload fails

## PR Contract
PR body must include:
1. A German explanation of what the feature does.
2. Test/check commands run and results.
3. Screenshot links (0x0 URLs or fallback repo paths).
4. Any known limitations.

## Task/Memory Discipline
Use Ralph runtime tools to persist state across iterations:
- Tasks:
  - `ralph tools task add ...`
  - `ralph tools task ready --format json`
  - `ralph tools task close <id>`
- Memories:
  - `ralph tools memory add --type decision|fix|context ...`
  - `ralph tools memory prime --budget 2000`

## Skill Loading Contract
Hats should load and apply project skills as needed:
- `feature-scope-small`
- `tdd-implement-slice`
- `qa-evidence-pack`
- `pr-german-template`

## Publish Rules
- Publish failure events with exact blockers and evidence.
- Never publish PR completion events before PR URL exists.
- After PR is created, emit the exact token: `PR_CREATED_LOOP_COMPLETE`.
