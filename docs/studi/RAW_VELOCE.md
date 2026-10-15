# Studio: `.raw` Thermo veloci (apertura, centroidi, mzML, navigazione, metadati)

Scopo: un `.raw` Thermo, anche un DDA da 600 MB, si apre, si indicizza, si naviga e si converte in centroidi/mzML nel sito senza attese percepibili; passa da Dati a Tabui (o entra direttamente in Tabui) senza conversioni manuali; tutto ciò che il file registra esce in `metodo/1` o in una traccia visibile. I prompt dei pacchetti sono in `docs/studi/PROMPT_RAW_VELOCE/`.

## 1. Come si misura (riproducibile)

- **Nativo, fase per fase**: `cargo run --release --example raw_tempi -p mzlab-core -- FILE.raw [--no-cen] [--no-mzml]` (`crates/mzlab-core/examples/raw_tempi.rs`). Rifà le fasi di `build_mzidx_native` (`crates/mzlab-wasm/src/nativi.rs:258`): impronta (`upload::impronte`), apertura (`lettori::apri`), decodifica di ogni scansione, tabella (`upload::prepare_lettore` → `mzidx_lettore::righe`), `.mzidx` (`mzidx::build_lettore`, budget 192 MB), «Scarica .mzML» (`lettori_mzml::scrivi`); ogni fase anche con `lettori_centroidi::centroidato`.
  Legge attraverso una cache a blocchi di 256 KB come `JsMember` del modulo e conta le letture; stampa il picco di memoria (VmHWM).
- **Dove va il tempo**: `valgrind --tool=callgrind target/release/examples/raw_tempi FILE.raw --no-cen` su un file piccolo (≤ 20 MB), poi `callgrind_annotate --inclusive=yes`.
- **Modulo `thermo` in WASM**: `wasm-bindgen --target nodejs` del `.wasm` di `tools/build_lettori_wasm.sh`, poi uno script Node che chiama `apri`, `indice`, `scansione` per ogni k, `testata`, `metodo`, `to_mzml` e legge `memory.buffer.byteLength` dopo ogni passo (il WASM non restituisce memoria: il massimo è il picco). Il pacchetto RV-01 lo trasforma in `tools/bench_raw.py`.
- File: il DDA grande è PRIDE PXD070375 `BAF_GEE_37.raw` (Q Exactive Plus, DDA HCD Top-N, CC0), 594 980 630 B; URL e SHA-256 stanno in `dati_formati/_grandi_thermo_aperti.tsv` del repository privato. Gli altri sono i `.raw` di `dati_formati/` e due Orbitrap del laboratorio (nel testo: LAB-A, 87 MB, 7952 scansioni «FTMS + c»; LAB-B, Orbitrap a profilo «FTMS + p»); i percorsi degli alias sono in `dati_formati/raw_veloce_alias.tsv` del repository privato.
- Macchina: container cloud a 4 CPU; profilo `release` dello spazio di lavoro (`opt-level = "z"`, LTO, 1 unità). I tempi nel browser sono ~1,7× quelli nativi (misura su LAB-A, `to_mzml`).

## 2. Misure

### 2.1 Nativo (s; `opt-level = "z"`, senza centroidi)

| File | Impronta SHA-256 | Apertura | Decodifica di tutto | Tabella (righe) | `.mzidx` | mzML | Picco memoria |
|---|---|---|---|---|---|---|---|
| BAF 595 MB, 56 061 scansioni (12 443 MS1, 43 618 MS2), 10,77 M picchi | 3,70 | 0,82 | 0,88 | 8,33 | 7,07 (2 passate, 196 MB) | 13,99 (239,5 MB) | 808 MB |
| LAB-A 87 MB, 7952 scansioni, 4,19 M picchi | 0,51 | 0,13 | 0,23 | 1,12 | 1,03 | 3,95 (56,7 MB) | – |
| PXD077492 32 MB | 0,21 | 0,14 | 0,05 | 0,60 | 0,34 | 1,03 | – |

