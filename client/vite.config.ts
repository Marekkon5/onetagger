import path from 'path';
import { defineConfig } from 'vite';
import { quasar, transformAssetUrls } from '@quasar/vite-plugin';
import vue from '@vitejs/plugin-vue';


// https://vitejs.dev/config/
export default defineConfig({
    plugins: [
        vue({
            template: { transformAssetUrls }
        }),
        quasar({
            sassVariables: path.resolve('src/style/quasar.scss')
        })
    ],
    server: {
        host: '127.0.0.1',
        port: 8080,
        strictPort: true,
        proxy: Object.fromEntries(['/session', '/ws', '/audio', '/thumb', '/spotify'].map(route => [route, {
            target: 'http://127.0.0.1:36913', ws: route === '/ws', changeOrigin: false,
        }]))
    },
    build: {
        assetsInlineLimit: 16 * 1024
    }
});

