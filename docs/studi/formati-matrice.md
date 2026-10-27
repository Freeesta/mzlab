# Matrice dei formati degli strumenti

Una tabella sola: per ogni formato, cosa legge mzLab, cosa manca, come si arriva all'mzML. Il dettaglio (parser, righe, esperimenti per formato, fattibilità) sta in `FORMATI_NATIVI_MS.md`; qui solo la sintesi. Fonti: `crates/mzlab-lettori/` (feature per famiglia), `crates/mzlab-core/src/wiff/`, `non_portati.rs`, `docs/agenti/` (`wiff*.md`, `lettura-nativa.md`, `lettori-modulo.md`, `agilent*.md`, `bruker.md`, `waters.md`, `shimadzu.md`, `lettori-andi.md`, `lettori-aperti.md`), `misure-formati.md`.

## Come leggere le colonne
- **Nativa**: sì = letto e confrontato con l'mzML di msconvert su file veri; parziale = letto, ma una parte non è decodificata o non è verificata; no = rifiutato con un errore che chiede l'mzML.
- **→ mzML**: ogni lettore scrive l'mzML con lo stesso scrittore (`mzmlwrite`): «Scarica .mzML» nella pagina, coda di conversione nel desktop, `mzlab-cli convert` (tutte le famiglie, feature `tutte`).
- **Sito (wasm)**: «motore» = il motore del sito apre il file senza mzML intermedio (indice `.mzidx` v2 sui membri, `lettura-nativa.md`); «modulo» = un `.wasm` della famiglia (`tools/build_lettori_wasm.sh`, tutte e 8 le famiglie, ciascuna con un tetto di peso) apre il file per il motore (Thermo, `lettore-esterno.js`) o scrive da solo un mzML in OPFS prima del caricamento (Regola R, `lettori-modulo.md`); «no» = avviso di conversione.
- **App/CLI (nativo)**: desktop (`mzlab-desktop`) e `mzlab-cli` usano `mzlab-lettori` con `tutte` e `mzlab_lettori::apri` sui file mappati: nessun tetto, nessun modulo, nessun mzML intermedio (`lettori-modulo.md`, `rust-desktop.md`).
- **Regola sito = app**: ogni funzione deve esistere in entrambi. Una cella diversa fra le due colonne è una disparità e sta in «Cosa fare dopo».
- **Test**: file di prova Rust (`crates/*/tests/`) e Python/e2e (`tests/`, `tests_e2e/`) con il nome della famiglia; file veri = conteggi dei documenti d'area, i file stanno solo in `mzlab-dati`.
- **Base legale**: noto = specifica pubblica o riferimento con licenza libera (Apache-2.0/MIT) scritta nel repository; da verificare = lettore scritto da zero sui byte osservati, con l'mzML di msconvert come oracolo, ma senza un registro clean-room (fonti, oracoli, licenze) per la famiglia.

## Tabella

