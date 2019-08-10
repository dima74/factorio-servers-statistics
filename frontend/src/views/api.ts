import axios from 'axios';

export default class Api {
  static async getServerInfo(serverId: ServerId, timeBegin: TimeMinutes, timeEnd: TimeMinutes): Promise<Game[]> {
    const { games } = (await axios.get(`/server/${serverId}?time_begin=${timeBegin}&time_end=${timeEnd}`)).data;
    for (const game of games) {
      game.playersIntervals = game.playersIntervals
          .map(([name, begin, end]) => ({ name, begin, end } as PlayerInterval));
    }
    return games;
  }
}