- Letture: su LAB-A la decodifica di tutte le scansioni chiede 8,46 M letture (`read_at`) per 270 blocchi da 256 KB: ~1060 per scansione, due per picco, perché `BinaryReader` legge campo per campo (`read_f32`/`read_f64` → `read_exact`). Nel modulo la cache è in Rust (`LETTURA`, `crates/mzlab-lettori-wasm/src/lib.rs:32`): le chiamate a JS sono 270, ma ogni lettura paga la copia e i controlli.
- La decodifica pura è una piccola parte: la tabella costa 9× la decodifica (BAF 8,33 contro 0,88 s), il `.mzidx` 8×.
- Con `opt-level = 3`: BAF tabella 5,30, `.mzidx` 3,59, mzML 12,15, SHA-256 3,73; LAB-A tabella 0,82, `.mzidx` 0,81, mzML 3,14. Il codice di mzLab perde 30-50 % con `z`; lo SHA-256 no.
- Centroidi (`lettori_centroidi`): PXD077492 decodifica 0,39 s (×8), da 520 k a 163 k punti.

### 2.2 Dove va il tempo (callgrind, MTBLS9839 17 MB, inclusivo)

| Voce | Quota |
|---|---|
| SHA-256 dell'impronta (`upload::impronte`) | 30,6 % |
| SHA-1 di tutto il file in `testata_mzml` (`rawlettore.rs:429-431`), chiamato da `righe` (`mzidx_lettore.rs:30`) | 23,7 % |
| zlib dell'mzML (`mzmlwrite.rs:56`, livello predefinito 6) | 17,6 % |
| `read_bytes_into` (letture campo per campo di opentfraw) | 7,3 % |
| `metodo_raw` (testo e XML del metodo) | 6,6 % |
| `analog_traces` | 4,6 % |

Il file è letto per intero 3-4 volte prima che l'utente veda la mappa: copia in OPFS, SHA-256 dell'impronta, SHA-1 della testata dentro la tabella; «Scarica .mzML» rifà lo SHA-1. Lo SHA-256 è identità (`.mzflow`, `sorgente_sha256` di mzFinder), non validità (la validità è `source.probe`).

### 2.3 Modulo `thermo` in WASM (Node, senza `wasm-opt`)

| File | `apri` | ogni scansione via `sessione` | `testata` (SHA-1) | `metodo` | `to_mzml` |
|---|---|---|---|---|---|
| LAB-A | 266 ms | 790 ms (99 µs/scansione; meta JSON 194 B/scansione) | 960 ms | 276 ms | 6654 ms |
| PXD077492 | 363 ms | 251 ms | 612 ms | – | 2698 ms |
| BAF 595 MB | 2597 ms, **240 MB di memoria WASM** | – | – | – | – |

**Il DDA da 595 MB oggi non si apre nel sito**: dopo `apri` la memoria del modulo è già 240 MB; `indice` (`crates/mzlab-lettori-wasm/src/sessione.rs:51-61`, un `serde_json::Value` per scansione) supera il tetto di 256 MB (`--max-memory=268435456` in `.cargo/config.toml`) e va in OOM. Causa: `RawFileReader` tiene in memoria tutti i trailer (`scan_parameters: Vec<GenericRecord>`, `vendor/opentfraw/src/reader.rs:334`) e `GenericRecord { values:
Vec<(String, GenericValue)> }` (`generic_data.rs:38-39`) ripete ogni nome di campo (~70 per scansione) in ogni record: 56 k × 70 × (String + valore) ≈ 150-200 MB; più `scan_events` (preambolo, reazioni, coefficienti) per scansione. Su iPad il rischio raddoppia: motore + modulo + i worker di `idxpool.js`, ognuno con il suo modulo e il suo `.raw` aperto.

### 2.4 Correttezza: centroidi del file segnati come profilo

