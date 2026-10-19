# Rosina C: Waters MassLynx `.raw` (cartella)

Legenda e indice: `SCHEDE.md`. Lettore: modulo `waters` (`crates/mzlab-lettori/src/waters/`); dettagli in `docs/agenti/waters.md`.

## Stato
- Funzioni mirate (MRM/SIR) P: cromatogrammi con gli id di msconvert; V2 possibile con Workbench ST004453 (TQ-XS), ST003942, ST003984 (ELENCO-1).
- Funzioni di scansione TOF (Xevo G2/G2-S/G2-XS, Synapt), MSe, DDA: P (13 cartelle vere, senza riferimento). Taratura `Cal Function` applicata; **lock mass non applicata** (30-100 ppm sui file con lock mass, dichiarato).
- IMS (HDMSe, SONAR, cyclic): sommata sulla deriva, avviso dichiarato. Codifica a 6 byte e funzioni MALDI: nessun file.
- Quota: 12 % degli studi MS MetaboLights, 5 % di Workbench, 2 % di PRIDE.

## Cosa manca
1. V1: msconvert (DLL Waters `MassLynxRaw`) in Docker; MassLynx Raw SDK (ha un lettore Linux, licenza Waters) per lock mass e deriva come oracolo locale, mai nel sito.
2. Lock mass: opzione dichiarata «correzione con la funzione di riferimento» (decisione 4 di `ROSINA.md`), provata contro l'SDK.
3. Deriva IMS: tempo di deriva per bin (msconvert divide in una scansione per bin): esporre come mobilità o restare sommati con avviso (decisione in R-05).
4. `.STS`/`.EE` oltre «Set Mass» e «Collision Energy», `_HISTORY.INF`, parametri di rampa: non decodificati.

## Prove possibili
- V1: msconvert in Docker; MassLynx SDK (Linux) per m/z corretti e deriva. V2: Workbench (mzML degli autori). V3: MassLynx se disponibile. V4: openwraw (spia), rainbow (Waters `.raw` UV e MS: da provare).

## Metadati campo per campo
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| Instrument (modello#seriale) | `_HEADER.TXT` `$$ Instrument:` | `strumento.modello`, `.seriale` | L | MassLynx «Header» · mzML `instrumentConfiguration` |
| Acquired Name, Date, Time | `_HEADER.TXT` | `origine.file_creato`, `campi` | L | MassLynx · mzML `startTimeStamp` |
| Sample Description, Bottle Number, Sample ID, Job Code, Task, Conditions, Plate Desc | `_HEADER.TXT` | `campione.nome`, `.vial`, `.id`, `campi` | L | MassLynx «Sample List» |
| User Name, Submitter, Laboratory Name | `_HEADER.TXT` | `operatore.*` (identita) | L | MassLynx |
| Software (MassLynx version), Instrument Type, Encoding | `_HEADER.TXT` | `strumento.software`, `campi` | L | MassLynx · mzML `softwareList` |
| `Cal Function N` (polinomio, T0/T1), Cal MS1/MS2 Static/Dynamic, temperature correction | `_HEADER.TXT` | `campi` (applicata agli m/z) | L | implicita in F3; MassLynx «Calibration» |
| Funzioni: tipo, sottotipo, tempo di ciclo, `tof_depth`, dwell, Q1/Q3 o intervallo ×32 | `_FUNCTNS.INF` (record 416 B) | `ms.periodi[0].esperimenti[]`, `transizioni[]` | L | MassLynx «Acquisition Method» · mzML (funzioni come `function=N`) |
| Sorgente: capillare, cono, temperature, gas desolvatazione/cono, modo, polarità, rampa o CE per funzione | `_extern.inf` | `ms.sorgente.parametri`, esperimenti | L | MassLynx «Tune Page» (testo) |
| Gradiente LC, flusso, forno, durata, campionatore | `_INLET.INF` | `lc`, `campionatore` | L | MassLynx «Inlet Method» |
| Tracce analogiche (pressione, flusso, % solventi, UV, temperature) | `_CHROMS.INF` + `_CHROnnn.DAT` | cromatogrammi analogici, `campi` | L | MassLynx «Analog» · mzML? (msconvert no: V3) |
| Nomi dei composti dei canali | `_FUNCnnn.CMP` | `transizioni[].nome` | L | MassLynx |
| Statistiche per scansione: Set Mass, Collision Energy | `_FUNCnnn.STS`/`.EE` | precursore, CE | L | mzML `precursor` |
| Altre colonne di `.STS` (pressioni, tensioni per scansione, LteffMulti, temperatura del TOF) | `_FUNCnnn.STS` | — | N (R-05) | MassLynx «Scan Statistics»? · SDK `GetScanItems` |
| Lock mass: funzione di riferimento, m/z, finestra, frequenza | `_FUNCTNS.INF` (funzione di riferimento), `_extern.inf` | `ms.calibrazione.lock_mass` («not applied») | L descrizione; correzione N | MassLynx «Lock Mass» · SDK `LockMass` |
| Deriva IMS: bin, tempo per bin, onda (velocità, altezza) | `_FUNCnnn.DAT` (blocchi per bin), `_extern.inf` | — (`avvisi`) | N (sommata) | msconvert (una scansione per bin) · SDK |
| Storia dei trattamenti | `_HISTORY.INF` | — | N | MassLynx |

## Lavoro, rischio, priorità
- **R-05**: 6-8 giorni. Verità msconvert e Workbench (LOC-2), TOF e MSe a F3, lock mass come opzione, deriva come decisione, `.STS` completo se l'SDK lo conferma.
- **Rischio**: medio (correzioni dello strumento non scritte nei dati; la licenza dell'SDK permette l'uso come lettore, non la ridistribuzione: resta sul Mac). **Priorità**: 2.
- Tetto del modulo `waters`: 303 104 B, misura 293 150 B (10 KB liberi).
