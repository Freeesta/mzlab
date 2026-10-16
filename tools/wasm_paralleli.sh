# Sourced by tools/build_feat_wasm.sh and tools/build_lettori_wasm.sh: runs the wasm-bindgen + wasm-opt of a module in the background while cargo builds the next one
# (the final link of a crate is single-threaded and so is wasm-opt, so they overlap on the 4 CPUs of a runner). Output stays in order: each job logs to a file, wasm_wait prints them.
# wasm_pack <dir> <crate wasm file> <limit> <label> [wasm-opt flags]: copies the .wasm cargo just wrote (the next build overwrites it) and packs it into <dir>.
wasm_jobs=$(mktemp -d)
wasm_pids=""
wasm_n=0
wasm_pack() {
  local dir=$1 src=$2 limit=$3 label=$4; shift 4
  wasm_n=$((wasm_n + 1))
  local own="$wasm_jobs/$wasm_n"
  mkdir -p "$own"
  cp "$src" "$own/"
  local file=$(basename "$src")
  (
    wasm-bindgen --target web --no-typescript --out-dir "$dir" "$own/$file" </dev/null &&
      tools/wasm_opt.sh "$dir/${file%.wasm}_bg.wasm" "$limit" "$label" "$@" </dev/null
  ) > "$own.log" 2>&1 &
  wasm_pids="$wasm_pids $!:$wasm_n"
}
# waits for every job, prints their logs in the order they started, fails if one failed
wasm_wait() {
  local bad=0 p
  for p in $wasm_pids; do
    wait "${p%%:*}" || bad=1
    cat "$wasm_jobs/${p##*:}.log"
  done
  wasm_pids=""
  rm -rf "$wasm_jobs"
  [ "$bad" = 0 ]
}