- `raw::spectrum` (`crates/mzlab-core/src/raw.rs:33`) mette `profile = (scan_mode == Profile)`, ma `LettoreRaw::scansione` (`rawlettore.rs:293`) chiama `extract_spectrum(..., false)` → `read_peaks_only` (`vendor/opentfraw/src/scan_data.rs:120`): i picchi sono la lista dei centroidi del file, non il profilo.
- In un'acquisizione Orbitrap a profilo (LAB-B, filtro «FTMS + p») la scansione 0 dà 492 centroidi con `profile = true`; `lettori_centroidi::centroidato` li ricentroida e ne restano 66. Su tutto il file 1 051 091 → 186 194 picchi (−82 %). Stesso difetto su PXD077492 (SIM «FTMS + p»: 71 → 15).
- Tocca il `.cen.mzidx` di mzFinder (`lr_index_cen`), l'mzML scaricato (dichiarato «profile spectrum» con dati centroidi) e lo spettro mostrato. LAB-A («FTMS + c») è corretto.

### 2.5 `.raw` che opentfraw non apre (11 su 41)

Exploris 480/240 (MTBLS14155, MTBLS13497: DDA), Fusion Lumos (MTBLS825: «I/O Invalid argument»), LTQ Orbitrap XL + PDA (MTBLS804), LTQ Velos (unittest), TSQ Vantage/Quantum (MTBLS1752, MTBLS1807, MTBLS5229, PXD019516), TSQ Quantum Ultra (PXD006054), TSQ 9000 GC (MTBLS5358). Errori «refusing to allocate» (un conteggio letto male supera `Limiti`) o EOF. MTBLS2956 si apre ma ha 0 scansioni leggibili (`error.badIndex k=0`).

### 2.6 Metadati: cosa c'è nel `.raw` e cosa esce in `metodo/1`

- **Nessuna immagine dello strumento**. Cercate in 41 file le firme PNG, JPEG (JFIF/Exif), GIF, BMP, EMF/WMF, TIFF, PDF, SVG, ZIP, anche dentro i flussi del metodo incorporato: le poche corrispondenze sono coincidenze nei dati binari. Un `.raw` non contiene foto o icone dello strumento.
- Il metodo incorporato è un documento OLE2 (Compound File) a ~5-7 KB dall'inizio con i flussi `LCQ Header`, `AuditData` e, per ogni dispositivo, `<Dispositivo>/Text`, `Data`, `Header`, `Comments`. I `Data` sono XML (Exactive in UTF-16; il driver SII/Chromeleon in UTF-8, fino a 414 KB, quasi tutto tabella dei simboli). opentfraw non legge l'OLE: trova i testi con un'euristica UTF-16 nei primi 4 MB (`extract_utf16le_text`).
- In `metodo/1` ci sono già: strumento (modello, seriale, hardware, software), operatore e computer (marcati `identita`), `file_creato`, campione (vial, volume, file del metodo), gradiente, flusso, forno e pressione per Vanquish/UltiMate (SII), esperimenti dell'XML del Q Exactive.
- Lacune:
  1. Lista di inclusione (tabella «INCLUSION LIST» del testo Exactive, con massa, formula, specie, nome, finestra di RT; LAB-A ne ha 18) non strutturata.
  2. NCE a gradini «(N)CE: 20, 35, 60» resa come media (38,33).
  3. Finestra di isolamento del dd-MS2 letta male (`Isolation_low` 20000 / `Isolation_high` 1,5 invece di 1,5 m/z).
  4. Impostazioni globali e dipendenti non strutturate: lock mass, larghezza del picco cromatografico, apex trigger, loop count (Top-N), soglia d'intensità, AGC minimo, esclusione dinamica, carica.
  5. LC vuoto per Dionex Chromatography MS Link, Proxeon EASY-nLC, Accela/LegacyDualPump/Surveyor, Waters ACQUITY.
  6. Metodo delle trappole lineari (LTQ, Velos, Elite, Orbitrap XL: scan event, impostazioni dipendenti): 0 esperimenti.
  7. Solventi (`%A1_Equate` «Water + 0.1% FA») fuori da `lc`.
  8. Colonna: assente in tutti i file visti; va scritto «non presente nel file», non inventato.

### 2.7 Navigazione

