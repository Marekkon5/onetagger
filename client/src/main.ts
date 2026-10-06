import { createApp } from 'vue';
import { Quasar, Notify } from 'quasar';
import iconSet from 'quasar/icon-set/mdi-v6';

import '@quasar/extras/mdi-v6/mdi-v6.css';
import 'quasar/src/css/index.sass';
import './style/app.scss';

import App from './App.vue';
import router from './scripts/router';

createApp(App)
  .use(router)
  .use(Quasar, { plugins: { Notify }, iconSet })
  .mount('#app');
