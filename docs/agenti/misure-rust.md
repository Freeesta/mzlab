# Misure dei motori Rust e dei moduli (seguito di `misure.md`)
Flusso Rust senza Pyodide, famiglie di ioni, origine di uno ione, mappa, composizione, spettri, picchi, HR/DDA, feature, gesti, lettori, profilo `desktop`, TP.

## Flusso Rust senza Pyodide (`tests_e2e/e2e_lr.py`, riga «MISURA»)
Container cloud (Chromium 1194), 2 file Full Scan sintetici (`tools/dati_sintetici.py`), un giro. «avvio→pagina pronta» = dall'apertura dell'indirizzo alla schermata di partenza; «apertura» = dal caricamento dei due file al primo TIC disegnato.

| flusso | avvio→pagina pronta (s) | apertura (s) |
|---|---|---|
| Rust (predefinito) | 0.46 | 1.3 |
| classico (`?motore=python`) | 2.47 | 1.2 |

L'apertura in Rust costa la costruzione dell'indice, circa 0.7 s (avvio del worker, 0.2 s di calcolo a file, in parallelo). Senza `indexList` (i file sintetici) lo scorrimento legge 1 MB per ogni spettro e ogni lettura sincrona di `Blob` in OPFS costa circa 1 ms: `idxread::Windowed` serve le letture piccole da una finestra di 1 MB e lo scorrimento tiene il suo buffer (688 letture da 1 MB → 5). Con file veri (centinaia di MB, di norma con `indexList`) non misurato.
Apertura progressiva ed early TIC: all'estrazione della tabella scansioni (~0,05-0,1 s), il worker emette `early_tic` nell'evento di avanzamento. La pagina mostra subito il TIC provvisorio e risponde alle richieste `api/chrom` senza attendere il completamento del `.mzidx`. Al termine dell'indice, la traccia provvisoria è sostituita con parità esatta.

## Famiglie di ioni per file (`cargo run --release -p mzlab-feat --example famiglie -- FILE.mzidx [--truth verita.json]`, 4 CPU, solo `analyse_families`)
Serie sintetica d'identità (9672 feature in `t05min`): 0,03 s, famiglie giuste 98,9 % (basi con la famiglia giusta 181/183), ISF 12/13, addotti 31/31, falsi ISF dichiarati ISF 0/13, NH4 lasciati aperti 8/8. Tre file Orbitrap da circa 80 MB (indice 54-56 MB): 141 208 feature → 119 317 famiglie in 0,76 s (il file con più rumore), 77 197 → 65 645 in 0,32 s, 79 560 → 67 594 in 0,32 s. Il tempo è per due terzi nella lettura delle tracce (`traces::gather` scorre quasi tutta la tabella dei picchi) e nelle finestre di m/z dei candidati.

## «Da dove viene questo ione?» (`api/origin`, `cargo test --release -p mzlab-core --test origin_parity` e `tools/origin_riferimento.py`, nativo, dati LR veri e sintetici, un report con 200 bootstrap, 400 spostamenti, 25 candidati)
| Motore | Tempo per rapporto (ms) |
|---|---|
| Python (numpy, `ionfamily.origin_report`) | 320-365 |
| Rust (`origin::origin_report`) | 80-90 |

Il tempo comprende ROI, tracce, somiglianza di profilo con p-value di permutazione, rapporto con bootstrap, cinetica, ruoli e candidati. Nel browser (WebAssembly) non misurato qui. Parità: stessi indici e conteggi, numeri uguali entro 1e-9, p-value Monte Carlo identici (stesso `default_rng`).

## Mappa RT×m/z e punti segnati (`api/map`, `mapsnap`, `mappunti`, `mappunti3d`; `cargo test --release -p mzlab-core --test map_parity -- --nocapture` e `tools/mappa_riferimento.py`, nativo, tre file Full Scan veri)
| Richiesta | Python (ms) | Rust (ms) |
|---|---|---|
| mappa intera (prima richiesta, a freddo) | 216 | 12 |
| mappa intera con sfondo (mediana lungo la corsa) | 20 | 22 |
| differenza con `ref` e `norm=max` | 6 (mappe già in memoria) | 12 |
| regione con zoom 120 × 100 | 3,6 | 2,7 |
| aggancio, misure dei punti, punti del 3D (300 punti) | 0,3-5 | 0,1-6 |

