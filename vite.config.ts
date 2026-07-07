import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 要求固定端口且不清屏
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    target: "es2021",
  },
});
