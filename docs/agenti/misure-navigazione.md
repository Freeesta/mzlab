# Warm page loads: navigation preload and `?nav=cache` (site only)

Site only: the desktop app serves `mzlab://` locally and has no service worker. Behaviour of both modes: `service-worker.md`. Tool: `tests_e2e/misura_avvio_sw.py --varianti` (scenarios `warm`, `warm-restarted` as in `misure-caricamento.md`, «Restarted service worker»).

Variants, built from the same commit except `sw.js` and its page-side files (`tools/prepara_sito.py`, engine `.wasm` included), each served by its own local mirror with GitHub-Pages headers and the same added delay on every answer:
- **main**: network first, the page fetched after the manifest answers (`navegar` awaited `expected()` before `traer`).
- **A**: network first with navigation preload: the browser asks the page with the start of the navigation, the worker checks it against the manifest (default mode of this branch).
- **B**: A plus `?nav=cache` (switch set on the first visit, the measured reload has no query): the page from the complete cache, the manifest checked in the background.

```
python3 tests_e2e/misura_avvio_sw.py --varianti main=DIR_MAIN,A=DIR_BRANCH,B=DIR_BRANCH?nav=cache --runs 5 --rtt 0,40,150 --cpu 1 --scenari warm,warm-restarted --out cpu1.json
python3 tests_e2e/misura_avvio_sw.py --varianti main=DIR_MAIN,A=DIR_BRANCH,B=DIR_BRANCH?nav=cache --runs 5 --rtt 0,40,150 --cpu 4 --scenari warm,warm-restarted --out cpu4.json
```

## Deploy: loads until a new version shows (`e2e_sw_versione`, asserted)

| mode | 1st load after the deploy | 2nd load |
|---|---|---|
| network first (default, main and A) | new version | new version |
| `?nav=cache` (B) | old version from the cache; once the newer worker has filled its cache in `install` (2.5 s + the changed files), the line «New version ready · Reload» | new version (Reload or any load: the old worker hands over to the waiting one) |
