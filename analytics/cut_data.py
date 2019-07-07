import json
import os
from pathlib import Path

from tqdm.auto import tqdm

root = Path('./data-cutted')
root.mkdir(exist_ok=True)

data_path = Path('./data')
files = sorted(file for file in os.listdir(data_path) if file.endswith('.json'))
# files = files[:10]

ignored_fields = ['application_version', 'max_players', 'has_password', 'tags', 'has_mods']

for file in tqdm(files):
    with open(data_path / file) as f:
        content = f.read()
    games = json.loads(content)
    games = [server for server in games if 'server_id' in server]
    for game in games:
        game.setdefault('players', [])
        for ignored_field in ignored_fields:
            if ignored_field in game:
                del game[ignored_field]
    with open(root / file, 'w') as f:
        json.dump(games, f, ensure_ascii=False)