| Formato | Nativa | Cosa manca | → mzML | Sito (wasm) | App/CLI (nativo) | Test (Rust / py-e2e; file veri) | Base legale | Priorità |
|---|---|---|---|---|---|---|---|---|
| **SCIEX `.wiff` + `.wiff.scan`, 3200 QTRAP** (motore del sito) | sì: full scan profilo (15), product ion MS2 (11), MRM (4), polarità, più campioni, metodo, DAD (matrice, spettro UV, mappa) | più periodi; precursore vero di un EPI (`DDERealTimeData`); DAD nell'mzML solo nel nativo (`opz.dad`, non nel `.wasm`) | sì | motore; DAD come mappa e spettro UV, non nell'mzML | sì, e DAD nell'mzML (`opz.dad`) | 10 / 6; 23 mzML veri di msconvert come riferimento | da verificare (scritto da zero, `wiff_specifica_completa.md`; oracolo msconvert) | **1** |
| **SCIEX `.wiff` QTRAP 4500/5500/6500/6500+, Triple Quad 4500** (modulo `sciex`, feature `estesi`) | parziale: codici confermati 4, 5 (PIS), 7 (NL), 11, 15; MRM densi e sMRM | MS3, EMS/ER/EMC non visti in file veri; più periodi rifiutati; precursore vero dell'EPI; metodo per campione euristico (`MethodN` più vicino) | sì | modulo → mzML in OPFS | sì | inclusi sopra; 32 file veri, 388 campioni letti, 6 rifiutati; parità su 3594 spettri PIS/NL | da verificare (come sopra) | **1** |
| **SCIEX `.wiff` TOF** (TripleTOF, ZenoTOF, X500; codici 8, 9) | no | codec dei blocchi diverso; nessun mzML di riferimento | no (msconvert esterno) | no | no | rifiuto in `wiff_varianti.rs`; 0 file TOF | da verificare | media |
| **SCIEX `.wiff2`** (SCIEX OS) | no: riconosciuto, rifiutato (`detail` = `wiff2`) | tutto; cifrato secondo `opensxraw` | no (msconvert con SDK) | no | no | rifiuto; 0 file | SDK del produttore | bassa |
| **SCIEX `.dam`** (metodo) | sì | — (non è un dato) | n/a | motore | sì | 1 / 1 | da verificare | — |
| **Thermo `.raw`** (file unico, v8-v66) | sì: ogni livello MSn, profilo/centroidi come nel file, precursore e attivazione, trailer, SRM dei TSQ, canali UV/analogici, `metodo/1` | spettro UV 3D del PDA; correzione lock mass; pressione solo nell'mzML scritto; TSQ e UV non confrontati con msconvert | sì (`peaks` = file/profile/mzlab) | modulo `thermo` (apre per il motore) | sì (nel core) | 7 / 2; `.raw` Q Exactive veri (`raw_parity.rs`) | noto (`vendor/opentfraw`, Apache-2.0; RawFileReader solo oracolo sul Mac, `LETTORE_THERMO_PERFETTO.md`) | media (velocità: `RAW_VELOCE.md`) |
| **Agilent MassHunter `.D` GC/MS** (5977 SQ, motore) | sì: scansione e SIM, SIC, TIC/BPC, metodo GC | `7890Method.bin`, tipo di campione | sì | motore | sì | 17 / 2 (tutta la famiglia); 3 corse vere | da verificare (formato ricavato dai file; schema `MSScan.xsd` nella cartella) | bassa |
| **Agilent `.D` LC/MS** QqQ 6460-6495, Q-TOF 6520-6550 (modulo `agilent`) | parziale: MRM/dMRM uguali a msconvert, Q-TOF profilo e centroidi (entro 12 ppm) | DAD (`DAD1.cd/.cg/.sd/.sp`); Q-TOF a 284 byte per record; MS2 non verificato; IMS 6560 | sì | modulo → mzML in OPFS | sì | 14 corse vere | da verificare | media |
| **Agilent ChemStation `DATA.MS`** | parziale: solo variante `01 32` | altre varianti | sì | motore | sì | come sopra | da verificare | bassa |
| **Bruker `.d` TDF/TSF** (timsTOF; modulo `bruker`) | parziale: MS1, DDA-PASEF e MRM confrontati; diaPASEF, PRM-PASEF, TSF letti senza riferimento | 1/K0 per punto (`non_portati::BRUKER`); immagini e regioni TSF; `.mcf`, `chromatography-data.sqlite` | sì (senza mobilità) | modulo → mzML in OPFS | sì | 3 / 1 | noto per la lettura (SQLite scritto nel crate, `ruzstd` MIT); calibrazione TIMS da verificare | bassa |
| **Bruker `.baf`** (micrOTOF, maXis, impact) | no (`err.file.brukerBaf`) | tutto | no (esportazione Bruker) | no | no | rifiuto in `robustezza_bruker.rs`; 0 file | SDK del produttore | bassa |
| **Bruker `.yep`, FT-ICR `fid`** | no | tutto | no | no | no | — | SDK probabile | bassa |
| **Waters `.raw`** (cartella MassLynx; modulo `waters`) | parziale: MRM/SIR come SRM (128 SRM uguali a msconvert su Xevo TQ-XS), MS, MSe, DDA, TIC, tracce analogiche, `metodo/1` | spettri TOF, DDA, SIR non verificati; mobilità (deriva sommata); correzione lock mass non applicata; UV non visto | sì | modulo → mzML in OPFS | sì | 6 / 2; 13 cartelle vere | da verificare (scritto da zero; `openwraw` come guida, licenza non scritta) | media |
| **Shimadzu `.qgd` / `.lcd`** (modulo `shimadzu`) | parziale: `.qgd` scansioni, SIM, MRM; `.lcd` LCMS-2020 e triplo quadrupolo (MRM) | Q-TOF/IT-TOF/9030 (`QTFL`) rifiutati; PDA (`LSS`); polarità del 2020; CE e dwell; nessun confronto con msconvert | sì | modulo → mzML in OPFS | sì | 4 / 1 | da verificare (`openszraw` come guida) | media |
| **Shimadzu `.lcdx`** | no (avviso di conversione) | formato non studiato | no | no | no | 0 file | da verificare | bassa |
| **ANDI-MS / AIA `.cdf`** (netCDF-3; modulo `andi`) | sì: spettri, SIM, TIC, metodo; ANDI-Chromatography come traccia | MSn (il formato non lo porta) | sì | modulo → mzML in OPFS | sì | 5 / 3; 3 `.cdf` veri (parità con la `.D` della stessa corsa) | noto (ASTM E1947/E2077) | — |
| **mzML** (core) | sì: spettri, cromatogrammi, SRM, MSn, DIA classificato, matrice DAD, array di mobilità | numpress nel core (lo legge `aperti`); mappa e XIC per mobilità; metadati di un mzML altrui fuori da `metodo/1` | — | motore | sì | 9 / 3 | noto (PSI) | — |
| **mzML numpress** (modulo `aperti`) | sì | `dataProcessing`, `scanSettings` della sorgente (`non_portati::NUMPRESS`) | sì (riscrive senza numpress) | modulo → mzML in OPFS | lettore nel registro (`VOCE_MZML_NUMPRESS`); da verificare che la pagina del desktop lo usi (il core rifiuta numpress) | 2 / 0 | noto (PSI) | — |
| **mzXML** (modulo `aperti`) | parziale: ISB 2.x/3.x, SIM, SRM, precursori | nessun file vero confrontato con msconvert | sì | modulo → mzML in OPFS | sì | 2 / 0 (+5 `aperti`); file costruiti nei test | noto (ISB) | bassa |
| **mzData** | no | tutto | no | no | no | — | noto (PSI, obsoleto) | bassa |
| **MGF / MSP / `.mzlib`** | solo come libreria (`mzlab-lib`) | MGF come corsa nella vista Dati | no | libreria (`mzlab-lib-wasm`) | libreria | `crates/mzlab-lib/tests/parse.rs` | noto | bassa |
| **JEOL, LECO `.peg/.smp`, PerkinElmer** | no | tutto | via ANDI | via ANDI | via ANDI | — | SDK del produttore | — (strada: esportazione ANDI) |

