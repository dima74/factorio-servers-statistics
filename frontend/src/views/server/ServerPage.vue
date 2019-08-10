<template>
  <v-layout column v-if="games">
    <!-- todo tags -->
    <div class="title-wrapper">
      <h1
        class="headline rich-text"
        v-html="transformRichText(lastGame.name)"
      ></h1>
      <h2
        class="subtitle-1 rich-text"
        v-html="transformRichText(lastGame.description)"
      ></h2>
    </div>
    <v-layout class="mt-4">
      <players-list
        :lastGame="lastGame"
        :timeEnd="timeEnd"
        :hoverPlot="hoverPlot"
      />
      <v-flex xs6 class="pa-1">
        <server-number-players-plot
          :games="games"
          :timeBegin="timeBegin"
          :timeEnd="timeEnd"
          @hoverPlot="hoverPlot = $event"
        />
      </v-flex>
      <v-flex xs3 class="pa-1 text-center">
        <h2 class="title">Server info</h2>
        <div>Game version: {{ lastGame.gameVersion }}</div>
      </v-flex>
    </v-layout>
  </v-layout>
</template>

<style scoped>
  .title-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>

<script lang="ts">
  import ServerNumberPlayersPlot from './ServerNumberPlayersPlot.vue';
  import Api from '../api';
  import PlayersList from '@/views/server/PlayersList.vue';
  import { assert } from '@/util';

  export default {
    name: 'ServerPage',
    components: { PlayersList, ServerNumberPlayersPlot },
    props: ['id'],
    data: () => ({
      timeBegin: null,
      timeEnd: null,

      games: null,
      hoverPlot: null,  // { time: TimeMinutes, players: PlayerInterval[] }
    }),
    async mounted() {
      const week = 7 * 24 * 60;
      // this.timeEnd = Math.round(Date.now() / 1000 / 60);
      // this.timeBegin = this.timeEnd - week;

      const durationBackend = 2 * 24 * 60;
      const duration = 1 * 24 * 60;
      this.timeBegin = 1 + durationBackend - duration;
      this.timeEnd = 1 + durationBackend;
      assert(this.timeBegin < this.timeEnd);

      const games = (await Api.getServerInfo(this.id, this.timeBegin, this.timeEnd));

      this.transformGames(games);
      this.games = Object.freeze(games);

      // todo if games are empty?
    },
    methods: {
      transformGames(games: Game[]) {
        for (const game of games) {
          for (const playerInterval of game.playersIntervals) {
            if (playerInterval.end === null) {
              assert(game === games[games.length - 1]);
              playerInterval.end = this.timeEnd;
              playerInterval.isOnline = true;
              assert(playerInterval.begin < playerInterval.end);
            }
          }
        }

        for (let i = 0; i + 1 < games.length; ++i) {
          const currGame = games[i];
          const nextGame = games[i + 1];
          if (currGame.timeEnd > nextGame.timeBegin) {
            // интервалы игр могут перекрываться (после завершения игра ещё некоторое время возвращается в /get-games)
            currGame.timeEnd = nextGame.timeBegin;
            currGame.playersIntervals = currGame.playersIntervals.filter(playerInterval => playerInterval.begin < currGame.timeEnd);
            for (const playerInterval of currGame.playersIntervals) {
              assert(playerInterval.begin < playerInterval.end);
              playerInterval.end = Math.min(playerInterval.end, currGame.timeEnd);
              assert(playerInterval.begin < playerInterval.end);
            }
          }
        }

        for (const game of games) {
          const isLastGame = game === games[games.length - 1];
          assert(isLastGame || game.timeEnd !== null);
          for (const playerInterval of game.playersIntervals) {
            assert(playerInterval.end !== null);
            assert(playerInterval.begin < playerInterval.end);
            assert(game.timeBegin <= playerInterval.begin);
            assert(isLastGame || playerInterval.end <= game.timeEnd);
          }
        }
      },
    },
    computed: {
      lastGame(): Game {
        return this.games[this.games.length - 1];
      },
    },
  };
</script>
