# Cat Colony Idle Game - AI Assistant Guide

## Project Overview

A real-time idle game where a cat colony runs autonomously. Built with Next.js, Convex, and TypeScript.

## Key Documentation

- **[README.md](./README.md)** - Project overview and setup
- **[docs/plan.md](./docs/plan.md)** - Complete game design document
- **[docs/TASKS.md](./docs/TASKS.md)** - Development tasks with TDD instructions
- **[docs/TESTING.md](./docs/TESTING.md)** - Testing guide and patterns

## Architecture

```
Frontend: Next.js 14 (App Router) + Tailwind CSS
Backend: Convex (serverless functions + real-time database)
Testing: Vitest + React Testing Library
```

## Directory Structure

```
cat_idler/
├── app/                  # Next.js pages
├── components/           # React components
│   ├── colony/           # Game-specific components
│   └── ui/               # Reusable UI components
├── convex/               # Convex backend
│   ├── schema.ts         # Database schema
│   └── *.ts              # Mutations, queries, actions
├── lib/game/             # Pure game logic (no DB access)
├── tests/                # All tests
│   ├── unit/             # Unit tests
│   ├── integration/      # Integration tests
│   └── factories/        # Test data factories
├── types/                # TypeScript types
└── docs/                 # Documentation
```

## Development Workflow (TDD)

1. Pick a task from `docs/TASKS.md`
2. Write failing tests first
3. Implement to make tests pass
4. Refactor

## Key Types

All game types are in `types/game.ts`:
- `Cat` - Cat entity with stats, needs, position
- `Colony` - Colony with resources and settings
- `Task` - Task queue items
- `Encounter` - Combat/event encounters
- `Building` - Colony buildings

## Game Logic

Pure functions in `lib/game/`:
- `needs.ts` - Hunger, thirst, rest, health
- `age.ts` - Life stages, death chance
- `skills.ts` - Skill learning
- `combat.ts` - Combat resolution
- `catAI.ts` - Autonomous cat behavior
- `tasks.ts` - Task assignment logic
- `paths.ts` - World map paths
- `worldResources.ts` - Resource harvesting

## Commands

```bash
npm run dev          # Start development
npm test             # Run tests
npm run test:watch   # Tests in watch mode
npx convex dev       # Start Convex backend
```

## Important Notes

- Game tick runs every 10 seconds (Convex cron)
- Cats have needs that decay over time
- Colony can die if all cats die
- User interactions are optional (game is idle-capable)