La mappa e i suoi gesti hanno già misure e pacchetti in `docs/agenti/mappa-fluida.md` (P2 piramide di tessere, P3 colore, P4 trasporto binario, P5 LRU/prefetch, P6 rAF, P7 GPU, P8 desktop): non si duplicano. Ciò che il `.raw` aggiunge:
- ogni scansione chiesta dal motore (spettro, XIC fuori indice, tessere) passa dal modulo come `Float64Array` di intensità più una meta JSON costruita con `serde_json` (`sessione.rs:64-66`): 99 µs/scansione in WASM, la metà di serializzazione;
- la rotta `/api/dda` (albero dei precursori) usa solo i metadati dell'indice: costa O(scansioni) una volta, non è un collo di bottiglia;
- lo scorrimento MS1/MS2 su un DDA costa una scansione per passo: con 99 µs è fluido; il limite è il primo indice (2.3).

### 2.8 Inventario di tutte le strutture del `.raw`

Misurato su 30 `.raw` aperti di `dati_formati/` (sonda temporanea su `RawFileReader`). Colonne: cosa contiene; se opentfraw la legge; dove arriva oggi in mzLab; pacchetto per ciò che manca. «campi» = `metodo/1.campi` (regola «nessuna perdita», `docs/agenti/grezzi-non-estratti.md`).

