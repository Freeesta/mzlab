# Potenza del browser: cosa usare, come rilevarlo, cosa lasciare
Studio ARCH WASM-POTENZA (sola lettura del prodotto: misure in una cartella temporanea). Parte da `docs/agenti/wasm-potenziale.md` (profili, SIMD128, binaryen, zlib-rs: non ripetuti) e guarda le API del browser. Prompt DEV in `docs/archivio/prompt-wasm/` (`a.md`…`f.md`), banco riproducibile in `docs/archivio/prompt-wasm/banco-browser.txt`.

## 1. Regola e risposta secca
**Regola di Federico (28/10)**: il sito è la punta di diamante e usa il meglio dei browser recenti (Chrome/Edge, Firefox, Safari aggiornati). Si controlla funzione per funzione con feature detection, mai lo user agent, in un punto unico (`capacita`, prompt WASM-A). La funzione che manca si disattiva da sola con l'avviso comune «non disponibile in questo browser: usa l'app» (link) e si riattiva da sola quando il browser la supporta; il resto del sito funziona sempre. Nessun motore di riserva nel sito: il ripiego è l'app. Parità sito/app: stessi byte.

**Risposta**:
- Già al massimo: SIMD128 (unico build), `compileStreaming` con un solo `WebAssembly.Module` passato ai worker, OPFS con `SyncAccessHandle`, risposte del motore con trasferibili, Web Locks, binaryen 132, zlib-rs.
- Leve vere, misurate: (1) `opt-level = 3` sui crate caldi: 10-30 % in Firefox e WebKit su indice, centroidi e composizioni (§3c); (2) blocco di lettura OPFS 1 → 4 MiB: in Chromium la lettura passa da ~490 a ~1230 MB/s (§3e); (3) isolamento COOP/COEP dal nostro `sw.js`: funziona nei tre motori con una ricarica di 0,2-0,4 s, e apre `SharedArrayBuffer`: 4 worker su un buffer condiviso 55-103 ms contro 122-842 ms con copie a messaggi (§3b), base per indicizzare un file grande in parallelo.
- Da lasciare: WebGPU per il calcolo (WGSL non ha `f64`: niente parità con l'app), `DecompressionStream` (17-58× più lento dell'inflate wasm sugli array), relaxed SIMD (risultati diversi fra CPU), thread Rust (`std` precompilata non sicura con più thread), memory64, WasmGC, exception handling, tail call, JSPI, Scheduler API.

## 2. Come si è misurato
- Macchina: container cloud 4 vCPU, 15 GB. Browser: Chromium 153.0.8010.12 (`/opt/pw-browsers`), Firefox 157.0 e WebKit 27.2 di Playwright 1.64 (installati in una cartella temporanea, con `playwright install-deps webkit`). WebKit su Linux è il motore di Safari, non Safari per macOS: dove differisce dal Safari stabile lo dice la colonna «stabile» (dati di `webassembly.org/features.json` già raccolti in `wasm-potenziale.md` §4).
- Server statico senza COOP/COEP (come GitHub Pages); mediana di 3 giri (5 per la compilazione); ogni uscita confrontata con una somma di controllo (inflate: stessi byte in tutti i casi). In WebKit OPFS vuole un contesto persistente (`PERSIST=1`).
- Comandi: spacchetta `banco-browser.txt`, `bash prepara.sh <repo>`, poi `python3 run.py chromium ""`, `python3 run.py chromium "coi=1&only=parallel"`, `PAGE=lock.html python3 run.py chromium` (e `firefox`, `webkit` con `PLAYWRIGHT_BROWSERS_PATH`). Il motore vero: `python3 tools/banco_wasm.py` (§3c).

## 3. Voci
Colonne: Cr = Chromium 153, Fx = Firefox 157, Wk = WebKit 27.2 (misurati qui). «Se manca» = cosa si disattiva con l'avviso comune.

