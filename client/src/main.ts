import { createApp } from 'vue';
import { Quasar, Dialog, Notify } from 'quasar';
import { get1t } from './scripts/onetagger';
import router from './scripts/router';
import iconSet from 'quasar/icon-set/mdi-v6';

// Style
import '@quasar/extras/mdi-v6/mdi-v6.css';
import 'quasar/src/css/index.sass';
import './style/app.scss';

import App from './App.vue';


// Handle WebView events
// @ts-ignore
window.onWebviewEvent = (e) => {
    get1t().onOSMessage(e);
}


createApp(App)
    .use(router)
    .use(Quasar, {
        plugins: {
            Dialog, Notify
        },
        iconSet
    })
    .mount('#app');
