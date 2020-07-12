import axios from 'axios';

export default class Api {
  static async getMainPageInfo(): Promise<MainPageInfo> {
    const info = (await axios.get('/main-page')).data;
    return Object.freeze(info) as MainPageInfo;
  }

  static async searchServers(query: string): Promise<GameSearchInfo[]> {
    // todo
    //  RFC says that symbols []{} must be encoded in query params
    //  browsers (at least chrome and firefox) don't encode these symbols (because it may break existing apps)
    //  rocket follows the RFC, so it not work well with existing browsers
    //  https://github.com/SergioBenitez/Rocket/issues/924

    const params = { query };
    return (await axios.get('/search-servers', { params })).data;
  }

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
