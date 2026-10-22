# Parametri di mzFlow dai segnali dei file

## 1. Principio

- Le proposte sono numeri calcolati da regole Rust deterministiche (`mzlab_llm::params_rules`, nuovo modulo sopra `mzlab-feat::params` che esiste già), ognuna con regola, segnale, prova e confidenza. Il modello non inventa valori: sceglie fra le proposte, chiede quando un segnale manca, spiega in parole, e se cambia un valore lo dice (`source: model`) con il motivo.
- Ogni proposta si applica e si toglie con un clic (carta «Apply»/«Discard» del pannello, voce di `S.undo`); nel campo del modulo resta il segno «estimated» finché l'utente non lo tocca (stessa regola di `floor` assente = stimato in `mzfeat-wasm.md`).
- Mai verdetti nel mondo LR: a risoluzione unitaria le proposte riguardano la rilevazione e l'allineamento, non l'identità dei prodotti; il profilo `corso` non ha l'assistente.
- Confidenza: `high` = misurata dai dati con abbastanza tracce (≥ 30 tracce forti, ≥ 3 QC per le derive); `medium` = dai metadati del metodo o da poche tracce; `low` = valore tipico del tipo di strumento, da confermare. Il pannello mostra `low` come domanda, non come proposta.

## 2. Contratto `proposta-param/1`

