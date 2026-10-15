# Studio: lettura nativa di tutti i formati di spettrometria di massa

Obiettivo: mzLab legge così come sono, senza conversione manuale in mzML, i file di ogni produttore, a bassa e ad alta risoluzione, con ogni tipo di esperimento. Questo studio dice cosa si legge oggi (con la prova nel codice), cosa manca, cosa si può fare nel browser e in che ordine. Base: `main` a `e450dce`.

## 0. Il vincolo che decide tutto: il peso del motore

- Il `.wasm` del motore pesa **1 703 607 B** (log del job `build` di `pages.yml`, run 38642852060, commit `e450dce`: «engine .wasm (optimised): 1703607 bytes»). Il tetto è **1 703 936 B** (`.github/workflows/pages.yml:61`, `tools/wasm_opt.sh`). **Margine: 329 byte.** Nessun lettore nuovo, nemmeno il più piccolo, entra nel motore così com'è.
- Per confronto, il codice dei lettori che il motore contiene già: Thermo `opentfraw` 8 245 righe (`vendor/opentfraw/src/`) + 1 319 righe di `rawlettore.rs`/`raw.rs`/`metodo_raw*`; Agilent 2 904 righe (`crates/mzlab-core/src/agilent/`); SCIEX 901 righe (`wiff/`) + 384 (`ole.rs`).
- Gli altri `.wasm` del sito hanno ciascuno il loro tetto e si caricano a richiesta (librerie 392 295 B su 614 400, `pages.yml:66`; mzFinder: `finder` 506 802 B, `finder-lr` 598 107 B, ecc.). I **moduli lettori separati** seguono quindi un modello che il sito usa già.
- Conseguenza: i lettori nuovi vivono in **moduli `.wasm` separati, uno per famiglia di formato** (decisione di Federico), caricati solo quando un file della famiglia arriva; ognuno scrive l'mzML del campione in OPFS e il motore lo legge come ogni mzML. Il motore non cresce di un byte. Architettura in §4.
- Misure di questo studio (`cargo build --release --target wasm32-unknown-unknown`, poi `wasm-opt -Oz` 1.4.0 senza `wasm-bindgen`, quindi valori relativi): un modulo con un solo lettore del motore più lo scrittore mzML pesa 344 574 B (Thermo), 258 608 B (Agilent), 217 906 B (SCIEX). Togliere `rawlettore::VOCE` da `LETTORI` riduce il motore da 2 024 460 a 1 868 164 B: **Thermo occupa circa 156 KB del motore**, il 7,7 %.

## 1. Inventario: cosa si legge oggi

Ingresso della pagina: `upload0` (`mzlab/web/shell.js`) passa prima per `wiffToMzml` e `rawToMzml` (i campioni nativi), poi `fileKind` (`shell.js:170`) lascia passare solo `.mzml` e `.dam`; `UNOPEN_RE = /\.(raw|wiff2?|wiff\.scan|d|lcd|lcdx)$/i` (`shell.js:169`) dà l'avviso «non si può aprire nel browser» con la guida di conversione (`conv.help.step1`); ogni altra estensione è «formato non riconosciuto» (`unopen.other`). Con il motore Rust pronto `CONVERT_RAW=true` (`shell.js:145`) e la guida resta solo per ciò che il motore non legge.

Registro dei lettori nativi: `LETTORI` in `crates/mzlab-core/src/lettori.rs:218` = `agilent-d`, `sciex-wiff`, `thermo-raw`. Contratto `Lettore` in `docs/agenti/estendere.md`; flusso nel sito in `docs/agenti/lettura-nativa.md` (`crates/mzlab-wasm/src/nativi.rs`: `native_samples`, `native_attach`, `build_mzidx_native`, `native_srm`, `native_to_mzml`).

