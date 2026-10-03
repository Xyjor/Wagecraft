import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
    passWithNoTests: true,
  },
});