```json
{"schema":"proposta-param/1","series":"s1","proposals":[
 {"step":"features.detect@1","algorithm":"centwave","name":"peak_width_s","value":[4,20],
  "rule":"R-width-1","signals":["fwhm_p10_s=6.1","fwhm_p90_s=14.2","traces=184"],
  "confidence":"high","range_ok":true,"proof":"estimate:f3:width"}],
 "missing":[{"signal":"blank_files","asks":"Q-blank"}],
 "warnings":["warn.params.mixed_polarity"]}
```
- `value` sta sempre nei limiti del descrittore `step/1` (`range_ok` falso = la regola ha tagliato al limite e lo dice).
- `proof` rimanda alla prova salvata con la ricetta (`params.rs` scrive istogramma, punto scelto e conteggi).
- Lo stesso ingresso dà gli stessi byte (test d'oro).

## 3. Segnali che Rust misura (`ispezione/1`)

| Segnale | Da dove | Note |
|---|---|---|
| polarità per scansione | intestazioni delle scansioni, filtro Thermo, esperimento SCIEX | polarità mista nello stesso file = alternanza: serie separate |
| classe di risoluzione | modello dello strumento (`metodo/1`) e larghezza dei picchi in m/z a m/z noto | `unit` se FWHM ≥ 0,3 Da; `high` se R ≥ 10 000 |
| modo dei dati | profilo o centroidi (`cvParam`, flag del produttore, punti per picco) | profilo HR → `spectra.centroid` prima |
| tipi di acquisizione | full scan, SIM, MRM (transizioni), product ion, DDA (MS2 legate a un MS1 con precursore), DIA (finestre fisse ripetute), polarità alternata | mai dal nome del file |
| infusione | TIC piatto (coefficiente di variazione basso su tutta la corsa), nessun picco cromatografico, molte MSn dello stesso precursore | porta al piano MSn (`msn-manuale.md`) |
| tempo di ciclo MS1 | differenza mediana fra scansioni MS1 consecutive | DDA top-N lo allunga |
| finestra di isolamento | intestazioni MS2 (`isolationWindow`) o metodo | per `features.ms2_link` |
| gradiente | `metodo/1` `lc.gradiente` (tempi, %B), flusso, forno | spesso assente in mzML: allora `medium` dalla TIC |
| tempo morto | `features.estimate` (prima salita del TIC) | |
| rumore | istogramma delle intensità (valle rumore-segnale), `features.estimate` | per file e mediana di serie |
| errore di massa | dispersione in m/z delle tracce più forti, 3 sigma robusti | |
| larghezza dei picchi | FWHM delle tracce più forti, P10–P90 | in secondi |
| deriva RT | RT delle tracce presenti in tutti i QC (o in tutti i file) contro l'ordine di iniezione | serve ≥ 3 file |
| deriva di intensità | mediana delle aree delle tracce comuni nei QC | |
| ruoli | `campioni/2` (proposte dai nomi), risposte dell'utente | i nomi sono una proposta, mai una prova |
| ordine di iniezione | data di creazione del campione (`metodo/1` `file_creato`) | ora locale dello strumento, senza fuso |

## 4. Dal tipo di acquisizione al modello di piano

| Acquisizione | Modello di partenza | Note |
|---|---|---|
| full scan LR (QqQ, trappola) | `features.estimate` → `features.detect` (asse `lr`, Da) → `features.align` → `features.gapfill` → `table.filter` → `export.table_csv` | in LR nessun passo `hr` |
| full scan HR | come sopra, asse ppm, + `table.drift` se ≥ 5 QC | |
| DDA HR | come full scan HR + `features.ms2_link` → `spectra.chimeric` → `formula.ms1` → `formula.ms2` → `spectra.match` → `annot.level` | identità solo come candidati con livello |
| DDA LR (IDA del 3200) | full scan LR + `spectra.ms2` | |
| DIA | come full scan HR; l'MS2 non si lega per precursore | segnalato come limite, nessun passo inventato |
| MRM / sMRM | `quant.calibrate` solo nel profilo `ricerca`; nel mondo didattico le rette le fanno gli studenti | |
| product ion, MSn, infusione | `spectra.msn_graph` → `msn.library_export` / `msn.table_export` | |
| prodotti di trasformazione | `tp.generate` → `tp.filter` → `tp.link` (HR) o `tp.dig_lr` (LR) | serve il genitore detto dall'utente |
| polarità alternata | due serie, un piano per polarità | |

## 5. Regole dei parametri fondamentali

Valori tipici di riferimento: impostazioni documentate di XCMS centWave (`ppm`, `peakwidth`, `snthresh`, `prefilter`), della stima automatica dai dati (IPO, AutoTuner) e di mzmine (soglia di rumore, tolleranza m/z, costruttore di cromatogrammi, allineamento). Le regole usano prima i dati misurati, poi i metadati, poi i valori tipici.

| Id | Passo.parametro | Regola | Segnale | Confidenza |
|---|---|---|---|---|
| R-pol-1 | `*.polarity` | polarità unica della serie; mista → serie separate e avviso | scansioni | high |
| R-axis-1 | `features.estimate.axis`, `features.detect.axis` | `lr` se classe `unit`, altrimenti ppm | risoluzione | high |
| R-ppm-1 | `features.detect.centwave.ppm`, `features.align.ppm`, `features.gapfill.ppm` | 3 sigma robusti dell'errore di massa, arrotondato per eccesso a 0,5 ppm, minimo 2, massimo 30 | errore di massa | high se ≥ 30 tracce |
| R-ppm-2 | gli stessi, senza misura | Orbitrap 5, Q-TOF 10, sconosciuto HR 10 | modello | low |
| R-da-1 | `tp.dig_lr.tol_da`, `spectra.ms2.tol_da` | metà della FWHM in m/z misurata (tipico 0,3–0,5 Da a risoluzione unitaria) | larghezza in m/z | high |
| R-floor-1 | `features.detect.floor`, `features.gapfill.floor` | valle dell'istogramma (stima esistente) | rumore | high |
| R-height-1 | `features.detect.min_height`, `table.filter.min_height` | 10 × rumore (S/N 10), mai sotto 3 × | rumore | medium |
| R-snr-1 | `features.detect.centwave.snr` | 10 per full scan, 5 per segnali deboli chiesti dall'utente | rumore, obiettivo | medium |
| R-width-1 | `features.detect.centwave.peak_width_s` | [0,5 × P10, 2 × P90] della FWHM in secondi | larghezza | high |
| R-pts-1 | `features.detect.masstrace.min_points`, `features.gapfill.min_points` | max(3, metà dei punti per picco), punti = FWHM mediana / tempo di ciclo MS1 | larghezza, ciclo | high |
| R-gap-1 | `features.detect.masstrace.max_gap`, `features.gapfill.max_gap` | 1 se il ciclo è regolare, 2 se DDA top-N > 5 | ciclo | medium |
| R-rt-1 | `features.align.rt_gap`, `features.gapfill.rt_tol`, `features.table.rt_tol` | 3 × deriva RT robusta dei QC, minimo FWHM mediana | deriva RT | high se ≥ 3 QC |
| R-rt-2 | gli stessi, senza QC | 0,5 % della durata del gradiente, minimo 0,05 min | gradiente | medium |
| R-warp-1 | `features.align.rt_warp` | acceso se la deriva supera 2 × FWHM mediana o è monotona con l'ordine | deriva RT | medium |
| R-win-1 | finestra RT di lavoro (`tp.dig_lr.rt_min` e filtri) | da tempo morto a fine gradiente, prima del lavaggio | gradiente, tempo morto | medium |
| R-blank-1 | `table.filter.blank_ratio` | 3 se ci sono bianchi dichiarati; assente senza bianchi (domanda) | ruoli | high se dichiarati |
| R-qc-1 | `table.filter.qc_cv_max`, `qc_min_files` | 30 % e metà dei QC se ≥ 3 QC; altrimenti nessun filtro QC | ruoli | high |
| R-det-1 | `table.filter.min_detected` | metà del gruppo più piccolo (minimo 2) | ruoli, gruppi | medium |
| R-ms2-1 | `table.filter.require_ms2` | spento di default; acceso solo se l'utente vuole identificare in DDA | obiettivo | – |
| R-drift-1 | `table.drift.method`, `span` | correzione solo con ≥ 5 QC distribuiti nella serie; `span` dalla distanza fra QC | ruoli, ordine | high |
| R-iso-1 | `features.ms2_link.default_half_window` | metà della finestra di isolamento letta | metodo | high |
| R-cent-1 | `spectra.centroid.fwhm`, `threshold` | FWHM in m/z misurata; soglia = rumore dello spettro | profilo | high |
| R-form-1 | `formula.ms1.tol_ppm`, `formula.ms2.ppm_ion` | R-ppm-1 | errore di massa | high |
| R-form-2 | `formula.ms1.elements` | CHNO di default; S, P, alogeni solo se l'utente li nomina o il profilo isotopico lo suggerisce (rapporto M+2) | domanda, isotopi | medium |

## 6. Domande che l'assistente fa quando un segnale manca

- Matrice (acqua, suolo, tessuto, soluzione standard): cambia rumore atteso e filtri sui bianchi.
- Bianchi e QC: quali file, se `campioni/2` non li ha riconosciuti o ne ha proposti con confidenza bassa.
- Composti attesi: genitore (formula o m/z, mai obbligatoria la struttura), standard interni (per la deriva RT).
- Obiettivo: screening non mirato, prodotti di trasformazione, quantificazione, solo controllo qualità.
- Gruppi o condizioni (tempi, trattamenti): per `table.stats` e `min_detected`.
- Domande chiuse con opzioni (strumento `ask_user`), al più 5 per volta, mai una domanda a cui Rust sa già rispondere.

## 7. Prova delle regole

- Test d'oro per regola su `ispezione/1` sintetiche: valore, confidenza, `range_ok`, stessi byte.
- Confronto con un chimico (`06-valutazione.md` §4): per 16 casi un chimico sceglie i parametri senza vedere le proposte; metrica = quota di parametri dentro l'intervallo accettato dal chimico.
- Ogni regola nuova = una riga in questa tabella e nel file d'area quando esisterà (`docs/agenti/assistente-dati.md`).
