import { defineConfig } from "drizzle-kit";

export default defineConfig({
	dialect: "sqlite",
	schema: "./db/schema.ts",
	out: "./db/migrations",
	dbCredentials: {
		url: process.env.GAME_DB_PATH ?? "data/game.db",
	},
});
