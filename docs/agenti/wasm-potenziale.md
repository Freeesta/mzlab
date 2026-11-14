# Potenziale di WebAssembly: cosa costruiamo, cosa lasciamo sul tavolo
Bersaglio: browser desktop aggiornati (Chrome/Edge, Firefox, Safari desktop) e app desktop; niente telefoni né iPad.

## 1. Sintesi
Il freno del sito è il peso, non la compatibilità. `opt-level = "z"` resta il profilo `release` dei `.wasm`, con `opt-level = 3` sui crate caldi (§2): sul codice vero (Chromium 153, 4 vCPU) O3 legge un mzML HR 1,5× più veloce, lo indicizza 1,4× più veloce e fa il peak picking 2,7× più veloce, con risultati identici bit per bit; O3 su tutto costerebbe +32-37 % sul motore.
SIMD128 vale +5-15 % (lo supportano tutti i browser bersaglio), zlib-rs al posto di miniz 1,5-2× sull'inflate; entrambi in uso. I tempi di compilazione dei moduli sono trascurabili (10-19 ms per modulo, anche a O3). Thread wasm, memory64 e relaxed SIMD non convengono (§7).

## 2. Come costruiamo (audit)
- **Toolchain**: Rust 1.97.0 stabile (`rust-toolchain.toml`), solo `wasm32-unknown-unknown`, niente nightly né `-Zbuild-std`. `wasm-bindgen` 0.2.129 (`--target web`). `wasm-opt` = **binaryen version_133** dal tarball ufficiale GitHub con SHA-256 fissato (`tools/wasm_opt.sh`; ripiego npm `wasm-opt@1.4.0` = version_112 solo se il tarball non è utilizzabile).
- **Profilo** (`Cargo.toml`): `release` = `opt-level = "z"`, `lto = true` (fat), `codegen-units = 1`, `panic = "abort"`; `opt-level = 3` per pacchetto su `mzlab-core`, `mzlab-picchi`, `quick-xml`, `base64`, `zlib-rs`, `miniz_oxide`, `sha1`, `sha2`, `opentfraw` (ogni voce con la sua misura nel commento). Niente `strip` (non serve: `wasm-opt` toglie la sezione `name`, 352 KB su 850 KB nel banco; DWARF assente). `desktop` e `cli` ereditano con `opt-level = 3`, ma non costruiscono `.wasm`.
- **wasm-opt**: `-Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-simd`; i lettori aggiungono `--converge --low-memory-unused --vacuum --dae …` (−2,5 %).
- **Target-feature attive** (`rustc --print cfg --target wasm32-unknown-unknown`, Rust 1.97): `bulk-memory`, `multivalue`, `mutable-globals`, `nontrapping-fptoint`, `reference-types`, `sign-ext` (predefinite del target, non scelte da noi). **Accesa da `.cargo/config.toml`**: `simd128` (`+simd128`, tutti i moduli). **Spente**: `extended-const`, `tail-call`, `exception-handling`, `relaxed-simd`, `atomics`. Nessun `-C target-cpu`. `.cargo/config.toml` dà memoria iniziale 32 MB, massima 4 GiB e `+simd128`.
- **zlib-rs**: backend di `flate2` in `mzlab-core`, `mzlab-lettori`, `mzlab-cli`, `mzlab-desktop` (lettura e scrittura; `conv/1` dichiara `compressore`).
- **Caricamento**: `rust-supervisor.js` `compileOnce` usa `WebAssembly.compileStreaming` e passa lo stesso `WebAssembly.Module` ai worker (motore, pool `idx-worker.js`); il service worker `sw.js` tiene i file in cache (cache prima, per versione del sito). Lettura dei file da OPFS con `SyncAccessHandle` in sola lettura (`lettura-sync.js`), una copia JS → memoria wasm per blocco.
- **Parallelismo**: niente `SharedArrayBuffer` (nessun COOP/COEP su GitHub Pages); pool di worker a messaggi (`idxpool`, `createFeatPool`, dimensione da `hardwareConcurrency`). L'app desktop usa il motore **nativo** (`rayon`, `opt-level = 3`): le leve di questo file riguardano solo il sito.
- **Divisione e tetti** (`wasm-tetti.md`): un modulo per gruppo, caricato a richiesta; tetto = misura × 1,10, solo in calo. I tetti tengono `z` come base: è lì che si perde velocità.

## 3. Misure
Banco: crate di prova fuori dal repository con le funzioni vere di `mzlab-core` (`mzml::read`, `upload::prepare` + `mzidx::build_mzml_to_vec`, `Routes::xic_raw`, `centroids::centroids`, `peaks::detect_peaks`, `composition::compose`, `entropy::entropy_similarity`, `flate2` inflate). File: mzML HR profilo sintetico 87 MB (1200 MS1 × 400 picchi in profilo, 900 MS2, zlib) e `B_FullMass-t15` di `tools/dati_sintetici.py` (2,5 MB, LR). Mediana di 5 giri (Chromium) o 3 (Node 22). Uscite identiche bit per bit in tutte le varianti (somme di controllo a 17 cifre).

