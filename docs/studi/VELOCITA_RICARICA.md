# Reload speed: first visit, repeat visit, «Full reset», clicks

Study of why the site is slow to come back after «New session → Full reset» and why it feels late in general.
Question from Federico: is it GitHub (Pages hosting) or do we download things before showing anything? Findings only: no product code was changed.

**Answer in one line**: mostly us. The service worker (`sw.js`), not GitHub, is the biggest single cost.
- It sends every one of the ~400 start-up files through the network again (`cache: "no-cache"`), so each one waits ~150 ms even on localhost.
- «Full reset» throws away the service worker and its cache, so the next two loads pay that cost from scratch.
- The page after a reset loses cross-origin isolation until the user reloads by hand.
- GitHub Pages adds about 1–2.5 s only to the very first visit (one round trip per dependency level, ~150 ms from the cloud runner, much less from Italy), and nothing measurable to repeat visits.

## 1. Method
- Probe: `tests_e2e/misura_ricarica.py` (headless Chromium 1243, 1400×900, 4 CPU cloud runner), summary `tools/misura_ricarica_riassunto.py RUN.json --dettaglio STEP`.
- Steps of one run, in one browser context:
  1. **cold**: new context, open `#/data`, wait until the page is usable.
  2. **warm**: reload.
  3. **file**: example `FullScan_t15.mzML` until the first plot (on `main` the probe clicks «Load data»).
  4. **clicks**: Disegno, Dati, settings, Esc, file row.
  5. **reset**: «New session → Full reset» with the file open.
  6. **warm-after-reset**: the reload that follows.
- **Usable** = engine ready (`qqReady`) and `#drop` visible, plus `crossOriginIsolated` except after a reset (§3).
- Every page records TTFB, FCP, LCP, long tasks, event timings, the whole resource waterfall and the engine phases. CDP on the service worker records what it really asks the network (200 or revalidated, bytes on the wire).
- Targets:
  - **live**: https://freeesta.github.io/mzlab/ through the cloud proxy (HTTP/2, Fastly edge `iad`, TTFB of a 1 KB file 140–530 ms, median 150 ms).
  - **mirror**: the same bytes served from localhost with the GitHub Pages headers (ETag, Last-Modified, `max-age=600`, gzip; HTTP/1.1). Network latency ≈ 0, so what remains is our own work.
  - **mirror --no-sw**: the mirror with the service worker blocked, to isolate its cost.
- Profiles:
  - `none`.
  - `cpu4`: CPU ×4 on the page and its workers.
  - `slow`: Fast 4G bandwidth (9 Mbps down) + CPU ×4. On live no latency is added, because the real path already has ~150 ms, the Fast 4G latency. On the mirror 165 ms are added.
- Runs: live ×3, mirror ×2 (after a warm-up run that fills the mirror), mirror `slow`, live `cpu4`, live `slow`, no-SW ×2 and no-SW `slow`. Times are ms from the start of the step; compare rows, not absolute values.

## 2. Numbers

**Time until the page is usable (ms)**

| run | cold | warm | first file | Full reset | reload after reset |
|---|---|---|---|---|---|
| live 1 / 2 / 3 | 7852 / 5889 / 6883 | 907 / 809 / 821 | 1705 / 1336 / 1556 | 5367 / 1284 / 1371 | 1005 / 5155 / 3773 |
| mirror 1 / 2 (no network latency) | 5620 / 5122 | 1199 / 794 | 1801 / 1236 | 4225 / 3201 | 3806 / 2205 |
| mirror **without service worker** 1 / 2 | 4562 / 4686 | **347 / 388** | **851 / 855** | **1064 / 1118** | **376 / 476** |
| live `cpu4` | 8059 | 1517 | 3013 | 5129 | 9090 |
| live `slow` (Fast 4G + CPU ×4) | 10404 | 2447 | 3319 | 4640 | 4849 |
| mirror `slow` (165 ms + 9 Mbps + CPU ×4) | 7864 | 2087 | 3470 | 4699 | 3167 |
| mirror `slow` without service worker | 17283* | 1608 | 9716* | 1957 | 1560 |

\* HTTP/1.1 mirror with 165 ms per request and 6 connections: not comparable with HTTP/2 GitHub Pages for a cold load (the 9.7 s file step is the 5.8 MB example download at 9 Mbps).

