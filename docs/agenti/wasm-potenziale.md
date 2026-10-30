# Potenziale di WebAssembly: cosa costruiamo, cosa lasciamo sul tavolo
Studio ARCH-F (nessuna modifica al prodotto). Bersaglio deciso da Federico: browser desktop aggiornati (Chrome/Edge, Firefox, Safari desktop) e app desktop; niente telefoni né iPad per ora. Prompt DEV pronti in `docs/studi/PROMPT_WASM_POTENZIALE/`.

## 1. Risposta secca
Sì, ci conteniamo, ma **per il peso, non per la compatibilità**. Il freno è `opt-level = "z"` nel profilo `release` di tutti i `.wasm`: sul nostro codice vero (Chromium 153, 4 vCPU) `opt-level = 3` legge un mzML HR 1,5× più veloce, lo indicizza 1,4× più veloce e fa il peak picking 2,7× più veloce, con risultati identici bit per bit.
Il prezzo è +32-37 % sul motore (1,77 → 2,34-2,42 MB). La compatibilità costa poco: SIMD128 vale solo +5-15 % (lo supportano tutti i browser bersaglio), zlib-rs al posto di miniz vale 1,5-2× sull'inflate. I tempi di compilazione dei moduli sono trascurabili (10-19 ms per modulo, anche a O3). Thread wasm, memory64 e relaxed SIMD oggi non convengono (§5).

## 2. Come costruiamo oggi (audit)
- **Toolchain**: Rust 1.97.0 stabile (`rust-toolchain.toml`), solo `wasm32-unknown-unknown`, niente nightly né `-Zbuild-std`. `wasm-bindgen` 0.2.129 (`--target web`). `wasm-opt` = **binaryen version_132** dal tarball ufficiale GitHub con SHA-256 fissato (`tools/wasm_opt.sh`; ripiego npm `wasm-opt@1.4.0` = version_112 solo se il tarball non è utilizzabile).
- **Profilo** (`Cargo.toml`): `release` = `opt-level = "z"`, `lto = true` (fat), `codegen-units = 1`, `panic = "abort"`; niente `strip` (non serve: `wasm-opt` toglie la sezione `name`, 352 KB su 850 KB nel banco; DWARF assente). `desktop` e `cli` ereditano con `opt-level = 3`, ma non costruiscono `.wasm`.
- **wasm-opt**: `-Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-simd`; i lettori aggiungono `--converge --low-memory-unused --vacuum --dae …` (−2,5 %).
- **Target-feature attive** (`rustc --print cfg --target wasm32-unknown-unknown`, Rust 1.97): `bulk-memory`, `multivalue`, `mutable-globals`, `nontrapping-fptoint`, `reference-types`, `sign-ext` (predefinite del target, non scelte da noi). **Accesa da `.cargo/config.toml`**: `simd128` (`+simd128`, tutti i moduli). **Spente**: `extended-const`, `tail-call`, `exception-handling`, `relaxed-simd`, `atomics`. Nessun `-C target-cpu`. `.cargo/config.toml` dà memoria iniziale 32 MB, massima 4 GiB e `+simd128`.
- **Caricamento**: `rust-supervisor.js` `compileOnce` usa `WebAssembly.compileStreaming` e passa lo stesso `WebAssembly.Module` ai worker (motore, pool `idx-worker.js`); il service worker `sw.js` tiene i file in cache (cache prima, per versione del sito). Lettura dei file da OPFS con `SyncAccessHandle` in sola lettura (`lettura-sync.js`), una copia JS → memoria wasm per blocco.
- **Parallelismo**: niente `SharedArrayBuffer` (nessun COOP/COEP su GitHub Pages); pool di worker a messaggi (`idxpool`, `createFeatPool`, dimensione da `hardwareConcurrency`). L'app desktop usa il motore **nativo** (`rayon`, `opt-level = 3`): le leve di questo studio riguardano solo il sito.
- **Divisione e tetti** (`wasm-tetti.md`): un modulo per gruppo, caricato a richiesta; tetto = misura × 1,10, solo in calo. I tetti hanno imposto `z` ovunque: è lì che si perde velocità.

