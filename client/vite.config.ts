import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { quasar, transformAssetUrls } from '@quasar/vite-plugin';

const fromRoot = (value: string) => fileURLToPath(new URL(value, import.meta.url));

export default defineConfig({
  plugins: [
    vue({ template: { transformAssetUrls } }),
    quasar({ sassVariables: fromRoot('./src/style/quasar.scss') }),
  ],
  resolve: { alias: { '@': fromRoot('./src') } },
  server: {
    host: '0.0.0.0',
    proxy: { '/api': { target: 'http://127.0.0.1:8787', changeOrigin: true } },
  },
});
