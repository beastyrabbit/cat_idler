CREATE TABLE `elections` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`kind` text NOT NULL,
	`status` text NOT NULL,
	`candidateCatIds` text NOT NULL,
	`targetCatId` text,
	`startedAt` integer NOT NULL,
	`endsAt` integer NOT NULL,
	`winnerCatId` text,
	`runNumber` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `elections_by_colony_status` ON `elections` (`colonyId`,`status`);--> statement-breakpoint
CREATE TABLE `votes` (
	`id` text PRIMARY KEY NOT NULL,
	`electionId` text NOT NULL,
	`playerId` text NOT NULL,
	`catId` text NOT NULL,
	`createdAt` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `votes_by_election` ON `votes` (`electionId`);--> statement-breakpoint
CREATE UNIQUE INDEX `votes_by_election_player` ON `votes` (`electionId`,`playerId`);