# Testing Guide

This project uses **Test-Driven Development (TDD)**. Write tests FIRST, then implement.

## TDD Workflow

```
1. Write a failing test
2. Run tests → RED (fail)
3. Write minimum code to pass
4. Run tests → GREEN (pass)
5. Refactor if needed
6. Repeat
```

## Testing Stack

| Tool | Purpose |
|------|---------|
| Vitest | Test runner |
| @testing-library/react | React component testing |
| @testing-library/jest-dom | DOM matchers |
| jsdom | Browser environment simulation |

## Running Tests

```bash
# Run all tests once
npm test

# Run tests in watch mode (recommended during development)
npm run test:watch

# Run specific test file
npm test -- tests/unit/game/needs.test.ts

# Run tests matching a pattern
npm test -- --grep "decayNeeds"

# Run with coverage report
npm run test:coverage

# Run only changed tests
npm run test:changed
```

## Test File Organization

```
tests/
├── unit/                     # Unit tests (no external deps)
│   ├── game/                 # Game logic tests
│   │   ├── needs.test.ts
│   │   ├── age.test.ts
│   │   ├── skills.test.ts
│   │   ├── combat.test.ts
│   │   ├── catAI.test.ts
│   │   ├── tasks.test.ts
│   │   ├── paths.test.ts
│   │   └── worldResources.test.ts
│   │
│   └── components/           # Component tests
│       ├── CatSprite.test.tsx
│       ├── ResourceBar.test.tsx
│       ├── TaskCard.test.tsx
│       └── ...
│
├── integration/              # Integration tests
│   ├── convex/               # Backend integration
│   │   ├── colonies.test.ts
│   │   ├── cats.test.ts
│   │   └── gameTick.test.ts
│   │
│   └── features/             # Feature integration
│       ├── feedColony.test.ts
│       └── encounterResolution.test.ts
│
└── e2e/                      # End-to-end tests (optional)
    └── colonyLifecycle.test.ts
```

## Writing Unit Tests

### Game Logic Tests (Pure Functions)

These are the easiest to test. Functions take input, return output, no side effects.

```typescript
// tests/unit/game/needs.test.ts
import { describe, it, expect } from 'vitest'
import { decayNeeds } from '@/lib/game/needs'

describe('decayNeeds', () => {
  // Describe what the function should do
  it('should decay hunger by 5 per tick', () => {
    // Arrange: set up test data
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    
    // Act: call the function
    const result = decayNeeds(needs, 1)
    
    // Assert: check the result
    expect(result.hunger).toBe(95)
  })

  // Test edge cases
  it('should not go below 0', () => {
    const needs = { hunger: 3, thirst: 1, rest: 1, health: 100 }
    const result = decayNeeds(needs, 1)
    expect(result.hunger).toBe(0)
  })

  // Test multiple scenarios
  it('should handle multiple ticks', () => {
    const needs = { hunger: 100, thirst: 100, rest: 100, health: 100 }
    const result = decayNeeds(needs, 3)
    expect(result.hunger).toBe(85)
  })
})
```

### Component Tests

Test React components in isolation.

```typescript
// tests/unit/components/ResourceBar.test.tsx
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ResourceBar } from '@/components/ui/ResourceBar'

describe('ResourceBar', () => {
  it('should render with correct width', () => {
    render(<ResourceBar value={50} max={100} />)
    
    const bar = screen.getByRole('progressbar')
    expect(bar).toHaveStyle({ width: '50%' })
  })

  it('should show label when provided', () => {
    render(<ResourceBar value={50} max={100} label="Food" showLabel />)
    
    expect(screen.getByText('Food: 50/100')).toBeInTheDocument()
  })

  it('should apply warning color when low', () => {
    render(<ResourceBar value={10} max={100} />)
    
    const bar = screen.getByRole('progressbar')
    expect(bar).toHaveClass('bg-red-500')
  })
})
```

### Testing User Interactions

```typescript
// tests/unit/components/FeedButton.test.tsx
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { FeedButton } from '@/components/colony/FeedButton'

describe('FeedButton', () => {
  it('should call onFeed when clicked', () => {
    const handleFeed = vi.fn()
    render(<FeedButton onFeed={handleFeed} />)
    
    fireEvent.click(screen.getByRole('button', { name: /feed/i }))
    
    expect(handleFeed).toHaveBeenCalledTimes(1)
  })

  it('should be disabled during cooldown', () => {
    render(<FeedButton onFeed={() => {}} cooldownActive />)
    
    const button = screen.getByRole('button', { name: /feed/i })
    expect(button).toBeDisabled()
  })

  it('should show cooldown timer', () => {
    render(<FeedButton onFeed={() => {}} cooldownActive cooldownSeconds={5} />)
    
    expect(screen.getByText('5s')).toBeInTheDocument()
  })
})
```

## Testing Patterns

### Pattern 1: Testing Functions with Randomness

Some game functions involve randomness. Test ranges and probabilities:

```typescript
describe('calculateCombatResult', () => {
  it('should favor high stat cats most of the time', () => {
    let catWins = 0
    
    // Run many times to test probability
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(80, 80, 20)
      if (result.won) catWins++
    }
    
    // High stat cat should win ~80% of the time
    expect(catWins).toBeGreaterThan(60)
    expect(catWins).toBeLessThan(95)
  })
})
```

### Pattern 2: Testing State Over Time

Test how state changes through multiple operations:

```typescript
describe('Cat aging', () => {
  it('should progress through life stages', () => {
    const birthTime = Date.now()
    
    // At birth
    expect(getLifeStage(getAgeInHours(birthTime, birthTime))).toBe('kitten')
    
    // After 6 hours
    const sixHoursLater = birthTime + (6 * 60 * 60 * 1000)
    expect(getLifeStage(getAgeInHours(birthTime, sixHoursLater))).toBe('young')
    
    // After 24 hours
    const dayLater = birthTime + (24 * 60 * 60 * 1000)
    expect(getLifeStage(getAgeInHours(birthTime, dayLater))).toBe('adult')
  })
})
```

