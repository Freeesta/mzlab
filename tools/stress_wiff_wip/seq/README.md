Seeded random UI sequences over real .wiff files on the built site (site/ must exist). Data never stored here.
Run from this folder (needs playwright, chromium, built site/):
  python3 stress.py SEED NSTEPS PATH1 [PATH2 ...]     # PATH = file path without ".wiff" (a .wiff.scan must sit next to it)
  ./batch.sh FIRSTSEED COUNT NSTEPS PATH1 ...         # COUNT parallel seeds, outputs out_<seed>.txt, log_<seed>.txt (action log), fail_<seed>_<step>.png
  STRESS_WIFF_DIR=/dir/with/files python3 probe.py NAME   # probe*.py: list file kinds / panels
Failures are written as (step, message); the action log replays a sequence by hand. MZLAB_REPO overrides the repo path.