| Struttura | Contenuto | opentfraw | mzLab oggi | Manca → pacchetto |
|---|---|---|---|---|
| FileHeader | versione, audit tag di creazione/modifica (utente, data) | sì | `origine.file_creato`, `operatore.utente` (identità), campi | – |
| SeqRow | vial, volume iniettato, diluizione, peso, ISTD, id, commento, 5 etichette utente, metodo strumento/elaborazione, nome e percorso | sì | `campione`, campi (identità per percorsi ed etichette) | le etichette utente possono portare nomi di persone (visto su un file PRIDE): restano identità, mai nell'mzML |
| ASInfo (autocampionatore) | preambolo di 24 B + stringa | **letto e scartato** (`reader.rs:457-459`) | nulla | RV-10 |
| RawFileInfo | data/ora d'acquisizione, numero di controller, intestazioni delle etichette, nome del computer | sì | `origine`, `operatore.computer` (identità), campi | – |
| RunHeader per controller | intervallo di scansioni, m/z min/max, tempi, corrente ionica massima, nomi di file, indirizzi | sì (solo quello MS in dettaglio) | campi (MS e `Controller<N>`) | – |
| InstID | modello, nome, seriale, software, hardware | sì | `strumento` | posizioni DA CONFERMARE fuori da v66 (`metodo.md`) |
| Metodo incorporato (OLE2) | `LCQ Header`, `AuditData`, per dispositivo `Text`, `Data` (XML), `Header`, `Comments` | testi con euristica UTF-16 | `lc`, `ms`, `campionatore`, `grezzo` | OLE vero, `AuditData`, `Comments`, lacune di §2.6 → RV-09 |
| Indice delle scansioni | offset, RT, TIC, picco base, m/z min/max, tipo di pacchetto per scansione | sì | `.mzidx` (TIC e picco base dai picchi) | – |
| Eventi di scansione | preambolo (polarità, analizzatore, dipendente…), reazioni (precursore, energia, attivazione), finestre, coefficienti di calibrazione | sì (preambolo in parte) | filtro, precursore, attivazione nell'mzML | byte del preambolo non decodificati: elenco in RV-10 |
| Trailer extra (`ScanParameters`) | 3-76 campi per scansione (iniezione, AGC, risoluzione, isolamento, carica, lock mass, ottica ionica, diagnostica) | sì | mzML (`opentfraw.*`, `opentfraw.trailer.*`), campi (costante o intervallo) | serie per scansione visibili (iniezione, AGC, lock mass) → RV-10 |
| Status log | 51-287 canali (vuoto, temperature, tensioni, correnti, gas, pompe) × 1-3599 istanti con il tempo | sì | campi: solo valore costante o `min..max` | **serie temporali visibili** → RV-10 |
| Error log | 0-18 voci con il tempo (instabilità dello spray, calibrazione fallita, avvisi dell'elettronica, configurazione del reagente ETD) in 5 file su 30 | sì | **nulla** | RV-10 |
| Instrument log per scansione | 0 record in tutti i file visti | sì | `opentfraw.status.*` | – |
| Controller analogici | pressioni delle pompe, forno, refrigeratore, lampada, UV a una lunghezza d'onda | sì (8 e 16 B per campione) | cromatogrammi `pressure`/`analog`/`pda` (selettore del tipo, `dati-3200.md`) | – |
| PDA 3D | 572 B per istante | no | avviso | grezzi-non-estratti; fuori da questo studio |
| Rumore e linea di base | pacchetti per scansione | sì | intestazione della scansione (`noise_fields`) | – |
| Tune | nei file Orbitrap visti i valori di sorgente stanno nello status log e nel testo del metodo; nessun blocco Tune separato trovato (`unk_addr` = 0 in tutti) | – | campi | RV-10 verifica sui file LTQ (blocco Tune dopo lo status log) |
| Immagini dello strumento | nessuna (§2.6) | – | – | – |

### 2.9 Percorso Dati → Tabui e caricamento diretto in Tabui

- **Dati → Tabui** («Reload from Data», `TP_Mine/js/20_tpmine.js:125,265`): un campione nativo passa come segnaposto e Tabui legge il `.mzidx` v2 della scheda Dati (`stubIndexed`, `TP_Mine/files/finder-idx.js`): nessun mzML, nessuna rilettura. Se la prima MS1 è segnata come profilo, la pagina costruisce `<nome>.cen.mzidx` (`ask("cenidx")` → `rb.nativeCentroidIndex` → `lr_index_cen`) rileggendo e decodificando il `.raw` da zero.
  LAB-B: indice a centroidi ≈ 4,8 s nativi (apertura 0,19 + impronta 0,49 + tabella 2,88 + `.mzidx` 1,27), ~8 s nel sito, e con l'82 % dei picchi persi (§2.4). Con RV-02 LAB-B è a centroidi: l'indice di Dati vale, costo 0.
- **Caricamento diretto in Tabui**: `#tp-in` accetta solo `.mzML` (`TP_Mine/js/20_tpmine.js:196`); un `.raw` va prima aperto in Dati. Il requisito: lo stesso `.raw` (o `.wiff`, `.D`) scelto in Tabui si copia e si indicizza durante il caricamento, con la barra a fasi di `nativo-fasi.js`, senza mzML e senza attese morte.
- Con un profilo vero (nessuna lista di centroidi nel file) l'indice a centroidi si costruisce nella stessa decodifica dell'indice di Dati quando il file entra da Tabui, non dopo.

## 3. Obiettivi numerici

| Misura | Oggi | Obiettivo |
|---|---|---|
| BAF 595 MB: si apre nel sito | OOM | sì, memoria del modulo dopo `apri` + `indice` ≤ 48 MB |
| BAF: dal file scelto alla mappa (nativo, senza copia) | 3,70 + 0,82 + 8,33 + 7,07 = 19,9 s | ≤ 6 s (nel sito ≤ 10 s) |
| LAB-A: idem | 2,79 s | ≤ 1,0 s |
| Letture intere del file prima della mappa | 3 (copia, SHA-256, SHA-1) | 1 (la copia; gli hash nella stessa passata) |
| Tabella (righe) | 9× la decodifica | ≤ 1,5× la decodifica |
| `.mzidx` del BAF, budget 192 MB | 7,07 s, 2 passate | ≤ 3 s (RV-04: una decodifica, righe dirette; misura in locale con `raw_tempi`) |
| mzML del BAF (nativo) | 13,99 s | ≤ 6 s con zlib veloce; ≤ 4 s con numpress (opzione) |
| Centroidi del file segnati come profilo | −82 % dei picchi | 0 scansioni ricentroidate per errore |
| Modulo per scansione (WASM) | 99 µs | ≤ 40 µs |
| `.raw` di `dati_formati/` aperti | 30/41 | ≥ 38/41 |
| Dati → Tabui, campione a centroidi | 0 s (indice riusato) | 0 s |
| Dati → Tabui, LAB-B (oggi segnato profilo) | ~4,8 s nativi, ~8 s nel sito, −82 % picchi | 0 s (RV-02) |
| `.raw` scelto in Tabui | non accettato | accettato; pronto nel tempo dell'indice di Dati, con barra |
| Status log ed error log visibili nel tempo | no | sì (selettore del tipo di cromatogramma, finestra Metodo) |

Equivalenza: il `.mzidx` e l'mzML restano byte per byte quelli di oggi dove il pacchetto non cambia il contenuto (stesso SHA-256 su LAB-A, PXD077492, BAF); dove lo cambia (centroidi, livello zlib) la parità è sui valori (`tools/golden.py` e l'mzML di msconvert).