**Paint and engine marks**

| run, step | TTFB | FCP | drop visible | engine ready | pages loaded |
|---|---|---|---|---|---|
| live 1, cold | 1000 | 4089 | 4088 | 5613 | 2 (isolation reload at ~950 ms) |
| live 1, warm | 67 | 151 | 131 | 898 | 1 |
| live 2, reset | 354 | 618 | 1281 | 1281 | 1, **not isolated** |
| live 2, reload after reset | 56 | 1936 | 1942 | 3378 | 1, isolated, empty service-worker cache |
| mirror 2, cold | 467 | 2617 | 2622 | 5102 | 2 |
| no-SW 1, cold | 18 | 1912 | 1915 | 3516 | 1 |
| no-SW 1, warm | 9 | 83 | 62 | 319 | 1 |

## 3. What happens at «Full reset» (`reset.js`, mode `all`)
- **What it deletes**:
  - OPFS: every folder, `mzlab` included.
  - IndexedDB: every database, `mzlab_lab` included.
  - Cache Storage: every cache, including the app cache `mzlab-app-<hash>` and `mzlab-big-*` (Ketcher, 29 MB).
  - The service-worker registration, `localStorage` and `sessionStorage`.
  - Then `location.replace("?riparti=1")`; the HTTP cache stays.