**Chromium 153, ms** (base = `z` + `-Oz` su tutto):

| lavoro | base | O3 | O3+SIMD | O3+SIMD+zlib-rs | guadagno |
|---|---|---|---|---|---|
| lettura mzML HR 87 MB | 1146 | 778 | 671 | 618 | ×1,86 |
| indice `.mzidx` HR | 4542 | 3278 | 3002 | 2744 | ×1,66 |
| lettura mzML LR | 19,8 | 13,2 | 11,1 | 10,3 | ×1,9 |
| indice LR | 54,8 | 41,8 | 38,5 | 38,2 | ×1,43 |
| 50 XIC + picchi (AsLS) LR | 223 | 83 | 81 | 80 | ×2,8 |
| centroidi 100 scansioni HR | 2737 | 2499 | 2527 | 2527 | ×1,08 |
| 200 composizioni elementari | 3724 | 3419 | 3424 | 3315 | ×1,12 |
| inflate 64 MB | 621 | 416 | 396 | 307 | ×2,0 |
| 200 XIC dall'indice HR | 51 | 51 | 49 | 51 | = |
| entropia 300×300 MS2 | 1159 | 1186 | 1129 | 1277 | = |

**Node 22 (V8 più vecchio), ms**: base → O3: lettura HR 1599 → 979, indice HR 6209 → 3623, centroidi 6823 → 2356 (×2,9), picchi LR 301 → 89, composizioni 6792 → 3539, inflate 822 → 426; zlib-rs da solo su `z`: inflate 822 → 353, indice HR 6209 → 4446. Il V8 di Chrome 153 recupera parte del divario di `z` (inlining wasm), un motore meno aggressivo (Firefox, Safari: non misurati, non installati qui) sta probabilmente fra i due.

**Peso** (byte dopo `wasm-opt -Oz`, gzip -9 fra parentesi):

| modulo | `z` | O3 su tutto | O3 solo su crate caldi¹ |
|---|---|---|---|
| motore `mzlab-wasm` | 1 765 995 (766 236) | 2 419 539 (+37 %) | 2 336 860 (+32 %) |
| HR `mzlab-feat-wasm` | 1 287 826 (564 255) | 1 557 979 (+21 %) | 1 324 332 (+2,8 %) |
| Attributio `all` | 1 392 870 (599 771) | 1 928 195 (+38 %) | 1 403 170 (+0,7 %) |
| banco (solo codice caldo) | 451 523 | 534 719 (+18 %) | 534 330 |

¹ `[profile.release.package.<crate>] opt-level = 3` per `mzlab-core`, `miniz_oxide`, `quick-xml`, `base64`: nel banco dà la stessa velocità di O3 su tutto; nel motore quasi tutto il codice è `mzlab-core`, quindi pesa come O3.
- **Livello di `wasm-opt`**: non cambia la velocità (O3 + `-Oz` = O3 + `-O3` entro il rumore, 535 contro 537 KB): conta solo `opt-level` di rustc. Tenere `-Oz`.
- **O2 / Os**: O2 come O3 (centroidi più lenti, 547 KB); Os a metà (499 KB, lettura HR 1168 ms in Node). Non convengono.
- **SIMD128 (autovettorizzazione)**: −1,5 % di peso con `z`, +0,6 % con O3; +5-15 % su lettura e indice, nullo sul resto. Coerente con `ottimizzazioni-tentate.md` (22/10, 20 Full Scan LR: 9,1 contro 9,3 s): il guadagno è piccolo ma gratuito.
- **zlib-rs** (feature `zlib-rs` di `flate2`, già nel `Cargo.lock` 0.6.8): +42-43 KB (+8 %) sul banco; inflate ×1,35-2,3, indice HR −9 % (Chromium).
- **Crate SHA** (`sha1`, `sha2` a O3 nel profilo `release`; impronte `upload::Passata` di un file mentre si copia in OPFS): SHA-256 + SHA-1 su 64 MB ×1,68 in Node (774 → 461 ms) e ×1,34 in Chromium 153 (654 → 489 ms), mediana di 9, `tools/banco_wasm.py` variante `sol_hash`, check value identico; ±1 KB per modulo. Con 300 MB in Chromium l'hash (2030 ms) resta più lento della copia in OPFS (1511 ms): copia + hash 2156 ms, quindi la copia aspetta l'hash.
- **binaryen 132/133 contro 112**: stesso peso (−0,1 %), stessa velocità, `wasm-opt` **8× più veloce** (1,5 contro 12,8 s per il banco).
- **Compilazione e istanza** (Chromium 153, `compileStreaming`, mediana di 5, a freddo, senza cache HTTP): motore 14-19 ms, HR 11-14 ms, Attributio 11-14 ms a `z` e a O3; `new WebAssembly.Module` sincrono 4-8 ms; istanza < 1 ms. V8 compila con Liftoff e ottimizza in background: il «costo di compilazione del modulo pesante» non esiste alle nostre taglie; conta solo il download (+205-225 KB gzip per il motore a O3, una volta: poi `sw.js`).
- **Pool di worker** (Chromium, 4 vCPU, modulo O3, lavori indipendenti): 1, 2, 4 worker in 444, 449, 456 ms (scala ×4), 8 worker 941 ms. Il parallelismo fra file c'è già e scala lineare fino ai core.