## Rispetto alla stella guida
- Lettura nativa, clean-room: Thermo (licenza libera nel repository) e ANDI/mzML/mzXML (specifiche pubbliche) sono a posto. Per SCIEX, Agilent, Waters, Shimadzu il codice è scritto da zero e confrontato con msconvert, ma manca un registro che dica, per famiglia, quali fonti sono state lette e quali no.
- Convertire tutto in mzML: vale per ogni formato con «Nativa» sì o parziale; restano fuori `.wiff2`, `.wiff` TOF, `.baf`, `.yep`, `.lcdx`, mzData e i formati JEOL/LECO/PerkinElmer.

## Disparità sito ↔ app
- **DAD del `.wiff`**: nell'mzML solo nel nativo (`opz.dad`, `lettori_mzml.rs`); nel sito resta mappa e spettro UV.
- **Lettura senza mzML intermedio**: nel nativo per tutte le famiglie; nel sito solo per il motore (3200, Agilent GC/MS, mzML) e Thermo. Le altre 6 famiglie passano da un mzML in OPFS (spazio, tempo, `CONV` non riconverte).
- **mzML numpress in ingresso**: nel sito lo apre il modulo `aperti`; nel desktop non è provato.
- **Conversione**: nel sito su richiesta e in sequenza; nel desktop in coda automatica, e nella CLI con codifica in parallelo (`Opzioni::thread`). Stessi byte, tempi diversi.
- **Misure**: il desktop non è mai stato misurato nel contenitore (`misure-formati.md`, «Buchi»).

## Cosa fare dopo
1. `.wiff` QTRAP nativo: chiedere ai colleghi `.wiff` 4500/5500/6500 con EPI dipendente (IDA), MS3 e più periodi, ciascuno con il suo mzML di msconvert; poi precursore vero dell'EPI (`DDERealTimeData`) e più periodi nel modulo `sciex`, uguali nel sito e nel nativo.
2. Disparità `.wiff`: DAD nell'mzML anche nel sito (se il tetto del `.wasm` regge) e varianti QTRAP lette dal motore o dal modulo senza mzML intermedio, come nel desktop.
3. `.wiff` → mzML: `mzlab-cli convert` su tutti i `.wiff` di `mzlab-dati` con confronto contro msconvert in un solo rapporto, e lo stesso confronto dalla pagina del sito: byte uguali fra le due strade.
4. Correggere `docs/agenti/wiff.md` riga 51: dice che il DAD del `.wiff` non si legge, ma `wiff/dad.rs`, `dad_uv.rs` e `dad_map.rs` lo leggono.
5. Pipeline TP: provare il Flow (`flow-copertura-tabui.md`) su un campione di ogni famiglia «sì/parziale», sia nel sito sia nella CLI.
6. Disparità di lettura: aprire senza mzML intermedio nel sito anche Waters, Agilent LC/MS, Bruker, Shimadzu, ANDI, mzXML (come Thermo con `lettore-esterno.js`); provare numpress in ingresso nel desktop.
7. Registro clean-room: un file per famiglia in `mzlab-dati/dati/formati/_licenze/` (fonti lette, oracoli, licenze); prima Waters (`openwraw`) e Shimadzu (`openszraw`).
8. Da «parziale» a «sì»: mzML di msconvert per Waters TOF/DDA e Shimadzu `.lcd`/`.qgd`; Agilent DAD (`DAD1.*`) e Q-TOF a 284 byte.
9. Thermo `.raw`: lettura diretta già fatta in entrambi; resta la velocità (`RAW_VELOCE.md`) e lo spettro UV 3D.
10. Fuori finché non c'è un formato documentato: Bruker 1/K0 per punto da fare; `.baf`, `.wiff2`, mzData, MGF come corsa, `.lcdx` solo su richiesta.