## 3. Misure
Banco: crate di prova fuori dal repository con le funzioni vere di `mzlab-core` (`mzml::read`, `upload::prepare` + `mzidx::build_mzml_to_vec`, `Routes::xic_raw`, `centroidi::centroids`, `peaks::detect_peaks`, `composition::compose`, `entropy::entropy_similarity`, `flate2` inflate). File: mzML HR profilo sintetico 87 MB (1200 MS1 × 400 picchi in profilo, 900 MS2, zlib) e `B_FullMass-t15` di `tools/dati_sintetici.py` (2,5 MB, LR). Mediana di 5 giri (Chromium) o 3 (Node 22). Uscite identiche bit per bit in tutte le varianti (somme di controllo a 17 cifre).

**Chromium 153, ms** (base = come oggi: `z` + `-Oz`):

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

| modulo | `z` (oggi) | O3 su tutto | O3 solo su crate caldi¹ |
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
- **binaryen 132 contro 112**: stesso peso (−0,1 %), stessa velocità, `wasm-opt` **8× più veloce** (1,5 contro 12,8 s per il banco).
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
| threads + atomics | 74 | 79 | 14.1 | il browser c'è; mancano COOP/COEP e Rust stabile (§5) |
| memory64 | 133 | 134 | solo Technology Preview | no |
| multi-memory | 120 | 125 | no | no |
| JS String Builtins | 130 | 134 | 26.2 | non usato da `wasm-bindgen` 0.2.129 |
| WasmGC | 119 | 120 | 18.2 | **non serve a Rust** (memoria lineare propria) |

**Minimo da dichiarare** con SIMD128: Chrome/Edge ≥ 91, Firefox ≥ 89, Safari ≥ 16.4 (già oggi Safari ≥ 15 per reference types). Ogni desktop aggiornato del 2026 lo supera di anni.

## 5. Leve valutate
- **(a) SIMD128**: compilazione `+simd128` sicura sul bersaglio; codice vettoriale esplicito (`core::arch::wasm32`) non giustificato: i cicli caldi misurati sono XML/base64/inflate (non vettoriali per natura) e AsLS/centroidi (divisioni e dipendenze in sequenza). `portable_simd` è ancora nightly. Determinismo: l'autovettorizzazione di LLVM non riassocia somme in virgola mobile senza fast-math: uscite identiche (misurato).
- **(b) O3 contro z**: la leva più grande (§3). Il prezzo è il peso e i tetti; non la compilazione.
- **(c) Thread**: in Rust stabile `wasm32-unknown-unknown` non ha `atomics` (std precompilata senza; servono nightly e `-Zbuild-std`, `wasm-bindgen-rayon`): **no**, rompe la CI a toolchain fissata. In più GitHub Pages non manda COOP/COEP: un service worker che li aggiunge (stile `coi-serviceworker`) richiede un ricaricamento alla prima visita e `require-corp` su ogni risorsa (Safari non ha `credentialless`). 
- **(c) Thread, guadagno**: atteso solo dentro UN file grande (fra file il pool già scala ×4). L'app desktop non ne ha bisogno: è nativa con `rayon`. Alternativa senza thread: dividere l'indice di un file grande per intervalli di scansioni fra i worker del pool (studio a parte se i file reali lo chiedono).
- **(d) memory64**: Safari non lo ha; i puntatori a 64 bit costano controlli dei limiti in più; il motore ha già budget e spill sotto 4 GiB (`memoria.md`). **No**. La crescita della memoria è già libera fino a 4 GiB.
- **(e) Altre proposte**: bulk memory già attiva; tail calls ed extended-const senza effetto su Rust; exception handling solo con unwind (nightly); reference types già attivi; multi-memory e JS String Builtins non usati dalla toolchain; relaxed SIMD vietato dal determinismo; WasmGC irrilevante.
- **(f) Toolchain**: Rust 1.97 è recente; `wasm32v1-none` è `no_std` (niente `std`): non fa per noi. `-Zbuild-std`: **no** (nightly). Il ritardo vero è binaryen 112 → 132/133.
- **(g) Caricamento**: già `compileStreaming` e un solo `Module` condiviso; compilazione 10-19 ms per modulo. Cache del codice compilato: V8 la fa da sé con risposte HTTP in cache; IndexedDB di `Module` non è più supportato dai browser. Nessuna leva.
- **(h) I/O**: OPFS `SyncAccessHandle` già usato; la copia JS → wasm costa ~10 GB/s contro ~80-140 MB/s della lettura mzML: < 1 %. `DecompressionStream` per gli array base64+zlib: è asincrono e per-array (migliaia di array da pochi KB): più lento dell'inflate in wasm. **No**.
- **(i) Pacchetti** (versioni del `Cargo.lock`): `flate2` 1.1.10 con `miniz_oxide` 0.8.9 → **zlib-rs 0.6.8** (già nel lock, ×1,35-2,3 inflate, +43 KB); `quick-xml` 0.42.0 e `base64` 0.23 sono le ultime; `serde_json` 1.0.151 (nessuna alternativa SIMD in wasm: `simd-json` vuole SIMD x86/NEON e non è più veloce in wasm); `ruzstd` 0.9.1 per Bruker; `rayon` solo nativo. Niente da aggiungere.

