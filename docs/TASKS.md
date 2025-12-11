# Development Tasks

This document contains all development tasks broken down for Test-Driven Development (TDD).

**How to use this document:**
1. Find a task in your assigned section
2. Read the task completely including acceptance criteria
3. Write the tests FIRST (they should fail)
4. Implement the code to make tests pass
5. Refactor if needed
6. Mark task as complete and create PR

---

## Table of Contents

- [Phase 1: Project Setup](#phase-1-project-setup)
- [Phase 2: Backend - Database Schema](#phase-2-backend---database-schema)
- [Phase 3: Backend - Game Logic (Pure Functions)](#phase-3-backend---game-logic-pure-functions)
- [Phase 4: Backend - Convex Mutations & Queries](#phase-4-backend---convex-mutations--queries)
- [Phase 5: Backend - Game Loop](#phase-5-backend---game-loop)
- [Phase 6: Frontend - Core Components](#phase-6-frontend---core-components)
- [Phase 7: Frontend - Colony Views](#phase-7-frontend---colony-views)
- [Phase 8: Frontend - User Interactions](#phase-8-frontend---user-interactions)
- [Phase 9: Integration & Polish](#phase-9-integration--polish)

---

## Phase 1: Project Setup

### SETUP-001: Initialize Next.js Project
**Assigned to:** Any developer  
**Difficulty:** Easy  
**Dependencies:** None

**Description:**
Create a new Next.js 14 project with TypeScript, Tailwind CSS, and the App Router.

**Steps:**
1. Run `npx create-next-app@latest cat_idler --typescript --tailwind --app`
2. Configure `tsconfig.json` for strict mode
3. Set up path aliases (`@/` for root)

**Acceptance Criteria:**
- [ ] `npm run dev` starts successfully
- [ ] `npm run build` completes without errors
- [ ] TypeScript strict mode enabled

**No tests needed** - this is configuration only.

---

### SETUP-002: Configure Convex
**Assigned to:** Any developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-001

**Description:**
Set up Convex backend with the project.

**Steps:**
1. Run `npm install convex`
2. Run `npx convex dev` and follow prompts
3. Create `convex/` directory structure
4. Add ConvexProvider to `app/layout.tsx`

**Acceptance Criteria:**
- [ ] `npx convex dev` connects successfully
- [ ] Convex provider wraps the app
- [ ] `.env.local` has Convex URL

**No tests needed** - this is configuration only.

---

### SETUP-003: Configure Testing Framework
**Assigned to:** Any developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-001

**Description:**
Set up Vitest and React Testing Library for TDD.

**Steps:**
1. Install: `npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom`
2. Create `vitest.config.ts`
3. Create `tests/setup.ts` for test configuration
4. Add test scripts to `package.json`

**Files to create:**

`vitest.config.ts`:
```typescript
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./tests/setup.ts'],
    globals: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './'),
    },
  },
})
```

**Acceptance Criteria:**
- [ ] `npm test` runs without errors
- [ ] Can run a sample test file
- [ ] Test coverage reporting works

**Verification Test:**
Create `tests/setup.test.ts`:
```typescript
import { describe, it, expect } from 'vitest'

describe('Test Setup', () => {
  it('should run tests', () => {
    expect(1 + 1).toBe(2)
  })
})
```

---

### SETUP-004: Create Type Definitions
**Assigned to:** Any developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-001

**Description:**
Create TypeScript type definitions for all game entities.

**File:** `types/game.ts`

**Types to define:**
- `Colony`
- `Cat`
- `Building`
- `BuildingType`
- `WorldTile`
- `TileType`
- `Encounter`
- `Task`
- `TaskType`
- `EventLog`
- `EventType`
- `CatStats`
- `CatNeeds`
- `Position`

**Acceptance Criteria:**
- [ ] All types from `docs/plan.md` are defined
- [ ] Types are exported and importable
- [ ] No TypeScript errors

**Test to write FIRST:**
```typescript
// tests/unit/types.test.ts
import { describe, it, expectTypeOf } from 'vitest'
import type { Cat, Colony, CatStats } from '@/types/game'

describe('Type Definitions', () => {
  it('Cat should have required properties', () => {
    const cat: Cat = {
      _id: 'test',
      colonyId: 'colony1',
      name: 'Whiskers',
      parentIds: [null, null],
      birthTime: Date.now(),
      deathTime: null,
      stats: { attack: 10, defense: 10, hunting: 10, medicine: 10, cleaning: 10, building: 10, leadership: 10, vision: 10 },
      needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
      currentTask: null,
      position: { map: 'colony', x: 0, y: 0 },
      isPregnant: false,
      pregnancyDueTime: null,
      spriteParams: null,
    }
    expect(cat.name).toBe('Whiskers')
  })
})
```

---

## Phase 2: Backend - Database Schema

### SCHEMA-001: Create Convex Schema
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-002, SETUP-004

**Description:**
Define the Convex database schema with all tables.

**File:** `convex/schema.ts`

**Tables to create:**
1. `colonies` - Colony data
2. `cats` - All cats (alive and dead)
3. `buildings` - Colony buildings
4. `worldTiles` - World map tiles
5. `encounters` - Active encounters
6. `tasks` - Task queue
7. `events` - Event log

**Acceptance Criteria:**
- [ ] All tables defined with correct fields
- [ ] Indexes created for common queries
- [ ] Schema validates in Convex

**Test to write FIRST:**
```typescript
// tests/unit/schema.test.ts
import { describe, it, expect } from 'vitest'

// We test that the schema compiles and types work
// Actual schema validation is done by Convex

describe('Schema Types', () => {
  it('should define all required tables', () => {
    // Import schema to ensure it compiles
    const schema = require('@/convex/schema')
    expect(schema).toBeDefined()
  })
})
```

**Implementation hint:**
```typescript
// convex/schema.ts
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  colonies: defineTable({
    name: v.string(),
    leaderId: v.union(v.id("cats"), v.null()),
    status: v.union(
      v.literal("starting"),
      v.literal("thriving"),
      v.literal("struggling"),
      v.literal("dead")
    ),
    resources: v.object({
      food: v.number(),
      water: v.number(),
      herbs: v.number(),
      materials: v.number(),
      blessings: v.number(),
    }),
    gridSize: v.number(),
    createdAt: v.number(),
    lastTick: v.number(),
    lastAttack: v.number(),
  }),
  // ... continue for other tables
});
```

---

## Phase 3: Backend - Game Logic (Pure Functions)

These are pure functions with NO database access. They take inputs and return outputs. Perfect for unit testing!

---

### LOGIC-001: Needs Decay Function
**Assigned to:** Game Logic Developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-004

**Description:**
Create a pure function that calculates needs decay for a cat.

**File:** `lib/game/needs.ts`

**Function signature:**
```typescript
function decayNeeds(currentNeeds: CatNeeds, tickDuration: number): CatNeeds
```

**Rules:**
- Hunger: -5 per tick
- Thirst: -3 per tick
- Rest: -2 per tick
- Health: unchanged (only decreases from injury)
- Minimum value is 0

**Tests to write FIRST:**
```typescript
// tests/unit/game/needs.test.ts
import { describe, it, expect } from 'vitest'
import { decayNeeds } from '@/lib/game/needs'

describe('decayNeeds', () => {
  it('should decay hunger by 5 per tick', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    const result = decayNeeds(needs, 1)
    expect(result.hunger).toBe(95)
  })

  it('should decay thirst by 3 per tick', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    const result = decayNeeds(needs, 1)
    expect(result.thirst).toBe(97)
  })

  it('should decay rest by 2 per tick', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    const result = decayNeeds(needs, 1)
    expect(result.rest).toBe(98)
  })

  it('should not decay health', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 50 }
    const result = decayNeeds(needs, 1)
    expect(result.health).toBe(50)
  })

  it('should not go below 0', () => {
    const needs = { hunger: 3, thirst: 1, rest: 1, health: 100 }
    const result = decayNeeds(needs, 1)
    expect(result.hunger).toBe(0)
    expect(result.thirst).toBe(0)
    expect(result.rest).toBe(0)
  })

  it('should handle multiple ticks', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    const result = decayNeeds(needs, 3)
    expect(result.hunger).toBe(85) // 100 - (5 * 3)
    expect(result.thirst).toBe(91) // 100 - (3 * 3)
    expect(result.rest).toBe(94)   // 100 - (2 * 3)
  })
})
```

**Acceptance Criteria:**
- [ ] All tests pass
- [ ] Function is pure (no side effects)
- [ ] Handles edge cases (0, negative)

---

### LOGIC-002: Needs Restore Functions
**Assigned to:** Game Logic Developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-004

**Description:**
Create functions to restore cat needs.

**File:** `lib/game/needs.ts`

**Functions:**
```typescript
function restoreHunger(needs: CatNeeds, amount: number): CatNeeds
function restoreThirst(needs: CatNeeds, amount: number): CatNeeds
function restoreRest(needs: CatNeeds, amount: number, hasBeds: boolean): CatNeeds
function restoreHealth(needs: CatNeeds, amount: number): CatNeeds
```

**Rules:**
- Eating: +30 hunger
- Drinking: +40 thirst
- Sleeping: +20 rest (or +30 with beds)
- Healing: +10 to +30 health
- Maximum value is 100

**Tests to write FIRST:**
```typescript
// tests/unit/game/needs.test.ts (add to existing)
describe('restoreHunger', () => {
  it('should add 30 hunger', () => {
    const needs = { hunger: 50, thirst: 100, rest: 100, health: 100 }
    const result = restoreHunger(needs, 30)
    expect(result.hunger).toBe(80)
  })

  it('should cap at 100', () => {
    const needs = { hunger: 90, thirst: 100, rest: 100, health: 100 }
    const result = restoreHunger(needs, 30)
    expect(result.hunger).toBe(100)
  })
})

describe('restoreRest', () => {
  it('should add 20 rest without beds', () => {
    const needs = { hunger: 100, thirst: 100, rest: 50, health: 100 }
    const result = restoreRest(needs, 20, false)
    expect(result.rest).toBe(70)
  })

  it('should add 30 rest with beds', () => {
    const needs = { hunger: 100, thirst: 100, rest: 50, health: 100 }
    const result = restoreRest(needs, 30, true)
    expect(result.rest).toBe(80)
  })
})
```

---

### LOGIC-003: Health Damage Functions
**Assigned to:** Game Logic Developer  
**Difficulty:** Easy  
**Dependencies:** LOGIC-001

**Description:**
Create functions that apply damage from starvation/dehydration.

**File:** `lib/game/needs.ts`

**Function:**
```typescript
function applyNeedsDamage(needs: CatNeeds): CatNeeds
```

**Rules:**
- If hunger = 0: -5 health
- If thirst = 0: -3 health
- If health = 0: cat is dead (return special value or flag)

**Tests to write FIRST:**
```typescript
describe('applyNeedsDamage', () => {
  it('should deal 5 damage when starving', () => {
    const needs = { hunger: 0, thirst: 50, rest: 50, health: 100 }
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(95)
  })

  it('should deal 3 damage when dehydrated', () => {
    const needs = { hunger: 50, thirst: 0, rest: 50, health: 100 }
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(97)
  })

  it('should deal 8 damage when both starving and dehydrated', () => {
    const needs = { hunger: 0, thirst: 0, rest: 50, health: 100 }
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(92)
  })

  it('should not damage if needs are above 0', () => {
    const needs = { hunger: 1, thirst: 1, rest: 50, health: 100 }
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(100)
  })
})
```

---

### LOGIC-004: Cat Age Calculator
**Assigned to:** Game Logic Developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-004

**Description:**
Create functions to calculate cat age and life stage.

**File:** `lib/game/age.ts`

**Functions:**
```typescript
function getAgeInHours(birthTime: number, currentTime: number): number
function getLifeStage(ageInHours: number): 'kitten' | 'young' | 'adult' | 'elder'
function getDeathChance(ageInHours: number, isLeaderOrHealer: boolean): number
```

**Rules:**
- Kitten: 0-6 hours
- Young: 6-24 hours
- Adult: 24-48 hours
- Elder: 48+ hours
- Death chance starts at 48h: 1% per tick
- +0.5% per hour after 48h
- Leaders/healers: multiply age threshold by 1.2

**Tests to write FIRST:**
```typescript
// tests/unit/game/age.test.ts
import { describe, it, expect } from 'vitest'
import { getAgeInHours, getLifeStage, getDeathChance } from '@/lib/game/age'

describe('getAgeInHours', () => {
  it('should calculate hours from birth', () => {
    const birthTime = Date.now() - (3 * 60 * 60 * 1000) // 3 hours ago
    const result = getAgeInHours(birthTime, Date.now())
    expect(result).toBeCloseTo(3, 0)
  })
})

describe('getLifeStage', () => {
  it('should return kitten for 0-6 hours', () => {
    expect(getLifeStage(0)).toBe('kitten')
    expect(getLifeStage(5)).toBe('kitten')
    expect(getLifeStage(6)).toBe('young')
  })

  it('should return young for 6-24 hours', () => {
    expect(getLifeStage(6)).toBe('young')
    expect(getLifeStage(23)).toBe('young')
  })

  it('should return adult for 24-48 hours', () => {
    expect(getLifeStage(24)).toBe('adult')
    expect(getLifeStage(47)).toBe('adult')
  })

  it('should return elder for 48+ hours', () => {
    expect(getLifeStage(48)).toBe('elder')
    expect(getLifeStage(100)).toBe('elder')
  })
})

describe('getDeathChance', () => {
  it('should return 0 for cats under 48 hours', () => {
    expect(getDeathChance(47, false)).toBe(0)
  })

  it('should return 1% at exactly 48 hours', () => {
    expect(getDeathChance(48, false)).toBe(0.01)
  })

  it('should increase by 0.5% per hour after 48', () => {
    expect(getDeathChance(50, false)).toBe(0.02) // 1% + (2 * 0.5%)
  })

  it('should use 57.6h threshold for leaders/healers', () => {
    expect(getDeathChance(48, true)).toBe(0) // 48 < 57.6
    expect(getDeathChance(58, true)).toBeCloseTo(0.01, 2)
  })
})
```

---

### LOGIC-005: Skill Learning Function
**Assigned to:** Game Logic Developer  
**Difficulty:** Medium  
**Dependencies:** LOGIC-004

**Description:**
Calculate skill XP gain from completing tasks.

**File:** `lib/game/skills.ts`

**Function:**
```typescript
function calculateSkillGain(
  currentSkill: number,
  taskType: TaskType,
  taskSuccess: boolean,
  lifeStage: LifeStage
): number
```

**Rules:**
- Base XP: 1-3 depending on task
- Success bonus: +50% if task succeeded well
- Age modifier: Young = 1.5x, Adult = 1.0x, Elder = 0.5x
- Skill cap: 100
- Returns the NEW skill value (not the gain amount)

**Task to Skill mapping:**
- hunt → hunting
- gather_herbs → medicine
- build → building
- teach → leadership
- etc.

**Tests to write FIRST:**
```typescript
// tests/unit/game/skills.test.ts
describe('calculateSkillGain', () => {
  it('should gain base XP for hunting task', () => {
    const result = calculateSkillGain(10, 'hunt', false, 'adult')
    expect(result).toBeGreaterThan(10)
    expect(result).toBeLessThanOrEqual(13) // base is 1-3
  })

  it('should gain 50% more on success', () => {
    const baseGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const successGain = calculateSkillGain(10, 'hunt', true, 'adult') - 10
    expect(successGain).toBeCloseTo(baseGain * 1.5, 1)
  })

  it('should gain 50% more as young cat', () => {
    const adultGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const youngGain = calculateSkillGain(10, 'hunt', false, 'young') - 10
    expect(youngGain).toBeCloseTo(adultGain * 1.5, 1)
  })

  it('should gain 50% less as elder', () => {
    const adultGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const elderGain = calculateSkillGain(10, 'hunt', false, 'elder') - 10
    expect(elderGain).toBeCloseTo(adultGain * 0.5, 1)
  })

  it('should cap at 100', () => {
    const result = calculateSkillGain(99, 'hunt', true, 'young')
    expect(result).toBe(100)
  })
})
```

---

### LOGIC-006: Task Assignment Quality
**Assigned to:** Game Logic Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-004

**Description:**
Determine which cat should be assigned to a task, considering leader quality.

**File:** `lib/game/tasks.ts`

**Functions:**
```typescript
function getOptimalCatForTask(cats: Cat[], taskType: TaskType): Cat | null
function getAssignedCat(
  cats: Cat[],
  taskType: TaskType,
  leadershipStat: number
): { cat: Cat | null; isOptimal: boolean }
function getAssignmentTime(leadershipStat: number): number
```

**Rules:**
- Optimal cat = highest relevant skill for task + appropriate age
- Bad leader (0-10): 40% chance wrong cat, 30s timer
- Okay leader (11-20): 20% chance wrong cat, 20s timer
- Good leader (21-30): 5% chance wrong cat, 10s timer
- Great leader (31+): 0% chance wrong cat, 5s timer

**Tests to write FIRST:**
```typescript
// tests/unit/game/tasks.test.ts
import { describe, it, expect } from 'vitest'
import { getOptimalCatForTask, getAssignedCat, getAssignmentTime } from '@/lib/game/tasks'

const mockCats = [
  { ...baseCat, name: 'Hunter', stats: { ...baseStats, hunting: 80 } },
  { ...baseCat, name: 'Healer', stats: { ...baseStats, medicine: 90 } },
  { ...baseCat, name: 'Builder', stats: { ...baseStats, building: 70 } },
]

describe('getOptimalCatForTask', () => {
  it('should pick cat with highest hunting for hunt task', () => {
    const result = getOptimalCatForTask(mockCats, 'hunt')
    expect(result?.name).toBe('Hunter')
  })

  it('should pick cat with highest medicine for heal task', () => {
    const result = getOptimalCatForTask(mockCats, 'heal')
    expect(result?.name).toBe('Healer')
  })

  it('should return null if no cats available', () => {
    const result = getOptimalCatForTask([], 'hunt')
    expect(result).toBeNull()
  })
})

describe('getAssignmentTime', () => {
  it('should return 30s for bad leader (0-10)', () => {
    expect(getAssignmentTime(5)).toBe(30)
    expect(getAssignmentTime(10)).toBe(30)
  })

  it('should return 20s for okay leader (11-20)', () => {
    expect(getAssignmentTime(15)).toBe(20)
  })

  it('should return 10s for good leader (21-30)', () => {
    expect(getAssignmentTime(25)).toBe(10)
  })

  it('should return 5s for great leader (31+)', () => {
    expect(getAssignmentTime(35)).toBe(5)
    expect(getAssignmentTime(100)).toBe(5)
  })
})
```

---

### LOGIC-007: Combat Resolution
**Assigned to:** Game Logic Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-004

**Description:**
Calculate combat outcomes between cats and enemies.

**File:** `lib/game/combat.ts`

**Functions:**
```typescript
function calculateCombatResult(
  catAttack: number,
  catDefense: number,
  enemyStrength: number
): { won: boolean; damage: number }

function getClicksNeeded(
  enemyStrength: number,
  colonyDefense: number,
  catVision: number
): number
```

**Rules:**
- Cat roll: attack + defense + random(1-20)
- Enemy roll: strength + random(1-20)
- Cat wins if cat roll > enemy roll
- Damage on loss: 30-70 based on how much they lost by
- Clicks needed: base_clicks * (1 - colonyDefense/100) * (1 - catVision/200)

**Tests to write FIRST:**
```typescript
// tests/unit/game/combat.test.ts
describe('calculateCombatResult', () => {
  // Note: combat has randomness, so we test ranges
  
  it('should favor high stat cats', () => {
    let catWins = 0
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(80, 80, 20)
      if (result.won) catWins++
    }
    // High stat cat should win most of the time
    expect(catWins).toBeGreaterThan(70)
  })

  it('should return damage between 30-70 on loss', () => {
    // Force a loss scenario
    const result = calculateCombatResult(1, 1, 100)
    if (!result.won) {
      expect(result.damage).toBeGreaterThanOrEqual(30)
      expect(result.damage).toBeLessThanOrEqual(70)
    }
  })
})

describe('getClicksNeeded', () => {
  it('should return base clicks with no modifiers', () => {
    const result = getClicksNeeded(50, 0, 0)
    expect(result).toBe(50)
  })

  it('should reduce clicks with colony defense', () => {
    const result = getClicksNeeded(50, 50, 0)
    expect(result).toBe(25) // 50 * 0.5
  })

  it('should reduce clicks with cat vision', () => {
    const result = getClicksNeeded(50, 0, 100)
    expect(result).toBe(25) // 50 * 0.5
  })

  it('should stack both modifiers', () => {
    const result = getClicksNeeded(100, 50, 100)
    expect(result).toBe(25) // 100 * 0.5 * 0.5
  })
})
```

---

### LOGIC-008: Cat Autonomous Behavior
**Assigned to:** Game Logic Developer  
**Difficulty:** Hard  
**Dependencies:** LOGIC-001, LOGIC-002, LOGIC-004

**Description:**
Determine what a cat should do autonomously based on needs.

**File:** `lib/game/catAI.ts`

**Function:**
```typescript
function getAutonomousAction(
  cat: Cat,
  colonyResources: Resources,
  colonyHasBuilding: (type: BuildingType) => boolean
): AutonomousAction | null

type AutonomousAction = 
  | { type: 'eat' }
  | { type: 'drink' }
  | { type: 'sleep'; position: Position }
  | { type: 'return_to_colony' }
  | { type: 'flee'; from: Position }
```

**Rules:**
1. If hunger < 30 AND food available → eat
2. If thirst < 40 AND water available → drink
3. If rest < 20 → sleep (in beds if available)
4. If any need < 15 AND on world map → return to colony
5. Otherwise → null (continue current task)

**Tests to write FIRST:**
```typescript
// tests/unit/game/catAI.test.ts
describe('getAutonomousAction', () => {
  const baseCat = { 
    needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
    position: { map: 'colony', x: 0, y: 0 }
  }

  it('should eat when hungry and food available', () => {
    const cat = { ...baseCat, needs: { ...baseCat.needs, hunger: 25 } }
    const resources = { food: 10, water: 10, herbs: 0, materials: 0, blessings: 0 }
    const result = getAutonomousAction(cat, resources, () => false)
    expect(result?.type).toBe('eat')
  })

  it('should not eat when hungry but no food', () => {
    const cat = { ...baseCat, needs: { ...baseCat.needs, hunger: 25 } }
    const resources = { food: 0, water: 10, herbs: 0, materials: 0, blessings: 0 }
    const result = getAutonomousAction(cat, resources, () => false)
    expect(result?.type).not.toBe('eat')
  })

  it('should prioritize eating over drinking', () => {
    const cat = { ...baseCat, needs: { hunger: 20, thirst: 30, rest: 100, health: 100 } }
    const resources = { food: 10, water: 10, herbs: 0, materials: 0, blessings: 0 }
    const result = getAutonomousAction(cat, resources, () => false)
    expect(result?.type).toBe('eat')
  })

  it('should return to colony when needs critical and on world map', () => {
    const cat = { 
      ...baseCat, 
      needs: { hunger: 10, thirst: 50, rest: 50, health: 100 },
      position: { map: 'world', x: 5, y: 5 }
    }
    const resources = { food: 0, water: 0, herbs: 0, materials: 0, blessings: 0 }
    const result = getAutonomousAction(cat, resources, () => false)
    expect(result?.type).toBe('return_to_colony')
  })

  it('should return null when all needs are fine', () => {
    const result = getAutonomousAction(baseCat, { food: 10, water: 10, herbs: 0, materials: 0, blessings: 0 }, () => false)
    expect(result).toBeNull()
  })
})
```

---

### LOGIC-009: Path System Calculations
**Assigned to:** Game Logic Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-004

**Description:**
Calculate path wear and travel effects.

**File:** `lib/game/paths.ts`

**Functions:**
```typescript
function addPathWear(currentWear: number, amount: number): number
function decayPathWear(currentWear: number): number
function getPathSpeedBonus(pathWear: number): number
function getPathDangerReduction(pathWear: number): number
```

**Rules:**
- Wear 0-29: No bonus
- Wear 30-59: +10% speed, -25% danger
- Wear 60-89: +25% speed, -60% danger
- Wear 90-100: +40% speed, no random encounters
- Decay: -1 per hour if unused

**Tests to write FIRST:**
```typescript
// tests/unit/game/paths.test.ts
describe('getPathSpeedBonus', () => {
  it('should return 0 for wear < 30', () => {
    expect(getPathSpeedBonus(0)).toBe(0)
    expect(getPathSpeedBonus(29)).toBe(0)
  })

  it('should return 0.1 for wear 30-59', () => {
    expect(getPathSpeedBonus(30)).toBe(0.1)
    expect(getPathSpeedBonus(59)).toBe(0.1)
  })

  it('should return 0.25 for wear 60-89', () => {
    expect(getPathSpeedBonus(60)).toBe(0.25)
  })

  it('should return 0.4 for wear 90+', () => {
    expect(getPathSpeedBonus(90)).toBe(0.4)
    expect(getPathSpeedBonus(100)).toBe(0.4)
  })
})

describe('getPathDangerReduction', () => {
  it('should return 0 for wear < 30', () => {
    expect(getPathDangerReduction(29)).toBe(0)
  })

  it('should return 0.25 for wear 30-59', () => {
    expect(getPathDangerReduction(30)).toBe(0.25)
  })

  it('should return 0.6 for wear 60-89', () => {
    expect(getPathDangerReduction(60)).toBe(0.6)
  })

  it('should return 1.0 for wear 90+ (no encounters)', () => {
    expect(getPathDangerReduction(90)).toBe(1.0)
  })
})
```

---

### LOGIC-010: World Tile Resource Management
**Assigned to:** Game Logic Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-004

**Description:**
Handle resource depletion and regeneration on world tiles.

**File:** `lib/game/worldResources.ts`

**Functions:**
```typescript
function harvestResources(
  tile: WorldTile,
  harvestType: 'food' | 'herbs' | 'water',
  catHuntingSkill: number
): { harvested: number; newTile: WorldTile }

function regenerateResources(
  tile: WorldTile,
  hoursSinceDepletion: number
): WorldTile

function isTileDepleted(tile: WorldTile): boolean
```

**Rules:**
- Harvest amount: 1 + (skill / 50) rounded down
- Resource goes to 0 = depleted
- Regeneration: 10% of max per hour, full at 6 hours
- Rivers have infinite water

**Tests to write FIRST:**
```typescript
// tests/unit/game/worldResources.test.ts
describe('harvestResources', () => {
  const baseTile = {
    type: 'forest',
    resources: { food: 50, herbs: 0, water: 0 },
    maxResources: { food: 100, herbs: 0 }
  }

  it('should harvest 1 food with 0 skill', () => {
    const result = harvestResources(baseTile, 'food', 0)
    expect(result.harvested).toBe(1)
    expect(result.newTile.resources.food).toBe(49)
  })

  it('should harvest 2 food with 50+ skill', () => {
    const result = harvestResources(baseTile, 'food', 50)
    expect(result.harvested).toBe(2)
  })

  it('should harvest 3 food with 100 skill', () => {
    const result = harvestResources(baseTile, 'food', 100)
    expect(result.harvested).toBe(3)
  })

  it('should not harvest more than available', () => {
    const lowTile = { ...baseTile, resources: { food: 1, herbs: 0, water: 0 } }
    const result = harvestResources(lowTile, 'food', 100)
    expect(result.harvested).toBe(1)
    expect(result.newTile.resources.food).toBe(0)
  })
})

describe('regenerateResources', () => {
  const depletedTile = {
    type: 'forest',
    resources: { food: 0, herbs: 0, water: 0 },
    maxResources: { food: 100, herbs: 50 }
  }

  it('should regenerate 10% per hour', () => {
    const result = regenerateResources(depletedTile, 1)
    expect(result.resources.food).toBe(10)
    expect(result.resources.herbs).toBe(5)
  })

  it('should fully regenerate at 6 hours', () => {
    const result = regenerateResources(depletedTile, 6)
    expect(result.resources.food).toBe(100)
    expect(result.resources.herbs).toBe(50)
  })
})
```

---

## Phase 4: Backend - Convex Mutations & Queries

### CONVEX-001: Colony CRUD Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001

**Description:**
Create Convex mutations and queries for colony management.

**File:** `convex/colonies.ts`

**Functions to implement:**
```typescript
// Queries
query getColony(colonyId: Id<"colonies">): Colony | null
query getActiveColony(): Colony | null
query getAllColonies(): Colony[]

// Mutations
mutation createColony(name: string): Id<"colonies">
mutation updateColonyResources(colonyId, resources): void
mutation setColonyLeader(colonyId, catId): void
mutation updateColonyStatus(colonyId, status): void
```

**Tests:**
Since Convex functions require a running backend, we test with integration tests:
```typescript
// tests/integration/colonies.test.ts
// These run against the Convex dev backend
```

**Acceptance Criteria:**
- [ ] Can create a new colony
- [ ] Can retrieve colony by ID
- [ ] Can update resources
- [ ] Can change leader
- [ ] Can update status

---

### CONVEX-002: Cat CRUD Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001, CONVEX-001

**Description:**
Create Convex mutations and queries for cat management.

**File:** `convex/cats.ts`

**Functions:**
```typescript
// Queries
query getCat(catId): Cat | null
query getCatsByColony(colonyId): Cat[]
query getAliveCats(colonyId): Cat[]
query getDeadCats(colonyId): Cat[]

// Mutations
mutation createCat(colonyId, name, stats, parentIds?): Id<"cats">
mutation updateCatNeeds(catId, needs): void
mutation updateCatPosition(catId, position): void
mutation assignTask(catId, taskType): void
mutation killCat(catId): void
```

---

### CONVEX-003: Building Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001, CONVEX-001

**File:** `convex/buildings.ts`

**Functions:**
```typescript
query getBuildingsByColony(colonyId): Building[]
query getBuildingAt(colonyId, position): Building | null

mutation placeBuilding(colonyId, type, position): Id<"buildings">
mutation progressConstruction(buildingId, amount): void
mutation upgradeBuilding(buildingId): void
```

---

### CONVEX-004: Task Queue Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001, CONVEX-001, CONVEX-002

**File:** `convex/tasks.ts`

**Functions:**
```typescript
query getTasksByColony(colonyId): Task[]
query getPendingTasks(colonyId): Task[]

mutation createTask(colonyId, type, priority): Id<"tasks">
mutation assignCatToTask(taskId, catId, isOptimal): void
mutation progressTask(taskId, amount): void
mutation completeTask(taskId): void
mutation speedUpAssignment(taskId): void // User intervention
mutation reassignTask(taskId): void // User fix
```

---

### CONVEX-005: World Map Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001

**File:** `convex/worldMap.ts`

**Functions:**
```typescript
query getWorldTiles(colonyId): WorldTile[]
query getTileAt(colonyId, x, y): WorldTile | null
query getVisibleTiles(colonyId, catVision): WorldTile[]

mutation initializeWorldMap(colonyId): void
mutation harvestTile(tileId, harvestType, catSkill): number
mutation addPathWear(tileId, amount): void
mutation regenerateTiles(colonyId): void // Called by cron
```

---

### CONVEX-006: Encounter Operations
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** SCHEMA-001, CONVEX-002

**File:** `convex/encounters.ts`

**Functions:**
```typescript
query getActiveEncounters(colonyId): Encounter[]
query getEncounterByCat(catId): Encounter | null

mutation createEncounter(catId, type, enemyType, position): Id<"encounters">
mutation addClicks(encounterId, clicks): void
mutation resolveEncounter(encounterId): { won: boolean; loot?: number }
mutation autoResolveExpired(): void // Called by game tick
```

---

### CONVEX-007: Event Log Operations
**Assigned to:** Backend Developer  
**Difficulty:** Easy  
**Dependencies:** SCHEMA-001

**File:** `convex/events.ts`

**Functions:**
```typescript
query getEventsByColony(colonyId, limit?): EventLog[]
query getEventsByCat(catId): EventLog[]

mutation logEvent(colonyId, type, message, catIds?, metadata?): void
```

---

## Phase 5: Backend - Game Loop

### TICK-001: Main Game Tick
**Assigned to:** Backend Developer  
**Difficulty:** Hard  
**Dependencies:** All CONVEX tasks, All LOGIC tasks

**Description:**
Create the main game loop that runs every 10 seconds.

**File:** `convex/gameTick.ts`

**Function:**
```typescript
mutation gameTick(colonyId: Id<"colonies">): void
```

**Order of operations:**
1. Decay all cat needs
2. Apply damage from starvation/dehydration
3. Process autonomous cat behaviors
4. Progress active tasks
5. Check leader task assignments
6. Process random encounter chances
7. Update cat ages
8. Check for deaths (age, health)
9. Process breeding (if applicable)
10. Update colony status

**Acceptance Criteria:**
- [ ] Tick processes all cats
- [ ] Needs decay correctly
- [ ] Deaths are recorded
- [ ] Tasks progress
- [ ] Events are logged

---

### TICK-002: Scheduled Cron Job
**Assigned to:** Backend Developer  
**Difficulty:** Easy  
**Dependencies:** TICK-001

**File:** `convex/crons.ts`

**Setup:**
```typescript
import { cronJobs } from "convex/server";

const crons = cronJobs();

crons.interval(
  "game tick",
  { seconds: 10 },
  internal.gameTick.tickAllColonies
);

export default crons;
```

---

### TICK-003: Resource Regeneration Job
**Assigned to:** Backend Developer  
**Difficulty:** Easy  
**Dependencies:** CONVEX-005

**Description:**
Scheduled job to regenerate world resources every 10 minutes.

---

### TICK-004: Major Attack Scheduler
**Assigned to:** Backend Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-006

**Description:**
Trigger major attacks every 4-8 hours.

---

## Phase 6: Frontend - Core Components

### UI-001: CatSprite Component
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-003

**Description:**
Create a component that displays cat sprites with fallback.

**File:** `components/colony/CatSprite.tsx`

**Props:**
```typescript
interface CatSpriteProps {
  cat: Cat;
  size?: 'small' | 'medium' | 'large';
  animated?: boolean;
  onClick?: () => void;
}
```

**Behavior:**
1. Try to fetch from renderer service
2. If unavailable, show CSS fallback
3. Apply idle animation (breathing, tail)
4. Support walking animation between positions

**Tests to write FIRST:**
```typescript
// tests/unit/components/CatSprite.test.tsx
import { render, screen } from '@testing-library/react'
import { CatSprite } from '@/components/colony/CatSprite'

describe('CatSprite', () => {
  const mockCat = { name: 'Whiskers', spriteParams: null }

  it('should render cat name as alt text', () => {
    render(<CatSprite cat={mockCat} />)
    expect(screen.getByAltText('Whiskers')).toBeInTheDocument()
  })

  it('should show fallback when no sprite params', () => {
    render(<CatSprite cat={mockCat} />)
    expect(screen.getByTestId('fallback-sprite')).toBeInTheDocument()
  })

  it('should apply size class', () => {
    render(<CatSprite cat={mockCat} size="large" />)
    expect(screen.getByTestId('cat-sprite')).toHaveClass('size-large')
  })

  it('should be clickable when onClick provided', () => {
    const handleClick = vi.fn()
    render(<CatSprite cat={mockCat} onClick={handleClick} />)
    screen.getByTestId('cat-sprite').click()
    expect(handleClick).toHaveBeenCalled()
  })
})
```

---

### UI-002: ResourceBar Component
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  
**Dependencies:** SETUP-003

**File:** `components/ui/ResourceBar.tsx`

**Props:**
```typescript
interface ResourceBarProps {
  value: number;
  max: number;
  color?: 'green' | 'blue' | 'red' | 'yellow';
  showLabel?: boolean;
  label?: string;
}
```

**Tests to write FIRST:**
```typescript
describe('ResourceBar', () => {
  it('should display correct percentage width', () => {
    render(<ResourceBar value={50} max={100} />)
    const bar = screen.getByRole('progressbar')
    expect(bar).toHaveStyle({ width: '50%' })
  })

  it('should show label when showLabel is true', () => {
    render(<ResourceBar value={50} max={100} showLabel label="Food" />)
    expect(screen.getByText('Food: 50/100')).toBeInTheDocument()
  })

  it('should change color based on value', () => {
    render(<ResourceBar value={10} max={100} />)
    expect(screen.getByRole('progressbar')).toHaveClass('bg-red')
  })
})
```

---

### UI-003: NeedsDisplay Component
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  
**Dependencies:** UI-002

**File:** `components/colony/NeedsDisplay.tsx`

Shows hunger, thirst, rest, health for a cat.

---

### UI-004: TaskCard Component
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-003

**File:** `components/colony/TaskCard.tsx`

**Props:**
```typescript
interface TaskCardProps {
  task: Task;
  assignedCat?: Cat;
  onSpeedUp: () => void;
  onReassign: () => void;
}
```

Shows task with countdown, assigned cat, warning if bad assignment.

---

### UI-005: EncounterPopup Component
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-003

**File:** `components/colony/EncounterPopup.tsx`

**Props:**
```typescript
interface EncounterPopupProps {
  encounter: Encounter;
  cat: Cat;
  onClickDefend: () => void;
}
```

Shows danger popup with click progress bar.

---

## Phase 7: Frontend - Colony Views

### VIEW-001: Colony Grid View
**Assigned to:** Frontend Developer  
**Difficulty:** Hard  
**Dependencies:** UI-001, UI-002

**File:** `components/colony/ColonyGrid.tsx`

**Description:**
Display the colony interior as a grid with buildings and cats.

**Features:**
- Grid of 3x3, 5x5, or 8x8
- Buildings displayed in cells
- Cats shown in their positions
- Click cell to select/build
- Drag cats to move them

---

### VIEW-002: World Map View
**Assigned to:** Frontend Developer  
**Difficulty:** Hard  
**Dependencies:** UI-001

**File:** `components/colony/WorldMap.tsx`

**Description:**
Display the 16x16 world map with fog of war.

**Features:**
- Tile types with different visuals
- Resource indicators
- Path visualization
- Cat positions
- Encounter markers ("!")
- Fog for unexplored areas

---

### VIEW-003: Task Queue Panel
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** UI-004

**File:** `components/colony/TaskQueue.tsx`

**Description:**
Sims-style task list showing all pending and active tasks.

---

### VIEW-004: Event Log Panel
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** SETUP-003

**File:** `components/colony/EventLog.tsx`

**Description:**
Scrollable log of colony events with filtering.

---

### VIEW-005: Lineage Tree
**Assigned to:** Frontend Developer  
**Difficulty:** Hard  
**Dependencies:** UI-001

**File:** `components/colony/LineageTree.tsx`

**Description:**
Interactive family tree showing cat relationships.

---

## Phase 8: Frontend - User Interactions

### ACTION-001: Feed Colony Button
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  
**Dependencies:** CONVEX-001

**File:** `components/colony/FeedButton.tsx`

10-second cooldown button that adds +1 food.

---

### ACTION-002: Heal Cat Click Handler
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  
**Dependencies:** CONVEX-002

5-second cooldown per cat, +10 health on click.

---

### ACTION-003: Encounter Defense Handler
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-006, UI-005

Click to progress encounter resolution.

---

### ACTION-004: Task Speed-Up Handler
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  
**Dependencies:** CONVEX-004

Instant assignment when clicked.

---

### ACTION-005: Blessing Sacrifice UI
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-001

UI to sacrifice resources for blessings.

---

### ACTION-006: Building Placement UI
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-003, VIEW-001

UI to select and place new buildings.

---

## Phase 9: Integration & Polish

### INTEGRATE-001: Main Colony Page
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** All VIEW tasks

**File:** `app/colony/[id]/page.tsx`

Combine all views into the main page.

---

### INTEGRATE-002: Colony Selection/Creation
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-001

**File:** `app/page.tsx`

Landing page to view/create/select colonies.

---

### INTEGRATE-003: New Colony Flow
**Assigned to:** Full Stack Developer  
**Difficulty:** Medium  
**Dependencies:** CONVEX-001, CONVEX-002

Flow for naming colony and selecting starting leader.

---

### POLISH-001: Loading States
**Assigned to:** Frontend Developer  
**Difficulty:** Easy  

Add loading skeletons for all async data.

---

### POLISH-002: Error Handling
**Assigned to:** Full Stack Developer  
**Difficulty:** Medium  

Handle and display errors gracefully.

---

### POLISH-003: Mobile Responsiveness
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  

Ensure UI works on mobile devices.

---

### POLISH-004: Animation Polish
**Assigned to:** Frontend Developer  
**Difficulty:** Medium  

Add micro-interactions and polish animations.

---

## Task Tracking

Use this checklist to track progress:

### Phase 1: Setup
- [ ] SETUP-001: Initialize Next.js
- [ ] SETUP-002: Configure Convex
- [ ] SETUP-003: Configure Testing
- [ ] SETUP-004: Create Types

### Phase 2: Schema
- [ ] SCHEMA-001: Convex Schema

### Phase 3: Game Logic
- [ ] LOGIC-001: Needs Decay
- [ ] LOGIC-002: Needs Restore
- [ ] LOGIC-003: Health Damage
- [ ] LOGIC-004: Age Calculator
- [ ] LOGIC-005: Skill Learning
- [ ] LOGIC-006: Task Assignment
- [ ] LOGIC-007: Combat Resolution
- [ ] LOGIC-008: Cat AI
- [ ] LOGIC-009: Path System
- [ ] LOGIC-010: World Resources

### Phase 4: Convex Backend
- [ ] CONVEX-001: Colony CRUD
- [ ] CONVEX-002: Cat CRUD
- [ ] CONVEX-003: Building Ops
- [ ] CONVEX-004: Task Queue
- [ ] CONVEX-005: World Map
- [ ] CONVEX-006: Encounters
- [ ] CONVEX-007: Event Log

### Phase 5: Game Loop
- [ ] TICK-001: Main Tick
- [ ] TICK-002: Cron Setup
- [ ] TICK-003: Resource Regen
- [ ] TICK-004: Attack Scheduler

### Phase 6: UI Components
- [ ] UI-001: CatSprite
- [ ] UI-002: ResourceBar
- [ ] UI-003: NeedsDisplay
- [ ] UI-004: TaskCard
- [ ] UI-005: EncounterPopup

### Phase 7: Views
- [ ] VIEW-001: Colony Grid
- [ ] VIEW-002: World Map
- [ ] VIEW-003: Task Queue
- [ ] VIEW-004: Event Log
- [ ] VIEW-005: Lineage Tree

### Phase 8: Actions
- [ ] ACTION-001: Feed Button
- [ ] ACTION-002: Heal Click
- [ ] ACTION-003: Encounter Defense
- [ ] ACTION-004: Task Speed-Up
- [ ] ACTION-005: Blessing UI
- [ ] ACTION-006: Building UI

### Phase 9: Integration
- [ ] INTEGRATE-001: Main Page
- [ ] INTEGRATE-002: Colony Selection
- [ ] INTEGRATE-003: New Colony Flow
- [ ] POLISH-001: Loading States
- [ ] POLISH-002: Error Handling
- [ ] POLISH-003: Mobile
- [ ] POLISH-004: Animations

