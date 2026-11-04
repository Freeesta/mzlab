# soak: long runs, OPFS state, multi-tab and killed-tab checks (site, real .wiff files)
Needs the built site (`python3 tools/build_site.py`), Playwright, and a folder with FullScan.wiff, MRM.wiff, MS2.wiff, SIM.wiff (+ `.wiff.scan`, copy or rename your own files).
Run from the repo root: `export STRESS_WIFF_DIR=/path/to/dir MZLAB_REPO=$PWD; cd tools/stress_wiff_wip/soak`. Logs/profiles go to /tmp/stress_soak (create it).
- `soak.py N SEED [QUOTA_BYTES]`: N x (open random wiff, add panels, remove all, wait 6.6 s, walk OPFS); logs LEAK / EMPTY panel / LR_MISS to soak_SEED.log.
- `tabs_min.py NTABS KEY`: NTABS tabs of one persistent profile open the same wiff together; prints LR_MISS and file counts per tab. `two.py KEY ID` (env NT) dumps the rows and toasts.
- `tabs.py KEY`: 3 tabs, then removal in one tab, reopen. `kill2.py KEY 0 ID`: kills the page mid-copy, relaunches the profile, reopens, removes. `kill.py KEY DELAY ID`: timed kill.
- A headless tab only advances while a rAF poll runs (`wait_for_function`); a bare `wait_for_timeout` leaves the load stuck at "0 of 1" (harness artifact).