## 6. Il testo incollato, punto per punto
| affermazione | giudizio per noi |
|---|---|
| moduli da 5/15/30 MB, «secondi di CPU» per compilarli | **sbagliata per noi**: i nostri sono 0,16-1,8 MB e compilano in 10-19 ms (Liftoff); il costo vero è solo il download |
| cause: DWARF, monomorfizzazione, `fmt`/panic, dipendenze | giusta in generale; DWARF qui assente, `wasm-opt` toglie i nomi |
| rimedi: `opt-level = "z"`, lto, cgu=1, strip, panic=abort | **già fatto tutto**; ed è proprio `z` che ci rallenta (×1,4-2,8) |
| `wasm-opt` toglie un altro 20-40 % | giusta: 850 → 451 KB nel banco (−47 %, ma 352 KB sono la sezione `name`) |
| «Wasm 3» non tutti i browser lo leggono | **imprecisa**: Wasm 3.0 è un'edizione della specifica (settembre 2025) che raccoglie proposte già spedite una per una; non si «abilita» a blocchi, si abilitano feature singole |
| opcode sconosciuto → `CompileError`, niente degradazione | giusta |
| MVP, bulk memory, SIMD128 ovunque | giusta per i desktop (Safari ≥ 16.4 per SIMD) |
| WasmGC e tail calls nei tre browser | giusta (Safari 18.2); irrilevante: Rust non usa WasmGC |
| memory64: Chrome/Firefox sì, Safari no | giusta (Safari solo Technology Preview) |
| relaxed SIMD limitato su Safari | giusta (solo flag); per noi vietato comunque dal determinismo |
| thread richiedono COOP/COEP | giusta; manca la parte che conta per noi: Rust stabile non li costruisce |
| «per il calcolo numerico basta SIMD128» | **imprecisa**: SIMD128 qui vale 5-15 %; `opt-level` vale ×1,4-2,8 |
| wasm-feature-detect + due build | non serve: tutti i bersagli hanno SIMD128; un solo build «veloce» |
| shell desktop: compatibilità non è un problema | vera ma irrilevante: l'app usa il motore nativo, non il `.wasm` |

## 7. Strategia raccomandata
**Un solo build, quello veloce**: SIMD128 attivo, `opt-level = 3` dove il tempo si spende, `-Oz` di binaryen recente per il peso, nessun ripiego (coerente con «un solo motore, nessun ripiego»). Due build (baseline + veloce) costerebbero un secondo giro di `sito-wasm-*` (~600 s di CPU in CI per PR), il doppio dei byte in `site/` e il doppio delle prove e2e, per servire browser fuori dal bersaglio. Un browser senza SIMD128 riceve l'errore di compilazione e la pagina dice di aggiornarlo (chiave i18n nuova).

