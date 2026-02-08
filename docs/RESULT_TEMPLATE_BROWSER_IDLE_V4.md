# Browser Idle V4 Test Results Template

**Date:**  
**Tester:**  
**Branch/Commit:**  
**Environment:**  
- App URL:
- Convex deployment:
- Worker process:
- Screenshot artifact folder:

## Summary

| Scenario | Result (PASS/PARTIAL/FAIL/SKIPPED) | Notes |
|----------|------------------------------------|-------|
| S1: Entry + Shared World |  |  |
| S2: Short Player Actions (Off) |  |  |
| S3: Long Leader/Cat Job Chain (Off) |  |  |
| S4: Click Boost + Diminishing Returns |  |  |
| S5: Acceleration Modes |  |  |
| S6: Ritual Request + Leader Gate |  |  |
| S7: Global Upgrade Economy |  |  |
| S8: Cat Identity + Attributes Visibility |  |  |
| S9: Specialization Progression |  |  |
| S10: Worker Independence |  |  |
| S11: Unattended Collapse + Auto Reset |  |  |
| S12: Concurrency + Integrity |  |  |

## Timing Measurements

Record measured values vs expected:

| Job Kind | Speed Mode | Expected | Measured | Pass? |
|----------|------------|----------|----------|-------|
| supply_water | Off | 15s |  |  |
| supply_food | Off | 20s |  |  |
| leader_plan_hunt | Off | 30m |  |  |
| hunt_expedition | Off | 8h |  |  |
| leader_plan_house | Off | 20h |  |  |
| build_house | Off | 8h |  |  |
| ritual | Off | 6h |  |  |
| supply_water | Fast | 1s |  |  |
| supply_food | Fast | 1s |  |  |
| leader_plan_hunt | Fast | 90s |  |  |
| hunt_expedition | Fast | 24m |  |  |
| leader_plan_house | Fast | 60m |  |  |
| build_house | Fast | 24m |  |  |
| ritual | Fast | 18m |  |  |
| supply_water | Turbo | 1s |  |  |
| supply_food | Turbo | 1s |  |  |
| leader_plan_hunt | Turbo | 15s |  |  |
| hunt_expedition | Turbo | 4m |  |  |
| leader_plan_house | Turbo | 10m |  |  |
| build_house | Turbo | 4m |  |  |
| ritual | Turbo | 3m |  |  |

## Screenshot Index

List every screenshot captured:

| Scenario | File Path | What It Proves |
|----------|-----------|----------------|
| S1 |  |  |
| S2 |  |  |
| S3 |  |  |
| S4 |  |  |
| S5 |  |  |
| S6 |  |  |
| S7 |  |  |
| S8 |  |  |
| S9 |  |  |
| S10 |  |  |
| S11 |  |  |
| S12 |  |  |

## Detailed Results

### S1: Entry + Shared World
Steps run:
-  

Observed:
-  

Evidence:
-  

Result:
-  

### S2: Short Player Actions (Off)
Steps run:
-  

Observed resource deltas:
- Food:
- Water:

Evidence:
-  

Result:
-  

### S3: Long Leader/Cat Job Chain (Off)
Steps run:
-  

Observed chain:
- `leader_plan_hunt` -> `hunt_expedition`:
- `leader_plan_house` -> `build_house`:

Duplicate request handling:
-  

Result:
-  

### S4: Click Boost + Diminishing Returns
Clicks/minute window observations:
- 1-30 clicks:
- 31-60 clicks:
- 61+ clicks:

Minimum floor behavior:
-  

Result:
-  

### S5: Acceleration Modes
Mode switch checks:
- Off -> Fast:
- Fast -> Turbo:
- Turbo -> Off:

Observed effect on timings:
-  

Result:
-  

### S6: Ritual Request + Leader Gate
Request/approval flow:
-  

Safety gate behavior:
-  

Duplicate ritual request handling:
-  

Result:
-  

### S7: Global Upgrade Economy
Ritual points earned:
-  

Purchase propagation across sessions:
-  

Point deduction correctness:
-  

Result:
-  

### S8: Cat Identity + Attributes Visibility
Per-cat card fields verified:
- Species + Sprite: Yes/No
- Lineage/Coat/Eyes/Marks: Yes/No
- Skin/Accessories/Scars: Yes/No
- Role XP: Yes/No
- Full stat grid (ATK/DEF/HUNT/MED/CLEAN/BUILD/LEAD/VIS): Yes/No

Notes:
-  

Result:
-  

### S9: Specialization Progression
Role XP observations:
- Hunter:
- Architect:
- Ritualist:

Spec transitions observed:
-  

Specialization effect observed:
-  

Screenshots:
- Role XP before progression:
- Role XP near threshold:
- Spec transition (if reached):

Result:
-  

### S10: Worker Independence
Before close:
-  

After reopen:
-  

Delta:
-  

Result:
-  

### S11: Unattended Collapse + Auto Reset
Collapse trigger evidence:
-  

Run number change:
-  

Reset baseline verification:
-  

Meta persistence verification:
-  

Result:
-  

### S12: Concurrency + Integrity
Concurrent action checks:
-  

Conflict handling:
-  

Data integrity checks:
-  

Result:
-  

## Defects

For each defect:
- ID:
- Scenario:
- Severity:
- Repro steps:
- Expected:
- Actual:
- Evidence:
- Timestamp:

## Overall Assessment

- Total scenarios PASS:
- Total scenarios PARTIAL:
- Total scenarios FAIL:
- Total scenarios SKIPPED:

Release recommendation:
- `Ready`
- `Ready with caveats`
- `Not ready`
