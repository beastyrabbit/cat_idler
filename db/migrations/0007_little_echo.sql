ALTER TABLE `cats` ADD `ageHours` real DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `cats` ADD `pregnancyDueAgeHours` real;--> statement-breakpoint
ALTER TABLE `cats` ADD `pregnancyMateId` text;