Nel browser (sintetici, due file di 2,4 MB, prima richiesta di ogni rotta, `e2e_lr`): mappa intera 54 ms in Rust contro 46 in Pyodide già avviato, con sfondo 102 contro 73, regione 36 contro 24, aggancio 7 contro 2, punti 26 contro 3, punti 3D 22 contro 37. Il guadagno non è la velocità della singola richiesta (Pyodide ha già la tabella in memoria) ma che Pyodide non parte: avvio della pagina 1,3 s contro 5,0 s. Parità: celle e punti uguali bit a bit; dei punti 3D tagliati a `max` si confrontano gli insiemi per intensità (numpy sceglie a caso fra intensità uguali).

## Formule compatibili (`api/composition`, `cargo test --release -p mzlab-core --test composition_route_parity -- --nocapture` e `tools/composizione_rotta_riferimento.py`, nativo) e XIC da formula (`api/subxic`, `--test subxic_parity` e `tools/subxic_riferimento.py`, due file Orbitrap veri di 80 MB)
| Richiesta | Python (ms) | Rust (ms) |
|---|---|---|
| composizione con intervalli di elementi (C 0-20, H 0-40, N 0-5, O 0-8, S 0-2) | 1-7 | 0,1-2,5 |
| composizione con gli intervalli di base | 20-44 | 5-7 |
| composizione con picchi isotopici e frammenti | 1-5 | 0,5-2,4 |
| XIC da formula, 24 000 sottoformule a ±5 ppm | 540-660 | 100-220 |
| XIC da formula, 26 000 sottoformule a ±10 ppm | 700-1240 | 140-280 |
| XIC da formula, 480 sottoformule | 9-21 | 9-45 (il tempo è quello di caricare la tabella dei picchi) |

Parità: stessi candidati nello stesso ordine e numeri entro 1e-9 per `composition`; righe, ordine e numeri identici per `subxic` (anche sui file veri). Nel browser non misurato: la pagina HR passa a Python alla prima richiesta di spettro, quindi `subxic` ci arriva solo da un motore già avviato; `composition` non ha bisogno di file.

