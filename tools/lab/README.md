# Lab site (Freeesta/qqq)

Parallel build of mzLab for a lab: only Data and Draw, a banner, `noindex`, and it stops
working after 2026-12-29 (UTC). It follows `main` of `Freeesta/mzlab` by itself.
The expiry is a soft block (the date is checked against the visitor's clock), not security.

Three manual steps for Federico:

1. Create the public repository `Freeesta/qqq` (no files needed).
2. In it: Settings > Pages > Build and deployment > Source = **GitHub Actions**.
3. Copy `tools/lab/qqq-pages.yml` to `.github/workflows/pages.yml` in `qqq` and commit. It builds
   every day (06:17 UTC) and on **Run workflow**; no secrets, no token. The site is at
   https://freeesta.github.io/qqq/.

GitHub switches off a scheduled workflow after 60 days without activity in the repository: if the
builds stop, press **Run workflow** (or push any commit).

To change the date, edit `MZLAB_LAB_EXPIRES` in that workflow. Local test:
`MZLAB_LAB=1 MZLAB_LAB_EXPIRES=2026-12-29 python3 tools/build_site.py`.
