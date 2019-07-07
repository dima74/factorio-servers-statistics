#!/bin/zsh -e

cd `dirname $0`
mkdir data
cd data

NUMBER_HOURS=48
for i in $(seq -w 0 $((${NUMBER_HOURS} * 60 - 1))); do
    FILE=${i}.json
    curl -sS --retry 3 "https://multiplayer.factorio.com/get-games?username=${FACTORIO_USERNAME}&token=${FACTORIO_TOKEN}" >${FILE}
    #bzip2 ${FILE}
    echo ${i}
    sleep 60
done

#bunzip2 *