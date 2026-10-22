# Studio: lettore `.raw` Thermo «perfetto» (RawFileReader come oracolo)

Scopo: il lettore Thermo di mzLab (`vendor/opentfraw` 2.0.0 modificato, Rust puro, Apache-2.0, anche in `.wasm`) dà per ogni scansione tutto ciò che dà RawFileReader di Thermo (RFR), con gli stessi valori, e in più etichette pulite (attivazione e NCE per reazione), tolleranza ai file rotti e velocità nel browser. Seguono `RAW_VELOCE.md` (velocità, `metodo/1`, RV-01…RV-11) e `ROSINA.md` (banco `verita/1`, livelli F0-F8).

**RFR solo come oracolo.** La licenza di RFR è proprietaria: DLL e pacchetto NuGet non entrano in nessun repository (né `mzlab` né `mzlab-dati`), né in binari, `.wasm` o CI; si usano solo sul Mac, in una cartella fuori dai repository, per produrre dump di riferimento. Mai copiarne il codice né decompilarlo (`ROSINA.md`, Metodo). Prima del primo dump la chat locale salva il testo della licenza accettata in `mzlab-dati/dati/formati/_licenze/` e verifica che l'uso per confronto sia permesso; un dubbio va al Coordinatore e a Federico.

## 1. Come leggiamo un `.raw` oggi
- **FileHeader** (`header.rs`): firma `Finnigan`, versione (dichiarate 8, 47, 57, 60, 62, 63, 64, 66; un'altra dà `err.file.rawVersion`), audit tag. Poi **SeqRow** (vial, volume, metodo), **ASInfo** (letto e scartato), **RawFileInfo** (data, numero e indirizzi dei controller).
- Per ogni controller un **RunHeader** (`run_header.rs`): intervallo di scansioni, tempi, indirizzi di indice, eventi, trailer, status log, error log, dati. Quello MS è seguito da **InstID** (modello, seriale, versioni) e dal **metodo incorporato** (OLE2, oggi testi UTF-16 trovati per euristica).
- **Indice delle scansioni** (`scan_index.rs`): per scansione offset, RT, TIC, picco base, m/z min/max, tipo di pacchetto, numero dell'evento.
- **Eventi di scansione** (`scan_event.rs`, `ScanEventTable`): preambolo a byte (polarità, analizzatore, livello MSn, dipendente, attivazione, wideband…), reazioni (`Reaction`: precursore, `unknown_double`, energia, `unknown_long1/2`), finestre m/z, coefficienti di calibrazione. In v66 tre dimensioni di evento (MS1, dipendente, DIA) trovate con `walk_events`, ed eventi «contati» (96 + 56 B per reazione, fino a MS8) con `walk_counted`.
- **Generic data** (`generic_data.rs`): schemi auto-descritti (etichetta, tipo) per trailer extra, status log, tune; i record restano nel file (`GenericRecordStream`).
- **Dati** (`scan_data.rs`): pacchetto con intestazione da 40 B, centroidi, profilo a blocchi, rumore/linea di base; formati FT, LT, TSQ (SRM, footer v66). Gli analogici (pompe, UV a 8 o 16 B per campione) da `analog_traces`.
- In mzLab: `rawlettore.rs` (trait `Lettore`: indice, `scansione`, `scansione_dati`, cromatogrammi), `raw.rs`, `rawfilter.rs` (filtro), `metodo_raw/` (`metodo/1`); nel sito il modulo `thermo` (`crates/mzlab-lettori-wasm`, `sessione.rs`) dietro il lettore esterno del motore.

## 2. Lacune (✔ nota nel codice, ? da misurare con l'oracolo)
1. ✔ **Catena delle reazioni**: `PrecursorInfo::reactions` dà per ogni passo di una MS2…MS8 precursore, attivazione, energia e (passo finale) larghezza d'isolamento; il filtro e il dump `scandump/1` portano un passo per clausola. ? Uguaglianza con RFR campo per campo: nessun dump MS3+ ancora (R-03), `VERIFICATI` vuoto.
2. ✔ **Attivazione per reazione**: passo finale dal preambolo (CID/HCD su FTMS dal flag della reazione), passi precedenti dai bit di `unknown_long1` (CID, MPD, ECD, PQD, ETD, HCD, UVPD); EThcD/ETciD = ETD + supplementare; il filtro prende l'attivazione del proprio passo. ? ETD, ECD, PQD, UVPD, MPD solo su eventi costruiti: non verificati contro RFR. ? `unknown_double` (larghezza d'isolamento dei passi non finali?) e `unknown_long2` (offset?): il comparatore li confronta con `iso_width` / `iso_offset` di RFR per livello; fino ad allora restano `raw_double` / `raw_long2`.
3. ✔ **Energia**: dal trailer per il passo finale, dalla reazione per gli altri; NCE a gradini come lista (`energy_steps`); l'unità NCE solo se il trailer la dice, eV solo nel dump per il passo finale con energia dal trailer. ? Unità dei passi non finali contro RFR.
4. ✔ **Precursori multipli**: `ReactionInfo::multiple` per MSX e multibanda (più reazioni valide dei passi del livello), `PrecursorInfo::sps_mz` e `sps_precursors` del dump per SPS-MS3, `faims_cv` dal trailer «FAIMS CV» (il filtro non porta `cv=`). ? Confronto con RFR e DIA multibanda su file reali.
5. ? **Isolamento**: larghezza del passo finale dal trailer; per i passi non finali vedi 2; offset d'isolamento non letto; la finestra dd-MS2 di `metodo/1` letta male (20000/1,5).
6. ? **Segmenti ed eventi**: numero di segmento/evento per scansione (`segment`, `event` nel dump, da confrontare con RFR), più segmenti di tempo nel metodo.
7. ✔ **Profilo/centroidi**: corretto dopo RV-02; ? centroidi «etichettati» (risoluzione, rumore, carica per picco) contro RFR.
8. ? **Lock mass**: campi del trailer letti, correzione non applicata; da confrontare con le masse di RFR.
9. ✔ **File che non si aprono**: versioni non dichiarate (tra 1 e 100: schema più vicino con avviso), file **troncati** (scansioni intere più avviso) e flussi non essenziali illeggibili (saltati con avviso) si aprono; i messaggi nominano struttura, versione e byte (`lettori-modulo.md`). Resta da confrontare con i dump RFR dei 41 file (`RAW_VELOCE.md` §2.5): non verificato contro RFR.
10. ✔ **UV/PDA 3D** (572 B per istante) non letto; ? canali analogici contro RFR.
11. ✔ **Metodo dello strumento**: euristica UTF-16, OLE non letto (RV-09). ? Testo per dispositivo contro `GetInstrumentMethod(i)`.
12. ? **Trailer, status log, error log, tune**: il dump nostro scrive trailer, status, error, sequenza e ASInfo con tutte le etichette e il comparatore li confronta chiave per chiave; ? da confrontare con RFR sul corpus (T-4 senza dump: «non verificato contro RFR»); tune per segmento non trovato nel file.
13. ✔ **MSn a 8 livelli** (#1235 in main): da riverificare su un'infusione reale con il dump.

## 3. Metodo oracolo: due dump simmetrici e un comparatore
- **Formato `scandump/1`** (JSON Lines, gzip; scritto uguale da RFR e dal nostro lettore): chiavi in inglese. Riga 1 `file` (nome logico, SHA-256, versione del formato, strumento, controller, numero di scansioni, produttore del dump e versione, tempi di apertura e di lettura in ms).
- Una riga per scansione: `n`, `rt`, `level`, `polarity`, `analyzer`, `mode` (profile/centroid), `filter`, `scan_type`, `segment`, `event`, `master`, `tic`, `bp_mz`, `bp_int`, `mz_low`, `mz_high`, `n_peaks`, `reactions[]` (`precursor`, `iso_width`, `iso_offset`, `activation`, `energy`, `energy_unit` NCE/eV, `multiple`, `supplemental`), `sps_precursors[]`, `faims_cv`, `trailer{label: value}`.
- Righe opzionali: `peaks` (m/z, intensità; per 8 scansioni scelte come in F.1, tutte con `--full`), `chromatogram` (TIC, BPC, analogici, UV), `status` (canali × tempi, campionati), `errors`, `method` (testo per dispositivo: lunghezza e SHA-256, il testo solo con `--text`), `tune`.
- Dimensione: ~0,5 KB per scansione senza trailer, ~2 KB con; BAF (56 061 scansioni) ≈ 30 MB grezzi, ≈ 4 MB gzip; con `--full` circa la dimensione del mzML.
- **Dump RFR** (solo sul Mac): lo scrive la chat locale da questa specifica, fuori da ogni repository (sorgente, DLL e pacchetto NuGet restano in una cartella locale; nessun codice che usa RFR in `mzlab` né in `mzlab-dati`).
  Chiama solo l'API pubblica documentata di RFR e mappa sui campi `scandump/1`: livello, polarità, analizzatore, modo, segmento/evento e reazioni (precursore, larghezza e offset d'isolamento, attivazione, energia, tipo di energia, multipla) dall'evento di scansione e dal filtro; `trailer` = tutte le etichette del trailer extra con il valore come testo; `tic`, `bp_*`, `mz_*`, `rt` dalle statistiche di scansione; i picchi dai centroidi del file se ci sono, altrimenti dal profilo.
  Un campo che RFR non espone si omette, mai inventato. Uscita in `mzlab-dati/scratchpad/rfr/<file>.scandump.jsonl.gz` (privato, ≤ 90 MB per file).
