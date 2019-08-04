import { assert } from '@/util';

export default class Plot {
  private begin: TimeMinutes;
  private end: TimeMinutes;
  private playersIntervals: PlayerInterval[];

  private numberParts: number;
  private partDuration: TimeMinutes;

  private partsPlayerList: Array<PlayerInterval[]>;
  private partsPlayerCount: number[];

  constructor(games: Game[], width: number, begin: TimeMinutes, end: TimeMinutes) {
    assert(begin < end);
    // todo uncomment
    // assert(end - begin >= 24 * 60);
    assert(width >= 100);

    // [begin, end)
    this.begin = begin;
    this.end = end;
    this.playersIntervals = this.extractIntervals(games);

    // разбиваем интервал [begin, end) на width частей
    const MINIMUM_PART_DURATION = 10;  // minutes
    this.numberParts = Math.min(Math.ceil(this.duration / MINIMUM_PART_DURATION), width);
    this.partDuration = this.duration / this.numberParts;

    this.buildPlot();
  }

  private get duration() {
    return this.end - this.begin;
  }

  private extractIntervals(games: Game[]): PlayerInterval[] {
    return games.flatMap(game =>
        game.playersIntervals.filter(playerInterval =>
            !(playerInterval.begin >= this.end || playerInterval.end <= this.begin)
        ),
    );
  }

  buildPlot() {
    this.partsPlayerList = Array.from(Array(this.numberParts), () => []);
    this.partsPlayerCount = Array(this.numberParts).fill(0);
    for (const playerInterval of this.playersIntervals) {
      assert(playerInterval.end !== null);
      assert(playerInterval.begin < playerInterval.end);
      const timeBegin = Math.max(playerInterval.begin, this.begin) - this.begin;
      const timeEnd = Math.min(playerInterval.end, this.end) - this.begin;
      let partBegin = timeBegin / this.partDuration;
      let partEnd = timeEnd / this.partDuration;
      assert(0 <= partBegin && partBegin < this.numberParts);
      assert(0 <= partEnd && partEnd <= this.numberParts);
      assert(partBegin < partEnd);
      // [partBegin, partEnd)
      // partI == [partI * duration, (partI + 1) * duration)

      if (Math.floor(partBegin) + 1 === Math.ceil(partEnd)) {
        const partIndex = Math.floor(partBegin);
        this.addToPartsPlayerListIfNotPresent(partIndex, playerInterval);
        continue;
      }

      if (!Number.isInteger(partBegin)) {
        const partBeginInteger = Math.floor(partBegin);
        assert(0 <= partBeginInteger && partBeginInteger < this.numberParts);
        this.addToPartsPlayerListIfNotPresent(partBeginInteger, playerInterval);
        this.partsPlayerCount[partBeginInteger] += (partBeginInteger + 1) - partBegin;
        partBegin = Math.ceil(partBegin);
      }
      if (!Number.isInteger(partEnd)) {
        const partEndInteger = Math.floor(partEnd);
        if (partEndInteger != this.numberParts && partEndInteger + 1 !== partBegin) {
          assert(0 <= partEndInteger && partEndInteger < this.numberParts);
          this.partsPlayerList[partEndInteger].push(playerInterval);
          this.partsPlayerCount[partEndInteger] += partEnd - partEndInteger;
        }
        partEnd = Math.floor(partEnd);
      }

      for (let partIndex = partBegin; partIndex < partEnd; ++partIndex) {
        this.partsPlayerList[partIndex].push(playerInterval);
        this.partsPlayerCount[partIndex] += 1;
      }
    }

    if (process.env.NODE_ENV === 'development') {
      this.checkPlot();
    }
  }

  private addToPartsPlayerListIfNotPresent(partIndex: number, playerInterval: PlayerInterval) {
    const partPlayerList = this.partsPlayerList[partIndex];
    if (partPlayerList.find(playerInterval2 => playerInterval2.name == playerInterval.name) === undefined) {
      partPlayerList.push(playerInterval);
    }
  }

  private checkPlot() {
    for (const playerIntervals of Object.values(this.partsPlayerList)) {
      const players = playerIntervals.map(playerInterval => playerInterval.name);
      assert(players.length === new Set(players).size);
    }
  }

  getPlot(): { time: TimeMinutes, numberPlayers: number }[] {
    return this.partsPlayerCount.map((numberPlayers, partIndex) => {
      const time = this.begin + this.duration * (partIndex + 0.5) / this.numberParts;
      return { time, numberPlayers };
    });
  }

  getPlayersAt(time: TimeMinutes): PlayerInterval[] {
    time = time - this.begin;
    assert(0 <= time && time < this.duration, `time: ${time}, duration: ${this.duration}`);
    let partIndex = Math.floor(time / this.partDuration);
    partIndex = Math.max(0, Math.min(this.numberParts - 1, partIndex));
    return this.partsPlayerList[partIndex]
        .sort((player1, player2) => player1.begin - player2.begin);
  }
}
