<template>
  <v-menu
    v-model="model"
    :close-on-content-click="false"
    transition="scale-transition"
    offset-y
    max-width="290px"
    min-width="290px"
  >
    <template v-slot:activator="{ on }">
      <slot :on="on" />
    </template>

    <v-date-picker range v-model="dates" />
  </v-menu>
</template>

<script lang="ts">
  import { dateToTimeMinutes, timeMinutesToDate } from '@/util';

  export default {
    name: 'CustomRangeSelector',
    props: {
      timeBegin: Number,
      timeEnd: Number,
    },
    data() {
      return {
        model: false,
        dates: [null, null],
      };
    },
    watch: {
      timeRange: 'updateDates',
      dates(dates) {
        if (dates[1] != null) {
          const [timeBegin, timeEnd] = dates.map(this.isoDateToTimeMinutes);
          this.$emit('update:timeBegin', timeBegin);
          this.$emit('update:timeEnd', timeEnd + 24 * 60);
        }
      },
    },
    methods: {
      updateDates() {
        this.dates = [this.timeBegin, this.timeEnd].map(this.timeMinutesToIsoDate);
      },
      timeMinutesToIsoDate(time: TimeMinutes): string {
        const date = timeMinutesToDate(time);
        return date.toISOString().substr(0, 10);
      },
      isoDateToTimeMinutes(date: string): TimeMinutes {
        return dateToTimeMinutes(new Date(date));
      },
    },
    computed: {
      timeRange() {
        return [this.timeBegin, this.timeEnd];
      },
    },
  };
</script>
