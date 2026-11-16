import { createApp } from 'vue';
import { Quasar, Dialog, Notify } from 'quasar';
import { establishSession } from './scripts/session';
import iconSet from 'quasar/icon-set/mdi-v6';

// Style
import '@quasar/extras/mdi-v6/mdi-v6.css';
import 'quasar/src/css/index.sass';
import './style/app.scss';




async function start() {
    await establishSession();
    const [{ default: App }, { default: router }, { get1t }] = await Promise.all([
        import('./App.vue'), import('./scripts/router'), import('./scripts/onetagger'),
    ]);
    // @ts-ignore
    window.onWebviewEvent = (e) => get1t().onOSMessage(e);
    createApp(App).use(router).use(Quasar, {
        plugins: { Dialog, Notify }, iconSet,
    }).mount('#app');
}

start().catch((error: Error) => {
    const app = document.getElementById('app');
    if (app) { app.textContent = error.message; app.style.padding = '2rem'; }
});