## Spettri in profilo (`api/spectrum` e `api/spectra` di un file LR in profilo; `cargo test --release -p mzlab-core --test profile_parity -- --nocapture` e `tools/profilo_riferimento.py`, nativo; `e2e_lr` nel browser, file sintetico di 400 scansioni)
| Richiesta | Python (ms) | Rust (ms) | Browser: Pyodide già avviato / Rust (ms) |
|---|---|---|---|
| spettro di una finestra di 14 scansioni | 7-11 | 4-9 | 84 / 38 |
| spettro medio di tutta la corsa | 15-20 | 4-5 | 58 / 27 |
| 11 scansioni singole | 14-20 | 8-9 | 38 / 31 |
| 60 scansioni singole | 92 | 29-30 | 220 / 85 |
| una scansione singola | 1,6-2,7 | 4-5 (una lettura dell'indice intero) | – |

Parità: stessi picchi, stessa linea con gli zeri rimessi, numeri entro 1e-9; i punti sono sommati sulla griglia `rint(m/z / passo)` con il passo letto da venti scansioni a metà corsa, come `Item._step`; se nessuna di quelle ha più di 20 punti (convertitore che omette i bin a zero, come il lettore `.wiff`) il passo viene dalle scansioni della sezione che li hanno (fino a venti, l'intervallo più corto).

## Rilevatori di picchi e misure di qualità (`peakq.rs` contro `peaks::detect_peaks`, nativo, release, media di 20 giri, traccia sintetica di 15 min con tre picchi gaussiani e rumore)
| Punti della traccia | CWT (ms) | prominenza + minimo locale (ms) | misure di qualità (µs per picco) |
|---|---|---|---|
| 2000 | 5,9 | 0,20 | 2,3 |
| 20 000 | 54 | 9,3 | 20 |

La CWT è il metodo predefinito (parità di bordi e aree con Python in `golden.rs`); la modalità a prominenza costa un decimo e separa i picchi doppi dove la CWT li unisce (prove sintetiche in `peakq.rs`: doppio parzialmente risolto, spalla, coda, solo rumore).

## Spettro HR e tabella DDA (`cargo test -p mzlab-core --test hr_parity --release -- --nocapture`, stessa macchina, un giro)
Due file Orbitrap DDA sintetici (`tools/hr_riferimento.py`, ~15.000 picchi MS1 per scansione); «rust» = `Routes::handle`/`spectrum_exact` nativo dall'indice, «python» = `api.dispatch` nativo (senza il costo di Pyodide). Parità ≤1e-9 relativa su tutti i valori. Tempi in ms per richiesta:

| richiesta | rust | python |
|---|---|---|
| `api/dda` | 0,7 | 1,4 |
| `api/spectrum` MS1, finestra di 12 s (circa 20 scansioni) | 2,0 | 2,9 |
| `api/spectrum` MS1, tutto il file | 59 | 99 |
| `api/spectrum` MS2, tutto il file | 0,9 | 1,7 |
| `api/spectrum` MS2 con precursore | 0,2 | 0,5 |
| `api/spectrum` MS1, una sola scansione (legge l'mzML) | 1,5 | 1,7 |

## Rilevamento delle feature (`cargo run --release -p mzlab-feat --example feat -- FILE.mzidx...`, nativo, 13 file reali Orbitrap da circa 80 MB con 3,8 M picchi MS1 ciascuno)
1 259 216 feature in totale, le stesse di `features.detect` file per file; 0,5-0,7 s e 62-78 MB di picco per file (Python: 3,5 s e 565 MB); due esecuzioni e budget di 8 MB (picco 42-43 MB) danno lo stesso SHA-256.

## Gesti della mappa (`python3 tools/bench_mappa.py lr,hrc,hrp --wd DIR`, Chromium headless 4 CPU, motore WASM, file sintetici)
- Tabella completa, confronto nativo e diagnosi in `mappa-fluida.md`. Prima mappa: LR 1500×500 791 ms, HRC 3000×3000 (9 M picchi) 6332 ms (nativo 254 ms). Nitido dopo rotella: 176 ms LR, 249 ms HR; zoom solo RT in HR 1617 ms. Cambio di file caldo: 211 ms LR, 441 ms HR. Frame durante la rotella p95 33-50 ms. Picco WASM HR 175 MB.
- Rendering colore accelerato (`grafici.js`, `mapImage`): scrittura a 32 bit con `Uint32Array` (`buf32` e `tab32`) e campionamento a 10 000 celle per il 99,5° percentile: generazione `ImageData` su griglia 600×901 ridotta da 314 ms a 52 ms (6× più veloce), cambio file caldo su HRC ≤ 50 ms.

## Moduli lettori (`tools/build_lettori_wasm.sh`, dopo `wasm_opt -Oz`)
- `.wasm` di mzFinder e dei lettori: misure, tetti (un modulo fuso ha un tetto di gruppo, non quelli per modulo) e build separata in `docs/agenti/wasm-tetti.md`. Il `.wasm` del motore pesa 1 439 240 B (senza il lettore Thermo, che sta nel modulo `thermo`).
- Memoria del modulo `thermo` (`tools/bench_raw.py`, memoria iniziale 32 MB) sul BAF DDA da 595 MB (56 061 scansioni): dopo `apri` 33,5 MB (`apri` 1,0 s), dopo `indice` 48,0 MB (186 ms, JSON da 13,7 MB), picco di tutti i passi 122 MB (`to_mzml` 21,8 s); su file più piccoli 44,4 MB per 7 952 scansioni e 85,1 MB per 16 524 (prima della memoria compatta).

## Profilo `desktop` (`Cargo.toml`, `opt-level = 3` contro `"z"`)
- `mzlab_wasm.wasm` con `cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`: SHA-256 `d5cca2b7…d00da` prima e dopo l'aggiunta del profilo (identico). Tempi del `.raw` nativo con `raw_tempi` e profilo `desktop` (studio `docs/studi/RAW_VELOCE.md`, DDA da 595 MB): tabella 8,33 → 5,30 s, `.mzidx` 7,07 → 3,59 s, mzML 13,99 → 12,15 s.

## Tabella e `.mzidx` con una decodifica (righe dirette da `Spectrum`)
- `upload::indicizza_lettore` decodifica ogni scansione una volta sola per la tabella e la prima passata di `build`, senza testa mzML né rilettura; passate successive solo se il budget è superato. `.mzidx` identico byte per byte alla strada via mzML (`tests/mzidx_righe_dirette.rs`). SHA-1 della testata non nel percorso dell'indice (`indicizza_lettore` usa lo SHA-256 già calcolato). Peso di `mzlab_wasm.wasm` prima di `wasm-opt`: 3 409 576 → 3 438 828 B.
- Efficienza memoria passata 1: i gruppi MS1 dei file HR tengono `Peak<f32>` (12 B anziché 24 B) passandoli alla scorta già arrotondati, dimezzando l'impronta di memoria durante il raggruppamento e riducendo il picco RSS di costruzione.
- LTO del profilo `desktop` (`mzidx_tempi` sintetico, 570 MB): `lto = "fat"` tabella + `.mzidx` 4,41 s, costruzione 99 s; `"thin"` 4,16 s, 68 s: differenza nel rumore, quindi resta `lto = true` come in `release`.
- Tempi sui dati veri (BAF, LAB-A, PXD077492, `.wiff`, `.D`): da misurare in locale con `cargo run --release --example raw_tempi -p mzlab-core -- FILE.raw` (fase «table + .mzidx (one call)»); obiettivo tabella ≤ 1,3 s e `.mzidx` ≤ 3 s sul BAF. Senza dati veri: `cargo run --release --example mzidx_tempi -p mzlab-core`.
- mzML veloce (`examples/mzml_tempi.rs`, nativo, 4 CPU, mzML sintetico a centroidi di 452,7 MB, 44 000 scansioni, 28,3 M di picchi, budget 64 MB del browser): `.mzidx` senza scorta 67,7 s (18 passate) → con scorta 11,6 s (1 passata), stessi 950,6 MB; lettura di tutti i picchi 3,5 s, `prepare` 2,9 s (senza SHA-1 intero).
- Scrittura mzML (`examples/mzml_scrivi.rs`, 2000 scansioni × 5000 picchi): zlib 1 149,0 MB in 1,60 s, zlib 3 148,6 MB in 3,22 s, zlib 6 148,6 MB in 3,61 s, numpress 67,8 MB in 1,28 s. `.wasm` del motore 1 469 691 B dopo bindgen e `wasm-opt`. Tempi nel browser: da rifare da Federico sul file vero.

## Dig LR con più condizioni (`cargo test --release -p mzlab-finder-lr-wasm --test lr_gruppi -- --ignored --nocapture scale`, `N=20` o `N=40`, nativo, 4 CPU, file sintetici di `tests/data/gruppi/`)
20 file (3 condizioni, bianco, standard): indici 90 ms, pipeline + Dig 130 ms, `summary` con `gruppi` 2 ms, picco di memoria 8,1 MB. 40 file (6 condizioni, bianco, standard, 2 repliche in più): 170-195 ms, 240-300 ms, 3 ms, 11,3 MB. Rapporto 40/20: tempo 1,9-2,3×, memoria 1,4×. `finder-lr.wasm` senza `wasm-opt`: 1 925 570 → 2 055 819 byte (+6,8 %).
- Browser (`tools/misura_gruppi.py`, Chromium headless, sito costruito senza `wasm-opt`, 2 file per condizione: t0 e t10): Dig da «Run» alla tabella, 20 file (10 condizioni) 0,9-1,9 s (mediana 1,3 s), 40 file (20 condizioni) 2,4-2,9 s (mediana 2,4 s): rapporto 1,8× (singole prove 1,5-2,7×, tempi brevi e rumorosi). Picco di memoria del browser durante il Dig 770-840 MB con 20 file, 850-915 MB con 40 (1,03-1,1×). Primo disegno della vista «Compare conditions» 38-67 ms.

## Interpretazione dei TP sui dati veri (`tools/valida/tp_timololo.py`)
Metriche (numeri solo nel repository privato): TP di letteratura in vista, regioni concordi, disaccordi di posizione da rivedere, candidati nuovi, FDR delle esche (`decoys.survived / decoys.rules`), distribuzione nulla del margine, concordanza con l'oro di mzmine (se la lista c'è).

## `.wasm` interp (`tools/build_feat_wasm.sh`, `cargo build --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features interp`)
Dopo `wasm-bindgen --target web` e `wasm-opt -Oz` (binaryen da npm): 496 569 B dopo `wasm-opt` (388 039 B senza i siti e il disegno): sopra il cancello di 470 000 B e sotto il tetto di 512 000 B, che non si alza; l'albero MSn lo costruisce ancora `interp` stesso e è un'eccezione accettata per ora: `interp` non deve crescere oltre; ogni peso nuovo si paga prima spostando l'albero in `finder_msn` (piccola PR a parte). Gli altri `.wasm` non cambiano peso.
Clic Tabui: `misure-clic.md`; mzTab-M: `misure-esporta.md`.
Gesti del visualizzatore, sito e nativo: `tools/bench_visore.py`; tabella in `visore-veloce.md`.

## Librerie spettrali: `libset_bench` (nativo, 1 milione di spettri sintetici, `cargo run --release -p mzlab-lib --example libset_bench -- 1000000 unit|hr [librerie]`)
hr, 1 libreria: identità 0.06 ms/query a caldo (0.28 a freddo); analoghi 51 ms; ricerca aperta ibrida con accumulatori piatti 49–52 ms contro 138 ms della versione a coppie (×2.8). unit, 1 libreria: identità 1.4 ms (1.7 a freddo); analoghi 73–93 ms; ibrida aperta 77–106 ms (la macchina varia) contro 207–230 ms a coppie (×2.1–2.8). unit, 3 librerie da 1M ciascuna fuse: identità 1.5 ms, ibrida aperta 70–81 ms per libreria cercata.
