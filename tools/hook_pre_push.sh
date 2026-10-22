#!/bin/sh
# Git pre-push hook (installed by tools/installa_hook.py): runs the cheap checks of the CI and refuses the push when they fail.
# Skip it once with `git push --no-verify`; a push that only deletes a remote branch is never checked.
root=$(git rev-parse --show-toplevel) || exit 0
while read -r _ local_sha _ _; do
  [ "$local_sha" = "0000000000000000000000000000000000000000" ] && exit 0
done
cd "$root" && exec python3 tools/pre_pr.py --veloce
