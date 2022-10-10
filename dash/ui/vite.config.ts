import { fileURLToPath, URL } from "node:url";

import { defineConfig, loadEnv } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
export default defineConfig(({ _command, mode }) => {
  const env = loadEnv(mode, process.cwd());
  return {
    plugins: [vue()],
    server: {
      proxy: {
        "/ws": {
          target: env.VITE_PRINTNANNY_EDGE_API_URL,
          changeOrigin: true,
          secure: false,
          ws: true,
        },
      },
    },
    envDir: ".env",
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
    define: {
      "process.env": {},
      UnixTransport: {},
    },
  };
});
