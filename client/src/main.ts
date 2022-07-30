import { createApp } from 'vue';
import { Quasar } from 'quasar';
import iconSet from 'quasar/icon-set/mdi-v6';

// Style
import '@quasar/extras/mdi-v6/mdi-v6.css';
import 'quasar/src/css/index.sass';
import './style/app.scss';

import App from './App.vue';


createApp(App)
    .use(Quasar, {
        plugins: {},
        iconSet
    })
    .mount('#app');
