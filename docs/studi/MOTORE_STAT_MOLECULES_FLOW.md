# Studio: peso di `stat`, `molecules`, `flow` e `campioni` nel motore `.wasm`

Misura, senza spostare codice. Base: `origin/main` c5fb8c5, `cargo build --release -p mzlab-wasm`, `wasm-bindgen` 0.2.129, `wasm-opt -O3 @motore` (binaryen 133), come `pages.yml`. Metodo A/B: nella copia di lavoro le funzioni di ingresso di `crates/mzlab-wasm/src/{stat,molecules,campioni}.rs` sono state svuotate (non committate) e il motore ricostruito; la differenza è ciò che il linker non porta più, cioè il costo netto (il codice condiviso con altre rotte resta nel motore).

## Byte (motore intero 2 527 138 B, tetto 2 744 320)
| variante | raw | gz -9 | br -11 | tolto (raw / gz / br) |
|---|---|---|---|---|
| motore com'è | 2 527 138 | 1 065 936 | 812 875 | |
| senza `stat` + `molecules` (`stat_post`, `molecules`) | 2 319 659 | 986 775 | 755 868 | 207 479 / 79 161 / 57 007 |
| senza `flow` + `campioni` (`campioni`, `flow_command`, `flow_line`, `flow_sched`) | 2 179 217 | 922 311 | 706 493 | 347 921 / 143 625 / 106 382 |
| senza tutti e quattro | 1 969 337 | 840 820 | 647 253 | 557 801 (22,1 %) / 225 116 / 165 622 |

Righe di Rust coinvolte: `flow/` 4 579 + `campioni/` 3 817 (senza test), `molecules/` 1 356, `stat*.rs` 1 105.

## Compilazione V8
`WebAssembly.compile` in Chromium (headless, 7 giri, mediana): motore intero 7,7 ms, senza i quattro 6,3 ms (−1,4 ms, nel rumore: ±0,5 ms). La compilazione è pigra (Liftoff, funzioni compilate alla prima chiamata): a freddo costa la validazione, proporzionale ai byte (~0,6 ms ogni 100 KB), non l'esecuzione. Il guadagno vero sta nello scaricamento (br −166 KB) e nel parse/validazione sui telefoni, non in ms di desktop.

## Chi li chiama all'avvio
Nessuno. Con la pagina vuota (`#/data` senza file) i messaggi al worker fino a `qqReady` sono solo `module`, `policy`, `lr_boot`, `lr_dam_sync`, `lr_kv_get`, `lr_adduct_table`: nessun `lr_stat`, `lr_molecules`, `lr_campioni`, `lr_flow_*`. Chiamate in seguito, tutte su richiesta: `lr_campioni` (`campioni-doc.js`, `ruoli-carica.js`) all'apertura di file e della tabella di caricamento; `lr_flow_sched`/`lr_flow_line`/`lr_flow_command` (`lr-map.js`) aprendo il Flow; `lr_molecules` (`liste-target.js`, `molecole-archivio.js`) con liste target e archivio; `lr_stat` (`profilo.js`, `pca.js`, `kmd.js`, `bianco.js`, `riferimento.js`, `qaqc.js`) con profilo, PCA, KMD, bianco. Il motore li scarica e li compila comunque prima della prima richiesta.

## Cosa potrebbe diventare un modulo pigro
Schema già in uso (`flowtab` 146 846 B, `formula` 403 567 B: crate `mzlab-feat-wasm` con una feature, `wasm-feat/<nome>/`, caricato dall'esecutore che serve). Stima, da confermare con una build: un modulo a parte ripete glue di `wasm-bindgen`, allocatore e il codice condiviso (JSON, utilità), quindi pesa un po' più del tolto.
- `stat` + `molecules`: tolti 207 KB → modulo ~230-260 KB, sotto il tetto standard 512 000 con margine.
- `flow` + `campioni`: tolti 348 KB → modulo ~380-430 KB, sotto 512 000 (ma il Flow è subito usato alla prima apertura del tab: il caricamento va coperto dallo stato di attesa che c'è già per `flowtab`).
- Tutti e quattro in un solo modulo: ~600 KB, OLTRE 512 000: servono due moduli (o un tetto di gruppo, decisione di Federico). Il motore scenderebbe a ~1,97 MB (−22 %): ~165 KB di brotli in meno da scaricare alla prima visita e a ogni invalidazione di cache, ~0,6 ms-1,5 ms di validazione in meno su desktop.
- Contro: parità sito/app (il nativo non ha il problema: il costo è solo del `.wasm`), una rotta per modulo da instradare in `lr-map.js`, `campioni` serve al caricamento dei file (un salto di rete in più al primo file) e il motore deve poter chiamare `flow::run_web` senza `campioni`: l'A/B mostra che il resto di `flow` (esecutore) resta nel motore se raggiunto da altre rotte, quindi il numero sopra è un minimo sicuro.
