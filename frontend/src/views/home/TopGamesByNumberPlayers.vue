<template>
  <v-flex xs4 class="top-games-by-players">
    <h2 class="title mb-5 text-center">Top current games by number players</h2>
    <router-link
      v-ripple
      class="line pa-2"
      v-for="game of info.topCurrentGamesByNumberPlayers"
      :to="`/server/${game.serverId}`"
    >
      <span class="rich-text mr-2" v-html="transformRichText(game.name)"></span>
      <span class="number-players">({{ game.numberPlayers }})</span>
    </router-link>
  </v-flex>
</template>

<style scoped>
  .line {
    display: flex;
    text-decoration: none;
  }

  .rich-text {
    display: block !important;
  }

  .number-players {
    margin-left: auto;
  }
</style>

<script>
  export default {
    name: 'TopGamesByNumberPlayers',
    props: {
      info: {},
    },
  };
</script>