| Formato | Stato | Parser | Cosa legge | Prove (test) e file reali |
|---|---|---|---|---|
| **mzML** | letto | Rust `crates/mzlab-core/src/mzml.rs` (`read`, `parse`), testata `mzmlhead.rs`, indice `.mzidx` (`mzidx.rs`, `docs/agenti/mzidx.md`) | spettri (livello, RT, polarità, precursore, finestra di isolamento `MS:1000827`, attivazione `scantable.rs:123-128`), m/z f64, intensità f32, base64 + zlib; cromatogrammi di ogni `ChromKind` (`mzml.rs:44`). **Numpress rifiutato**: `MS:1002312-14` → `err.file.numpress` (`mzml.rs:412-413`) | `mzidx_parity.rs`, `robustezza_mzml.rs`, `mzmlhead_parity.rs`, `golden.rs`; mzML di msconvert dei file del laboratorio (`mzlab-dati/mzML/`, `HRMS/`) |
| **Thermo `.raw`** (file unico, v8-v66) | letto dal modulo `thermo` (il motore del sito lo chiama con il lettore esterno; il desktop lo ha nel core) | Rust `vendor/opentfraw` 2.0.0 (Apache-2.0, `MODIFICHE-MZLAB.md`) + `rawlettore.rs` (riconosce dai byte, `:25-34`) | spettri di ogni livello (profilo/centroidi come nel file), filtro, precursore, trailer, rumore FT, `metodo/1`, pressione nell'mzML scritto. **Cromatogrammi del lettore: solo TIC** (`rawlettore.rs:222-248`). PDA/UV: controller `Pda`/`Uv` noti (`opentfraw/src/reader.rs:386-387`), non letti | `raw_lettore.rs`, `raw_parity.rs`, `raw_profile_parity.rs`, `metodo_raw_parity.rs`; solo `.raw` Q Exactive di `mzlab-dati/HRMS/`, nessun TSQ |
| **SCIEX `.wiff` + `.wiff.scan`** | letto (3200 QTRAP nel motore; altri QTRAP nel modulo `sciex`) | Rust `crates/mzlab-core/src/wiff/` + `ole.rs`, scritto da zero (`docs/agenti/wiff.md`) | full scan (profilo), prodotto di ione (MS2, codice 11), MRM (un f32 per transizione → cromatogrammi SRM), polarità dal segno di `IS`, metodo, più campioni per file | `wiff_parity.rs` (sintetici di `tools/wiff_riferimento.py` + 23 mzML veri di `3200 qtrap/wiff inquinanti 25-26 B/`), `wiff_lettore.rs`, `tests_e2e/e2e_wiff.py` |
| **SCIEX `.wiff2`** | assente | — | — | in `UNOPEN_RE` (guida di conversione) |
| **SCIEX `.dam`** (metodo, non dati) | letto | `metodo_wiff*`, `dam_parity.rs` | metodo di acquisizione | `dam_parity.rs`; `mzlab-dati/dam/` |
| **Agilent MassHunter `.D`** | letto (solo GC/MS 5977 SQ) | Rust `crates/mzlab-core/src/agilent/` (`agilent-d`) | `MSScan.bin` + `MSPeak.bin` (centroidi f64, formato 1), scansione e SIM, SIC per ione SIM, TIC/BPC, metodo GC, file laterali; `data.ms` ChemStation (`datams.rs`) come riserva | `agilent_*.rs`, `robustezza_agilent.rs`, `tests_e2e/e2e_agilent.py`, `e2e_lettori.py`; 3 corse vere 5977. Altre varianti → `err.file.agilent` (`agilent.md`, «Non verificato») |
| **Agilent ChemStation `.D`** (`DATA.MS` GC/MS) | parziale | `agilent/datams.rs` | solo la variante `01 32` delle tre corse 5977; m/z in ventesimi | stessi test Agilent |
| **mzXML** | assente | — | solo nell'elenco delle estensioni di progetto (`crates/mzlab-core/src/project.rs:17`) e nei nomi base (`tabella.js:13`, `campioni.js:50`); nessun parser | — |
| **mzData** | assente | — | — | — |
| **netCDF / ANDI-MS (`.cdf`)** | assente | — | solo estensione in `project.rs:17` | — |
| **MGF / MSP / JSON / `.mzlib`** | letto **come libreria**, non come corsa | Rust `crates/mzlab-lib/src/parse.rs` (`detect`, `parse_all`) + `libreria-worker.js` (`formatOf`) | spettri di riferimento (precursore, polarità, metadati); `.lib`/`.mzvault`/`sqlite`/NIST → `lib.err.unsupported` (`libreria-worker.js:93`) | `crates/mzlab-lib/tests/parse.rs`; nessun MGF trattato come file di dati della vista Dati |
| **Waters `.raw` (cartella MassLynx)** | assente | — | guida di conversione | — |
| **Bruker `.d`** (TDF/TSF timsTOF, BAF) | assente | — | `.d` va al lettore Agilent, che lo rifiuta senza `MSScan.bin` | — |
| **Shimadzu `.lcd` / `.qgd`** | assente | — | `.lcd`/`.lcdx` in `UNOPEN_RE`; `.qgd` «non riconosciuto» | — |
| **JEOL, LECO (`.peg`, `.smp`), PerkinElmer, Bruker ion trap (`.yep`)** | assente | — | — | — |

