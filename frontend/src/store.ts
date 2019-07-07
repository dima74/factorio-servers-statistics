import Vue from 'vue';
import Vuex from 'vuex';

Vue.use(Vuex);

const store = new Vuex.Store({
  state: {
    pageHeight: window.innerHeight,
    pageWidth: window.innerWidth,
  },
  mutations: {
    updatePageWidth(state) {
      state.pageHeight = window.innerHeight;
      state.pageWidth = window.innerWidth;
    },
  },
  actions: {},
});
export default store;

window.addEventListener('resize', () => store.commit('updatePageWidth'));
