# Cat Colony Idle Game

A real-time idle game where a cat colony runs autonomously. Users can help by providing food, defending against intruders, and building upgrades. The colony lives and dies based on how well it's managed - but it also runs completely on its own when no one is watching.

## Quick Start

```bash
# Install dependencies
bun install

# Set up Convex (Terminal 1)
bun run convex:dev

# Start frontend (Terminal 2)
bun run dev

# Run tests (Terminal 3)
bun run test:unit        # Unit tests
bun run test:e2e         # E2E tests (Selenium)
```

## Documentation

- **[Game Design](./docs/plan.md)** - Complete game mechanics
- **[Development Tasks](./docs/TASKS.md)** - Task breakdown
- **[Testing Guide](./docs/TESTING.md)** - How to write tests

## Testing

### Unit Tests
```bash
bun run test:unit
```
Tests all pure game logic functions.

### E2E Tests (Selenium)
```bash
bun run test:e2e
```
Tests the actual GUI in a real browser.

## Tech Stack

- **Frontend:** Next.js 14, React, Tailwind CSS
- **Backend:** Convex (serverless + real-time database)
- **Testing:** Vitest (unit), Selenium (E2E)
- **Language:** TypeScript

## Features

- Autonomous cat colony (runs 24/7)
- Real-time updates
- Task queue system
- Building placement
- World map exploration
- Combat and encounters
- Breeding system
- Skill progression
- Age-based life stages
- User interactions (feed, heal, build)

## Game Loop

The game runs automatically every 10 seconds:
1. Decay cat needs
2. Process autonomous behaviors
3. Progress tasks
4. Check encounters
5. Update ages
6. Process births
7. Building effects
8. Update colony status

## Project Structure

```
cat_idler/
├── app/              # Next.js pages
├── components/       # React components
│   ├── colony/       # Game-specific components
│   └── ui/           # Reusable UI components
├── convex/           # Backend (Convex)
├── lib/game/         # Pure game logic
├── tests/            # All tests
│   ├── unit/         # Unit tests
│   └── e2e/          # Selenium E2E tests
├── types/            # TypeScript types
└── docs/             # Documentation
```

## How to Play

1. **Create a Colony** - Enter a name and click "Create Colony"
2. **Select a Leader** - Choose a cat with high leadership
3. **Watch the Colony** - Cats autonomously manage their needs
4. **Help Your Colony** - Click "Give Food", "Give Water", or heal cats
5. **Build and Expand** - Place buildings to improve efficiency
6. **Explore the World** - Switch to "World Map" to see exploration

---

**Ready to play!** Create a colony and watch it thrive! 🐱
