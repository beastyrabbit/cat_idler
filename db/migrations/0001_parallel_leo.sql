DROP INDEX `colonies_by_is_global`;--> statement-breakpoint
CREATE UNIQUE INDEX `colonies_by_is_global` ON `colonies` (`isGlobal`);