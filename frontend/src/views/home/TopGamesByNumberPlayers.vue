<template>
  <v-flex xs4 class="top-games-by-players">
    <h2 class="title mb-5 text-center">
      <slot name="title"></slot>
    </h2>
    <!-- todo кастомизация ссылки для учёта даты -->
    <router-link
      v-ripple
      class="line pa-2"
      v-for="game of games"
      :key="game.serverId"
      :to="`/server/${game.serverId}`"
    >
      <span class="rich-text mr-2" v-html="transformRichText(game.name)"></span>
      <div class="number-players">
        <slot name="row" :game="game"></slot>
      </div>
    </router-link>
  </v-flex>
</template>

<style scoped>
  .line {
    display: flex;
    text-decoration: none;
  }

  .line:hover {
    background: #424242;
  }

  .rich-text {
    display: block !important;
  }

  .number-players {
    margin-left: auto;
  }
</style>

<script lang="ts">
  export default {
    name: 'TopGamesByNumberPlayers',
    props: ['games'],
  };
</script>
