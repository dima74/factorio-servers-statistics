import base64
import json
import os
import sys
from collections import namedtuple, defaultdict
from pathlib import Path

import numpy as np
from tqdm.auto import tqdm

# если имя переменной начинается с ts_, то это означает что это переменная — список, являющийся временным рядом (== time series)

root = Path('./data-cutted')
files = sorted(file for file in os.listdir(root) if file.endswith('.json'))
# files = files[:10]

Game = namedtuple('Game', ['game_id', 'name', 'players', 'game_time_elapsed', 'server_id', 'last_heartbeat', 'mod_count'])

ts_games = []
for file in tqdm(files, disable=len(files) <= 100):
    with open(root / file) as f:
        content = f.read()
    games = json.loads(content)
    for game in games:
        game['players'] = frozenset(game.get('players', []))
    games = [Game(**game) for game in games]
    ts_games.append(games)
print(f'данные за {len(ts_games)} минут ({len(ts_games) / 60:.3g} часов)\n')

for games in ts_games:
    assert len(set(game.game_id for game in games)) == len(games), 'found non-unique game_id in single response'

ts_games_by_game_id = []
ts_games_by_server_id = []
for games in ts_games:
    games_by_server_id = defaultdict(list)
    games_by_game_id = dict()
    for game in games:
        games_by_server_id[game.server_id].append(game)
        games_by_game_id[game.game_id] = game
    ts_games_by_server_id.append(games_by_server_id)
    ts_games_by_game_id.append(games_by_game_id)

games_by_game_id = {}
for minute, games in enumerate(ts_games):
    for game in games:
        if game.game_id in games_by_game_id:
            game_old = games_by_game_id[game.game_id]
            for field in ['game_id', 'name', 'server_id', 'mod_count']:
                if getattr(game, field) != getattr(game_old, field):
                    print(f'warning: game_id {game.game_id} has different values for field `{field}` at {minute}: \n\t{getattr(game, field)}\n\t{getattr(game_old, field)}', file=sys.stderr)
        games_by_game_id[game.game_id] = game
games_all = games_by_game_id.values()


# ts_multiservers_by_id = [{server_id: [server for ]}]

# for servers_by_id in ts_games_by_server_id:
#     for server_id in servers_by_id:
#         if server_id not in multiservers_ids:
#             servers_by_id[server_id] = servers_by_id[server_id][0]


# мультисервер — сервер (server_id), которому хотя бы раз соответствовало несколько game_id
# multiservers — map из server_id в пару
#   * set всех game_id для этого мультисервера
#   * максимальное количество одновременных game_id для этого server_id

# @dataclass
# class Multiserver:
#     game_ids_all: Set[int]
#     ts_game_ids: List[Set[int]]
#
#
# multiservers = {}
# for servers in ts_servers:
#     server_ids = [server.server_id for server in servers]
#     for server_id, count in Counter(server_ids):
#         if count > 1:
#             server = multiservers.setdefault(server_id, (0, set()))
#             server.count = max(server.count, count)
#             # = max(multiservers[server_id], count)


def get_field(ts_games, field):
    return [set([getattr(game, field) for game in games]) for games in ts_games]


ts_server_ids = get_field(ts_games, 'server_id')
ts_game_ids = get_field(ts_games, 'game_id')
ts_players = get_field(ts_games, 'players')
ts_game_names = get_field(ts_games, 'name')
ts_number_games = [len(games) for games in ts_games]

server_ids_all = set.union(*ts_server_ids)
game_ids_all = set.union(*ts_game_ids)
players_all = frozenset.union(*set.union(*ts_players))
game_names_all = set.union(*ts_game_names)
assert len(game_ids_all) == len(games_all)

for server_id in server_ids_all:
    assert len(server_id) == 44
    assert len(base64.b64decode(server_id)) == 32

with open('results/server_ids.txt', 'w') as f:
    for server_id in server_ids_all:
        print(server_id, file=f)

def describe_lengths(lengths):
    qs = [10, 25, 75, 90, 95, 99]
    percentilies = [np.percentile(lengths, q, interpolation='nearest') for q in qs]
    return '\n'.join([
        f'min:       {np.min(lengths)}',
        f'max:       {np.max(lengths)}',
        f'mean:      {np.mean(lengths):g}',
        f'median:    {np.percentile(lengths, 50, interpolation="nearest")}',
        f'quantiles: {percentilies}'
    ])


print(len(server_ids_all), '- всего наблюдаемых серверов')
print(len(game_ids_all), '- всего наблюдаемых game_id')
print(len(players_all), '- всего наблюдаемых игроков')
print()

print(f'{np.mean(ts_number_games):g} — среднее число игр одновременно')
print(f'{np.median(ts_number_games):g} — медиана числа игр одновременно')
print()

