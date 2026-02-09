import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [
    react({
      jsxRuntime: "automatic",
    }),
  ],
  test: {
    // Use jsdom for React component testing
    environment: "jsdom",

    // Setup file runs before all tests
    setupFiles: ["./tests/setup.ts"],

    // Enable globals (describe, it, expect without imports)
    globals: true,

    // Include these patterns for test files
    include: ["tests/**/*.test.{ts,tsx}"],

    // Coverage configuration
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      include: [
        "lib/game/needs.ts",
        "lib/game/idleEngine.ts",
        "lib/game/idleRules.ts",
        "lib/game/testAcceleration.ts",
        "lib/game/seededRng.ts",
        "lib/game/policy.ts",
        "lib/game/survival.ts",
        "lib/game/housePlanner.ts",
        "lib/game/morale.ts",
      ],
      exclude: ["**/*.test.ts", "**/*.test.tsx", "**/index.ts"],
      thresholds: {
        lines: 99,
        statements: 99,
        branches: 99,
        functions: 99,
      },
    },

    // Pool settings are optional; defaults work fine here.
  },
  resolve: {
    alias: {
      // Match tsconfig paths
      "@": path.resolve(__dirname, "./"),
    },
  },
});
