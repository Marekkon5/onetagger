import { createApp } from 'vue';
import { Quasar, Dialog, Notify } from 'quasar';
import { establishSession } from './scripts/session';
import iconSet from 'quasar/icon-set/mdi-v6';
import { i18n, quasarLanguagePacks, type SupportedLocale } from './i18n';

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
    createApp(App).use(i18n).use(router).use(Quasar, {
        plugins: { Dialog, Notify }, iconSet,
        // The application locale owns HTML language/direction. Quasar's
        // English control pack uses en-US while our catalog locale is en.
        config: { lang: { noHtmlAttrs: true } },
        lang: quasarLanguagePacks[i18n.global.locale.value as SupportedLocale],
    }).mount('#app');
}

start().catch((error: Error) => {
    const app = document.getElementById('app');
    if (app) { app.textContent = error.message; app.style.padding = '2rem'; }
});
