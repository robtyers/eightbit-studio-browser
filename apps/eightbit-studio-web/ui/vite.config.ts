import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";
export default defineConfig({
  base: "/studio/",
  plugins: [react()],
  build: { target: "safari16" },
  server: process.env.STUDIO_GITHUB_PROXY === "http://127.0.0.1:1426" ? { proxy: { "/studio-api/": { target: "http://127.0.0.1:1426", changeOrigin: false } } } : undefined,
  test: { environment: "node", globals: true },
});