## 4. Supporto dei browser (webassembly.org/features.json, ottobre 2026)
| feature | Chrome | Firefox | Safari | per noi |
|---|---|---|---|---|
| bulk memory, sign-ext, mutable globals, nontrapping, multivalue, reference types | ≤ 96 | ≤ 79 | 15 | già attive (predefinite del target) |
| SIMD128 | 91 | 89 | 16.4 | sì |
| extended-const | 114 | 112 | 17.4 | sì, gratuito, guadagno nullo |
| tail calls | 112 | 121 | 18.2 | inutile (Rust non le genera di suo) |
| exception handling (exnref) | 137 | 131 | 18.4 | no: serve `panic = "unwind"` → `-Zbuild-std` |
| relaxed SIMD | 114 | 145 | solo flag | **no**: risultati diversi fra CPU (FMA, min/max, dot) → rompe il determinismo |
| threads + atomics | 74 | 79 | 14.1 | il browser c'è; mancano COOP/COEP e Rust stabile (§7) |
| memory64 | 133 | 134 | solo Technology Preview | no |
| multi-memory | 120 | 125 | no | no |
| JS String Builtins | 130 | 134 | 26.2 | non usato da `wasm-bindgen` 0.2.129 |
| WasmGC | 119 | 120 | 18.2 | **non serve a Rust** (memoria lineare propria) |

**Minimo di SIMD128 da solo**: Chrome/Edge ≥ 91, Firefox ≥ 89, Safari ≥ 16.4; i browser minimi dichiarati sono nel §8.

## 5. Leve in uso
- **SIMD128**: compilazione `+simd128` su tutti i moduli, un solo binario; l'autovettorizzazione di LLVM non riassocia somme in virgola mobile senza fast-math: uscite identiche (misurato). Un browser senza SIMD128 mostra `error.noSimd` (`capacita.js`).
- **O3 sui crate caldi** (§2): la leva più grande (§3); il prezzo è il peso e i tetti, non la compilazione.
- **zlib-rs** (`flate2` 1.1.10, `zlib-rs` 0.6.8): inflate ×1,35-2,3, indice HR −9 %, +43 KB per modulo che legge mzML.
- **binaryen 133**: stesso peso di 112, `wasm-opt` 8× più veloce.
- **Pacchetti**: `quick-xml` 0.42.0 e `base64` 0.23 sono le ultime; `serde_json` 1.0.151; `ruzstd` 0.9.1 per Bruker; `rayon` solo nativo.

## 6. Leve aperte
- **LTO reale nel sito** (§10): con `crate-type = ["cdylib", "rlib"]` cargo scarta `lto = true`; `cargo rustc --crate-type cdylib` la ripristina. Misurato nel banco (`lto1`, `lto16`), non adottato: lettura mzML HR −12 % con `lto16` in Chromium, ma «200 XIC dall'indice HR» +9-27 %.
- **O3 sui lettori**: tra i lettori solo `opentfraw` è a 3; peso e velocità di `mzlab-lettori` a O3 da misurare (`wasm-tetti-misure.md`).
- **Un file grande fra più worker**: fra file il pool scala già ×4; dentro UN file grande, senza thread, si potrebbe dividere l'indice per intervalli di scansioni fra i worker del pool (studio a parte se i file reali lo chiedono).

## 7. Strade scartate
Thread wasm, memory64, relaxed SIMD, exception handling, SIMD esplicito, `DecompressionStream`, doppio build, O2/Os, livelli di `wasm-opt` diversi da `-Oz`, `wasm32v1-none`, `-Zbuild-std`, cache dei `Module` in IndexedDB, `simd-json`: una riga ciascuna, con i numeri, in `ottimizzazioni-tentate.md`, sezione «WebAssembly: leve scartate».

