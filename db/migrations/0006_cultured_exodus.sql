ALTER TABLE `buildings` ADD `productionProgress` real DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `cats` ADD `assignedBuildingId` text;