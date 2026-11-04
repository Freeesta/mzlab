# race: seeded concurrency/rapid-input bursts on real .wiff files (WIP, not part of the product)
Needs `site/` built (`python3 tools/build_site.py`) and served: `python3 tests_e2e/servi.py PORT site &`.
Run from the repo root:
    export STRESS_WIFF_POOL='["/path/a.wiff","/path/b.wiff",...]'   # each with its .wiff.scan beside it; >=6 files, mixed types
    export STRESS_OUT=/tmp/stress_race/                              # logs, screenshots, symlinks (renamed P0..Pn)
    python3 tools/stress_wiff_wip/race/burst.py START END [PORT]
Burst seed = its index; every action is logged to `burst_START.log` before it runs. Failures write fail_SEED.png/json.
Known false positives: a conversion row in state `cancel` stays until Close (flagged as "stuck rows").
