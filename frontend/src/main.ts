import Vue from 'vue';
import App from './App.vue';
import router from './router';
import store from './store';
import vuetify from './plugins/vuetify';
import './plugins/axios';
import { transformRichText } from '@/misc/richText';
import { sync } from 'vuex-router-sync';

Vue.config.productionTip = false;

sync(store, router);

Vue.mixin({
  methods: {
    transformRichText,
  },
});

new Vue({
  router,
  store,
  // @ts-ignore
  vuetify,
  render: h => h(App),
}).$mount('#app');
