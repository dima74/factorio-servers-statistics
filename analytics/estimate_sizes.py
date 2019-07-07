time_data = 2880  # минут
time_target = 10 * 365 * 24 * 50  # 10 лет в минутах
time_coefficient = time_target / time_data

# players intervals
players_changes_all = 27124
player_interval_size_bytes = 12  # (id игрока, начало интервала, конец интервала)
players_intervals_size_bytes = players_changes_all / 2 * player_interval_size_bytes * time_coefficient
print(f'{players_intervals_size_bytes / 1024 ** 3:g} GB')

# sizeof структуры Game (не включает сами данные)
games_all = 17232
game_struct_size_bytes = 112
game_structs_size_bytes = games_all / 2 * game_struct_size_bytes * time_coefficient
print(f'{game_structs_size_bytes / 1024 ** 3:g} GB')