**Ordine per guadagno/costo**:
1. **binaryen 132** in `tools/wasm_opt.sh` (DEV-1): stesso peso, `wasm-opt` 8× più veloce (≈ 11 s risparmiati per modulo, ~20 moduli, in parallelo con cargo). Rischio basso.
2. **`+simd128`** (DEV-3): +5-15 % su lettura e indice, peso −1,5 %/+0,6 %, browser minimi Chrome 91 / Firefox 89 / Safari 16.4.
3. **O3 sui crate caldi** (DEV-2): lettura ×1,5, indice ×1,4, picchi ×2,7 (Chromium; ×1,6-2,9 in Node); peso motore +32 % (+570 KB, +205 KB gzip), HR +3 %, Attributio +1 %. Richiede di alzare i tetti.
4. **zlib-rs** (DEV-4): inflate ×1,35-2,3, indice HR −9 %, +43 KB per modulo che legge mzML; cambia i byte del gzip/zlib che scriviamo (convertitore) se usato anche per comprimere.
5. **Banco permanente** (DEV-5): il banco di questo studio nel repository, così ogni leva si misura allo stesso modo e i numeri finiscono in `misure-velocita.md`. Va fatto per primo: le altre PR lo usano.
- **Non fare ora**: thread wasm, memory64, relaxed SIMD, exception handling, doppio build, `DecompressionStream`, SIMD esplicito.

## 8. Decisioni di Federico
- **Tetti**: motore da 1 884 160 a ≈ 2 572 288 B (2 336 860 × 1,10 arrotondato a 4 KiB) se si accetta O3 su `mzlab-core`; HR e Attributio +3 % e +1 % (restano sotto i tetti: HR 1 324 332 contro `limit_hr` 1 339 392, di poco; Attributio 1 403 170 contro 1 536 000).
- **Lettori**: peso a O3 da misurare in DEV-2 (`opentfraw` e `mzlab-lettori` fra i crate caldi).
- **Browser minimi dichiarati**: Chrome/Edge 91, Firefox 89, Safari 16.4 desktop (con SIMD128).
- **zlib-rs**: è una feature di un crate già presente (non una dipendenza nuova), ma cambia i byte compressi del convertitore: o solo decompressione (zlib-rs diretto per l'inflate, miniz per scrivere) o `conv/1` nuova con prima/dopo (già previsto in `docs/studi/raw-mzml-immediato/DUBBI.md` punto 5).
- **binaryen da GitHub** al posto di npm (`wasm-opt@1.4.0` è fermo dal 2023; `wasm-opt@2.0.1` è del 17 ott 2026, troppo nuovo e con pacchetti per piattaforma): tarball `version_132` con SHA-256 fissato.

## 9. Come rifare le misure
Il banco (DEV-5) costruisce lo stesso crate nelle varianti `base` (`z`, `-Oz`), `o3`, `o3simd`, `o3simd_zrs`, `mix` e lo esegue in Node (`initSync`) e in Chromium (Playwright, `compileStreaming`) con il mzML HR sintetico da 87 MB generato da `mzmlwrite` (seme fisso) e `B_FullMass-t15` di `dati_sintetici.py`. Ogni variante deve dare le stesse somme di controllo; i tempi si confrontano solo sulla stessa macchina, mediana di 5 giri.
Nel cloud il banco non gira (`tools/wasm_opt.sh` solo in CI): il workflow `banco-wasm.yml` (solo `workflow_dispatch`, ingressi `varianti` e `giri`) lo lancia in CI su un ramo e scrive la tabella dei tempi nel riassunto del run; è un solo ramo per volta (per confrontare due rami si lancia due volte).