| voce | oggi nel codice | Cr / Fx / Wk (stabile) | guadagno misurato | se manca | S/M/L | prio |
|---|---|---|---|---|---|---|
| a SIMD128 | `.cargo/config.toml` `+simd128`; `rust-supervisor.js:16` `simdOk` → `error.noSimd` | sì / sì / sì (Safari ≥ 16.4) | inflate 534 → 462 ms Cr, = Fx, 468 → 487 Wk; lettura/indice +5-15 % | tutto il motore (Dati, Flow, HR): resta così, passa da `capacita` | S (in A) | fatto |
| b thread + SAB | nessun COOP/COEP; pool a messaggi `idxpool.js` | isolamento via `sw.js`: sì / sì / sì; senza: no | 4 worker su SAB 103 / 55 / 66 ms contro copie 265 / 842 / 300 e 1 worker 869 / 462 / 548 | solo l'indice parallelo di un file (N = 1) | M (D) + L (E) | 1 |
| c flag di build | `release` `z` + LTO + `-Oz`; O3 solo `mzlab-picchi` | — | O3 sui crate caldi: indice HR −10 / −10 / 0 %, composizioni −7 / −23 / −25 %, centroidi −1 / −10 / −30 %; +12 % gzip | — | S (B) | 1 |
| d caricamento | `compileStreaming` + un `Module` per tutti i worker; `sw.js` rete-poi-cache; moduli a richiesta | sì / sì / sì | motore 4,3 MB (senza `wasm-opt`): compilazione 9 / 41 / 20 ms; da Cache API 13 / 95 / 42 ms | — | — | 3 |
| e OPFS | `SyncAccessHandle` nei worker, blocco 1 MiB (`mzlab-wasm/src/lib.rs:73`); cartella con `showDirectoryPicker` | sync sì / sì / sì; più lettori `read-only` sì / no / no; `showDirectoryPicker` sì / no / no | lettura Cr 1 MiB 461-521 MB/s, 4 MiB 1191-1273, 8 MiB 1432-1630; Fx 4,7 → 5,7 GB/s; Wk ~6-7 GB/s piatto | cartella puntata (già oggi), nient'altro | S (C) | 1 |
| f WebGPU | non usato (`mappa3d.js` usa WebGL2) | sì (SwiftShader) / no / no qui; Safari 26 e Chrome sì | istogramma 8 M interi: JS 15 ms, GPU 113 ms (SwiftShader); WGSL senza `f64` | — | — | lasciare |
| g stream e copie | inflate in wasm (zlib-rs); risposte del motore con `transfer` | `DecompressionStream` sì / sì / sì; `waitAsync`, `structuredClone` sì ovunque | 20 000 array zlib: DS 26 988 / 8 150 / 10 853 ms, wasm 462 / 533 / 487 ms; 64 MB in un flusso DS 530 / 548 / 924 ms; 64 MB a/r copia 217 / 155 / 132 ms, trasferito 0,4 / 0 / 1 ms | — | — | lasciare / fatto |
| h wasm recenti | nessuno oltre le predefinite | relaxed SIMD, memory64, GC, exnref, tail call, JSPI: sì nei tre qui; Safari stabile: memory64 e relaxed SIMD no | nessun guadagno per Rust (sotto) | — | — | lasciare |
| i altre API | Web Locks (`migra.js:33`, `qqq-db.js:198`, `archivio.js:32`); `requestIdleCallback` con ripiego | OffscreenCanvas 2D nel worker sì / sì / sì; `scheduler.postTask` sì / sì / no; `FileSystemObserver` sì / no / no | 20 ridisegni di 200 000 punti: thread principale bloccato 121 / 910 / 407 ms, nel worker pausa max 13 / 56 / 234 ms | il grafico spostato nel worker (F) | M (F) | 2 |

### 3a SIMD128
Le sonde `WebAssembly.validate` di SIMD128 danno vero nei tre. Sull'inflate (zlib-rs, O3) SIMD vale −13 % in Chromium, nulla in Firefox, +4 % in WebKit: il guadagno resta quello di `wasm-potenziale.md` (5-15 % su lettura e indice). Doppio build con scelta a runtime: **no** (regola: nessun secondo percorso; tutti i browser recenti lo hanno). `simdOk` diventa una capacità di WASM-A: se manca si spegne il motore, come oggi.

