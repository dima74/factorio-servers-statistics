<template>
  <v-layout align-center justify-center>
    <v-flex v-if="servers" xs6>
      <v-autocomplete
        autofocus
        solo
        auto-select-first
        autocomplete="off"
        placeholder="Server name"
        v-model="server"
        :items="servers"
      >
        <template #item="{ item }">
          <span class="rich-text" v-html="transformRichText(item.text)"></span>
        </template>
      </v-autocomplete>
    </v-flex>
    <v-progress-circular
      v-else
      :size="70"
      color="primary"
      indeterminate
    ></v-progress-circular>
  </v-layout>
</template>

<script>
  import axios from 'axios';

  export default {
    components: {},
    data: () => ({
      server: null,
      servers: null,
    }),
    watch: {
      server(value) {
        const params = { id: value };
        this.$router.push({ name: 'server', params });
      },
    },
    async mounted() {
      const { servers } = (await axios.get('/servers_search_index')).data;
      // todo sort by ???
      this.servers = Object.entries(servers)
          .map(([gameName, serverId]) => ({ text: gameName, value: serverId }));
    },
  };
</script>
