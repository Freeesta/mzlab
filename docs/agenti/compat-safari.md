# Safari (macOS, iPadOS) e WebKit: API usate, versione minima, ripiego

- Pavimento del sito: **Safari / iPadOS 16.4** (SIMD128 di ogni `.wasm`, `opfsSync` nei worker, lookbehind delle regex di `calcola.js`). Sotto: il motore è spento con l'avviso comune di `capacita.js` (`motore`) o con `load.err.oldBrowser`; niente secondo `.wasm` senza SIMD (raddoppierebbe il peso e la parità), l'app desktop resta la via. Il motore non usa thread wasm, eccezioni wasm, tail call né memory64.
- `compat.js` (modulo ES, primo import di `capacita.js`): ripieghi per API mancanti, solo feature detection. `installa(g)` restituisce i nomi installati (`window.__compat`). Test Node: `tests/compat_node.mjs` (`tests/test_compat.py`); nel browser con iPad emulato: `e2e_safari_ipad`.
- WebKit vero: nel cloud non c'è (Playwright ha solo Chromium); di notte `notte.yml` gira `--fumo` con `--browser webkit` su macOS, `e2e_safari_ipad` usa WebKit quando c'è e altrimenti Chromium con iPad emulato (lo dice nell'uscita). `webkit-mac.yml` (a mano) diagnostica l'avvio del motore.

| API / funzione | Dove | Safari min | Ripiego | Test |
|---|---|---|---|---|
| WebAssembly SIMD128 | ogni `.wasm` (`.cargo/config.toml`) | 16.4 | motore spento, avviso `motore` | `e2e_capacita` (`cap-off=simd128`) |
| bulk-memory, sign-ext, reference-types, multivalue | `.wasm` (predefiniti di rustc) | 15 | sotto il pavimento | — |
| OPFS `getDirectory` | `archivio.js`, `lettura-sync.js`, molti | 15.2 | `capacita` `opfs`; finestra privata: `lrInit` (`browser.js`) prova `getDirectory` prima del motore e, se rifiutato, il banner di `AVVIO` dice `load.err.noStorage` (finestra normale o app) | `e2e_capacita`, `e2e_safari_ipad` |
| `createSyncAccessHandle` (worker) | `rust-worker.js`, `libreria-cartella.js`, `compat-scrivi-worker.js` | 15.2 (metodi con promesse prima di 16.4: attesi con `await` in `compat-scrivi-worker.js`) | `capacita` `opfsSync`; `libreria-worker.js` → IndexedDB | `e2e_capacita` |
| `FileSystemFileHandle.createWritable` | `infusione-grafo-esporta.js`, `flow-web-msn.js`, `52_attributio_banco.js`, `libreria-cartella.js` | 26 | `compat.js`: pezzi in memoria, scritti a `close()` dal worker con handle sincrono (solo file OPFS; niente `seek`, `position`, `keepExistingData`) | `test_compat`, `e2e_safari_ipad` |
| `FileSystemHandle.move(dir, nome)` | `archivio.js` | 15.2 (`move(nome)` rifiutato) | copia | `e2e_opfs_promote` |
| `SharedArrayBuffer`, `crossOriginIsolated` | `idxpool.js`, `idx-aiuti.js` | 15.2 (COOP/COEP da `sw.js`) | indicizzazione con un solo costruttore, stessi byte | `e2e_isolamento` |
| `AbortSignal.any` | `pannelli.js` (riepilogo correzioni) | 17.4 | `compat.js` | `test_compat`, `e2e_safari_ipad` |
| `Array.prototype.at`, `findLast`, `Object.hasOwn` | `avvio.js`, `flow-run-end.js`, `libreria-worker.js` | 15.4 | sotto il pavimento | — |
| `requestIdleCallback` | `first-paint.js`, `mappa.js`, `shell.js`, `rust-budget.js`, `tpmine-loader.js`, `flow-tool.js`, `reatt-auto.js` | assente | ogni uso cade su `setTimeout` | — |
| `navigator.clipboard.writeText` | ~30 pulsanti «Copia» | 13.1; rifiutato dopo un `await` (gesto consumato) | `compat.js`: `execCommand("copy")` con una textarea, poi il fuoco torna | `test_compat` |
| `<input type=file accept=".raw,.wiff,…">` | `#pick`, librerie, liste, campioni | iPadOS ingrigisce le estensioni ignote | `compat.js`: su `(hover: none) and (pointer: coarse)` toglie `accept` (resta in `data-accept`) a tocco, `click()` o `showPicker()`; il tipo lo controlla l'app | `test_compat`, `e2e_safari_ipad` |
| `showDirectoryPicker` | `cartella-puntata.js`, `libreria-corpo.js` | assente | `capacita` `cartellaLocale`; `#pickdir` (`webkitdirectory`) o OPFS | `e2e_capacita` |
| `webkitdirectory`, `webkitGetAsEntry` | `#pickdir`, `cartella-carica.js` | macOS 11.1; iPadOS: da provare sul dispositivo | trascinare i file | — |
| `DecompressionStream` | `gz-flusso.js`, `normative-scarica.js`, `insilico-scarica.js`, `flow-xlsx.js` | 16.4 (`deflate-raw` 16.4) | `gz-flusso.js` risponde vuoto; gli altri sotto il pavimento | — |
| `OffscreenCanvas` 2d (worker) | `rust-worker.js` | 16.4 | `capacita` `offscreen2d` | — |
| `navigator.locks`, `BroadcastChannel` | `gz-flusso.js`, `migra.js`, `rust-carica.js`, `archivio.js` | 15.4 | `capacita` `locks`; `listen` in try | — |
| `storage.estimate` / `persist` | `archivio.js`, `shell.js`, `libreria-corpo.js` | 17 / 15.2 | in try: 0/0 | — |
| WebGL2 | `mappa3d.js`, `mappa.js` | 15 | `capacita` `webgl2`, mappa 2D | `e2e_capacita` |
| `<dialog>` `showModal`, `inert` | molti | 15.4, 15.5 | sotto il pavimento | — |
| `startViewTransition` | `tema-morbido.js` | 18 | cambio di tema senza transizione | — |
| `requestFullscreen` | `shell.js` `#fs-btn` | macOS 16.4, iPadOS 16.4 (elementi) | pulsante nascosto | — |
| `gesturestart` (pizzico) | `infusione-grafo-gesti.js` | solo WebKit | Pointer Events altrove | — |
| Pointer Events, `setPointerCapture`, `pressure` | `eventi.js`, `mappa.js`, `affianca.js`, Disegno | 13 | — | `e2e_disegno_tablet` |
| CSS `:has()`, `@container`, `color-mix()`, `@property` | `app.css`, `canali.css`, `msngrafo-vista.js` | 15.4, 16, 16.2, 16.4 | sotto il pavimento | — |
| `svh`/`dvh`, `env(safe-area-inset-*)` | `app.css` (`#kframe`, `#phone`, barra laterale `#dfiles`) | 15.4, 11 | `100vh` prima, `@supports` dopo | — |
| `:hover` (lista della legenda con più di 4 tracce) | `grafici.js` `legCollapse` | — (iPad: niente hover) | il chip «N tracce» è un `<button>` da 24 px: tocco, Invio o Spazio tengono aperta la lista (`aria-expanded`), Esc la chiude | `e2e_safari_ipad` |
| `visualViewport` | `shell.js` (`dprNow`) | 13 | scala 1 | — |
| `prefers-reduced-motion`, `prefers-color-scheme` | molti | 10.1, 12.1 | — | `e2e_tema_impostazioni` |
| service worker, Cache API | `sw.js`, `browser.js` | 11.1 | sito senza copia offline | `e2e_sw_versione` |
