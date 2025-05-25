import path from "path";
import { defineConfig } from "vite";
import { quasar, transformAssetUrls } from "@quasar/vite-plugin";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue({
      template: { transformAssetUrls },
    }),
    quasar({
      sassVariables: path.resolve("src/style/quasar.scss"),
    }),
  ],
  server: {
    port: 8080,
  },
  build: {
    assetsInlineLimit: 16 * 1024,
  },
});
