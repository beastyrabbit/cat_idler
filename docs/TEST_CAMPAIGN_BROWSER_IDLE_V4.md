# Browser Idle V4 Test Campaign

This campaign is for an AI/human tester with browser access.  
Target branch: `feat/browser-idle-rework`

## Goal

Validate the current browser idle game end-to-end, including:
- Shared global world (all players in one colony)
- Unified backend tick loop with different action durations
- Short player-triggered jobs vs long leader/cat autonomous jobs
- Click boost behavior and diminishing returns
- Ritual economy and global upgrades
- Cat specialization progression
- Cat identity/appearance attributes in UI
- Worker independence and unattended reset behavior

## Environment Setup

Run in separate terminals:

```bash
bun install
bun run convex:dev
bun run dev
bun run dev:worker
```

Open:
- `http://localhost:3000/` (must redirect to `/game`)
- `http://localhost:3000/game?test=1` for acceleration controls

## Test Accounts

Use at least 2 distinct sessions:
1. Session A nickname: `QA-A`
2. Session B nickname: `QA-B`

Use separate browser profiles/incognito so each has unique localStorage.

## Timing Reference (Expected Baseline)

`Speed: Off`:
- `supply_food`: 20s
- `supply_water`: 15s
- `leader_plan_hunt`: 30m
- `hunt_expedition`: 8h
- `leader_plan_house`: 20h
- `build_house`: 8h
- `ritual`: 6h

`Speed: Fast` (timeScale 20):
- supply jobs: 1s
- `leader_plan_hunt`: 90s
- `hunt_expedition`: 24m
- `leader_plan_house`: 60m
- `build_house`: 24m
- `ritual`: 18m

`Speed: Turbo` (timeScale 120):
- supply jobs: 1s
- `leader_plan_hunt`: 15s
- `hunt_expedition`: 4m
- `leader_plan_house`: 10m
- `build_house`: 4m
- `ritual`: 3m

## Pass/Fail Rules

- A scenario is PASS only if all expected outcomes are observed.
- Record measured values (ETA, resource deltas, points, run number changes).
- Capture screenshot/evidence for every scenario (not only failures).
- For each scenario, attach at least one screenshot with visible timestamp/system clock.

## Screenshot Requirements

Store screenshots under a single folder, for example:
- `artifacts/browser-idle-v4/`

Use naming convention:
- `S{scenario}-{short-label}-{YYYYMMDD-HHMMSS}.png`
- Example: `S8-cat-card-20260208-174530.png`

Minimum required screenshots:
- S1: redirect result and shared colony header in both sessions
- S2: job start and post-completion resource delta
- S3: planning -> execution chain plus duplicate-block error message
- S4: click boost before/after ETA (include at least one 31+ clicks/min example)
- S5: Off/Fast/Turbo labels + one job ETA per mode
- S6: ritual request event and leader approval event
- S7: points before buy, after buy, and upgrade level change
- S8: at least 2 cat cards showing full identity/attribute sections
- S9: role XP progression over time (and specialization transition if reached)
- S10: before close vs after reopen state
- S11: run reset evidence and meta persistence evidence
- S12: concurrent actions reflected in both sessions

## Scenarios

### S1: Entry + Shared World
1. Open `/`.
2. Confirm redirect to `/game`.
3. Open Session A and B.
4. Confirm same colony name and run number in both sessions.
5. Confirm online count reflects both sessions.

Expected:
- Single shared colony.
- Real-time shared state between sessions.

### S2: Short Player Actions (Off)
1. Ensure `Speed: Off`.
2. Trigger `+ Supply Water (15s)` in Session A.
3. Verify job appears immediately and completes near 15s.
4. Verify water increases by +8 on completion.
5. Repeat with `+ Supply Food (20s)` and verify +8 food.

Expected:
- Short jobs are immediate to request and finish quickly.
- Correct resource rewards are applied.

### S3: Long Leader/Cat Job Chain (Off)
1. Ensure `Speed: Off`.
2. Trigger `Request Hunt (plan + expedition)`.
3. Confirm `leader_plan_hunt` appears (~30m ETA).
4. After plan completes (or in accelerated follow-up), confirm it queues/starts `hunt_expedition` (~8h).
5. Trigger `Request House Build`.
6. Confirm `leader_plan_house` (~20h) then `build_house` (~8h).
7. Attempt duplicate strategic request while one is active.

