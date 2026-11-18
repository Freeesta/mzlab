# The first file of a batch: the copy gate, measurements

Measurements of the site's copy gate (`createCopyGate` in `rust-carica.js`, choice `copiesOf` in `lr.js`; behaviour in `data-loading.md`, "The first file first"): in a batch at most K copies into OPFS (`lr_put`) at once, in list order; K = 0 = all together, as without the gate. Default K = 2. Site only: the desktop app already loads one file at a time (chain of `upload0` in `shell.js`) and does not copy the files.

Tool: `tests_e2e/misura_caricamento.py --scenarios copie,raw-copie,copie-mem,cinquanta` (arms in `tests_e2e/scenari_caricamento.py`, `copie` = localStorage `mzlab.dev.copie`). Base `main` `7e81096`, site from `tools/prepara_sito.py`, Chrome for Testing 153 headless, one warm persistent profile per run (normal window, cross-origin isolation), cloud machine 4 vCPU Intel Xeon 2.30 GHz. N = 5 per cell, arms alternated run by run, median [min-max], ms from the input's `change`. CPU 1× and 4× slow down only the page's main thread (the workers run at full speed).

Files: synthetic mzML 50.7 MB (2400 scans) and synthetic Thermo `.raw` 50.6 MB as in `misure-caricamento.md` ("First file and many files"); the batch of 50 = mzML of ~10 MB (480 scans). A batch = the first file plus copies with other names. Provisional TIC = `#ticprov` drawn; first chart = a panel drawn; all indexed = nothing left reading or opening.

**Threshold** (decided before measuring): first chart with ten files ≥ 25 % earlier than K = 0 AND the whole batch within +5 %. K = 2 passes it at 1× (mzML −27 %, batch −13 %; `.raw` −49 %, −15 %) and for the 50 files (−53 %, −1 %; 4×: −41 %, −4 %); at 4× with ten mzML the first chart is only −10 % (K = 1: −23 %).

K = 1 gives the first chart sooner but stretches the batch beyond +5 % in two measures out of three at 1× (50 files +12 %, arm with memory +15 %): one copy at a time leaves the builders idle when the files are small. Identical indexes: SHA-256 of every `.mzidx` equal between K = 2 and K = 0 (`tests_e2e/e2e_copie_coda.py`).

Where the first file's time goes (mzML, 1×, a typical run): alone the builder reads in 213 ms and the chart arrives 309 ms after the index; with K = 1 the read takes 586 ms (CPU shared with the next copy and the other builders) and the chart arrives 535 ms after the index (the engine worker also serves the copies); with K = 0 the first file's copy finishes only with the others (1.8 s instead of 0.3 s).

**Ten mzML of 53 MB**

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | uno | 471 [468-554] | 1306 [1268-1483] | 1313 [1280-1494] | - | - |
| 1× | dieci-k0 | 2036 [1908-2124] | 3456 [3104-3683] | 6764 [6308-6966] | +0 % | +0 % |
| 1× | dieci-k1 | 629 [571-657] | 2372 [2233-2595] | 6440 [6157-6646] | -31 % | -5 % |
| 1× | dieci-k2 | 885 [750-1184] | 2538 [2376-2990] | 5871 [5169-5995] | -27 % | -13 % |
| 1× | dieci-k3 | 1061 [975-1425] | 2776 [2536-3182] | 5648 [5104-6373] | -20 % | -16 % |
| 4× | uno | 627 [594-752] | 2230 [2155-2444] | 2310 [2269-2570] | - | - |
| 4× | dieci-k0 | 2445 [2243-2708] | 4554 [4403-4919] | 8546 [7863-8892] | +0 % | +0 % |
| 4× | dieci-k1 | 941 [791-1119] | 3487 [3181-4365] | 7497 [7304-8981] | -23 % | -12 % |
| 4× | dieci-k2 | 1160 [930-1306] | 4097 [3494-4365] | 7616 [7143-8022] | -10 % | -11 % |
| 4× | dieci-k3 | 1421 [1238-1755] | 4323 [3992-4982] | 7996 [7526-9027] | -5 % | -6 % |

**Ten `.raw` of 51 MB**

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | raw-uno | 269 [226-310] | 992 [926-1051] | 1026 [950-1061] | - | - |
| 1× | raw-dieci-k0 | 581 [481-650] | 4390 [3848-4727] | 5613 [4802-5963] | +0 % | +0 % |
| 1× | raw-dieci-k1 | 245 [210-287] | 1593 [1528-1680] | 4895 [4415-5223] | -64 % | -13 % |
| 1× | raw-dieci-k2 | 349 [292-480] | 2239 [2090-2747] | 4763 [4562-5213] | -49 % | -15 % |
| 1× | raw-dieci-k3 | 422 [307-526] | 3115 [2813-3472] | 5057 [4788-5084] | -29 % | -10 % |
| 4× | raw-uno | 406 [386-429] | 1910 [1446-1942] | 1948 [1530-2046] | - | - |
| 4× | raw-dieci-k0 | 792 [675-1024] | 5603 [5238-5998] | 7900 [7291-8986] | +0 % | +0 % |
| 4× | raw-dieci-k1 | 440 [418-519] | 2980 [2774-3327] | 5975 [5739-6435] | -47 % | -24 % |
| 4× | raw-dieci-k2 | 566 [459-665] | 3479 [3371-3737] | 6679 [6505-7896] | -38 % | -15 % |
| 4× | raw-dieci-k3 | 597 [528-1040] | 4034 [3482-5637] | 6848 [6698-9131] | -28 % | -13 % |