### 3b Thread reali e SharedArrayBuffer
- **Isolamento**: il trucco «service worker che aggiunge COOP/COEP» (`coi-sw.js` del banco) rende `crossOriginIsolated` vero in Chromium, Firefox e WebKit dopo UNA ricarica (424 / 318 / 214 ms). Va dentro il nostro `sw.js` (un solo service worker per scope). Rischi: `require-corp` blocca ogni risorsa d'altra origine senza CORP/CORS (oggi tutto è della stessa origine; da verificare `llm/trasporto.html` verso i fornitori), Safari non ha `credentialless`, Firefox in navigazione privata non ha service worker (resta non isolato: si spegne solo ciò che richiede l'isolamento).
- **Thread Rust**: in Rust 1.97 stabile `-C target-feature=+atomics,+bulk-memory` con `--shared-memory --import-memory` si collega (provato: memoria importata condivisa), ma la `std` precompilata di `wasm32-unknown-unknown` è senza `atomics`: allocatore e TLS senza lock, quindi più thread sulla stessa memoria la corrompono. Servono nightly e `-Zbuild-std` (contro la toolchain fissata): **no**.
- **Senza thread Rust**: più istanze wasm (una per worker, memoria propria) che si scambiano le fette su un `SharedArrayBuffer`. Misura su 128 MB di Float64: 1 worker 869 / 462 / 548 ms; 4 worker con copie a messaggi 265 / 842 / 300 ms (in Firefox le copie costano più del lavoro); 4 worker sul buffer condiviso 103 / 55 / 66 ms. Fra file il pool scala già (`wasm-potenziale.md` §3): il guadagno serve dentro UN file grande (WASM-E), con byte identici a `in_parallelo` dell'app (`mzidx_ordine.rs:161`).
- `Atomics.waitAsync` c'è nei tre: serve solo con il buffer condiviso (attese fra worker senza bloccare).

### 3c Flag di build
- Banco del motore vero (`tools/banco_wasm.py --varianti zsimd,mixsimd,o3simd --chromium --giri 3 --lavoro DIR`, con le varianti `zsimd` = profilo del sito di oggi e `mixsimd` = O3 su `mzlab-core`, `miniz_oxide`, `quick-xml`, `base64`, aggiunte al dizionario `VARIANTI` in memoria; Firefox e WebKit con `run.html` dello stesso banco servito dalla cartella `DIR`). Stesse somme di controllo nelle tre varianti e nei quattro motori.
- Variante «crate caldi» contro oggi (ms, Cr / Fx / Wk): indice HR 4254 → 3840 / 4962 → 4452 / 4174 → 4219; 200 composizioni 4144 → 3860 / 4071 → 3142 / 4173 → 3114; centroidi 2966 → 2922 / 4696 → 4223 / 4062 → 2823; lettura LR 18,8 → 12,2 / 19 → 27 / 19 → 13 (millisecondi piccoli, rumore). Node 22: indice HR ×1,42, centroidi ×3,3, composizioni ×2,2.
- O3 su tutto: lettura HR 820 → 731 / 1014 → 854 / 825 → 773, indice HR −8 / −18 / −10 %. Peso del banco: 498 470 B (231 807 gzip) → 586 140 (259 869, +12 % gzip) con i crate caldi, 596 741 con O3 su tutto.
- Lettura: V8 di Chromium 153 recupera quasi tutto il divario di `z` (guadagno 0-10 %); Firefox e WebKit guadagnano 10-30 % su indice, centroidi e composizioni. Per «massimo in ogni browser» la leva vale. I picchi (`mzlab-picchi`) sono già a O3 nel sito; `-C target-cpu` in wasm non aggiunge nulla oltre alle feature già scelte. Prompt WASM-B (sostituisce DEV-2 di `PROMPT_WASM_POTENZIALE`).

### 3d Compilazione e caricamento
`WebAssembly.compileStreaming` del motore senza `wasm-opt` (4 282 274 B) dalla rete locale senza cache: 29,5 / 56 / 105 ms; `WebAssembly.compile` da `ArrayBuffer` 8,8 / 41 / 20 ms; da Cache API 13 / 95 / 42 ms. Il costo è il download, già tenuto da `sw.js`; un `WebAssembly.Module` non si salva più in IndexedDB. `instantiateStreaming` non cambia nulla (compiliamo una volta e istanziamo in ogni worker). Nessuna leva.

