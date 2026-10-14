# Studio: lettura nativa di tutti i formati di spettrometria di massa

Obiettivo: mzLab legge così come sono, senza conversione manuale in mzML, i file di ogni produttore, a bassa e ad alta risoluzione, con ogni tipo di esperimento. Questo studio dice cosa si legge oggi (con la prova nel codice), cosa manca, cosa si può fare nel browser e in che ordine. Base: `main` a `e450dce`.

## 0. Il vincolo che decide tutto: il peso del motore

- Il `.wasm` del motore pesa **1 703 607 B** (log del job `build` di `pages.yml`, run 38642852060, commit `e450dce`: «engine .wasm (optimised): 1703607 bytes»). Il tetto è **1 703 936 B** (`.github/workflows/pages.yml:61`, `tools/wasm_opt.sh`). **Margine: 329 byte.** Nessun lettore nuovo, nemmeno il più piccolo, entra nel motore così com'è.
- Per confronto, il codice dei lettori che il motore contiene già: Thermo `opentfraw` 8 245 righe (`vendor/opentfraw/src/`) + 1 319 righe di `rawlettore.rs`/`raw.rs`/`metodo_raw*`; Agilent 2 904 righe (`crates/mzlab-core/src/agilent/`); SCIEX 901 righe (`wiff/`) + 384 (`ole.rs`).
- Gli altri `.wasm` del sito hanno ciascuno il loro tetto e si caricano a richiesta (librerie 392 295 B su 614 400, `pages.yml:66`; mzFinder: `finder` 506 802 B, `finder-lr` 598 107 B, ecc.). Un **modulo lettori separato** segue quindi un modello che il sito usa già.
- Conseguenza: il primo pacchetto (DEV-0) è una decisione architetturale, non un lettore. Senza di essa ogni altro pacchetto è bloccato. Vedi §4.

## 1. Inventario: cosa si legge oggi

Ingresso della pagina: `upload0` (`mzlab/web/shell.js`) passa prima per `wiffToMzml` e `rawToMzml` (i campioni nativi), poi `fileKind` (`shell.js:170`) lascia passare solo `.mzml` e `.dam`; `UNOPEN_RE = /\.(raw|wiff2?|wiff\.scan|d|lcd|lcdx)$/i` (`shell.js:169`) dà l'avviso «non si può aprire nel browser» con la guida di conversione (`conv.help.step1`); ogni altra estensione è «formato non riconosciuto» (`unopen.other`). Con il motore Rust pronto `CONVERT_RAW=true` (`shell.js:145`) e la guida resta solo per ciò che il motore non legge.

Registro dei lettori nativi: `LETTORI` in `crates/mzlab-core/src/lettori.rs:218` = `agilent-d`, `sciex-wiff`, `thermo-raw`. Contratto `Lettore` in `docs/agenti/estendere.md`; flusso nel sito in `docs/agenti/lettura-nativa.md` (`crates/mzlab-wasm/src/nativi.rs`: `native_samples`, `native_attach`, `build_mzidx_native`, `native_srm`, `native_to_mzml`).

