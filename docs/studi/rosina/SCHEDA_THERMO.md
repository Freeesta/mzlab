# Rosina C: Thermo `.raw`

Legenda e indice: `SCHEDE.md`. Lettore: modulo `thermo` (`opentfraw` adattato), `metodo_raw.rs` e sottomoduli; dettagli in `docs/agenti/metodo.md` e `docs/agenti/lettura-nativa.md`.

## Stato
- Spettri e cromatogrammi L su Orbitrap/Q Exactive/Exploris (parità con l'mzML di msconvert sui file del corpus che lo hanno); metodo L (seriale e versione uguali a msconvert, gradiente uguale allo script).
- P: TSQ Vantage/Quantum (possono non aprirsi in `opentfraw`), trappole LTQ/LCQ/Velos, Orbitrap Tribrid DIA multipla, Astral (non nel corpus), PDA 3D (non decodificato), versioni del formato v57-v66 non contate.
- Quota: 81 % dei progetti PRIDE, 47 % degli studi MS MetaboLights, 51 % delle analisi Workbench. È il formato più usato nel mondo: ogni difetto qui pesa più che altrove.

## Cosa manca
1. V1 automatico: TRFP (RawFileReader) dà spettri e metadati JSON; oggi la parità è solo contro mzML di msconvert presenti nel corpus.
2. Famiglie senza riferimento: TSQ (PXD043691, ELENCO-1), trappole (corpus), Exploris con FAIMS (nessun file), Astral (nessun file piccolo).
3. PDA/UV: le tracce a una lunghezza d'onda sono lette, il campo 3D no (572 byte per istante, layout ignoto). Prova: MTBLS2956 (ELENCO-1) con l'export di TRFP? TRFP non esporta il PDA: solo V3 (FreeStyle) o lettore libero (`rawrr` no).
4. Tune data e mass calibration (coefficienti per scansione) non in `metodo/1`.
5. Dati dei picchi dei centroidi (rumore, baseline, risoluzione, carica per picco: «label data») non esportati.

## Prove possibili
- V1: TRFP sul Mac (`-f 2` mzML, `-m 0` metadati JSON, `-L` livelli) e msconvert in Docker; ripetibile, gratuito, licenza RawFileReader (uso consentito, niente reverse engineering della libreria: si confrontano solo le uscite).
- V2: PXD con mzML degli autori (rari). V3: FreeStyle/Xcalibur se il laboratorio ne ha una copia. V4: nessuno indipendente (`opentfraw` è la nostra base).
- Prova minima per «L» per famiglia: F0-F5 su 1 file Orbitrap, 1 Q Exactive, 1 TSQ, 1 trappola, 1 Tribrid, 1 con PDA.

## Metadati campo per campo
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| Instrument Model, Name, Serial Number, Software Version, Hardware Version | blocco InstID (stringhe 1-5) | `strumento.modello`, `.serie`, `.software`, `.hardware` | L (posizioni misurate su v66, ? altre versioni: R-02) | FreeStyle «Instrument» · TRFP `InstrumentModel`, `SerialNumber`, `SoftwareVersion`; mzML `instrumentConfiguration` |
| File version, creation date, file creator | FileHeader | `origine.file_creato`, `campi` | L | TRFP `CreationDate`; mzML `run@startTimeStamp` |
| Operator (audit tag), computer | AuditStart `tag2`, nome del computer | `operatore.utente`, `.computer` (identita) | L | TRFP `Operator` |
| Sample Name, Sample ID, Vial, Inj Vol, Dilution, Sample Wt, Sample Vol, ISTD Amt, Comment, Instrument Method, Processing Method, Calibration File | riga di sequenza (SeqRow) | `campione.*` | L | FreeStyle «Sample Information» · TRFP `SampleData` |
| Sample Type, Row Number, Level Name, User Text 0-4, Path, Raw File Name | riga di sequenza | `campi` | ? (R-02: elencare quali escono) | FreeStyle · TRFP |
| Start/End Time, Low/High Mass, number of scans, expected run time, mass resolution, tolerance unit | RunHeader | `campi` | L | TRFP `StartTime`, `EndTime`, `NumberOfMs1Scans` |
| Instrument method (testo per dispositivo: MS, LC, autocampionatore) | contenitore OLE incorporato (`Text`, `Data`, `Header`, `Comments`) | `grezzo`, `lc`, `campionatore`, `ms.impostazioni[]`, `ms.periodi[0].esperimenti[]` | L | FreeStyle «Instrument Method» · TRFP `-m` (method text) |
| Lista di inclusione/esclusione | tabella dopo `INCLUSION LIST` del testo | `ms.periodi[].esperimenti[].inclusione[]` | L (prova su testo vero: salta senza privato) | FreeStyle |
| Tabella SRM del TSQ (Q1, Q3, CE, RF/Tube Lens, Dwell, tempi, polarità) | testo del metodo MS | `transizioni[]` | L; troncamento dichiarato (`meta.nonDecodificato`) | TSQ Tune/FreeStyle · mzML `SRM SIC` |
| Scan filter per scansione | ScanEvent + filtro testuale | scansioni (`filterString`) | L | TRFP `filter string`; mzML `MS:1000512` |
| MS order, polarity, scan mode, analyzer, ionization, activation, CE, isolation width e offset, multiplex, SPS | ScanEvent | scansioni; esperimenti dall'XML | L (SPS/MS3 ?: R-02) | mzML `precursor`, `activation`; TRFP |
| Trailer extra (62 campi: Ion Injection Time, AGC Target, Monoisotopic M/Z, Charge State, Master Scan Number, FT Resolution, LM Correction (ppm), Micro Scan Count, HCD Energy, FAIMS CV, Scan Description…) | trailer per scansione | `campi` (valore o intervallo); alcuni nelle scansioni | L | FreeStyle «Scan Header» · TRFP `-L` · mzML (injection time, monoisotopic) |
| Status log (pressioni, temperature, tensioni, flussi per canale e tempo) | status log | `campi` (`thermo-raw:StatusLog`), `serie_registro`, `/api/registro` | L | FreeStyle «Status Log» · TRFP? (non lo esporta: V3) |
| Error log | error log | `registro[]` | L | FreeStyle «Error Log» |
| Tune data (Spray Voltage, Capillary Temp, Sheath/Aux/Sweep Gas, S-Lens RF, Heater Temp…) | tune header/tune data per segmento | — | N (R-02) | FreeStyle «Tune Data» · RawFileReader `GetTuneData` (via TRFP? no: V3) |
| Lock mass: modo, elenchi, numero nel trailer, correzione | testo del metodo + trailer | `ms.calibrazione.lock_mass` | L (solo descrizione, nessun dato corretto: giusto, lo strumento la applica già) | FreeStyle |
| Coefficienti di taratura di massa per scansione (A, B, C…) | scan header (conversion params) | — | N (usati per gli m/z, non esposti) | nessun oracolo diretto: implicito negli m/z (F3) |
| Canali analogici e UV a una λ (nome, unità, campioni, driver, modello, seriale) | controller `Controller<N>` | cromatogrammi analogici/UV, `campi` | L | FreeStyle · TRFP? (no) · mzML (msconvert li scrive come cromatogrammi) |
| PDA 3D (spettri UV per istante) | controller PDA (572 B per istante) | — (`avvisi` «UV 3D field») | N | FreeStyle; nessun oracolo automatico |
| Rivelatori UV (lunghezze d'onda, banda, riferimento, campo 3D, velocità) | script Chromeleon | `rivelatori[]` | L | FreeStyle «Instrument Method» |
| Dati dei centroidi per picco (rumore, baseline, risoluzione, carica) | centroid stream / label data | — | N | TRFP `-N` (noise) · RawFileReader `GetCentroidStream` |
| Profilo e centroidi dello strumento | scan data + centroid stream | spettri (`profilo`) | L | TRFP `-p` (no peak picking) contro default |
| Sequence/batch (`.sld`) | file separato, non nel `.raw` | — | fuori: solo la riga del campione | Xcalibur Sequence |

## Lavoro, rischio, priorità
- **R-02**: 6-8 giorni. Verità TRFP per ogni famiglia (LOC-2), tune data, SPS/MS3, campi della sequenza mancanti, TSQ Quantum/Vantage (aprire o rifiutare con la chiave giusta), versioni del formato contate sul corpus, PDA 3D solo se un file con export noto esiste (altrimenti `avviso` resta).
- **Rischio tecnico**: medio (varianti di versione). **Legale**: basso (formato osservato; RawFileReader usato solo come oracolo, la sua licenza vieta di decompilarla, non di confrontare le uscite).
- **Priorità**: 1. Tetto del modulo `thermo`: 540 672 B, misura 523 597 B: il nuovo codice deve stare nei 17 KB liberi o andare in un modulo `thermo-ext` (feature `estesi`) proposto dalla chat.
