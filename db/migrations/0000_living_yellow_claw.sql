CREATE TABLE `buildings` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`type` text NOT NULL,
	`level` integer NOT NULL,
	`position` text NOT NULL,
	`constructionProgress` real NOT NULL
);
--> statement-breakpoint
CREATE INDEX `buildings_by_colony` ON `buildings` (`colonyId`);--> statement-breakpoint
CREATE TABLE `cats` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`name` text NOT NULL,
	`parentIds` text NOT NULL,
	`birthTime` integer NOT NULL,
	`deathTime` integer,
	`stats` text NOT NULL,
	`needs` text NOT NULL,
	`currentTask` text,
	`position` text NOT NULL,
	`isPregnant` integer NOT NULL,
	`pregnancyDueTime` integer,
	`spriteParams` text,
	`specialization` text,
	`roleXp` text
);
--> statement-breakpoint
CREATE INDEX `cats_by_colony` ON `cats` (`colonyId`);--> statement-breakpoint
CREATE INDEX `cats_by_colony_alive` ON `cats` (`colonyId`,`deathTime`);--> statement-breakpoint
CREATE TABLE `colonies` (
	`id` text PRIMARY KEY NOT NULL,
	`name` text NOT NULL,
	`leaderId` text,
	`status` text NOT NULL,
	`resources` text NOT NULL,
	`gridSize` integer NOT NULL,
	`createdAt` integer NOT NULL,
	`lastTick` integer NOT NULL,
	`lastAttack` integer NOT NULL,
	`worldSeed` integer,
	`isGlobal` integer,
	`runNumber` integer,
	`runStartedAt` integer,
	`lastPlayerActivityAt` integer,
	`lastResetAt` integer,
	`automationTier` real,
	`globalUpgradePoints` real,
	`ritualRequestedAt` integer,
	`criticalSince` integer,
	`testTimeScale` real,
	`testResourceDecayMultiplier` real,
	`testResilienceHoursOverride` real,
	`testCriticalMsOverride` integer,
	`testRngSeed` integer
);
--> statement-breakpoint
CREATE INDEX `colonies_by_status` ON `colonies` (`status`);--> statement-breakpoint
CREATE INDEX `colonies_by_is_global` ON `colonies` (`isGlobal`);--> statement-breakpoint
CREATE TABLE `events` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`catId` text,
	`timestamp` integer NOT NULL,
	`type` text NOT NULL,
	`message` text NOT NULL,
	`involvedCatIds` text NOT NULL,
	`metadata` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `events_by_colony` ON `events` (`colonyId`);--> statement-breakpoint
CREATE INDEX `events_by_colony_time` ON `events` (`colonyId`,`timestamp`);--> statement-breakpoint
CREATE TABLE `globalUpgrades` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`key` text NOT NULL,
	`level` integer NOT NULL,
	`maxLevel` integer NOT NULL,
	`baseCost` integer NOT NULL,
	`description` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `globalUpgrades_by_colony` ON `globalUpgrades` (`colonyId`);--> statement-breakpoint
CREATE UNIQUE INDEX `globalUpgrades_by_colony_key` ON `globalUpgrades` (`colonyId`,`key`);--> statement-breakpoint
CREATE TABLE `jobs` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`kind` text NOT NULL,
	`status` text NOT NULL,
	`requestedByType` text NOT NULL,
	`requestedByPlayerId` text,
	`assignedCatId` text,
	`baseDurationSec` real NOT NULL,
	`speedMultiplier` real NOT NULL,
	`yieldMultiplier` real NOT NULL,
	`clickTimeReducedSec` real NOT NULL,
	`createdAt` integer NOT NULL,
	`startedAt` integer NOT NULL,
	`endsAt` integer NOT NULL,
	`completedAt` integer,
	`metadata` text
);
--> statement-breakpoint
CREATE INDEX `jobs_by_colony_status` ON `jobs` (`colonyId`,`status`);--> statement-breakpoint
CREATE INDEX `jobs_by_colony_end` ON `jobs` (`colonyId`,`endsAt`);--> statement-breakpoint
CREATE INDEX `jobs_by_player` ON `jobs` (`requestedByPlayerId`);--> statement-breakpoint
CREATE TABLE `players` (
	`id` text PRIMARY KEY NOT NULL,
	`sessionId` text NOT NULL,
	`nickname` text NOT NULL,
	`lastSeenAt` integer NOT NULL,
	`clickWindowStart` integer NOT NULL,
	`clicksInWindow` integer NOT NULL,
	`lifetimeClicks` integer NOT NULL,
	`lifetimeContribution` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `players_by_session` ON `players` (`sessionId`);--> statement-breakpoint
CREATE INDEX `players_by_last_seen` ON `players` (`lastSeenAt`);--> statement-breakpoint
CREATE TABLE `runHistory` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`runNumber` integer NOT NULL,
	`startedAt` integer NOT NULL,
	`endedAt` integer NOT NULL,
	`durationSec` integer NOT NULL,
	`reason` text NOT NULL,
	`finalResources` text NOT NULL,
	`activePlayers` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `runHistory_by_colony_run` ON `runHistory` (`colonyId`,`runNumber`);--> statement-breakpoint
CREATE TABLE `worldTiles` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`x` integer NOT NULL,
	`y` integer NOT NULL,
	`type` text NOT NULL,
	`resources` text NOT NULL,
	`maxResources` text NOT NULL,
	`dangerLevel` real NOT NULL,
	`pathWear` real NOT NULL,
	`lastDepleted` integer NOT NULL,
	`overlayFeature` text
);
--> statement-breakpoint
CREATE INDEX `worldTiles_by_colony` ON `worldTiles` (`colonyId`);--> statement-breakpoint
CREATE UNIQUE INDEX `worldTiles_by_colony_position` ON `worldTiles` (`colonyId`,`x`,`y`);