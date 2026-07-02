ALTER TABLE `cats` ADD `destination` text;--> statement-breakpoint
ALTER TABLE `cats` ADD `activity` text DEFAULT 'idle' NOT NULL;