- **Dump nostro**: `cargo run --release -p mzlab-lettori --features thermo --example thermo_scandump -- FILE.raw [--full]`, dalle stesse funzioni che usa il modulo `thermo`. Uscite a valle: con `--downstream` aggiunge per scansione il filtro di `rawfilter.rs`, i precursori e l'attivazione come escono nell'mzML scritto, la riga del `.mzidx` (livello, precursore, RT) e il nodo d'ingresso di `msngrafo/1` (catena dei precursori, energia, attivazione): ogni uscita confrontata con RFR, non solo il lettore.
- **Comparatore**: `tools/rosina/scandump_compare.py A.jsonl.gz B.jsonl.gz [--tol hr|lr] [--out report.json]` (libreria standard, nomi in inglese): allinea per `n`, confronta campo per campo con le soglie di F.2 (RT 1e-4 min; m/z 0,5 ppm HR, 1e-3 Da LR; intensità 1e-5 relativo; testo uguale dopo normalizzazione) e scrive per campo: confronti, uguali, diversi, assenti di qua/di là, scarto massimo, primi 5 esempi. `--aggregate DIR` somma N file in una tabella campo × strumento. Un campo che RFR non dà e noi sì è «extra», non un errore.
- Test: `crates/mzlab-lettori/tests/scandump_thermo.rs` legge i dump RFR del privato (salta senza `MZLAB_DATI`), come `verita_thermo.rs`; nel pubblico solo fixture sintetiche (`scandump` scritti a mano su un `.raw` sintetico o finto) e le tabelle aggregate in `docs/agenti/misure.md` (nessun nome di file privato). Il dump RFR si converte anche in `verita/1` (`tools/rosina/verita_scandump.py`) così il banco F0-F5 esistente lo usa senza codice nuovo.

