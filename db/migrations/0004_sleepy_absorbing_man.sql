CREATE TABLE `zones` (
	`id` text PRIMARY KEY NOT NULL,
	`colonyId` text NOT NULL,
	`kind` text NOT NULL,
	`x1` integer NOT NULL,
	`y1` integer NOT NULL,
	`x2` integer NOT NULL,
	`y2` integer NOT NULL,
	`playerId` text NOT NULL,
	`createdAt` integer NOT NULL,
	`expiresAt` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `zones_by_colony` ON `zones` (`colonyId`);--> statement-breakpoint
CREATE INDEX `zones_by_colony_expires` ON `zones` (`colonyId`,`expiresAt`);--> statement-breakpoint
CREATE INDEX `zones_by_player` ON `zones` (`playerId`);