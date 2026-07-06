import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";

const eslintConfig = defineConfig([
  ...nextVitals,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    // Local source asset packs contain third-party files that are not app code.
    "public/Kenney Game Assets All-in-1 3.5.0/**",
    "public/Paws & Whiskers - Isometric Cats Pack (Free)/**",
  ]),
]);

export default eslintConfig;
