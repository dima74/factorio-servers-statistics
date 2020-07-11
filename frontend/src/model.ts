type TimeMinutes = number;
type ServerId = string;
type GameId = number;

class PlayerInterval {
  name: string;
  begin: TimeMinutes;
  end: TimeMinutes;
  isOnline?: boolean;
}

class Mod {
  name: string;
  version: string;
}

class Game {
  gameId: GameId;
  serverId: ServerId;
  prevGameId: GameId;
  nextGameId?: GameId;
  timeBegin: TimeMinutes;
  timeEnd?: TimeMinutes;

  playersIntervals: PlayerInterval[];

  hostId: string;
  name: string;
  description: string;
  maxPlayers: number;
  gameVersion: string;
  gameTimeElapsed: number;
  hasPassword: boolean;
  tags: string[];
  modCount: number;

  hostAddress: string;
  mods?: Mod[];
}

class GameSearchInfo {
  serverId: ServerId;
  name: string;
  timeBegin: TimeMinutes;
  timeEnd?: TimeMinutes;
}

class TopGamesByNumberPlayersNow {
  server_id: ServerId;
  name: string;
  number_players: number;
}

class TopGamesByNumberPlayersMax {
  server_id: ServerId;
  name: string;
  number_players: number;
  time: TimeMinutes;
}

class MainPageInfo {
  topGamesByNumberPlayersNow: TopGamesByNumberPlayersNow[];
  topGamesByNumberPlayersMax: TopGamesByNumberPlayersMax[];
}
