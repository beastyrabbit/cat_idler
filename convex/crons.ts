import { cronJobs } from 'convex/server';

// Worker-owned simulation loop for browser-idle v2.
// Keep Convex cron empty to avoid double-processing ticks.
const crons = cronJobs();

export default crons;