## 4. Dove battere RFR
- **Ovunque**: RFR richiede .NET (niente browser, niente iPad); il nostro gira nativo e in `.wasm`, i file non lasciano il computer.
- **Velocità**: accesso casuale per scansione (63-99 µs in `.wasm`), tabella + `.mzidx` del BAF 3,6-5,3 s; il dump registra i tempi di RFR (apertura, tutte le scansioni, una a caso) sullo stesso Mac: obiettivo uguale o meglio nativo, ≤ 1,7× nel browser.
- **Robustezza**: un file troncato o in acquisizione si apre fino all'ultima scansione intera con un avviso; versione non dichiarata letta con lo schema più vicino e avviso «non verificata»; fuzz ≥ 300 s, nessun panico (F7).
- **Etichette pulite**: per reazione attivazione (CID, HCD, ETD, EThcD, UVPD, ETciD), energia con unità (NCE/eV), NCE a gradini come lista, precursori SPS, CV FAIMS; RFR le dà sparse tra filtro, `GetScanEventForScanNumber` e trailer.
- **Campi extra**: ASInfo, error log nel tempo, status log come serie, lista d'inclusione e impostazioni dd-MS2 strutturate (RV-09, RV-10).
- **Errori utili**: chiave i18n con causa (versione, offset, troncamento a byte N, quante scansioni si leggono), «Copia diagnosi».

