CREATE TABLE `raiders` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`raidId` text NOT NULL,
	`position` text NOT NULL,
	`target` text NOT NULL,
	`strength` real NOT NULL,
	`hp` real NOT NULL,
	`status` text DEFAULT 'advancing' NOT NULL,
	`spawnedAt` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `raiders_by_colony` ON `raiders` (`colonyId`);--> statement-breakpoint
CREATE INDEX `raiders_by_raid` ON `raiders` (`raidId`);--> statement-breakpoint
ALTER TABLE `colonies` ADD `threatPressure` real;--> statement-breakpoint
ALTER TABLE `colonies` ADD `lastRaidAt` integer;--> statement-breakpoint
ALTER TABLE `colonies` ADD `activeRaidId` text;--> statement-breakpoint
ALTER TABLE `colonies` ADD `raidClicks` real;