print(len(ts_server_ids[-1] - ts_server_ids[0]), '- появилось серверов')
print(len(ts_server_ids[0] - ts_server_ids[-1]), '- исчезло серверов')
print(len(server_ids_all - ts_server_ids[0] - ts_server_ids[-1]), '- появилось и исчезло серверов')
print()

print(len(ts_game_ids[-1] - ts_game_ids[0]), '- появилось game_id')
print(len(ts_game_ids[0] - ts_game_ids[-1]), '- исчезло game_id')
print(len(game_ids_all - ts_game_ids[-1] - ts_game_ids[0]), '- появилось и исчезло game_id')
print(max(ts_game_ids[-1]) - max(ts_game_ids[0]), '- число всех новых game_id')
print()

number_mods = [game.mod_count for game in games_all if game.mod_count > 1]
print(f'число игр с модами: {len(number_mods)} из {len(games_all)}')
print('число модов:')
print(describe_lengths(number_mods))
print()

server_id_to_game_ids = defaultdict(set)
for games in ts_games:
    for game in games:
        server_id_to_game_ids[game.server_id].add(game.game_id)
number_game_ids_for_server_id = [len(game_ids) for game_ids in server_id_to_game_ids.values()]
print('количество всех наблюдаемых game_id, соответствущих одному server_id:')
print(describe_lengths(number_game_ids_for_server_id))
print()

multiservers_ids = set(server_id for games_by_server_id in ts_games_by_server_id for server_id, games in games_by_server_id.items() if len(games) > 1)
number_multiserver_game_ids_all = len(set(
    game.game_id
    for games_by_server_id in ts_games_by_server_id
    for server_id, games in games_by_server_id.items()
    if server_id in multiservers_ids
    for game in games
))
print(len(multiservers_ids), '- число мультисерверов')
print(number_multiserver_game_ids_all, '- суммарное число наблюдаемых game_ids для мультисерверов')
print()

multiservers_number_minutes_with_more_than_one_game = [
    sum(
        1
        for games_by_server_id in ts_games_by_server_id
        if len(games_by_server_id[multiserver_id]) > 1
    )
    for multiserver_id in multiservers_ids
]
print('число минут, когда мультисерверу соответствовало 2 и больше игр:')
print(describe_lengths(multiservers_number_minutes_with_more_than_one_game))
print()

with open('results/multiservers.txt', 'w') as f:
    for server_id in multiservers_ids:
        print(server_id, file=f)
        printed_game_ids = set()
        for games_by_server_id in ts_games_by_server_id:
            for game in games_by_server_id.get(server_id, []):
                if game.game_id not in printed_game_ids:
                    printed_game_ids.add(game.game_id)
                    print('\t', game.name, sep='', file=f)

player_nick_lengths = [len(player) for player in players_all]
print('имя ника:')
print(describe_lengths(player_nick_lengths))
print()

game_name_lengths = [len(name) for name in game_names_all]
print('название игры:')
print(describe_lengths(game_name_lengths))
print()

players_changes_all = 0
players_initial_all = 0
for D1, D2 in zip(ts_games_by_game_id[:-1], ts_games_by_game_id[1:]):
    game_ids_prev = set(D1.keys())
    game_ids_curr = set(D2.keys())
    game_ids_common = game_ids_prev & game_ids_curr
    game_ids_added = game_ids_curr - game_ids_prev
    game_ids_removed = game_ids_prev - game_ids_curr
    for game_id in game_ids_common:
        game_prev = D1[game_id]
        game_curr = D2[game_id]
        players_changes_all += len(game_curr.players ^ game_prev.players)
    for game_id in game_ids_added:
        players_initial_all += len(D2[game_id].players)
    for game_id in game_ids_removed:
        players_initial_all += len(D1[game_id].players)
print(players_changes_all, '- общее число входов и выходов игроков')
print(players_initial_all, '- общее число игроков, которые были в игре, когда она появилась или исчезла')
print()

game_durations = []  # в минутах
for game_id in game_ids_all:
    minute_indexes = [minute_index for minute_index, games_by_game_id in enumerate(ts_games_by_game_id) if game_id in games_by_game_id]
    diffs = np.diff(minute_indexes)
    game_duration = max(minute_indexes) - min(minute_indexes)

    diffs = diffs[diffs != 1]
    # if len(diffs) > 0:
    #     print(f'warning: игра #{game_id} (game_duration = {game_duration}) исчезала на следующие количества минут: {diffs}', file=sys.stderr)

    if (game_id not in ts_games_by_game_id[0]) and (game_id not in ts_games_by_game_id[-1]):
        game_durations.append(game_duration)
print(f'продложительность игры в минутах (для {len(game_durations)} из {len(game_ids_all)} игр):')
print(describe_lengths(game_durations))
print()
