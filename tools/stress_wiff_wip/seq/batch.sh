#!/bin/bash
# batch.sh FIRSTSEED COUNT NSTEPS files...
cd "$(dirname "$0")"
a=$1; n=$2; st=$3; shift 3
for ((s=a; s<a+n; s++)); do python3 stress.py $s $st "$@" > out_$s.txt 2>&1 & done
wait
