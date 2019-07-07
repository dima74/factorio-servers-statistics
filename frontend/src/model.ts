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
  name: String;
  version: String;
}

class Game {
  gameId: GameId;
  serverId: ServerId;
  prevGameId: GameId;
  nextGameId?: GameId;
  timeBegin: TimeMinutes;
  timeEnd?: TimeMinutes;

  playersIntervals: PlayerInterval[];

  hostId: String;
  name: String;
  maxPlayers: number;
  gameVersion: String;
  gameTimeElapsed: number;
  hasPassword: boolean;
  tags: String[];
  lastHeartbeat: number;
  modCount: number;

  description: String;
  hostAddress: String;
  mods?: Mod[];
}