**Fifty mzML of 10 MB** (up to 50 files the batch runs as a pipeline: no "Choose" question)

| CPU | arm | provisional TIC | first chart | all indexed | first chart vs K0 | all vs K0 |
|---|---|---|---|---|---|---|
| 1× | cinquanta-k0 | 2395 [2197-2752] | 3183 [2818-3658] | 12841 [11414-14527] | +0 % | +0 % |
| 1× | cinquanta-k1 | 588 [513-995] | 1310 [1224-1832] | 14362 [13178-15887] | -59 % | +12 % |
| 1× | cinquanta-k2 | 744 [646-810] | 1482 [1390-1629] | 12692 [12517-13147] | -53 % | -1 % |
| 4× | cinquanta-k0 | 3739 [3410-3762] | 5598 [5177-6009] | 25515 [23631-28049] | +0 % | +0 % |
| 4× | cinquanta-k1 | 1730 [1443-2288] | 3428 [3142-4313] | 23054 [20942-24078] | -39 % | -10 % |
| 4× | cinquanta-k2 | 1617 [1535-1994] | 3308 [3163-4479] | 24551 [23479-25358] | -41 % | -4 % |

**Memory, ten mzML** (`measureUserAgentSpecificMemory` every 500 ms, the page's agent group; RSS of all browser processes from `/proc`; no error in any run)

| CPU | arm | first chart | all indexed | all vs K0 | page memory peak MB | RSS peak MB |
|---|---|---|---|---|---|---|
| 1× | dieci+mem-k0 | 3475 [3437-4459] | 6191 [6049-7677] | +0 % | 65 [64-200] | 1910 [1795-2012] |
| 1× | dieci+mem-k1 | 2662 [2343-2772] | 7139 [6553-7820] | +15 % | 100 [64-115] | 1816 [1765-1917] |
| 1× | dieci+mem-k2 | 2614 [2264-2803] | 6277 [5858-6596] | +1 % | 66 [59-102] | 1929 [1778-1957] |
| 4× | dieci+mem-k0 | 5292 [4507-6339] | 9563 [8470-10602] | +0 % | 67 [66-161] | 1832 [1799-1884] |
| 4× | dieci+mem-k1 | 4002 [3671-4186] | 8494 [7781-9022] | -11 % | 104 [60-161] | 1832 [1763-1875] |
| 4× | dieci+mem-k2 | 3955 [3172-4642] | 8152 [6841-8811] | -15 % | 117 [59-161] | 1792 [1767-1819] |

## Critical path: verdicts (tables in `misure-caricamento.md`, "First file and many files")

- **(a) `rotte` on the path to the first chart: no.** Held 3 s the first chart does not move (1514 → 1611 ms at 1×, 2432 → 2307 at 4×, inside the min-max); the page asks for it about 1.4 s after the drop, when the chart is nearly drawn, and does not wait for it. Gain bound: 0 ms.
- **(b) `thermo` download serial before the `.raw` copy start: no, but on the path to the first chart: yes.** The first event of the file does not move (50 → 62 ms, 84 → 83) while the chart comes about 0.6 s (1×) / 1.1 s (4×) after the module arrives (held 3 s: 3608 and 4104 ms).
  Cold with the module filled by the SW fill the first chart is as the warm one (985 vs 1013 ms): up to ≈ 0.4 s (1×) / 0.7 s (4×) of download hides behind the copy and the scan table, beyond that every ms adds one. Gain bound: the whole download and compilation of the 768 KB module on a first visit with a slow link (not measurable on localhost).
- **(c) ten files delay the first one: yes.** First chart +2.2 s (+137 %) mzML and +3.1 s (+307 %) `.raw` at 1×, +2.6 s / +3.8 s at 4×; the provisional TIC +1.7 s mzML. Measured with every copy at once (`?copie=0`); the copy gate (two at once, this file above) takes back 0.5-2.2 s of it. Gain bound: 2.2-3.8 s per first chart (the whole difference to the file alone, unreachable in full).
- **(d) idle jobs slow a file dropped right after `qqReady`: only the SW fill on a first visit.** Cold +0 vs +20 s: first chart −471 ms at 1× (−22 %), −952 ms at 4× (−27 %), with 333-438 requests in the window; warm +0 vs +20 s: −17 / +135 ms (noise): the page's own idle jobs (mzFinder package, OpenChemLib, SW cache reads) are not on the path. The OPFS clean-up at 8 s is not covered (the chart is drawn before it). Gain bound: 0.5 s (1×) / 1.0 s (4×), first visit only.
- **(e) the first chart stays responsive while files 2..10 index: yes at 1×, same as alone at 4×.** Wheel p95 38 vs 28 ms and click → spectrum 64 vs 66 ms at 1×; at 4× 83 vs 94 ms and 209 vs 198 ms, i.e. the same as with nothing else loading; the cost of 4× is the page itself (rAF delta p95 ≈ 190 ms with or without gestures). Gain bound: ≈ 10 ms (1× wheel p95), nothing at 4×.
- **(f) peak memory of (c): page agent cluster 64-65 MB (floor), RSS of all processes 1.8-1.9 GB (ceiling), no memory error or pool lowering seen.** No cap was reached in these runs, so no S6/S8/S10 decision rests on memory at 10 × 53 MB.

Method notes: `tests_e2e/lib.py` replaces `wait_for_timeout` and `set_input_files` (quiet pauses, waits for rows and writes times): the scenario module puts the real ones back, an earlier pass without that fix is discarded. The tool measures page-side events; the server (a thread of the tool) logs every request that reaches the network, so SW-internal fills show.