| Formato | Stato | Parser | Cosa legge | Prove (test) e file reali |
|---|---|---|---|---|
| **mzML** | letto | Rust `crates/mzlab-core/src/mzml.rs` (`read`, `parse`), testata `mzmlhead.rs`, indice `.mzidx` (`mzidx.rs`, `docs/agenti/mzidx.md`) | spettri (livello, RT, polarità, precursore, finestra di isolamento `MS:1000827`, attivazione `scantable.rs:123-128`), m/z f64, intensità f32, base64 + zlib; cromatogrammi di ogni `ChromKind` (`mzml.rs:44`). **Numpress rifiutato**: `MS:1002312-14` → `err.file.numpress` (`mzml.rs:412-413`) | `mzidx_parity.rs`, `robustezza_mzml.rs`, `mzmlhead_parity.rs`, `golden.rs`; mzML di msconvert dei file del laboratorio (`mzlab-dati/mzML/`, `HRMS/`) |
| **Thermo `.raw`** (file unico, v8-v66) | letto | Rust `vendor/opentfraw` 2.0.0 (Apache-2.0, `MODIFICHE-MZLAB.md`) + `rawlettore.rs` (riconosce dai byte, `:25-34`) | spettri di ogni livello (profilo/centroidi come nel file), filtro, precursore, trailer, rumore FT, `metodo/1`, pressione nell'mzML scritto. **Cromatogrammi del lettore: solo TIC** (`rawlettore.rs:222-248`). PDA/UV: controller `Pda`/`Uv` noti (`opentfraw/src/reader.rs:386-387`), non letti | `raw_lettore.rs`, `raw_parity.rs`, `raw_profile_parity.rs`, `metodo_raw_parity.rs`; solo `.raw` Q Exactive di `mzlab-dati/HRMS/`, nessun TSQ |
| **SCIEX `.wiff` + `.wiff.scan`** | letto (solo 3200 QTRAP) | Rust `crates/mzlab-core/src/wiff/` + `ole.rs`, scritto da zero (`docs/agenti/wiff.md`) | full scan (profilo), prodotto di ione (MS2, codice 11), MRM (un f32 per transizione → cromatogrammi SRM), polarità dal segno di `IS`, metodo, più campioni per file | `wiff_parity.rs` (sintetici di `tools/wiff_riferimento.py` + 23 mzML veri di `3200 qtrap/wiff inquinanti 25-26 B/`), `wiff_lettore.rs`, `tests_e2e/e2e_wiff.py` |
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
| Precursor ion / neutral loss | P: letti come spettri, il tipo di scansione (MS:1000341, MS:1000326) non è riconosciuto (nessuna occorrenza nel codice) | NV | A: letti come full scan (`wiff.md`, «Limiti noti») | A |
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

## 4. Piano a pacchetti DEV

Regola di ogni pacchetto: una PR piccola; un lettore = un modulo con `Lettore` + `VOCE` + una riga in `LETTORI` (`docs/agenti/estendere.md`, «Aggiungere un formato»); prove: file costruito nel test, parità con l'mzML di msconvert sui dati privati (salta senza `MZLAB_DATI`), casi ostili per ogni numero letto (`robustezza_*.rs`), chiavi `err.file.*` in `lang/it.js` e `lang/en.js`, `e2e_lettori.py` esteso; una riga nel file d'area. Il tetto del motore non si alza.

