# Browser Idle V2 Test Campaign

This campaign is for a browser-capable AI/human QA runner.

## Goal

Validate that the global shared idle game works end-to-end:
- one global colony
- worker-driven simulation
- short player actions
- long leader/cat jobs
- specialization progression
- click speed-ups
- ritual -> global points -> shared upgrades
- unattended collapse and auto-reset

## Environment Setup

Run in separate terminals:

```bash
bun install
bun run convex:dev
bun run dev
bun run dev:worker
```

Open app at:
- `http://localhost:3000/`
- should redirect to `/game`

For accelerated QA scenarios, open:
- `http://localhost:3000/game?test=1`
- then use `Speed: Fast` or `Speed: Turbo` in Player Actions panel

## Test Accounts

Use at least 2 browser sessions:
1. Session A nickname: `QA-A`
2. Session B nickname: `QA-B`

Use separate browser profiles/incognito windows to ensure distinct localStorage sessions.

## Pass/Fail Rules

- Each scenario is PASS only if all expected results are observed.
- Record screenshots for failures.
- Include timestamp and session nickname in notes.

## Scenarios

### S1: Global Entry and Shared World
1. Load `/`.
2. Confirm redirect to `/game`.
3. In Session A and Session B, verify same colony name and run number.
4. Verify online count increases when both sessions are open.

Expected:
- One shared colony, same run id/state for all sessions.

### S2: Player Short Actions (Immediate Jobs)
1. In Session A, trigger `Supply Water`.
2. Verify a short-duration job appears (about 15s).
3. Wait for completion.
4. Confirm water resource increases.
5. Repeat for `Supply Food` (about 20s).

Expected:
- Jobs appear immediately.
- Complete in short times.
- Resources increase after completion.

### S3: Long Leader/Cat Jobs
1. Trigger `Request Hunt`.
2. Verify planning/execution style long job flow appears (leader plan + hunt expedition).
3. Trigger `Request House Build`.
4. Verify long planning/build flow appears.

Expected:
- Long jobs have hour-scale ETAs.
- Duplicates are blocked while strategic jobs are active.

### S4: Click Speed-Up
1. Pick an active long job.
2. Click `Boost -10s` repeatedly.
3. Verify ETA reduces after each click.
4. Continue clicking past 30 and 60 clicks in a minute.

Expected:
- Early clicks reduce more time.
- Diminishing returns after thresholds.
- Job cannot be reduced below minimum floor instantly.

### S5: Specialization Progression
1. Let hunt/build/ritual jobs complete repeatedly.
2. Verify each cat shows lineage/coat/eyes/markings in the cat panel.
3. Monitor cat specialization field in cat panel.
4. Confirm specialization appears only after repeated role work.
5. If needed, enable `Speed: Fast` to shorten long jobs.

Expected:
- Lineage/attribute data is visible per cat.
- Cats start unspecialized.
- Hunter/Architect/Ritualist appears after enough role XP.
- Specialization affects speed and/or rewards over time.

### S6: Ritual Request and Leader Gate
1. In Session A, click `Request Ritual`.
2. Confirm request is accepted (event/log feedback).
3. If resources are below threshold, verify ritual does not start immediately.
4. Raise resources with supply actions.
5. Confirm leader later starts ritual when conditions are safe.

Expected:
- Player requests ritual.
- Leader decides start timing.
- Duplicate ritual requests are blocked while pending/active.

### S7: Global Upgrade Economy
1. After ritual completes, note ritual points.
   - Use `Speed: Fast`/`Turbo` so ritual can complete quickly.
2. In Session A, buy one upgrade.
3. In Session B, verify upgrade level changed immediately.
4. Verify points decrease globally.

Expected:
- Upgrade purchase is shared globally.
- Any player can purchase with shared points.

### S8: Worker Independence (No Browser Required)
1. Close all browser sessions.
2. Keep `convex:dev` and `dev:worker` running for several minutes.
3. Reopen browser.
4. Compare events/resources/jobs before and after.

Expected:
- State progressed while no browser was open.

### S9: Unattended Collapse and Auto-Reset
1. Allow colony to run unattended/under-supplied until collapse.
   - Enable `Speed: Turbo` first to shorten depletion/resilience/critical windows.
2. Verify run reset event is created.
3. Verify run number increments.
4. Verify baseline resources/cats reset for new run.
5. Verify shared upgrades remain (meta progression preserved).

Expected:
- Colony can collapse.
- Automatic new run starts.
- Meta upgrades persist.

### S10: Concurrency and Integrity
1. Session A and B trigger actions simultaneously.
2. Attempt duplicate strategic requests from both sessions.
3. Attempt purchasing same upgrade near-simultaneously.

Expected:
- No data corruption.
- Conflicts handled with clear errors.
- Shared state remains consistent.

## Optional API-Level Spot Checks

Use Convex dashboard/query runner to verify:
- `colonies.isGlobal` is true for active colony.
- `jobs` status transitions: `queued -> active -> completed`.
- `events` include `job_queued`, `job_completed`, `ritual_ready`, `upgrade_purchased`, `run_reset`.
- `players` rows update `lastSeenAt` and click counters.

## Defect Report Template

For each defect provide:
1. Scenario ID
2. Steps to reproduce
3. Expected result
4. Actual result
5. Screenshot/video
6. Session nickname(s)
7. Timestamp
8. Severity (`blocker`, `major`, `minor`)