## 4. Pacchetti

| Pacchetto | Cosa | Motore | Parte | Dopo il merge di |
|---|---|---|---|---|
| RV-01 | Memoria e apertura di opentfraw: trailer compatti/pigri, indice senza `serde_json::Value`; banco `tools/bench_raw.py` | no (cambia `thermo`) | ondata 1 | – |
| RV-02 | Centroidi del file segnati come profilo; vecchio `.cen` invalidato | no (cambia `thermo`) | ondata 1 | – |
| RV-03 | Una sola passata per gli hash, fuori dal percorso critico; tabella senza SHA-1 | sì, catena 1/5 | ondata 1 | – |
| RV-04 | Tabella e `.mzidx` con una decodifica sola, senza il giro testo mzML | sì, catena 2/5 | ondata 2 | RV-03 |
| RV-05 | Decodifica a blocchi e indice binario a colonne tra modulo e motore | sì, catena 3/5 | ondata 3 | RV-04, RV-01 |
| RV-06 | mzML veloce: zlib rapido, numpress opzionale, scrittura a flusso | sì, catena 4/5 | ondata 4 | RV-05, RV-02 |
| RV-07 | Desktop: profilo `opt-level = 3` per il binario nativo | no | ondata 1 | – |
| RV-08 | `.raw` che non si aprono; ASInfo e preambolo degli eventi | no (cambia `thermo`) | ondata 2 | RV-01 |
| RV-09 | Lacune di `metodo/1` (OLE, inclusione con tasti «XIC»/«MS2», NCE, isolamento, globali, LC, LTQ, solventi) | no (cambia `thermo`) | ondata 1 | – |
| RV-10 | Status log e trailer come serie nel tempo, error log, autocampionatore, Tune | no (cambia `thermo`) | ondata 1 | – |
| RV-11 | Tabui: `.raw`/`.wiff`/`.D` scelti direttamente, indice durante il caricamento, riuso da Dati | punti 1-3, 5, 6 no; punto 4 sì, catena 5/5 | ondata 2 (punto 4: ondata 5) | RV-02, RV-03 (punto 4: RV-06) |

Ondata 1 (sei chat insieme): RV-01, RV-02, RV-03, RV-07, RV-09, RV-10. Ondata 2: RV-04, RV-08, RV-11 (punti 1-3, 5, 6). Poi la catena del motore in serie, RV-05 → RV-06 → RV-11 punto 4: ogni pacchetto che cambia `mzlab_wasm.wasm` parte dopo il merge del precedente e riporta il peso prima → dopo.
File condivisi tra chat parallele (funzioni diverse, `git merge origin/main` prima di unire): `rawlettore.rs` (RV-01 chiamate, RV-03 `testata_mzml`, RV-10 cromatogrammi), `sessione.rs` (RV-01, RV-03), `metodo_raw.rs` e `metodo-doc.js` (RV-09; RV-10 una riga), `lang/{it,en}.js` (RV-09, RV-10: chiavi nuove in un blocco proprio), `grezzi-non-estratti.md` (RV-08, RV-09, RV-10). Nessun'altra sovrapposizione: le chat restano al massimo sei alla volta, sotto il tetto di 15.
La navigazione segue `mappa-fluida.md` (P2-P6) dopo RV-05.
Scelte di prodotto (Coordinatore): la lista di inclusione va in `metodo/1` e si mostra in sola lettura nella finestra Metodo del file in Dati (m/z, RT/finestra, polarità, CE, nome), esportabile con il metodo, mai trasformata in «TP attesi» in Tabui (RV-09); «Scarica .mzML» ha una casella numpress spenta di default, con una riga che dice che è con perdita, e zlib a livello rapido (RV-06).
