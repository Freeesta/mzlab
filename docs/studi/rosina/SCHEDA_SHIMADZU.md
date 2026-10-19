# Rosina C: Shimadzu `.lcd` (LabSolutions LC/MS) e `.qgd` (GCMSsolution)

Legenda e indice: `SCHEDE.md`. Lettore: modulo `shimadzu` (`crates/mzlab-lettori/src/shimadzu/`: contenitore OLE, zlib); dettagli in `docs/agenti/shimadzu.md`.

## Stato
- `.qgd` GC/MS (QP2010/QP2020): scansione e SIM P; `.lcd` LC/MS QqQ (8040/8050/8060) MRM P, LCMS-2020 (singolo quadrupolo) P. Nessun file vero con riferimento.
- Non decodificati: polarità degli eventi `Mass Raw Data`, CE e dwell delle MRM (due transizioni con gli stessi Q1/Q3 e CE diverse si fonderebbero), PDA (`PDA 3D Raw Data`), parametri del GC, stato dello strumento, metodo `.qgm` incorporato. LCMS-9030 (Q-TOF) e IT-TOF: rifiutati.
- Quota: 1,7 % degli studi MS MetaboLights, 0,7 % di Workbench; ma molto diffuso nei laboratori didattici italiani (GC/MS).

## Cosa manca
1. V1: msconvert ha la DLL Shimadzu solo per i `.lcd` LC/MS; per il `.qgd` nessun oracolo automatico: V2 con un `.cdf` esportato dallo stesso file (GCMSsolution «File Conversion → AIA») o V3.
2. File con riferimento: MTBLS12691 (8060), MTBLS13204 (9030), Zenodo 18633076 (2020 + PDA), ST001609 (QP2010 Ultra) in ELENCO-1.
3. Polarità, CE, dwell, PDA, GC; 9030 in un modulo nuovo `shimadzu-tof`.

## Prove possibili
- V1 (`.lcd`): msconvert in Docker. V2 (`.qgd`): `.cdf` della stessa corsa, letto da `andi`. V3: LabSolutions/GCMSsolution (Postrun) se disponibile. V4: openszraw (immaturo, spia).

## Metadati campo per campo
| Campo del produttore | Dove nel file | `metodo/1` | Stato | Controllo (V3 · oracolo) |
|---|---|---|---|---|
| Nome dello strumento, configurazione | `GCMS Configuration/System`, flussi di sistema | `strumento.modello` | L (come il file lo dichiara: «GCMS-QP2010» anche per QP2020NX) | Postrun «Instrument» · mzML (`.lcd`) |
| Versione del file e del software | `File Property` (offset 4, 148 nel `.qgd`) | `strumento.software`, `campi` | L | Postrun |
| generatedBy, modifiedBy, date | `File Property` (20, 52, 60, 92) | `campi` (identita) | L | Postrun «File Property» |
| Sample name, sample ID, vial, injection volume | `File Property` (204, 236) / XML delle proprietà (`.lcd`) | `campione.*` | L | Postrun «Sample Information» · mzML `sampleList` |
| Operatore, data di acquisizione | `File Property` (300, 508) | `operatore.utente` (identita), `origine.file_creato` | L | Postrun |
| Percorsi dei file di dati, metodo, tune, batch | `File Property` (580, 836, 1092, 1604) | `campi` (identita, non nell'mzML) | L | Postrun |
| Eventi: tipo (scan/SIM), intervallo m/z, ioni SIM, tempi | `Mass Raw Data`, eventi visti | `ms.periodi[].esperimenti[]` | L (euristica SIM non verificata) | mzML (`.lcd`); `.cdf` (`.qgd`) |
| Polarità per evento | `Mass Raw Data` (testata dell'evento) | scansioni | N (R-07) | mzML (`.lcd`) |
| Transizioni MRM: Q1, Q3, polarità | `TLM Raw Data` (record zlib, byte 33 bit 6-7) | `transizioni[]` | L | mzML `SRM SIC` |
| CE, dwell, pausa, Q1/Q3 pre-bias | metodo MS (`Mass Parameters`?) | — | N (R-07) | LabSolutions «Method» · mzML? (msconvert scrive CE: da vedere) |
| Metodo GC: forno, iniettore, colonna, gas | `GC-2010/GC17A Instrument Parameters` | — (`avvisi`) | N (R-07) | GCMSsolution «Method» |
| Tune e stato MS (vuoto, filamento, detector, temperature) | `MS Status Data`, `<storage>/Status`, `Status Curve`, `GUMM_Information` | — (`avvisi`) | N (R-07) | Postrun «Tuning»/«Status» |
| PDA: spettri e λ singole | `PDA 3D Raw Data`, `LSS Raw Data` | — (`avvisi`) | N (R-07) | LabSolutions «PDA» · msconvert? (no) |
| Metodo incorporato `.qgm` | flusso OLE | — (`avvisi`) | N | GCMSsolution |
| 9030 / IT-TOF (spettri TOF) | altri flussi | — | N, rifiutato | LabSolutions · msconvert |

## Lavoro, rischio, priorità
- **R-07**: 6-8 giorni. Verità msconvert (`.lcd`) e `.cdf` (`.qgd`) (LOC-2), polarità, CE/dwell, PDA, GC come `campi` decodificati, 9030 in `shimadzu-tof` (tetto = misura + 3 %) solo con V1.
- **Rischio**: medio (molti flussi senza nome chiaro, nessun lettore libero maturo). **Priorità**: 3 (quote piccole, ma utile per i corsi).
- Tetto del modulo `shimadzu`: 290 816 B, misura 281 572 B (9 KB liberi).
