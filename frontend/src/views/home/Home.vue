<template>
  <div v-if="info" class="grid">
    <v-layout justify-center>
      <v-flex xs6>
        <v-autocomplete
          autofocus
          solo
          auto-select-first
          hide-details
          autocomplete="off"
          placeholder="Search servers"
          v-model="server"
          :items="servers"
        >
          <template #item="{ item }">
            <span class="rich-text" v-html="transformRichText(item.text)"></span>
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

<script>
  import axios from 'axios';
  import TopGamesByNumberPlayers from '@/views/home/TopGamesByNumberPlayers';

  export default {
    components: { TopGamesByNumberPlayers },
    data: () => ({
      server: null,
      servers: null,
      info: null,
    }),
    watch: {
      server(value) {
        const params = { id: value };
        this.$router.push({ name: 'server', params });
      },
    },
    async mounted() {
      const info = (await axios.get('/main-page')).data;
      this.info = Object.freeze(info);

      // todo sort by ???
      this.servers = Object.entries(info.searchIndex)
          .map(([gameName, serverId]) => ({ text: gameName, value: serverId }));
    },
  };
</script>