## 5. Pacchetti DEV (piccoli, test su sintetici; ognuno misurato col comparatore dopo T-0)
| Pacchetto | Cosa | Dopo |
|---|---|---|
| T-0 | `scandump/1` (schema in `docs/agenti/`), `thermo_scandump`, `scandump_compare.py`, `verita_scandump.py`, fixture sintetiche, test | – |
| T-1 | Catena delle reazioni: `PrecursorInfo` per ogni reazione (precursore, isolamento, energia), precursore finale nelle MSn, mzML con più `precursor`; riverifica MS8 (#1235) | T-0 |
| T-2 | Attivazione ed energia per reazione: CID/HCD/ETD/EThcD/UVPD, supplementare, NCE a gradini, NCE/eV; significato di `unknown_double`/`unknown_long2` dal confronto | T-1 |
| T-3 | Precursori multipli (SPS, MSX, DIA multibanda), CV FAIMS, segmento/evento | T-1 |
| T-4 | Trailer, status, error log, tune: tutti i campi con tipo; parità con RFR (coordinato con RV-10) | T-0 |
| T-5 | Robustezza: troncati, versioni non dichiarate, gli 11 file che non si aprono (con RV-08), messaggi | T-0 |
| T-6 | Velocità contro RFR (assorbito da T-7) | T-0 |
| T-7 | Velocità e zero perdite: criteri prima del codice; A lettore (registro dei byte, andata e ritorno, impronte, MB/s contro RFR), B ponte verso Flow sui centroidi (colonne, matrice scansione × bin, contro mzmine) | T-3, T-5 |
T-0 e T-1/T-2 sono in main. Prompt di T-3…T-7 in `docs/studi/PROMPT_LETTORE_THERMO/` (ordine e priorità nel `LEGGIMI.md`): T-3, T-4, T-5 in parallelo su file diversi (ognuno aggiunge una riga a `MODIFICHE-MZLAB.md`), T-7 (che assorbe T-6) dopo T-3 e T-5.

**Da Federico** (con metadati scritti in un `LEGGIMI` accanto): sullo stesso composto noto, un `.raw` per caso: (a) MS2 HCD a NCE 20, 35, 50 e a gradini 20/35/50; (b) MS2 CID; (c) MS3 CID→HCD e un'infusione MSn fino a MS5-MS8; (d) se lo strumento lo consente ETD/EThcD, SPS-MS3, FAIMS, DIA; (e) un'acquisizione interrotta a metà (file troncato vero). Bastano 1-3 minuti ciascuno.
**Dalla chat locale**: .NET SDK 8 sul Mac, pacchetto NuGet di RFR scaricato fuori dai repository, licenza salvata in `_licenze/`, il programma di dump scritto da §3 e tenuto fuori dai repository, dump di tutti i `.raw` di `dati/HRMS/` e `dati/formati/` (prima `--full` sui piccoli), esito per file in `coordinamento/dati_esterni/rosina/ORACOLI.md`.
