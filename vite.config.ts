import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// Two entry pages: the settings window and the floating player.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/target/**"] } },
  build: {
    target: "safari16",
    rollupOptions: {
      input: { main: resolve(__dirname, "index.html"), player: resolve(__dirname, "player.html") },
    },
  },
});
