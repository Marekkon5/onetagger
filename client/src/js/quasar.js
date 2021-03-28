import Vue from 'vue';

import '../styles/quasar.scss';
import 'quasar/dist/quasar.ie.polyfills';
import iconSet from 'quasar/icon-set/mdi-v5.js';
import '@quasar/extras/mdi-v5/mdi-v5.css';
import { Quasar, Dialog, QChip, Notify } from 'quasar';

Vue.use(Quasar, {
  config: {},
  plugins: {
    Dialog, QChip, Notify
  },
  iconSet: iconSet
});