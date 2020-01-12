import axios from 'axios';

export default class Api {
  static async getServerInfo(serverId: ServerId, time_begin: TimeMinutes, time_end: TimeMinutes): Promise<Game[]> {
    const params = { time_end, time_begin };
    const { games } = (await axios.get(`/server/${serverId}`, { params })).data;
    for (const game of games) {
      game.playersIntervals = game.playersIntervals
          .map(([name, begin, end]) => ({ name, begin, end } as PlayerInterval));
    }
    return games;
  }
}