### Pattern 3: Testing Edge Cases

Always test boundaries:

```typescript
describe('Skill cap', () => {
  it('should not exceed 100', () => {
    expect(calculateSkillGain(99, 'hunt', true, 'young')).toBe(100)
    expect(calculateSkillGain(100, 'hunt', true, 'young')).toBe(100)
  })

  it('should not go below 0', () => {
    expect(calculateSkillLoss(2, 5)).toBe(0)
    expect(calculateSkillLoss(0, 5)).toBe(0)
  })
})
```

### Pattern 4: Testing Complex Objects

Use factories for complex test data:

```typescript
// tests/factories/cat.ts
export function createMockCat(overrides: Partial<Cat> = {}): Cat {
  return {
    _id: 'cat_123',
    colonyId: 'colony_1',
    name: 'Test Cat',
    parentIds: [null, null],
    birthTime: Date.now(),
    deathTime: null,
    stats: {
      attack: 50,
      defense: 50,
      hunting: 50,
      medicine: 50,
      cleaning: 50,
      building: 50,
      leadership: 50,
      vision: 50,
    },
    needs: {
      hunger: 100,
      thirst: 100,
      rest: 100,
      health: 100,
    },
    currentTask: null,
    position: { map: 'colony', x: 0, y: 0 },
    isPregnant: false,
    pregnancyDueTime: null,
    spriteParams: null,
    ...overrides,
  }
}

// In tests:
import { createMockCat } from '@/tests/factories/cat'

it('should return to colony when hungry', () => {
  const hungyCat = createMockCat({
    needs: { hunger: 10, thirst: 100, rest: 100, health: 100 },
    position: { map: 'world', x: 5, y: 5 }
  })
  
  const action = getAutonomousAction(hungryCat, resources, () => false)
  expect(action?.type).toBe('return_to_colony')
})
```

## Integration Tests

### Testing Convex Functions

Convex integration tests require the dev backend running:

```typescript
// tests/integration/convex/colonies.test.ts
import { ConvexReactClient } from "convex/react"
import { api } from "@/convex/_generated/api"

// These tests run against the Convex dev backend
// Start with: npx convex dev

describe('Colony mutations', () => {
  let client: ConvexReactClient

  beforeAll(() => {
    client = new ConvexReactClient(process.env.NEXT_PUBLIC_CONVEX_URL!)
  })

  afterAll(() => {
    client.close()
  })

  it('should create a colony', async () => {
    const colonyId = await client.mutation(api.colonies.createColony, {
      name: 'Test Colony'
    })
    
    expect(colonyId).toBeDefined()
    
    const colony = await client.query(api.colonies.getColony, { colonyId })
    expect(colony?.name).toBe('Test Colony')
  })
})
```

## Mocking

### Mocking Convex Hooks

```typescript
import { vi } from 'vitest'
import { useQuery, useMutation } from 'convex/react'

vi.mock('convex/react', () => ({
  useQuery: vi.fn(),
  useMutation: vi.fn(),
}))

describe('ColonyView', () => {
  beforeEach(() => {
    vi.mocked(useQuery).mockReturnValue({
      name: 'Test Colony',
      resources: { food: 50, water: 50, herbs: 10, materials: 5, blessings: 0 }
    })
  })

  it('should display colony name', () => {
    render(<ColonyView colonyId="colony_1" />)
    expect(screen.getByText('Test Colony')).toBeInTheDocument()
  })
})
```

### Mocking External Services

```typescript
import { vi } from 'vitest'

// Mock the renderer service
vi.mock('@/lib/renderer', () => ({
  fetchCatSprite: vi.fn().mockRejectedValue(new Error('Service unavailable')),
}))

describe('CatSprite with fallback', () => {
  it('should show CSS fallback when renderer fails', async () => {
    const cat = createMockCat()
    render(<CatSprite cat={cat} />)
    
    // Wait for async fallback
    await screen.findByTestId('fallback-sprite')
  })
})
```

## Test Coverage

We aim for:
- **Game logic (lib/game/)**: 90%+ coverage
- **Components**: 80%+ coverage
- **Convex functions**: 70%+ coverage (integration tests)

Run coverage report:
```bash
npm run test:coverage
```

## Debugging Tests

### Run a single test
```bash
npm test -- tests/unit/game/needs.test.ts
```

### Run with verbose output
```bash
npm test -- --reporter=verbose
```

### Debug in VS Code
Add to `.vscode/launch.json`:
```json
{
  "type": "node",
  "request": "launch",
  "name": "Debug Current Test",
  "program": "${workspaceFolder}/node_modules/vitest/vitest.mjs",
  "args": ["run", "${relativeFile}"],
  "cwd": "${workspaceFolder}"
}
```

## Common Issues

### "Cannot find module"
Make sure path aliases are configured in `vitest.config.ts`:
```typescript
resolve: {
  alias: {
    '@': path.resolve(__dirname, './'),
  },
}
```

### "document is not defined"
You're testing React components without jsdom. Add to `vitest.config.ts`:
```typescript
test: {
  environment: 'jsdom',
}
```

### "act() warnings"
Wrap state updates in act():
```typescript
import { act } from '@testing-library/react'

await act(async () => {
  fireEvent.click(button)
})
```

### Async test not waiting
Use `findBy` instead of `getBy` for async content:
```typescript
// BAD: might fail if content loads async
const element = screen.getByText('Loading...')

// GOOD: waits for element to appear
const element = await screen.findByText('Loaded!')
```