- **`wipe()` itself is fast**: 13–30 ms when timed alone after a file was opened (report: OPFS `mzlab`, IndexedDB `mzlab_lab` «blocked» by the page's own connection, two caches, one registration).
  - Click → new request was 20–60 ms in isolated tries.
  - In the probe sequences the new page started 270–600 ms after the click in 7 of 10 runs, and 2.9–4.3 s in 3. A 4.3 s case had the document still served by the old worker: the unregister did not finish and `wait(…, 4000)` ran out.
  - So the time comes from the safety timeouts of `wipe()` (4 s on the unregister, 3 s per blocked database), not from deleting.
- **The page after the reset is not isolated** in 9 of 10 service-worker runs.
  - Without a registration its document comes from the network without COOP/COEP.
  - `browser.js` registers `sw.js` again and the worker takes control (`controller` set ~0.3 s later). `isolateOnce` writes `mzlab.coiReload`, but the reload never happens: the tab stays non-isolated until the user reloads by hand.
  - Without isolation there are no shared-memory helpers for indexing (`idxpool.js`), so big files index more slowly after a reset.
  - Reproduce: `python3 tests_e2e/misura_ricarica.py --out r.json`, then in the summary the `reset` page has no `coi` mark.
- **The next reload is slow** (2.2–9.1 s against 0.8 s for a normal warm reload).
  - The new worker has an empty app cache, so every one of the ~420 files goes to the network (424 revalidations in live 2, median 127 ms each) and is written into Cache Storage.
  - FCP moves from ~150 ms to ~1.9 s.
- **What the user sees** (live, same machine):
  1. The reset itself: 1.3 s, sometimes 5.4 s.
  2. A non-isolated page.
  3. Later, the first reload: another 2–9 s (usually Ctrl+R, or the next visit).
  4. Then Ketcher downloads again in the background (29 MB, its cache was deleted).
- Without the service worker the same reset takes 1.06–1.12 s and the next reload 0.38–0.48 s.

## 4. Where the time goes (waterfall of the reload)
**Before first paint**
- 63–64 files, 0.62 MB decoded:
  - 32 language files (277 KB): `lang/it.js` writes 31 `<script>` with `document.write`.
  - 26–27 scripts of the head chain (233 KB).
  - 4 style sheets (105 KB).
- 42 of them are render-blocking (417 KB). The chain has three levels: `index.html` → `lang/it.js` → 31 `lang/it/*.js`.

**Before the engine is ready**
- About 380 files, 7.7 MB decoded:

  | part | files | decoded size |
  |---|---|---|
  | `carica.js` list `L` and modules | 211 scripts | 2.48 MB |
  | engine `mzlab_wasm_bg.wasm` | 1 | 2.53 MB (1.08 MB gzip) |
  | Tabui (`tpmine-loader.js`, «run when the page starts»): `static/mzfinder/` | 125–164 | 1.75–2.36 MB |
  | Tabui: `contaminanti.json` | 1 | 457 KB |
  | language | 32 | 277 KB |

- Tabui alone is a third of the requests, and nothing on the Dati page needs it.
- The `.wasm` is not the bottleneck on this machine: with a warm HTTP cache and no service worker the engine is ready at 319 ms. It is compiled with `compileStreaming` in `rust-supervisor.js`, then instantiated in the worker.

**The service worker: the largest single cost**
- `sw.js` answers every same-origin GET with `fetch(req, { cache: "no-cache" })`, then `cache.match` + `cache.put` when the ETag changed, and re-wraps every response to add COOP/COEP.
- With ~400 requests at once, each one waits for the worker:
  - median 131–191 ms per request on live;
  - **150–180 ms on localhost too**;
  - the slowest scripts wait 1.4 s (`shell.js`, `tpmine-loader.js` in mirror 2 cold).
- The bytes on the wire are tiny (0.02–0.06 MB for a warm reload): the cost is the round trip and the worker's own queue, not the download.
- Blocking it: warm 0.35 s instead of 0.8–1.2 s, reset 1.1 s instead of 3.2–4.2 s, reload after reset 0.4 s instead of 2.2–3.8 s (mirror).

**The isolation reload on the first visit**
- The first page is thrown away at ~0.45–0.95 s, once the worker takes control, before it paints.
- The second page then runs the service-worker cold path: FCP at 2.6 s (mirror) to 4.1 s (live).
- Without the service worker the cold FCP on the same mirror is 1.9 s, with no reload.

**GitHub Pages: what it does and does not do**
- HTTP/2, Fastly CDN, gzip only (no brotli: the 2.53 MB `.wasm` travels as 1.08 MB), weak ETag + `Last-Modified`, `Cache-Control: max-age=600`.
- No custom headers, so no COOP/COEP; that is why the service worker adds them.
- A conditional request answers 304 in ~150 ms from the cloud runner.
- Live vs mirror (same bytes):
  - cold 5.9–7.9 s vs 5.1–5.6 s: about 1–2.5 s of network at ~150 ms per round trip;
  - warm 0.81–0.91 vs 0.79–1.2: no difference;
  - reset: no difference.
- From Torino the round trip to the Fastly edge is shorter, so the hosting share is smaller still.

## 5. Clicks
- Unthrottled, every click has its slowest event at 24–80 ms and no long task: Dati ↔ Disegno, settings, Esc, file row.
- At CPU ×4:
  - clicks stay at 56–96 ms;
  - **after opening Disegno** the main thread has long tasks of 2.1–2.5 s plus 0.5–0.6 s ones. They land on the next click (worst event 152–232 ms) and match the Ketcher bundle starting up (29 MB of script; the `ketcher-ready` frame).
- Opening the first file costs 7–15 long tasks, 0.9–1.4 s in total, at CPU ×4.
- Lazy fetches on click are small: Disegno 5 files, 822 KB (OpenChemLib already prefetched); Dati 2 files, 16 KB.
- **The «noticeable latency when clicking around»** is the Ketcher start-up when Disegno is visited, and the first file opening. The click handlers themselves are fast.
- The sample table (`#samptab`) was not visible on `main` with one file and was not measured.

## 6. Ranking of causes (largest first)
1. **The service worker on the critical path for every file**: network-first with `no-cache`, ~150 ms per request in its queue, on every load. Cost: 0.5–0.8 s per warm load, 2–4 s per load after a reset or on the first visit.
2. **«Full reset» deletes the app cache and the worker**: the reset reload and the next reload go through the cold paths, and the safety timeouts of `wipe()` add 0–4 s.
3. **Isolation is lost after a reset**: `isolateOnce` never reloads, so isolation comes back only at a manual reload, which is then the slow, empty-cache one.
4. **Isolation reload on the first visit**: the first page is discarded before it paints (~0.5–1 s lost, blank screen until FCP at 2.6–4.1 s).
5. **Tabui loads at start**: 125–164 extra requests and 2.2–2.8 MB before the Dati page is usable, all through the worker queue.
6. **Language and head chain**: 32 language files behind `document.write` and 42 render-blocking files; three dependency levels before first paint.
7. **GitHub latency on the first visit**: about 1–2.5 s from the cloud runner. Nothing to configure on Pages (no headers); fewer requests and fewer levels is the only lever.
8. **Ketcher start-up after Disegno**: 2.1–2.5 s main-thread task at CPU ×4.

## 7. Quick wins and their order
1. **Service worker cache-first by site version** (prompt [01](prompt_velocita/01-sw-cache-versione.md)).
   - `sw.js` already carries the hash of the whole site (`mzlab-app-<hash>`, `tools/build_site.py`) and `static/manifest.json` lists every file with its SHA-256.
   - Serve app files from the versioned cache with no per-file revalidation. A new deploy changes `sw.js`, and the new worker fills its cache in the background.
   - Keeps offline use and the COOP/COEP headers.
   - Expected: warm and post-reset reloads near the no-SW numbers (0.35–0.5 s).
2. **«Full reset» keeps what holds no user data** (prompt [02](prompt_velocita/02-reset-snello.md); **decision for Federico**, because `e2e_reset.py` today requires caches and workers to go).
   - Keep the app caches (`mzlab-app-*`, `mzlab-big-*`) and the registration: they hold only the program's own files.
   - Still delete OPFS, IndexedDB, `mzlab-libs` and storage.
   - Run the deletions in parallel with short timeouts.
   - Fix `isolateOnce` after a reset (or keep the worker, which makes the reset page isolated from the first byte).
   - Expected: reset ≈ 1 s, next reload ≈ 0.4–0.8 s, isolation kept.
3. **Tabui after the page is usable** (prompt [03](prompt_velocita/03-tabui-dopo.md)): `tpmine-loader.js` loads `static/mzfinder/` and `contaminanti.json` at idle after `qqReady`, or on the first Tabui use, not at page start. Expected: −125–164 requests and −2.2 MB before usable.
4. **First visit without the lost page** (prompt [04](prompt_velocita/04-primo-arrivo.md)):
   - Do the isolation reload only when it is useful, at the first file drop of a big file, or not at all on the first visit.
   - Ship one language file per language at build time (31 → 1 request, one level less).
   - Expected: cold FCP −0.5–1 s, −31 requests.
5. **Ketcher off the main thread's way** (prompt [05](prompt_velocita/05-ketcher-click.md)): measure the long task with attribution, and start the editor at idle after the first visit to Disegno.

Rejected or not needed:
- **Preloading the engine earlier**: on a warm cache it is ready in 0.3 s without the worker; the queue is the problem.
- **Streaming compile**: already used.
- **A skeleton first**: `#drop` and the header already paint before the engine.
- **Raising the start-up cap**: `MAX_TOTAL` 1 900 000 and the `.wasm` caps never go up.

## 8. Risks
- **Cache-first**: version skew if a page of version N gets files of N+1. The cache is per version (`mzlab-app-<hash>`), and the page uses only the cache of the worker that controls it. An update is taken at the next navigation (`skipWaiting` + `clients.claim` as now) with the `aggiorna.js` notice. Test a deploy between two loads (two builds, one browser).
- **Privacy**: the app caches hold only the published files, never a user file (`static/librerie/` is already excluded from the worker). The rule «the student's files leave nothing behind» still holds if OPFS, IndexedDB and `mzlab-libs` go.
- **Isolation**: `e2e_isolamento.py` must stay green. `?cap-off=isolamento` must still work.
- **Collisions**:
  - Landing D3 #2207 / D1b #2211: `first-paint.js`, `avvio*.js`, landing.
  - LAZY-WIFF (session `session_01Ly8ApmF7tTC43eWmFeBeCs`): slim engine, lazy readers, idle prefetch in `shell.js` / `carica.js`.
  - FILE-GROUPS: `explore.js`.
  - Prompt 03 touches the idle prefetch (`prefetchIdle`), so it goes after LAZY-WIFF or coordinates with it.
  - Prompts 01 and 02 touch `sw.js`, `reset.js`, `browser.js`, which none of the above touch.

## 9. How to measure again
- `python3 tests_e2e/misura_ricarica.py --out live.json`: the live site, through the proxy in the cloud.
- `--mirror DIR`: a local mirror, filled from the live site at its first run; repeat it, the first run is the warm-up.
- `--no-sw`: the same loads without the service worker.
- `--profile cpu4|fast4g|slow [--rtt 165]`: throttled.
- `python3 tools/misura_ricarica_riassunto.py live.json --dettaglio cold,reset,warm-after-reset`: the summary.
- Raw JSON of the runs of this study: not committed (they hold nothing private, but they are 5–15 MB each); re-run to compare.
