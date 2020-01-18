<template>
  <div class="svgWrapper" ref="svgWrapper">
    <svg ref="svg"></svg>
  </div>
</template>

<style scoped>
  .svgWrapper {
    position: relative;
  }

  svg {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    left: 0;
    right: 0;
  }

  >>> svg text {
    fill: white;
  }

  >>> .line {
    fill: none;
    stroke: #ffab00;
    stroke-width: 3;
  }

  >>> g.plot {
    pointer-events: all;
    /* работает только в Chrome */
    /* pointer-events: bounding-box; */
  }

  >>> .cursor-vertical-line {
    stroke: #ffffffd7;
    stroke-width: 2;
  }
</style>

<script lang="ts">
  import * as d3 from 'd3';
  import Plot from './plot';
  import { mapState } from 'vuex';
  import { assert, dateToTimeMinutes, timeMinutesToDate } from '@/util.ts';

  export default {
    name: 'ServerNumberPlayersPlot',
    props: {
      games: {
        type: Array,
        required: true,
      },
      timeBegin: {
        type: Number,
        required: true,
      },
      timeEnd: {
        type: Number,
        required: true,
      },
    },
    data: () => ({
      svgHeight: 0,
      svgWidth: 0,

      margin: { top: 50, right: 50, bottom: 50, left: 50 },
      plotHeight: 0,
      plotWidth: 0,
      cursorTime: null,
    }),
    watch: {
      pageWidth: 'updateSvgWidth',
      pageHeight: 'updateSvgWidth',
      svgSize: 'updatePlot',
      games: 'updatePlot',
      cursorTime(time) {
        if (time === null) {
          this.$emit('hoverPlot', null);
        } else {
          const players = this.plot.getPlayersAt(time);
          this.$emit('hoverPlot', { time, players });
        }
        this.updateVerticalLine(time);
      },
    },
    mounted() {
      this.updateSvgWidth();
    },
    methods: {
      updateSvgWidth() {
        const aspectRatio = 0.75;

        const svgWrapper = this.$refs.svgWrapper;
        const svgWrapperHeight = svgWrapper.offsetHeight;
        this.svgWidth = svgWrapper.offsetWidth;
        this.svgHeight = Math.min(svgWrapperHeight, this.svgWidth * aspectRatio);
        assert(this.svgHeight > 0);
        assert(this.svgWidth > 0);

        this.plotHeight = this.svgHeight - this.margin.top - this.margin.bottom;
        this.plotWidth = this.svgWidth - this.margin.left - this.margin.right;
      },
      updatePlot() {
        const dataset = this.plotNumberPlayers;

        const width = this.plotWidth;
        const height = this.plotHeight;

        // 2. Scale
        const maximumNumberPlayers = Math.max(...dataset.map(point => point.numberPlayers));
        const yMax = Math.max(2, Math.round(maximumNumberPlayers * 0.1)) + maximumNumberPlayers;
        const xs = dataset.map(point => point.time);
        const xMax = +10 + Math.max(...xs);
        const xMin = -10 + Math.min(...xs);

        const xScale = d3.scaleTime()
            .domain([timeMinutesToDate(xMin), timeMinutesToDate(xMax)])
            .range([0, width]);
        this.xScale = xScale;

        const yScale = d3.scaleLinear()
            .domain([0, yMax])
            .range([height, 0]);

        // 3. d3's line generator
        const line = d3.line<{ time: TimeMinutes, numberPlayers: number }>()
            .x(d => xScale(timeMinutesToDate(d.time)))
            .y(d => yScale(d.numberPlayers))
            .curve(d3.curveBasis);

        // 4. Set svg size
        const svgElement = d3.select(this.$refs.svg)
            .attr('width', this.svgWidth)
            .attr('height', this.svgHeight);
        svgElement.selectAll('*').remove();
        const plot = svgElement
            .append('g')
            .attr('class', 'plot')
            .attr('transform', `translate(${this.margin.left}, ${this.margin.top})`);
        plot
            .append('rect')
            .attr('class', 'fake-rect-for-pointer-events')
            .attr('height', height)
            .attr('width', width)
            .attr('fill', 'transparent');

        // todo extract
        this.cursorVerticalLine = plot.append('line')
            .attr('visibility', 'hidden')
            .attr('class', 'cursor-vertical-line');

        // 3. Axis
        plot.append('g')
            .attr('transform', `translate(0, ${height})`)
            .call(d3.axisBottom(xScale));

        const yAxisTicks = yScale.ticks()
            .filter(tick => Number.isInteger(tick));
        const yAxis = d3.axisLeft(yScale)
            .tickValues(yAxisTicks)
            .tickFormat(d3.format('d'));
        plot.append('g')
            .call(yAxis); // Create an axis component with d3.axisLeft

        // 4. Axis labels
        svgElement.append('text')
            .attr('y', height + this.margin.top + this.margin.bottom * 0.9)
            .attr('x', (width + this.margin.left) / 2)
            .style('text-anchor', 'middle')
            .text('Date');

        svgElement.append('text')
            .attr('transform', 'rotate(-90)')
            .attr('y', this.margin.left * 0.1)
            .attr('x', -(this.margin.top + height / 2))
            .attr('dy', '1em')
            .style('text-anchor', 'middle')
            .text('Number players');

        // 5. Append the path, bind the data, and call the line generator
        plot.append('path')
            .datum(dataset)
            .attr('class', 'line')
            .attr('d', line);

        // todo: extract
        plot.on('mouseleave', () => {
          this.cursorTime = null;
        });
        plot.on('mousemove', (d, i, nodes) => {
          const [x, y] = d3.mouse(nodes[i]);
          const cursorTime = dateToTimeMinutes(xScale.invert(x));
          if (this.timeBegin <= cursorTime && cursorTime < this.timeEnd) {
            this.cursorTime = cursorTime;
          } else {
            this.cursorTime = null;
          }
        });
      },
      updateVerticalLine(timeMinutes) {
        if (timeMinutes === null) {
          this.cursorVerticalLine.attr('visibility', 'hidden');
        } else {
          const date = timeMinutesToDate(timeMinutes);
          this.cursorVerticalLine
              .attr('visibility', 'visible')
              .attr('x1', this.xScale(date))
              .attr('y1', 0)
              .attr('x2', this.xScale(date))
              .attr('y2', this.plotHeight);
        }
      },
    },
    computed: {
      svgSize() {
        // https://github.com/vuejs/vue/issues/844#issuecomment-390498696
        return (this.svgHeight, this.svgWidth, Date.now());
      },
      plot() {
        return new Plot(this.games, this.plotWidth, this.timeBegin, this.timeEnd);
      },
      plotNumberPlayers() {
        return this.plot.getPlot();
      },
      ...mapState(['pageWidth', 'pageHeight']),
    },
  };
</script>