Expected:
- Strategic jobs are long-running.
- Planning jobs chain into execution jobs.
- Duplicate strategic requests are blocked with clear error text.

### S4: Click Boost + Diminishing Returns
1. With an active long job, click `Boost -10s` repeatedly.
2. Verify ETA reduction after each click.
3. Continue until past 30 clicks in 60s, then past 60 clicks in 60s.
4. Verify job end time never drops below near-immediate floor.

Expected:
- Boost works every click.
- Diminishing returns are visible after click thresholds.
- Job still has a minimum time floor.

### S5: Acceleration Modes
1. Open `/game?test=1`.
2. Set `Speed: Fast`, then queue one short and one long job.
3. Verify durations align with the Fast timing reference.
4. Set `Speed: Turbo`, queue fresh jobs, verify Turbo timings.
5. Set `Speed: Off` and verify timings return to baseline.

Expected:
- Current mode label updates.
- Job durations scale correctly by mode.
- Resource decay/collapse-related behavior is visibly faster in Fast/Turbo.

### S6: Ritual Request + Leader Gate
1. Click `Request Ritual`.
2. Confirm request acknowledgment event appears.
3. If food/water low, verify ritual does not start immediately.
4. Raise resources if needed.
5. Confirm leader approves and ritual job appears.
6. Attempt duplicate ritual request while pending/active.

Expected:
- Ritual request is player-triggered but leader-scheduled.
- Ritual starts only under safe conditions.
- Duplicate ritual requests are blocked.

### S7: Global Upgrade Economy
1. Complete rituals until Ritual Points > 0 (use Fast/Turbo).
2. Buy one upgrade in Session A.
3. Confirm Session B sees new level quickly.
4. Confirm shared ritual points decrease.
5. Validate upgrade button disable states (not enough points / maxed).

Expected:
- Upgrades are truly global.
- Shared points are consumed exactly once per purchase.

### S8: Cat Identity + Attributes Visibility
1. In `Leader + Cats`, inspect each cat card.
2. Verify these lines are present:
   - Species + Sprite
   - Lineage/Coat/Eyes/Marks
   - Skin/Accessories/Scars
   - Role XP
3. Verify full stat grid is shown:
   - ATK, DEF, HUNT, MED, CLEAN, BUILD, LEAD, VIS
4. Complete relevant jobs and confirm role XP increments on assigned cats.

Expected:
- Cat identity and appearance metadata are visible (not blank/missing).
- Full attribute block is visible for each cat.
- Role XP changes after completed role jobs.

### S9: Specialization Progression
1. Run repeated hunt/build/ritual completions for the same cats (Fast/Turbo recommended).
2. Track `Spec:` field transitions from `none` to role specialization.
3. Verify once specialized, the cat remains specialized.
4. Verify specialization impact:
   - Hunter: better hunt speed/reward profile over time.
   - Architect: faster build execution profile.
   - Ritualist: faster ritual profile.
5. If specialization is not reached in-session, attach screenshots proving role XP climbed but stayed `< 10`, and mark scenario `PARTIAL`.

Expected:
- Specialization appears only after enough role XP.
- Specialization persists and affects outcomes.

### S10: Worker Independence
1. Note active jobs/resources.
2. Close all browser tabs.
3. Keep backend + worker running for at least 2 minutes.
4. Reopen `/game`.
5. Compare ETA/resource/event progression.

Expected:
- World progresses with no browser open.
- ETAs continue decreasing while offline.

### S11: Unattended Collapse + Auto Reset
1. Enable `Speed: Turbo`.
2. Let resources deplete and leave game unattended long enough for collapse path.
3. Confirm run reset event appears.
4. Confirm run number increments.
5. Confirm baseline colony state is restored for new run.
6. Confirm global upgrade rows/levels persist across reset.

Expected:
- Collapse and reset happen automatically.
- Meta progression persists while run state resets.

### S12: Concurrency + Integrity
1. Trigger simultaneous actions in both sessions.
2. Try near-simultaneous strategic requests and upgrade buys.
3. Verify no duplicate strategic jobs slip through.
4. Verify no corrupted resource/points totals.

Expected:
- Conflict handling is deterministic and safe.
- Shared state stays consistent under concurrency.

## Defect Report Format

For each defect, include:
1. Scenario ID
2. Repro steps
3. Expected result
4. Actual result
5. Evidence (screenshot/video/log)
6. Session(s)
7. Timestamp
8. Severity (`blocker`, `major`, `minor`)
