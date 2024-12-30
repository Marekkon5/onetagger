import { defineConfig } from 'vite';
import { quasar, transformAssetUrls } from '@quasar/vite-plugin';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
    plugins: [
        vue({
            template: { transformAssetUrls }
        }),
        quasar()
    ],
    css: {
        preprocessorOptions: {
            scss: {}
        }
    },
    server: {
        port: 8080
    },
    build: {
        assetsInlineLimit: 16 * 1024
    }
});