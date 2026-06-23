import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri serves its own dev server; Vite is the asset source.
  // Prevent Vite from obscuring Rust panics.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port; bail if taken.
    strictPort: true,
    port: 1420,
    host: "127.0.0.1",
    // Tauri 2 launches the browser via the dev URL; let it.
    fs: { strict: false },
    watch: {
      // Don't watch the Rust build output — its .dll/.exe are locked during
      // cargo builds, which throws EBUSY on Windows. Only the React src matters.
      ignored: [
        "**/src-tauri/target/**",
        "**/src-tauri/gen/**",
        "**/node_modules/**",
        "**/tools/**",
        "**/claude-design-output/**",
      ],
    },
  },
  resolve: {
    alias: { "@": resolve(__dirname, "src") },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows; keep modern target.
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
