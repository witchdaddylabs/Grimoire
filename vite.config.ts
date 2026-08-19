import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  // Relative asset paths are REQUIRED for the Tauri release bundle. The
  // packaged app serves from tauri://localhost, where Vite's default absolute
  // "/assets/..." resolves to the filesystem root instead of the bundle — the
  // JS then never loads, React never boots, and every button in the app is
  // silently inert while the shell still renders. `npm run dev` hides this
  // because the dev server does serve from "/".
  base: "./",
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
  },
});

