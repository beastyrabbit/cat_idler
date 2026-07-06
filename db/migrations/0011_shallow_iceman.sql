ALTER TABLE `players` ADD `createdAt` integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `players` ADD `presenceCount` integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE `votes` ADD `subscriberHash` text;--> statement-breakpoint
CREATE UNIQUE INDEX `votes_by_election_subscriber` ON `votes` (`electionId`,`subscriberHash`);
