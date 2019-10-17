#!/bin/zsh -e

STATE_BIN_XZ='../../temp/state/state.bin.xz'

cd $(dirname $0)
rm -rf state.bin
xz -cdk ${STATE_BIN_XZ} >state.bin
#rm -rf xz gz bz2 zst lz4
mkdir -p xz gz bz2 zst lz4
alias time='/usr/bin/time -f "%e %M"'

function run() {
	type=$1
	extension=$2
	level_start=$3
	level_end=${4:-9}

	echo ${type}
	for level in {$level_start..$level_end}; do
		file="${extension}/${level}.${extension}"
		time_output=$(time 2>&1 $type -q -ck "-${level}" state.bin >$file)
		read command_time command_memory <<<$time_output
		size=$(du -k $file | cut -f1)
		printf "%2s:  %5.1fMB  %6ss  %6s\n" $level $((command_memory / 1024.0)) $command_time $size
	done
	echo ''
}

echo '<level>  <ram>  <time>  <result size>\n'
run lz4 lz4 1 12
run zstd zst 1 19
run bzip2 bz2 1 9
run gzip gz 1 9
run xz xz 0 9

# ls -s1 --block-size=1K state.bin xz gz bz2 zst lz4
rm -rf xz gz bz2 zst lz4