**DEV-0 — Decisione: dove vivono i lettori nuovi (blocca tutto il resto; decide Federico).**
- Opzione A (raccomandata): un modulo `crates/mzlab-lettori-wasm` (un `.wasm` a parte, caricato solo quando l'utente apre un formato nuovo, come `mzlab-lib-wasm`), con il suo tetto in `pages.yml` (scelta di Federico: i tetti non li tocca questa chat). Il modulo apre il file con i lettori nuovi e scrive in OPFS un mzML (lo scrittore unico `lettori_mzml::scrivi` esiste già) che il motore legge come ogni mzML: **0 byte in più nel motore**. Costo: spazio OPFS doppio e il tempo di una scrittura; «Scarica .mzML» diventa gratuito.
- Opzione B: spostare nel modulo separato anche `thermo-raw` (circa 9 500 righe con `opentfraw`), liberando decine di KB nel motore, e mettere nel motore un solo lettore generico di un formato intermedio compatto. Più pulito a regime, ma tocca un percorso che oggi funziona (`lettura-nativa.md`) ed è una PR grande.
- Opzione C: lettori nuovi solo nell'app desktop (nessun tetto). Contraddice «distribuzione = sito».
- Uscita di DEV-0: una riga in `docs/agenti/estendere.md` e `lettura-nativa.md` che dice la scelta.

Dopo DEV-0 (ordine per valore/rischio; tra parentesi le dipendenze):

| # | Pacchetto | Dipende da | Test attesi | Da Federico |
|---|---|---|---|---|
| DEV-1 | Numpress nell'mzML (lineare, pic, slof) al posto di `err.file.numpress` | DEV-0 se finisce nel motore (3-6 KB), altrimenti nel modulo come «mzML numpress → mzML» | vettori della specifica PSI nel test; parità con lo stesso file scritto senza numpress | 1 mzML scritto con `msconvert --numpressAll` di un file già nel repository privato |
| DEV-2 | Lettore mzXML (`Lettore`, un campione) | DEV-0 | sintetico nel test; parità con mzML di msconvert; casi ostili (base64, zlib, `peaksCount` enorme) | 2 mzXML (LR, HR) + mzML degli stessi |
| DEV-3 | Lettore netCDF/ANDI-MS (netCDF-3 classico: `scan_acquisition_time`, `mass_values`, `intensity_values`, `scan_index`, `point_count`) | DEV-0 | file `.cdf` costruito nel test; parità con il `.D` 5977 (stesso campione esportato in AIA da ChemStation/MassHunter) | 1 `.cdf` esportato da una delle 3 corse 5977 già nei dati; meglio anche 1 Shimadzu/LECO |
| DEV-4 | Thermo SRM: `cromatogrammi` del lettore dà una traccia SRM per transizione dalle scansioni SRM (Q1 dal precursore, Q3 dai punti) | DEV-0 solo se tocca il motore (1-3 KB: va liberato spazio) | parità con i cromatogrammi SRM di msconvert; `native_srm` per mzFinder | 1 `.raw` TSQ (Altis/Quantis/Endura) in MRM + mzML msconvert |
| DEV-5 | Agilent LC/MS: QqQ MRM e Q-TOF (profilo e centroidi), negativo | DEV-0; `agilent/` | `agilent_msconvert.rs` esteso ai nuovi file; tracce SRM | 1 `.D` QqQ MRM, 1 Q-TOF profilo, 1 centroidi, ognuno con mzML msconvert (64 bit, senza peak picking) |
| DEV-6 | Waters MassLynx `.raw` (cartella): TQ MRM/SIR, QTof MS/MSe, funzione lock mass come cromatogramma, correzione lock mass come opzione dichiarata | DEV-0 | `waters_parity.rs` contro msconvert; casi ostili sugli `_FUNC*.IDX` | 1 cartella Xevo TQ MRM, 1 G2-XS MSe con lock mass, + mzML msconvert |
| DEV-7 | SCIEX `.wiff` oltre il 3200: QTRAP 4500/5500/6500 (MRM, EPI, MS3, precursori, perdita neutra, più periodi), poi TripleTOF/ZenoTOF (IDA, SWATH) | DEV-0; `wiff/` | `wiff_parity.rs` esteso per strumento; un esperimento non riconosciuto dà un errore con chiave, non un full scan | per ogni strumento ed esperimento un `.wiff` + `.wiff.scan` + mzML msconvert |
| DEV-8 | UV/PDA: Thermo (`Pda`/`Uv`), SCIEX `DAD*`, Agilent `*.ch`/`*.uv`, come cromatogrammi PDA (`ChromKind::Pda`) e spettri UV | DEV-0; serve decidere dove si mostra uno spettro UV (vista Dati) | parità con msconvert (mzML con `electromagnetic radiation spectrum`) | 1 file con PDA per produttore + mzML |
| DEV-9 | Shimadzu `.qgd` (GC/MS) e `.lcd` LCMS-2020; poi 8040/8050/8060 QqQ | DEV-0; `ole.rs` | parità con mzML msconvert o con l'esportazione netCDF dello stesso file | 1 `.qgd`, 1 `.lcd` SQ, 1 `.lcd` QqQ MRM, ognuno con mzML o `.cdf` |
| DEV-10 | Bruker TDF/TSF (MS1, DDA-PASEF, diaPASEF): lettore SQLite in sola lettura minimo + zstd; la mobilità come campo per scansione | DEV-0; decisione su come mostrare la mobilità | parità con mzML msconvert (`--combineIonMobilitySpectra`) | 1 TDF DDA-PASEF, 1 diaPASEF, 1 TSF |
| DEV-11 | Precursor ion / neutral loss / EPI riconosciuti come tipi di scansione (CV `MS:1000341`, `MS:1000326`, e i codici dei produttori), in `iteminfo.rs::acq` | DEV-7 per i codici SCIEX | file costruiti nel test; un mzML msconvert per tipo | 1 file per tipo (QTRAP) |
| DEV-12 | Mobilità ionica in mzML (array `MS:1002476`/`MS:1002815`) | DEV-10 (stessa interfaccia) | mzML costruito nel test | 1 mzML con mobilità |

Ultimi, ciascuno su richiesta esplicita: MGF come corsa (DDA esportata), mzData. Restano fuori: `.wiff2`, BAF, formati JEOL/LECO/PerkinElmer (via netCDF di DEV-3).

Cosa Federico deve fornire, in sintesi: la decisione DEV-0 (e il tetto del modulo separato se sceglie A o B); i file reali con il loro mzML di msconvert nel repository privato, in una cartella per produttore; per i file di altri laboratori, il permesso di tenerli nel repository privato. Senza file veri un lettore non si scrive (regola di `estendere.md`, punto 3).
