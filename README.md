# Cat Colony Idle Game

A shared browser idle game where everyone contributes to one global cat colony. The simulation runs continuously in a dedicated backend worker, so time passes even when nobody has the browser open.

## Quick Start

```bash
# Install dependencies
bun install

# Start Convex backend (Terminal 1)
bun run convex:dev

# Start web app (Terminal 2)
bun run dev

# Start simulation worker (Terminal 3)
bun run dev:worker

# Run tests (Terminal 4)
bun run test:unit
```

## Gameplay Overview

- One global colony shared by all players
- Real-time jobs with long cat timelines (hours) and short player support actions (seconds)
- Leader auto-assigns strategic work based on colony health
- Cats specialize over time (hunter, architect, ritualist)
- Click boosting reduces active job time
- Rituals grant shared upgrade points for global progression
- Colony can collapse when unattended and automatically start a new run

## Tech Stack

- Frontend: Next.js, React, Tailwind CSS
- Backend: Convex
- Simulation: dedicated Node worker (`worker/index.ts`)
- Tests: Vitest + Selenium
- Language: TypeScript

## Project Structure

```text
cat_idler/
├── app/                  # Next.js routes (global dashboard at /game)
├── components/           # React components
├── convex/               # Convex functions and schema
├── lib/game/             # Pure game mechanics
├── worker/               # Always-on simulation loop
├── tests/                # Unit and E2E tests
└── types/                # Shared TypeScript types
```