### 3e OPFS e File System Access
- Velocità di `SyncAccessHandle.read` per blocco (file da 256 MB): Chromium 64 KiB 152-159 MB/s, 256 KiB 181-205, 1 MiB 461-521, 2 MiB 818-894, 4 MiB 1191-1273, 8 MiB 1432-1630, 16 MiB 1747-2085; Firefox 837 → 6564 MB/s; WebKit 6-7 GB/s a ogni blocco. In Chromium ogni chiamata passa dal processo del browser: il blocco da 1 MiB del motore costa ~170 ms per 87 MB, a 4 MiB ~70 ms (WASM-C). `File.arrayBuffer()` 858-937 / 253-262 / 793-862 MB/s.
- `mode: "read-only"` (più lettori dello stesso file): solo Chromium. Firefox e WebKit danno il primo handle e rifiutano il secondo (`NoModificationAllowedError`, `InvalidStateError`), qualunque `mode`. `lettura-sync.js:22` lo gestisce già (l'handle si chiude a fine indice); un disegno parallelo deve avere UN lettore per file.
- `showDirectoryPicker`/`showOpenFilePicker`: solo Chromium. Oggi `cartella-puntata.js:35` e `libreria.js:65` scelgono OPFS altrove: è un secondo percorso (§5).

### 3f WebGPU
Chromium headless con SwiftShader: adattatore sì, istogramma di 8 M interi giusto (stessi conteggi), 113 ms contro 15 ms in JS (il trasferimento domina). Firefox 157 e WebKit 27.2 su Linux: `navigator.gpu` assente qui (Firefox lo ha su Windows, Safari 26 su macOS). Il blocco vero è WGSL: niente `f64`, somme in `f32` con ordine non fissato → risultati diversi dall'app. Mappe, istogrammi di intensità e PCA del motore sono in `f64`: **lasciare**. Per il solo disegno basta WebGL2 (`mappa3d.js`).

### 3g Stream di compressione, copie, trasferibili
- `DecompressionStream("deflate")` su 20 000 array zlib da ~3 KB (come negli mzML): 26 988 / 8 150 / 10 853 ms contro l'inflate wasm 462 / 533 / 487 ms (stessi byte); su un solo flusso da 64 MB 530 / 548 / 924 ms, non meglio del wasm. **Lasciare**. Formati: `gzip`, `deflate`, `deflate-raw` ovunque, `brotli` solo Firefox.
- `postMessage` di 64 MB andata e ritorno: copia 217 / 155 / 132 ms, trasferito 0,4 / 0 / 1 ms. Le risposte del motore (`rust-worker.js:172,417,465`) e i pezzi dei convertitori già trasferiscono; restano copie piccole (`peaks-worker.js` con `Array.from`, `impronta-worker.js:24`): nessuna leva misurabile.

### 3h WebAssembly recente
Sonde nei tre motori di prova: SIMD128, relaxed SIMD, tail call, exnref, extended-const, GC, multi-memory, memory64, JSPI e JS String Builtins tutti presenti (WebKit 27.2 è più avanti del Safari stabile, che per `features.json` non ha memory64 né relaxed SIMD).
- Per noi: relaxed SIMD rompe il determinismo (FMA, min/max dipendono dalla CPU); memory64 non serve (budget sotto 4 GiB, `memoria.md`) e costa controlli in più; WasmGC, exnref e tail call non li genera Rust stabile; JSPI non serve (le letture sono già sincrone nei worker). **Lasciare**, con la sonda in `capacita` solo se un DEV futuro ne ha bisogno.

### 3i Altre API
- **OffscreenCanvas**: 2D nei worker nei tre; 20 ridisegni di una spezzata da 200 000 punti bloccano il thread principale 121 / 910 / 407 ms, nel worker la pagina resta libera (pausa massima 13 / 56 / 234 ms; in WebKit poco guadagno). Vale solo per i grafici che oggi bloccano: WASM-F misura prima.
- **Scheduler API** (`postTask`, `yield`): Chromium e Firefox sì, WebKit no; il guadagno è la priorità dei compiti dell'interfaccia, che oggi non ha un problema misurato: **lasciare**.
- **Web Locks**: già usato; `FileSystemObserver` solo Chromium: lasciare.

## 4. Cosa manca in un browser recente e proposta
| funzione | dove manca (misurato) | proposta |
|---|---|---|
| cartella del computer (`showDirectoryPicker`) | Firefox, WebKit | richiedere: la cartella puntata si disattiva con l'avviso; i file si aprono comunque |
| più lettori dello stesso file OPFS | Firefox, WebKit | rinunciare: un lettore per file in ogni disegno |
| `crossOriginIsolated` senza service worker (Firefox privato, service worker spenti) | ovunque in quei casi | richiedere solo per l'indice parallelo (N = 1, stessi byte) |
| WebGPU | Firefox e WebKit di prova su Linux | rinunciare (§3f) |
| WebGL2 nel worker | Firefox, WebKit | rinunciare (la 3D resta nel thread principale) |
| Scheduler API | WebKit | rinunciare |
| memory64, relaxed SIMD | Safari stabile | rinunciare (§3h) |
| `DecompressionStream("brotli")` | Chromium, WebKit | rinunciare (non usato) |

## 5. Ripieghi già nel codice (decide Federico)
Sono secondi percorsi per browser senza una funzione; con la regola nuova diventano «funzione disattivata + avviso» oppure restano:
- `lettura-sync.js:15-25` e `rust-worker.js:72-86`: senza `SyncAccessHandle` o con l'handle occupato si legge il `Blob` con `FileReaderSync`. Stessi byte, stesso motore, cambia solo da dove arrivano i byte: proposta **tenere** (serve anche quando un altro lettore tiene il file, il caso di Firefox e WebKit).
- `rust-supervisor.js:10`: senza `compileStreaming` (tipo MIME sbagliato) si compila l'`ArrayBuffer`: **tenere** (protegge da server, non da browser).
- `libreria-worker.js:254` `probeOpfs` e `cartella-puntata.js:35`, `libreria.js:65,139,140`: OPFS al posto della cartella del computer dove manca `showDirectoryPicker`: proposta **tenere OPFS come scelta normale** e disattivare solo «punta una cartella».
- `mappa3d.js:213` (`nowebgl2`): diventa la capacità `webgl2` di WASM-A.

## 6. Modulo `capacita` e avviso comune (WASM-A)
- `capacita.js` rileva una volta per pagina (sonde sincrone della pagina 0,7 / 1 / 1 ms) e riceve dal worker del motore, nel suo messaggio `ready`, ciò che vive solo nei worker (aprire un worker apposta costa 491 / 175 / 45 ms). Nulla si salva: al caricamento dopo un aggiornamento del browser la funzione torna da sola.
- `REQUISITI`: funzione → capacità (motore: `simd128`, `opfs`, `opfsSync`; cartella: `cartellaLocale`; 3D: `webgl2`; indice parallelo: `isolamento`; grafico nel worker: `offscreen2d`). Una funzione si spegne solo se manca una SUA capacità.
- `avviso-capacita.js`: comando con `aria-disabled`, testo «{funzione}: non disponibile in questo browser. Usa l'app» con il link all'app, una riga riassuntiva per sessione; interruttore di prova `?cap-off=…` per gli e2e.

## 7. Priorità, prompt e decisioni
| prio | prompt | voce | costo | guadagno |
|---|---|---|---|---|
| 1 | `a.md` WASM-A capacita + avviso | regola | M | prerequisito di D, E, F |
| 1 | `b.md` WASM-B O3 sui crate caldi | c | S | 10-30 % in Firefox e WebKit, 0-10 % in Chromium |
| 1 | `c.md` WASM-C blocco OPFS 4 MiB | e | S | lettura Chromium ×2,4 (−100 ms per 87 MB) |
| 1 | `d.md` WASM-D isolamento in `sw.js` | b | M | apre `SharedArrayBuffer` nei tre motori |
| 1 | `e.md` WASM-E indice parallelo su SAB | b | L | fino a ×4 sulla parte divisibile (misura prima) |
| 2 | `f.md` WASM-F grafici nel worker | i | M | thread principale libero (solo se > 50 ms oggi) |

Ordine: A e C subito (indipendenti); B dopo la decisione 1; D dopo A e la decisione 2; E dopo C e D; F dopo A.

**Decisioni per Federico**:
1. Tetti del `.wasm` per O3 sui crate caldi (§3c).
2. Isolamento con ricarica alla prima visita (0,2-0,4 s, una volta) e vincolo `require-corp` su ogni risorsa futura.
3. Indice parallelo: più istanze wasm su `SharedArrayBuffer` (toolchain stabile) al posto dei thread Rust (nightly).
4. I ripieghi del §5: tenere o trasformare in avviso.
5. Nuove dipendenze: nessuna proposta (tutto con API del browser e codice nostro).
