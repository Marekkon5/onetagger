//MacOS Polyfill
require('intersection-observer');

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

		//Windows webview
		if (window.chrome && window.chrome.webview)
			window.chrome.webview.addEventListener('message', e => {
				this.$1t.onOSMessage(JSON.parse(e.data), this);
			});
	},
	render: h => h(App)
}).$mount('#app');