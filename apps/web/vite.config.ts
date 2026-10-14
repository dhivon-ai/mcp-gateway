import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  build: {
    outDir: process.env.GATEWAY_BUILD_CACHE
      ? `${process.env.GATEWAY_BUILD_CACHE}/web`
      : "dist",
    emptyOutDir: true,
  },
  test: { environment: "jsdom", restoreMocks: true },
});