Desktop (`crates/mzlab-desktop`): stessi lettori nativi, compilati senza tetto di peso (`docs/agenti/rust-desktop.md`).

## 2. Matrice formati × esperimenti

Legenda: **L** letto e provato · **P** parziale (si legge, ma manca un pezzo detto nella cella) · **A** assente · **NV** il codice c'è ma nessun file vero l'ha provato · — non esiste nel formato. Le colonne dei formati assenti (Waters, Bruker, Shimadzu, mzXML, CDF) sono tutte **A** e non sono ripetute.

| Esperimento | mzML | Thermo `.raw` | SCIEX `.wiff` (3200) | Agilent `.D` |
|---|---|---|---|---|
| Full scan LR | L (`mzidx_parity.rs`) | NV (trappola ionica: codice c'è, nessun file) | L (`wiff_parity.rs`) | L (5977, `agilent_msconvert.rs`) |
| Full scan HR | L | L (Q Exactive, `raw_parity.rs`) | — | NV (Q-TOF: `agilent.md` «Non verificato», profili non localizzati) |
| SIM | L (filtro `sim`, `iteminfo.rs:284-290`) | NV (filtro letto, nessun file) | — | L (+ SIC derivato) |
| SRM/MRM | L (cromatogrammi SRM, `srm.rs::da_voci`) | P: le scansioni SRM si leggono come spettri, ma `cromatogrammi` dà solo TIC (`rawlettore.rs:222`) → nessuna traccia SRM per mzFinder/MRM | L (`wiff/run.rs`) | A (`agilent.md`: «i cromatogrammi SRM no») |
| Product ion (MS2) | L | L | L (codice 11) | NV (`MSLevel`, `MzOfInterest` nello schema) |
| Precursor ion / neutral loss | P: letti come spettri, il tipo di scansione (MS:1000341, MS:1000326) non è riconosciuto (nessuna occorrenza nel codice) | NV | L nel modulo `sciex` (codici 5 e 7, `MS:1000341` / `MS:1000326`, parità con msconvert su 3594 spettri; `wiff-varianti.md`) | A |
| EPI / MS3 su QTRAP | L (come MSn) | — | A (non visti, letti come full scan) | — |
| DDA | L (`/api/dda`, `hrroutes.rs:61`) | L | — | NV |
| DIA / SWATH / MSe / AIF | P: classificati (`iteminfo.rs:278-325`: DIA, AIF), nessuna deconvoluzione dedicata | L (eventi DIA dei Tribrid corretti, `MODIFICHE-MZLAB.md`) | — | NV |
| PRM | P: classificato (`iteminfo.rs:319`) | L come MS2 | — | — |
| Mobilità ionica / PASEF | A (nessun array di mobilità letto in `mzml.rs`) | — | — | A (campo nello schema, mai letto) |
| MSn (n ≥ 3) | L (`msnalbero.rs`, `msn_albero.rs`) | L | A | — |
| UV/PDA | P: solo cromatogrammi PDA (`mzml.rs:280-281`), non gli spettri UV 3D | A (controller `Pda`/`Uv` elencati, non letti) | A (flussi `DAD*`, `grezzi-non-estratti.md`) | A |
| TIC / BPC / XIC | L | L (TIC dal file; XIC dal `.mzidx`) | L (TIC dell'indice) | L (TIC/BPC dai record) |
| Tracce accessorie (pressione, analogici) | L (`analog.rs`) | P: pressione solo nell'mzML scritto, non come cromatogramma del lettore | A | A (`GC.ini` come metadati) |
| Calibrazione / lock mass | — (già applicata) | P: campi del trailer (`opentfraw.lock_mass_*`, `extra.rs:122-125`), nessuna correzione | A (`GLPTables`) | A |
| Polarità mista | L (`iteminfo.rs`: parti «mixed») | L | L | NV (codice 1 mai visto) |
| Metodo e metadati (`metodo/1`) | P (solo testata; `metodo/1` solo se scritto da mzLab) | L | P (nome, vial, calibrazione mancanti) | L per GC/MS, A per LC/MS |

### Metodo e metadati (colonna «metodo/metadati» per formato)

Documento unico `metodo/1` (`docs/agenti/metodo.md`, schema `mzlab/web/schemi/metodo-1.json`, `crates/mzlab-core/src/metodo.rs`): strumento, origine, campione, operatore (campi `identita`, mai copiati nell'mzML), `ms` (sorgente, periodi, esperimenti, transizioni), `lc`, campionatore, rivelatori, `campi`, `grezzo`, `avvisi`.

| Formato | Metodo/metadati oggi | Lacune |
|---|---|---|
| mzML | testata letta (`mzmlhead.rs:322-432`: strumento, seriale, componenti, analizzatori, `sourceFile` con SHA-1, software); parametri per scansione (`scanparams.rs`: finestra, risoluzione, tempo d'iniezione, attivazione, CE, isolamento); `metodo/1` solo se l'mzML l'ha scritto mzLab (`userParam mzLab metodo/1`, `from_userparam`) | un mzML di altri non ha `metodo/1` (`metodo.md`: «Senza di esso l'mzML non ha metodo»): `startTimeStamp`, `sample`, `scanSettings`, sorgente non entrano nel documento (DEV-11) |
| Thermo `.raw` | `metodo/1` completo (`metodo_raw.rs`, `metodo_raw/script.rs`, `campi.rs`): riga di sequenza (vial, volume, id, diluizione, ISTD, file del metodo), InstID (modello, seriale, versioni), testo del metodo, gradiente dallo script, trailer, status log, tune | liste di inclusione, finestre DIA e transizioni del metodo solo nel testo (`grezzo`), non strutturate in `ms.periodi`; lock mass solo come campo (DEV-2) |
| SCIEX `.wiff` | `from_wiff` (`metodo_wiff.rs:200`): Log («Config Table»: modello, seriale), utente e computer, pompe, campionatore, inizio/fine; esperimenti e transizioni da `methodinfo.rs` | `vial` «DA CONFERMARE» (`metodo.md`; nome e versione di Analyst confermati); `GLPTables`, `DabsInfo`, `SampleTable` e intestazione del DAD in `campi` (modulo `sciex`); `MSConfigInfo*`, `MassSpecMethod*`, valori del DAD non decodificati (`grezzi-non-estratti.md`) |
| SCIEX `.dam` | `from_dam` | — |
| Agilent `.D` GC/MS | `agilent/doc.rs` (`AgilentD::metodo`): `sequence.xml`, `Devices.xml`, `Contents.xml`, `acqmeth.txt` (gruppi SIM, tune, forno, iniettore, colonna), `GC.ini`; cartella `.m` di quantificazione (`agilent/quant.rs`) | tipo di campione, `7890Method.bin`, audit trail; metodo LC/MS (QqQ dMRM, Q-TOF: masse di riferimento, liste) mai visto (DEV-4) |
| formati assenti | — | ogni lettore nuovo nasce con `metodo(c)` = `metodo/1` (vedi prompt) |

## 3. Le lacune: fattibilità nel browser

Criteri: formato aperto/documentato; esiste un lettore libero (licenza compatibile, Apache-2.0/MIT) da usare come riferimento del formato; serve l'SDK del produttore; peso stimato nel `.wasm`; file reali necessari; priorità per un laboratorio chimico (inquinanti, metaboliti, GC/MS, LC-QqQ).

Riferimenti liberi verificati (2026-10): la famiglia Sigilweaver/OpenMassSpec (Apache-2.0, Rust puro, già fonte di `opentfraw`): `openwraw` (Waters, 4.0.0), `openaraw` (Agilent), `opensxraw` (SCIEX `.wiff`), `opentimstdf` (Bruker TDF), `openszraw` (Shimadzu, 0.3.0). Le loro pagine dichiarano limiti forti (sotto). Per la lettura di un formato nell'UE, osservare il file e scriverne un lettore interoperabile è il metodo già usato per `.wiff` e `.D` (codice scritto da zero, confronto con msconvert); decompilare il software del produttore no.

| Lacuna | Aperto? | Riferimento libero | SDK necessario? | Peso wasm stimato | File reali | Rischio | Priorità |
|---|---|---|---|---|---|---|---|
| **mzML con numpress** | sì (specifica PSI) | ProteoWizard (Apache-2.0) | no | 3-6 KB | un mzML numpress di msconvert (`--numpressAll`) | basso | **alta** (msconvert lo produce su richiesta: oggi errore) |
| **mzXML** | sì (ISB, XML + base64, zlib) | ProteoWizard | no | 10-20 KB (riusa `xmltree`, base64, zlib del motore) | 2 mzXML (LR e HR) + mzML degli stessi | basso | **alta** (molti dati vecchi e GNPS/MassIVE) |
| **netCDF/ANDI-MS** | sì (ASTM E1947/E2077, netCDF classico) | specifica pubblica | no | 15-25 KB (parser netCDF-3 minimo, niente HDF5) | 2-3 `.cdf` esportati (ChemStation, Shimadzu GCMSsolution, LECO) | basso | **alta** per GC/MS (è l'uscita comune di JEOL, LECO, Shimadzu, PerkinElmer) |
| **mzData** | sì (PSI, obsoleto) | ProteoWizard | no | 8-12 KB | 1 file | basso | bassa |
| **MGF come corsa** (DDA esportata) | sì | — | no | 0 nel motore se passa da `mzlab-lib` | 1 MGF di DDA | basso | media |
| **Thermo SRM (TSQ)** | lettore presente | `opentfraw` | no | 1-3 KB (cromatogrammi SRM dalle scansioni) | 1 `.raw` TSQ MRM + mzML msconvert | basso | **alta** (QqQ Thermo frequenti nei laboratori di inquinanti) |
| **Thermo PDA/UV** | lettore presente | `opentfraw` (`ControllerType::Pda/Uv`) | no | 5-10 KB | 1 `.raw` con PDA + mzML msconvert | medio | media |
| **SCIEX `.wiff` altri strumenti** (QTRAP 4500/5500/6500, TripleTOF, ZenoTOF, SWATH, EPI, MS3, precursori, perdita neutra, più periodi) | no, osservato | `opensxraw` (dichiara m/z TOF inaffidabili, QTRAP «raw detector bins», MRM senza Q1/Q3) | no, ma reverse engineering lungo | 5-20 KB | per ogni strumento/esperimento un `.wiff` + mzML msconvert | **alto** (il riferimento libero stesso non è affidabile) | alta per QTRAP (stesso laboratorio), media per TOF |
| **SCIEX `.wiff2`** | no: cifrato (`opensxraw`: rifiutato) | nessuno | **sì: impossibile senza SDK SCIEX** | — | — | — | — |
| **Agilent LC/MS QqQ (MRM), Q-TOF profilo/centroide** | no, osservato; schema `MSScan.xsd` nella cartella | `openaraw` (QQQ 6410, Q-TOF 6530/40/50; MRM senza precursore né SRM) | no | 10-25 KB (compressione dei profili sconosciuta) | 1 `.D` QqQ MRM, 1 Q-TOF profilo, 1 centroide, ciascuno + mzML | medio | **alta** (6470/6495 e 6545 diffusi) |
| **Agilent DAD/UV (`*.ch`, `*.uv`)** | no, osservato | progetti liberi (es. rainbow) | no | 5-10 KB | 1 `.D` LC-DAD | medio | media |
| **Agilent 6560 IMS** | no | nessuno (`openaraw`: non coperto) | probabilmente sì per la calibrazione CCS | — | — | alto | bassa |
| **Waters `.raw` MassLynx** (Xevo TQ, G2-XS, Synapt) | no, osservato | `openwraw` (Rust puro; Q-Tof/Synapt/Vion; MSe, HDMSe, DDA; m/z senza lock mass 30-100 ppm; mobilità senza tempo di deriva; TQ non provato) | no per leggere; la correzione lock mass si rifà dai dati della funzione lock mass | 30-60 KB | 1 cartella TQ MRM, 1 QTof MSe, 1 con lock mass, ciascuna + mzML msconvert | medio-alto | **alta** (Xevo TQ molto diffuso) |
| **Bruker TDF/TSF (timsTOF)** | parzialmente: `analysis.tdf` è SQLite (formato aperto), `analysis.tdf_bin` compresso zstd | `opentimstdf` (usa `rusqlite` + `zstd`), timsrust | no per leggere; la calibrazione TIMS segue il modello del file | 60-120 KB con un lettore SQLite minimo in sola lettura e un decodificatore zstd; **SQLite compilato intero (~600 KB+) non entra** | 1 TDF DDA-PASEF, 1 diaPASEF, 1 TSF | medio | media (proteomica/metabolomica, meno in laboratorio di inquinanti) |
| **Bruker BAF** (micrOTOF, maXis, impact, solarix) | no | nessuno libero noto | **sì nella pratica: solo Bruker baf2sql/SDK** | — | — | alto | media |
| **Bruker ion trap `.yep`, FT-ICR `fid`** | no | nessuno affidabile | probabilmente sì | — | — | alto | bassa |
| **Shimadzu `.qgd` GC/MS, `.lcd` LCMS-2020/9030** | no, OLE2 (CFBF, già letto da `ole.rs`) | `openszraw` 0.3.0 (QTOF m/z distorti, polarità mai, niente TIC, QqQ 8040/50/60 rifiutati) | no, ma il riferimento libero è immaturo | 15-30 KB (riusa `ole.rs`) | per ogni strumento 1 file + mzML/CDF esportato | **alto** | media (GC/MS Shimadzu diffuso; per l'LC il QqQ è il caso più comune ed è il meno coperto) |
| **JEOL, LECO, PerkinElmer** | no | nessuno | sì | — | — | — | via netCDF/ANDI (già una uscita standard dei loro software) |
| **Mobilità ionica in mzML** | sì (CV PSI) | — | no | 5-10 KB + interfaccia | mzML con array di mobilità | medio (la vista Dati non ha l'asse) | bassa |

Impossibili oggi senza SDK del produttore: SCIEX `.wiff2` (cifrato), Bruker BAF, formati proprietari JEOL/LECO/PerkinElmer (per loro la strada è l'esportazione ANDI/netCDF, che il programma leggerà), calibrazioni CCS proprietarie. L'SDK di un produttore non entra nel sito: è codice Windows/.NET con licenza propria, non compilabile in `wasm32` né ridistribuibile su GitHub Pages.

## 4. Architettura decisa (DEV-0) e piano a pacchetti

### Un modulo per famiglia
- **Famiglie e moduli**: `aperti` (mzML numpress, mzXML, netCDF/ANDI), `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`. Un file usa di solito una sola famiglia: la pagina carica solo il modulo che serve, alla prima richiesta, e lo tiene per la sessione.
- **Crate `crates/mzlab-lettori`** (libreria, dipende da `mzlab-core`): la base comune (`registro()` composto dalle feature accese, l'adattatore `SrmDaScansioni` che fa delle scansioni SRM i cromatogrammi `SRM SIC Q1= Q3=` come `wiff/run.rs`, l'interfaccia di sessione) e una cartella per famiglia, ciascuna dietro una feature Cargo con il nome della famiglia. Riconoscimento con `lettori::riconosci_in`, scrittura con `lettori_mzml::scrivi` (scrittore unico, `testata_mzml`, `metodo/1` nel `<run>`).
- **Crate `crates/mzlab-lettori-wasm`** (ponte `wasm-bindgen` come `mzlab-lib-wasm`, stesse feature): `estensioni()`, `probe(nomi, dimensioni, read)` → `{formato, campioni, motivi}`, `to_mzml(...)` a pezzi di ~64 KB (contratto di `native_to_mzml`, `crates/mzlab-wasm/src/nativi.rs:275`) e l'interfaccia di sessione (`apri`, `indice`, `scansione`, `cromatogramma`, `intestazione`, `metodo`) per il lettore esterno del motore (DEV-1).
- **Varianti dei lettori che stanno nel motore** (Agilent LC/MS, `.wiff` di altri strumenti, numpress): il codice sta in `mzlab-core` dietro la feature **`estesi`**, spenta per `mzlab-wasm` e accesa dai moduli; il motore resta identico (stesso SHA-256 del `.wasm`).
- **Costruzione e tetti**: `tools/build_lettori_wasm.sh`, chiamato in fondo a `tools/build_feat_wasm.sh` (che `pages.yml:68`, `release.yml` e `sito-rust.yml` già eseguono: nessun workflow cambia), costruisce ogni famiglia con `--no-default-features --features <famiglia>` in `site/static/wasm-lettori/<famiglia>/`, la passa a `tools/wasm_opt.sh` con il suo tetto e scrive `wasm-lettori/manifest.json` (SHA-256 e byte per famiglia).
- `tests/test_lettori_wasm.py` (pytest) tiene i tetti, controlla che `mzlab-wasm` non dipenda da `mzlab-lettori` né accenda `estesi`, e misura ogni `.wasm` se `site/` esiste.
- **Tetti proposti** (le misure di §0 più il margine dei lettori nuovi; una chat di famiglia non li alza, il Coordinatore può abbassarli dopo la prima misura): `aperti` 393 216 B, `thermo` 458 752 B, `agilent` 393 216 B, `sciex` 327 680 B, `waters` 327 680 B, `shimadzu` 327 680 B, `bruker` 524 288 B (lettore SQLite minimo e zstd).
- **Pagina, regola R**: `mzlab/web/lettori-famiglie.js` è la tabella fissa famiglia → estensioni, membri che la riconoscono (`_FUNC001.DAT`, `analysis.tdf`), errori del motore che la chiamano (`err.file.agilent`, `err.file.wiff`, `err.file.numpress`); un test Rust controlla che dica le stesse estensioni dei `VOCE`.
- In `upload0` (`mzlab/web/shell.js`) un insieme di file va al modulo della sua famiglia quando l'estensione o un membro lo dice, o quando il motore lo rifiuta con un errore della tabella. Il modulo scrive l'mzML in OPFS (`rust-opfs.js::rawSink`), poi il file entra in `upload()` come ogni mzML; «Scarica .mzML» lo dà senza riconvertire; le righe «In conversione» sono quelle di `CONVROWS` (`conversione.js`).
- **Desktop**: `mzlab-desktop` collega `mzlab-lettori` con tutte le feature, in nativo (nessun tetto), e legge gli stessi formati (`docs/agenti/rust-desktop.md`).

### Thermo fuori dal motore (DEV-1, valutazione)
- Il motore raggiunge Thermo solo da `LETTORI` (`crates/mzlab-core/src/lettori.rs:221`, `rawlettore::VOCE`); `mzidx::build_raw` (`mzidx.rs:676`) non è chiamato da `mzlab-wasm`; `agilent/doc.rs:8` e `agilent/quant.rs:9` usano `metodo_raw::campi`, che resta nel motore. Togliere il lettore libera circa 156 KB (§0): lo spazio che servono DEV-9, 10 e 11, senza una dieta del motore.
- Il `.raw` oggi si legge senza mzML intermedio (`docs/agenti/lettura-nativa.md`): un Orbitrap di qualche GB scritto come mzML in OPFS raddoppierebbe lo spazio e il tempo d'apertura. Per questo DEV-1 aggiunge al motore un **lettore esterno**: un `Lettore` di `mzlab-wasm` che chiama in modo sincrono il modulo `thermo` caricato nello stesso worker (`rust-worker.js` è un worker modulo; le letture dei membri sono già chiamate JS sincrone, `nativi.rs::JsMember`). Indice v2, rotte, `native_srm` e «Scarica .mzML» restano quelli di oggi; anche i worker di `idxpool.js` caricano il modulo.
- Costo: qualche KB nel motore per il lettore esterno, il download del modulo `thermo` al primo `.raw`, una copia degli array di ogni scansione tra le due istanze. Raccomandazione: farlo (il formato del laboratorio è il `.wiff` del 3200, che resta nel motore). Lo stesso lettore esterno può servire più avanti le altre famiglie senza mzML intermedio.

### Pacchetti
I prompt pronti e autosufficienti per le chat DEV sono in `docs/studi/PROMPT_FORMATI/` (`DEV-00.md` … `DEV-11.md`, indice in `LEGGIMI.md`): ognuno dice i file e i punti di codice da toccare, le prove, i metodi e i metadati da leggere e i file reali. Ordine e dipendenze:

| # | Pacchetto | Modulo | Dipende da | File reali da chiedere a Federico (ognuno con il suo mzML di msconvert, 64 bit, zlib, senza peak picking) |
|---|---|---|---|---|
| DEV-0 | Infrastruttura comune: crate, ponte, costruzione per famiglia, tetti, caricamento a richiesta, regola R, mzML in OPFS, desktop | — | — | nessuno (file sintetici) |
| DEV-1 | Thermo fuori dal motore: modulo `thermo` e lettore esterno | `thermo` | DEV-0 | i `.raw` già nei dati (`HRMS/`) |
| DEV-2 | Thermo: SRM dei TSQ, lock mass, PDA/UV | `thermo` | DEV-1 | 1 `.raw` TSQ in SRM (meglio positivo e negativo), 1 Orbitrap con lock mass, 1 con PDA |
| DEV-3 | Formati aperti: numpress, mzXML, netCDF/ANDI (tre PR nella stessa chat, in quest'ordine) | `aperti` | DEV-0 | 1-2 mzML `msconvert --numpressAll` di file già nei dati; 2 mzXML (LR full scan, HR DDA); 1 `.cdf` AIA da una corsa 5977 già nei dati, 1 Shimadzu o LECO se c'è |
| DEV-4 | Agilent `.D` LC/MS: QqQ (MRM, dMRM), Q-TOF profilo e centroidi, negativo, DAD/UV, metodo LC/MS | `agilent` | DEV-0 | 1 `.D` QqQ 64xx MRM (meglio anche dMRM), 1 Q-TOF 65xx profilo, 1 centroidi, 1 negativo, 1 LC con DAD |
| DEV-5 | SCIEX `.wiff` oltre il 3200: altri QTRAP, TripleTOF/ZenoTOF, DAD | `sciex` | DEV-0 | un `.wiff`+`.wiff.scan` per strumento ed esperimento (MRM, sMRM, EPI, MS3, precursori, perdita neutra, IDA, SWATH), 1 con DAD |
| DEV-6 | Waters MassLynx `.raw` (cartella): TQ MRM/SIR, QTof MS/MSe/DDA, lock mass, tracce analogiche | `waters` | DEV-0 | 1 Xevo TQ MRM, 1 Xevo G2-XS (o Synapt) MSe con lock mass, 1 DDA |
| DEV-7 | Shimadzu `.qgd` (GC/MS), `.lcd` (LCMS-2020, 8040/8050/8060, 9030) | `shimadzu` | DEV-0 (DEV-3 per il confronto con `.cdf`) | 1 `.qgd`, 1 `.lcd` SQ, 1 `.lcd` QqQ MRM, 1 `.lcd` 9030; con mzML o `.cdf` esportato |
| DEV-8 | Bruker TDF/TSF (timsTOF): MS1, DDA-PASEF, diaPASEF | `bruker` | DEV-0 | 1 TDF DDA-PASEF, 1 diaPASEF, 1 TSF |
| DEV-9 | Tipi di scansione nel motore: precursor ion, neutral loss, EPI, MS3 | motore | DEV-1, DEV-5 | 1 mzML per tipo (QTRAP) |
| DEV-10 | Mobilità ionica: array dell'mzML, informazioni, scansione | motore | DEV-1, DEV-8 | 1 mzML con mobilità (da TDF o Waters HDMSe) |
| DEV-11 | `metodo/1` di un mzML di altri (testata, `startTimeStamp`, `sample`, `scanSettings`) | motore | DEV-1 | gli mzML di msconvert già nei dati bastano |

Ordine definitivo:
1. DEV-0.
2. DEV-1, approvata: Thermo esce dal motore e il SHA-256 del motore cambia.
3. DEV-3…DEV-8 in parallelo dopo DEV-0, e DEV-2 dopo DEV-1. Ognuna tocca solo la sua cartella di `mzlab-lettori`, la sua riga di `lettori-famiglie.js` e il suo file d'area.
4. DEV-9, 10 e 11 (motore) dopo DEV-1.

**Dieta del motore: assorbita da DEV-1, non più necessaria.** I circa 156 KB liberati bastano ai pacchetti del motore. Una dieta che tocchi `vendor/opentfraw/` o `raw.rs` andrebbe in conflitto con DEV-1.

Fuori (impossibili senza SDK del produttore): `.wiff2`, Bruker BAF, formati proprietari JEOL/LECO/PerkinElmer (strada: l'esportazione ANDI di DEV-3). MGF come corsa e mzData: solo su richiesta.