## 8. Decisioni in vigore
- **Un solo build, quello veloce**: SIMD128 attivo, `opt-level = 3` dove il tempo si spende, `-Oz` di binaryen recente per il peso, nessun ripiego. Due build costerebbero un secondo giro di `sito-wasm-*` (~600 s di CPU in CI per PR), il doppio dei byte in `site/` e il doppio delle prove e2e.
- **Browser minimi dichiarati** (il solo posto che li fissa): Chrome/Edge 111, Firefox 121, Safari 16.4. Il motore richiede SIMD128, OPFS e gli handle sincroni (`capacita.js`), `DecompressionStream` chiede Firefox 113, `:has()` Chrome 105 e Firefox 121. Sotto il pavimento il codice non ha ripieghi per `ResizeObserver`, `IntersectionObserver`, `MutationObserver`.
- **Tetti**: in `wasm-tetti.md` (misura × 1,10 arrotondata a 4 KiB); alzarli per O3 richiede la misura nella stessa PR.
- **binaryen da GitHub** al posto di npm (`wasm-opt@1.4.0` è fermo dal 2023; `wasm-opt@2.0.1` ha pacchetti per piattaforma): tarball `version_133` con SHA-256 fissato.

## 9. Come rifare le misure
Il banco (`tools/banco_wasm.py`, crate `tools/banco_wasm/`) costruisce lo stesso crate nelle varianti `base` (`z`, `-Oz`), `o3`, `o3simd`, `o3simd_zrs`, `mix` e lo esegue in Node (`initSync`) e in Chromium (Playwright, `compileStreaming`) con il mzML HR sintetico da 87 MB generato da `mzmlwrite` (seme fisso) e `B_FullMass-t15` di `dati_sintetici.py`. Ogni variante deve dare le stesse somme di controllo; i tempi si confrontano solo sulla stessa macchina, mediana di 5 giri.
Nel cloud il banco non gira (`tools/wasm_opt.sh` solo in CI): il workflow `banco-wasm.yml` (solo `workflow_dispatch`, ingressi `varianti` e `giri`) lo lancia in CI su un ramo e scrive la tabella dei tempi nel riassunto del run; è un solo ramo per volta (per confrontare due rami si lancia due volte).

## 10. Collegamento con LTO reale
Con `crate-type = ["cdylib", "rlib"]` cargo scarta `lto = true`: i moduli del sito sono collegati senza LTO. `cargo rustc --crate-type cdylib` costruisce solo la cdylib e l'LTO gira. Il banco ha le varianti `lto1` e `lto16` (stesso profilo e stesso `-O3` di `site_z`, 1 o 16 unità di codegen), da confrontare con `site_z`.
Misure (4 vCPU, Chromium 153 e Node 22, mediana di 5 giri, stesse somme di controllo in tutte le varianti; ms, base = `site_z`). Peso del banco dopo `wasm-opt -O3`: 656 055 B → 600 914 (`lto1`) → 579 129 (`lto16`).

| lavoro | Chromium base | `lto1` | `lto16` | Node base | `lto1` | `lto16` |
|---|---|---|---|---|---|---|
| lettura mzML HR | 742,7 | 757,9 | 651,2 | 837,7 | 840,5 | 816,8 |
| indice HR | 3376,3 | 3428,9 | 3227,6 | 3670,3 | 3658,6 | 3522,8 |
| 200 XIC dall'indice HR | 22,9 | 27,3 | 25,0 | 23,4 | 29,8 | 26,1 |
| centroidi 100 scansioni HR | 1076,3 | 1048,3 | 1068,5 | 1069,3 | 1034,6 | 1080,8 |
| 50 XIC + picchi HR | 24,2 | 21,7 | 21,8 | 20,7 | 21,5 | 21,4 |
| entropia 300×300 MS2 | 611,8 | 636,7 | 612,7 | 659,6 | 612,7 | 619,2 |
| lettura mzML LR | 19,5 | 18,3 | 15,6 | 20,7 | 18,4 | 19,8 |
| indice LR | 51,0 | 42,8 | 45,2 | 47,7 | 44,0 | 47,4 |
| 200 XIC dall'indice LR | 2,8 | 2,5 | 2,7 | 2,7 | 2,7 | 2,9 |
| 50 XIC + picchi (AsLS) LR | 40,9 | 37,8 | 58,0 | 48,8 | 46,4 | 44,5 |
| 200 composizioni | 2683,8 | 2707,4 | 2753,9 | 2682,5 | 2717,7 | 2692,3 |
| inflate 64 MB | 333,1 | 330,7 | 305,5 | 327,0 | 314,1 | 336,6 |
| SHA-256 + SHA-1 64 MB | 492,3 | 517,7 | 563,7 | 555,5 | 505,9 | 557,2 |

"200 XIC dall'indice HR" è più lento di `base` in entrambe le varianti e in entrambi i motori (+9-27 %); i lavori sotto i 30 ms oscillano di qualche ms fra i giri.

