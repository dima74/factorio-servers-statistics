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
        :timeEnd="gamesTimeRange[1]"
        :hoverPlot="hoverPlot"
      />
      <v-flex xs6 class="pa-1">

        <v-btn-toggle
          v-model="duration"
          dense
          group
          mandatory
          class="interval-select"
        >
          <v-btn
            v-for="[text, _] of Object.entries(availableDurations)"
            :value="text"
          >
            {{ text }}
          </v-btn>
          <v-btn value="all">All time</v-btn>
          <custom-range-selector
            v-slot="{ on }"
            :timeBegin.sync="customTimeBegin"
            :timeEnd.sync="customTimeEnd"
          >
            <v-btn value="custom" v-on="on">Custom range</v-btn>
          </custom-range-selector>
        </v-btn-toggle>

        <server-number-players-plot
          :games="games"
          :timeBegin="gamesTimeRange[0]"
          :timeEnd="gamesTimeRange[1]"
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

  .interval-select {
    margin-left: 50px;
  }

  .interval-select > :first-child {
    margin-left: 0 !important;
  }
</style>

<script lang="ts">
  import ServerNumberPlayersPlot from './ServerNumberPlayersPlot.vue';
  import Api from '../api';
  import PlayersList from '@/views/server/PlayersList.vue';
  import { assert, dateToTimeMinutes } from '@/util';
  import CustomRangeSelector from '@/views/server/CustomRangeSelector.vue';

  const DEFAULT_DURATION = 'last_day';

  function currentTimeMinutes() {
    return dateToTimeMinutes(new Date());
  }

  export default {
    name: 'ServerPage',
    components: { CustomRangeSelector, PlayersList, ServerNumberPlayersPlot },
    props: ['id'],
    data: () => ({
      customTimeBegin: null,
      customTimeEnd: null,

      // timeRange for current this.games
      // it is updated with this.games simultaneously
      gamesTimeRange: null,

      games: null,
      hoverPlot: null,  // { time: TimeMinutes, players: PlayerInterval[] }

      availableDurations: {
        'last day': 24 * 60,
        'last week': 7 * 24 * 60,
        'last month': 30 * 24 * 60,
        'last year': 365 * 24 * 60,
      },
    }),
    watch: {
      $route: 'fetchGames',
      customTimeRange(newValue, oldValue) {
        if (this.duration === 'custom' && newValue[1] != null && oldValue[1] != null) {
          this.doFetchGames();
        }
      },
    },
    mounted() {
      this.fetchGames();
    },
    methods: {
      fetchGames() {
        if (this.duration !== 'custom') {
          this.doFetchGames();
        }
      },
      async doFetchGames() {
        const duration = this.duration;
        // todo keep the longest duration, and use it for shorter ones

        let [timeBegin, timeEnd] = this.timeRange;
        const games = (await Api.getServerInfo(this.id, timeBegin, timeEnd));
        if (this.duration !== duration) return;

        if (duration === 'all') {
          timeBegin = games[0].timeBegin;
          timeEnd = games[games.length - 1].timeEnd || currentTimeMinutes();
        }

        this.transformGames(games, timeBegin, timeEnd);
        this.games = Object.freeze(games);
        this.gamesTimeRange = [timeBegin, timeEnd];

        // todo if games are empty?
      },
      transformGames(games: Game[], timeBegin, timeEnd) {
        // обработка игроков которые сейчас онлайн
        for (const game of games) {
          for (const playerInterval of game.playersIntervals) {
            if (playerInterval.end === null) {
              assert(game === games[games.length - 1]);
              playerInterval.end = timeEnd;
              playerInterval.isOnline = true;
              assert(playerInterval.begin < playerInterval.end);
            }
          }
        }

        // делаем интервалы игр неперекрывающимися
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

        // just asserts
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
      duration: {
        get() {
          const duration = this.$store.state.route.query.duration || DEFAULT_DURATION;
          return duration.replace('_', ' ');
        },
        set(duration) {
          if (duration === 'custom') {
            [this.customTimeBegin, this.customTimeEnd] = this.timeRange;
          } else {
            this.customTimeBegin = null;
            this.customTimeEnd = null;
          }

          if (!duration) return;
          duration = duration.replace(' ', '_');
          if (duration === DEFAULT_DURATION) {
            duration = undefined;
          }

          if (this.$route.query.duration == duration) return;
          const query = { ...this.$route.query, duration };
          this.$router.replace({ query });
        },
      },
      timeRange() {
        if (this.duration === 'custom') {
          return [this.customTimeBegin, this.customTimeEnd];
        } else if (this.duration === 'all') {
          return [null, null];
        }

        const duration = this.availableDurations[this.duration];
        const timeEnd = currentTimeMinutes();
        const timeBegin = timeEnd - duration;
        return [timeBegin, timeEnd];
      },
      customTimeRange() {
        return [this.customTimeBegin, this.customTimeEnd];
      },
    },
  };
</script>
