# Возможности нашего сайти
## Основная
Узнать статистику посещений конкретного сервера за последнее время (~неделю)
* каждый сервер должен иметь уникальный идентификатор (просто использовать server_id не получится из-за мультисерверов)
* за последнюю неделю могли происходить рестарты сервера (⇒ после каждого будет новый game_id), их надо учитывать
* нечёткий поиск сервера по имени

## Дополнительные
* Узнать статистику посещений конкретного сервера за всё время
    - интересна также история изменений метаинформации о сервере (название, версия, моды)
* Момент времени и id сервера, когда на этом сервере число игроков было наибольшим среди всех серверов (мб топ-10 таких пар?)
* График онлайна за последний день/неделю в виде картинки для вставки в discord сервера / его сайт
* График с общей статистикой посещений топ-10 (?) серверов за неделю (на главной странице, для привлечения внимания)

# Проблемы

## Что использовать в качестве id сервера?
* просто нумеровать все сервера
* server_id + номер мультисервера

## Как хранить метаинформацию о сервере (он может часто перезапускаться, но метаинформация не изменяется)
## Как хранить ники игроков (учитывая `sizeof(String)` == 24 и, кажется, длина ника ограничена 32)

## Непонятно, как объединять различные игры (игра == game_id), и надо ли вообще
- одному server_id могут соответствовать несколько game_id
- игра может пропадать на некоторое время (вплоть до 1929 минут), и возвращаться с тем же game_id
    * радует что за 48 часов таких игр было немного (~50 игр, для каждой 2-3 исчезновения)

# Things to consider
* `tags` может содержать пустые строки, в частности `tags: ["", ""]` (но иногда это специально используется как разделитель)
* players может содержать пустые строки
* В теории разные игры одного мультисервера могут иметь одинаковое название
* Максимальная наблюдаемая длина имени игры — 50 символов. Есть ощущение, что имя специально обрезается (даже в /get-game-details), потому что вместо `Name of the game as it will appear in the game list` возвращается `Name of the game as it will appear in the game lis` (без последней буквы, как раз получается 50 симолов)
* Может быть стоит сразу удалять все игры с названием `Name of the game as it will appear in the game lis`