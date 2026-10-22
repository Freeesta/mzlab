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
1. ✔ **Un solo precursore per scansione**: `mzml.rs` (`PrecursorInfo`) usa `reactions.first()`; in una MS3…MS8 il precursore, l'isolamento e l'energia della reazione finale non escono; il filtro (`scan_filter.rs`) ha la catena, i dati strutturati no.
2. ✔ **Attivazione una per scansione** (byte 24 del preambolo, CID/HCD da `unknown_long1` della prima reazione, #1238): manca per reazione (MS3 CID→HCD, EThcD con energia supplementare, UVPD, ETD con tempo di reazione). ? significato di `unknown_double` (larghezza d'isolamento?) e `unknown_long2`.
3. ✔ **Energia**: dal trailer (un valore) o dalla prima reazione; NCE a gradini resa come media (`RAW_VELOCE.md` §2.6); eV contro NCE dedotto solo da `activation_energy_is_nce`.
4. ? **Precursori multipli**: SPS-MS3 (TMT, fino a 10-20 precursori), MSX multiplexato, finestre DIA a più bande, FAIMS (CV nel filtro).
5. ? **Isolamento**: larghezza e offset per reazione; la finestra dd-MS2 di `metodo/1` letta male (20000/1,5).
6. ? **Segmenti ed eventi**: numero di segmento/evento per scansione (RFR li dà), più segmenti di tempo nel metodo.
7. ✔ **Profilo/centroidi**: corretto dopo RV-02; ? centroidi «etichettati» (risoluzione, rumore, carica per picco) contro RFR.
8. ? **Lock mass**: campi del trailer letti, correzione non applicata; da confrontare con le masse di RFR.
9. ✔ **File che non si aprono**: 11 su 41 (`RAW_VELOCE.md` §2.5); ? versioni non dichiarate (v65, v67+, LTQ v8-v47) e file **troncati** (acquisizione interrotta): oggi errore, RFR legge fino all'ultima scansione intera.
10. ✔ **UV/PDA 3D** (572 B per istante) non letto; ? canali analogici contro RFR.
11. ✔ **Metodo dello strumento**: euristica UTF-16, OLE non letto (RV-09). ? Testo per dispositivo contro `GetInstrumentMethod(i)`.
12. ? **Trailer, status log, error log, tune**: tutti i campi per etichetta e tipo contro RFR (oggi `opentfraw.trailer.<etichetta>`).
13. ✔ **MSn a 8 livelli** (#1235 in main): da riverificare su un'infusione reale con il dump.

## 3. Metodo oracolo: due dump simmetrici e un comparatore
- **Formato `scandump/1`** (JSON Lines, gzip; scritto uguale da RFR e dal nostro lettore): chiavi in inglese. Riga 1 `file` (nome logico, SHA-256, versione del formato, strumento, controller, numero di scansioni, produttore del dump e versione, tempi di apertura e di lettura in ms).
- Una riga per scansione: `n`, `rt`, `level`, `polarity`, `analyzer`, `mode` (profile/centroid), `filter`, `scan_type`, `segment`, `event`, `master`, `tic`, `bp_mz`, `bp_int`, `mz_low`, `mz_high`, `n_peaks`, `reactions[]` (`precursor`, `iso_width`, `iso_offset`, `activation`, `energy`, `energy_unit` NCE/eV, `multiple`, `supplemental`), `sps_precursors[]`, `faims_cv`, `trailer{label: value}`.
- Righe opzionali: `peaks` (m/z, intensità; per 8 scansioni scelte come in F.1, tutte con `--full`), `chromatogram` (TIC, BPC, analogici, UV), `status` (canali × tempi, campionati), `errors`, `method` (testo per dispositivo: lunghezza e SHA-256, il testo solo con `--text`), `tune`.
- Dimensione: ~0,5 KB per scansione senza trailer, ~2 KB con; BAF (56 061 scansioni) ≈ 30 MB grezzi, ≈ 4 MB gzip; con `--full` circa la dimensione del mzML.
- **Dump RFR** (locale, Mac): un piccolo programma .NET nostro (`rfr-dump`), sorgente in `mzlab-dati/strumenti/rfr-dump/` senza DLL né pacchetto (`.gitignore` su `bin/`, `obj/`, `*.nupkg`; il riferimento NuGet si scarica sul Mac); chiama solo l'API pubblica documentata di RFR. Uscita in `mzlab-dati/scratchpad/rfr/<file>.scandump.jsonl.gz` (privato, file ≤ 90 MB).
- **Dump nostro**: `cargo run --release -p mzlab-lettori --features thermo --example thermo_scandump -- FILE.raw [--full]`, dalle stesse funzioni che usa il modulo `thermo`.
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
| T-6 | Velocità contro RFR: tabella in `misure.md`, interventi solo se sotto | T-0 |
T-0 è l'unico prerequisito; poi T-1→T-2 in serie (stesso `scan_event.rs`/`mzml.rs`), T-3, T-4, T-5 in parallelo su file diversi (ognuno aggiunge una riga a `MODIFICHE-MZLAB.md`).

**Da Federico** (con metadati scritti in un `LEGGIMI` accanto): sullo stesso composto noto, un `.raw` per caso: (a) MS2 HCD a NCE 20, 35, 50 e a gradini 20/35/50; (b) MS2 CID; (c) MS3 CID→HCD e un'infusione MSn fino a MS5-MS8; (d) se lo strumento lo consente ETD/EThcD, SPS-MS3, FAIMS, DIA; (e) un'acquisizione interrotta a metà (file troncato vero). Bastano 1-3 minuti ciascuno.
**Dalla chat locale**: .NET SDK 8 sul Mac, pacchetto NuGet di RFR scaricato fuori dai repository, licenza salvata in `_licenze/`, build di `rfr-dump`, dump di tutti i `.raw` di `dati/HRMS/` e `dati/formati/` (prima `--full` sui piccoli), esito per file in `coordinamento/dati_esterni/rosina/ORACOLI.md`.
