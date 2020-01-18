<template>
  <div v-if="info" class="grid">
    <v-layout justify-center>
      <v-flex xs6>
        <v-autocomplete
          autofocus
          solo
          auto-select-first
          hide-details
          hide-no-data
          autocomplete="off"
          placeholder="Search servers"
          v-model="server"
          :items="servers"
          :loading="serversLoading"
          :search-input.sync="serversQuery"
        >
          <template #item="{ item }">
            <v-icon small :color="item.isOnline ? 'green' : 'transparent'">mdi-circle</v-icon>
            <span class="rich-text ml-3" v-html="transformRichText(item.text)"></span>
          </template>
        </v-autocomplete>
      </v-flex>
    </v-layout>
    <v-layout class="mt-10">
      <top-games-by-number-players :info="info" />
    </v-layout>
  </div>
  <v-layout align-center justify-center v-else>
    <v-progress-circular
      :size="70"
      color="primary"
      indeterminate
    ></v-progress-circular>
  </v-layout>
</template>

<style scoped>
  .grid {
    height: 100%;
    width: 100%;
  }
</style>

<script lang="ts">
  import TopGamesByNumberPlayers from '@/views/home/TopGamesByNumberPlayers.vue';
  import Api from '@/views/api';
  import { timeMinutesToDate } from '@/util';
  import debounce from 'lodash.debounce';

  export default {
    components: { TopGamesByNumberPlayers },
    data: () => ({
      info: null,

      // search servers
      server: null,
      serversQuery: null,
      servers: [],
      serversLoading: false,
    }),
    watch: {
      server(value) {
        const params = { id: value };
        this.$router.push({ name: 'server', params });
      },
      serversQuery(query) {
        if (query && query.length >= 2) {
          this.serversLoading = true;
          this.makeSearchRequestDebounced();
        }
      },
    },
    async mounted() {
      this.info = await Api.getMainPageInfo();
      this.makeSearchRequestDebounced = debounce(this.makeSearchRequest, 500);
    },
    methods: {
      async makeSearchRequest() {
        const query = this.serversQuery;
        const games = await Api.searchServers(query);
        if (this.serversQuery !== query) return;
        const servers = games.map(info => ({
          text: this.formatGameName(info),
          value: info.serverId,
          isOnline: info.timeEnd === null,
        }));
        this.servers = Object.freeze(servers);
        this.serversLoading = false;
      },
      formatGameName(info: GameSearchInfo) {
        if (!info.timeEnd) return info.name;

        const timeBegin = timeMinutesToDate(info.timeBegin);
        const timeEnd = timeMinutesToDate(info.timeEnd);
        return `${info.name} (${timeBegin.toLocaleDateString()}-${timeEnd.toLocaleDateString()})`;
      },
    },
  };
</script>
