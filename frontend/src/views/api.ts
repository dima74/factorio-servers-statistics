import axios from 'axios';

export default class Api {
  static async getServerInfo(serverId: ServerId): Promise<Game[]> {
    const { games } = (await axios.get(`/server/${serverId}`)).data;
    for (const game of games) {
      game.playersIntervals = game.playersIntervals
          .map(([name, begin, end]) => ({ name, begin, end } as PlayerInterval));
    }
    return games;
  }
}