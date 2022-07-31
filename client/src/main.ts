import { createApp } from 'vue';
import { Quasar, Dialog, Notify } from 'quasar';
import { get1t } from './scripts/onetagger';
import iconSet from 'quasar/icon-set/mdi-v6';

// Style
import '@quasar/extras/mdi-v6/mdi-v6.css';
import 'quasar/src/css/index.sass';
import './style/app.scss';

import App from './App.vue';


// Handle Windows webview messages
// @ts-ignore
if (window.chrome && window.chrome.webview) {
    // @ts-ignore
    window.chrome.webview.addEventListener('message', e => {
        get1t().onOSMessage(JSON.parse(e.data));
    });
}


createApp(App)
    .use(Quasar, {
        plugins: {
            Dialog, Notify
        },
        iconSet
    })
    .mount('#app');
