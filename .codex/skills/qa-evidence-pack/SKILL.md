---
name: qa-evidence-pack
description: Run quality gates, collect screenshots, upload to 0x0.st, and prepare evidence for PR.
tools: Bash, Read, Grep
---

# QA Evidence Pack

## Required Checks
Run and record results:
1. `bun run lint`
2. `bun run typecheck`
3. `bun run test`
4. `bun run test:coverage`

## Screenshot Workflow
1. Capture at least two feature-relevant screenshots.
2. Upload each to `0x0.st`:
```bash
curl -F'file=@<path>' https://0x0.st
```
3. If upload fails, store files under `docs/pr-assets/<branch>/`.
4. Produce evidence list with URLs or repo paths.

## Output
- Gate results
- Coverage note
- Screenshot evidence list
- Fallback note (if used)
