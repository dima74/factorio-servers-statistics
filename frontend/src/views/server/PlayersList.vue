<template>
  <v-flex xs3 class="pa-1 text-center">
    <!-- todo: scroll -->
    <h2 class="title mb-3">
      {{ title }}
    </h2>
    <div class="list custom-scrollbar">
      <div
        v-for="player of (hoverPlot ? hoverPlot.players : currentOnlinePlayers)"
      >
        {{ player.name }} ({{ formatPlayerOnlineDuration((hoverPlot ? hoverPlot.time : timeEnd) - player.begin) }})
      </div>
    </div>
  </v-flex>
</template>

<style scoped>
  .list {
    overflow-y: auto;
    max-height: 60vh;
  }

  .custom-scrollbar {
    scrollbar-width: thin;
  }
</style>

<script lang="ts">
  import { timeMinutesToDate } from '@/util';

  export default {
    name: 'PlayersList',
    props: {
      lastGame: {
        type: Object,  // Game
        required: true,
      },
      timeEnd: {
        type: Number,  // TimeMinutes
        required: true,
      },
      hoverPlot: {
        required: true,
      },
    },
    methods: {
      formatPlayerOnlineDuration(duration: TimeMinutes) {
        if (duration <= 1) {
          return '1 minute';
        }
        if (duration < 70) {
          return `${Math.round(duration)} minutes`;
        }

        const hours = duration / 60;
        return `${hours < 2.8 ? hours.toFixed(1) : Math.round(hours)} hours`;
      },
    },
    computed: {
      currentOnlinePlayers(): [string, number][] {
        const intervals = this.lastGame.playersIntervals;
        let firstOnlinePlayerIndex = intervals.length;
        while (firstOnlinePlayerIndex > 0 && intervals[firstOnlinePlayerIndex - 1].isOnline) {
          --firstOnlinePlayerIndex;
        }
        return intervals.slice(firstOnlinePlayerIndex)
            .sort((player1, player2) => player1.begin - player2.begin);
      },
      title() {
        return this.hoverPlot
            ? `Players at ${timeMinutesToDate(this.hoverPlot.time).toLocaleString()}`
            : 'Current online players';
      },
    },
  };
</script>
