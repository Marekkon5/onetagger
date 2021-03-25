import Vue from 'vue';
import App from './App.vue';
import router from './js/router';
import './js/quasar';
import OneTagger from './js/onetagger';

import './styles/app.scss';

Vue.config.productionTip = false;

//Global instance
Vue.prototype.$1t = new OneTagger();

new Vue({
  router,
  mounted() {
    //Backend error dialog
    this.$1t.onError = (msg) => {
      console.log(msg);
      this.$q.dialog({
        title: 'Error',
        message: msg,
        ok: {
          color: 'primary',
        }
      });
    }
  },
  render: h => h(App)
}).$mount('#app');
