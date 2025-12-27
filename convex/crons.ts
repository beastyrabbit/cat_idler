/**
 * Convex Cron Jobs
 *
 * Scheduled tasks for the game.
 * TASK: TICK-002
 */

import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();

// Game tick runs every 10 seconds
crons.interval(
  "game tick",
  { seconds: 10 },
  internal.gameTick.tickAllColonies
);

// Resource regeneration runs every 10 minutes
crons.interval(
  "regenerate resources",
  { minutes: 10 },
  internal.worldMap.regenerateTilesForAllColonies
);

// Path decay runs every hour
crons.interval(
  "decay paths",
  { hours: 1 },
  internal.worldMap.decayPathsForAllColonies
);

export default crons;



