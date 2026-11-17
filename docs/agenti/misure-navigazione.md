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

## Timings (4 vCPU cloud container, Chromium headless)

A new browser and profile per run; `warm` and `warm-restarted` interleaved over main, A and B, 5 runs per cell, CPU throttled over CDP. The mirror adds the RTT to every answer, the worker's manifest included. Columns: `idx_end` end of the page answer, `fcp` first contentful paint, `engine` engine ready, `qqReady` the page ready; full columns (manifest, boot order) in the tool's report.

**warm, CPU ×1** (ms, median [min-max] of 5)

| RTT | variant | idx_end | fcp | engine | qqReady |
|---|---|---|---|---|---|
| 0 | main | 13 [11-16] | 95 [81-99] | 356 [346-426] | 357 [346-426] |
| 0 | A | 14 [11-17] | 94 [89-95] | 369 [361-381] | 369 [361-382] |
| 0 | B | 6 [5-9] | 94 [80-122] | 346 [322-425] | 346 [322-426] |
| 40 | main | 95 [94-97] | 185 [174-203] | 485 [386-629] | 486 [387-631] |
| 40 | A | 53 [50-61] | 133 [126-216] | 453 [381-665] | 454 [382-666] |
| 40 | B | 7 [6-8] | 92 [87-125] | 395 [340-397] | 396 [341-398] |
| 150 | main | 316 [315-318] | 391 [378-408] | 672 [621-714] | 673 [622-715] |
| 150 | A | 162 [161-166] | 239 [229-253] | 502 [474-553] | 503 [475-554] |
| 150 | B | 7 [6-8] | 101 [76-106] | 375 [308-391] | 376 [308-391] |

**warm-restarted, CPU ×1** (ms, median [min-max] of 5)

| RTT | variant | idx_end | fcp | engine | qqReady |
|---|---|---|---|---|---|
| 0 | main | 26 [24-29] | 111 [99-138] | 384 [375-441] | 385 [376-442] |
| 0 | A | 28 [23-31] | 100 [87-154] | 402 [325-464] | 403 [326-465] |
| 0 | B | 18 [15-25] | 90 [77-125] | 390 [322-452] | 391 [323-452] |
| 40 | main | 64 [63-67] | 152 [147-160] | 448 [417-537] | 448 [418-537] |
| 40 | A | 62 [61-70] | 144 [129-176] | 428 [344-510] | 428 [345-510] |
| 40 | B | 17 [15-33] | 99 [87-118] | 380 [365-423] | 380 [365-424] |
| 150 | main | 178 [176-183] | 248 [248-281] | 530 [513-567] | 531 [514-567] |
| 150 | A | 175 [173-178] | 254 [243-281] | 528 [497-621] | 528 [498-622] |
| 150 | B | 18 [17-23] | 98 [93-104] | 376 [357-388] | 377 [357-389] |

**warm, CPU ×4** (ms, median [min-max] of 5)

| RTT | variant | idx_end | fcp | engine | qqReady |
|---|---|---|---|---|---|
| 0 | main | 16 [15-18] | 224 [206-260] | 871 [846-1066] | 878 [847-1068] |
| 0 | A | 13 [13-17] | 205 [200-231] | 941 [829-1096] | 944 [830-1112] |
| 0 | B | 7 [6-10] | 184 [150-251] | 842 [791-1066] | 846 [792-1068] |
| 40 | main | 96 [95-98] | 283 [271-381] | 928 [874-1082] | 934 [882-1085] |
| 40 | A | 53 [51-54] | 228 [215-240] | 924 [904-987] | 926 [910-991] |
| 40 | B | 6 [6-8] | 204 [188-224] | 868 [810-1053] | 873 [814-1062] |
| 150 | main | 317 [314-320] | 488 [477-508] | 1125 [1102-1385] | 1131 [1104-1395] |
| 150 | A | 164 [161-165] | 342 [325-362] | 956 [936-1119] | 976 [944-1130] |
| 150 | B | 7 [5-9] | 190 [158-207] | 924 [849-1059] | 931 [851-1064] |

**warm-restarted, CPU ×4** (ms, median [min-max] of 5)

| RTT | variant | idx_end | fcp | engine | qqReady |
|---|---|---|---|---|---|
| 0 | main | 32 [27-34] | 217 [196-227] | 939 [838-1005] | 942 [842-1008] |
| 0 | A | 35 [27-44] | 245 [226-256] | 982 [917-1122] | 987 [920-1136] |
| 0 | B | 22 [20-23] | 198 [183-246] | 890 [763-1010] | 892 [768-1019] |
| 40 | main | 69 [66-78] | 247 [227-285] | 977 [819-1110] | 986 [822-1115] |
| 40 | A | 65 [62-72] | 232 [207-268] | 930 [858-944] | 932 [859-947] |
| 40 | B | 23 [17-27] | 230 [179-249] | 995 [936-1183] | 999 [943-1189] |
| 150 | main | 178 [175-180] | 354 [338-380] | 1078 [1004-1111] | 1084 [1010-1115] |
| 150 | A | 178 [175-180] | 365 [343-377] | 1034 [1023-1105] | 1035 [1026-1108] |
| 150 | B | 20 [20-24] | 210 [188-240] | 951 [862-1094] | 952 [864-1108] |

Verdict:
- **A** (navigation preload): with the worker running the page comes one RTT earlier (`idx_end` 316 → 162 ms at RTT 150), FCP −152 ms (×1) and −146 ms (×4), qqReady −170 and −155 ms; at RTT 40 FCP −52 and −55 ms. At RTT 0 the differences stay inside the min-max spread.
  With a restarted worker A changes nothing: Chromium already sends the page request at the start of the navigation while it starts the worker (`index_srv` 2-3 ms on main too). Adopted as the default mode.
- **B** (`?nav=cache`): the page in 6-23 ms at every RTT, FCP about 100 ms (×1) and 190-230 ms (×4) whatever the RTT. At RTT 150: qqReady ×1 376 ms against 503 (A) and 673 (main), warm-restarted 377 against 528 and 531; ×4 931 against 976 and 1131, warm-restarted 952 against 1035 and 1084.
  At RTT 0-40 the gain is mostly FCP. The cost: a deploy shows one load later (table below). Opt-in; the default is the owner's decision.

## Deploy: loads until a new version shows (`e2e_sw_versione`, asserted)

| mode | 1st load after the deploy | 2nd load |
|---|---|---|
| network first (default, main and A) | new version | new version |
| `?nav=cache` (B) | old version from the cache; once the newer worker has filled its cache in `install` (2.5 s + the changed files), the line «New version ready · Reload» | new version (Reload or any load: the old worker hands over to the waiting one) |
