# RM-2 — Picchi senza perdite: colonne per picco, profilo e zeri, `--peaks tutto`

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-1. Ha bisogno dei dump RFR di R-b (`RICHIESTE_LOCALE.md`). Coordinarsi con T-7b (decisione 4 di `DUBBI.md`).

---

Pacchetto RM-2 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` i criteri F1-F4, F7, F10 ed E6 di `CRITERI.md`, D1, D2, D4 e D7 e le decisioni 1, 2 e 4 di `DUBBI.md`, e `RISPOSTE.md` §2.1 (tabella del pacchetto).

**Leggi**:
- `vendor/opentfraw/src/scan_data.rs` (descrittore saltato, «unknown» = risoluzione, triplette);
- `crates/mzlab-core/src/rawlettore.rs` (`scansione_dati`, `resolve_scan_arrays`);
- `docs/agenti/scandump.md`;
- `tools/rosina/scandump_compare.py`.

**Tocca**:
- `scan_data.rs`, solo se T-7b non è aperto (altrimenti usa ciò che espone T-7b);
- `rawlettore.rs` e lo scrittore degli array non standard in `mzmlwrite`, una funzione piccola, d'accordo con RM-3 se è aperto;
- `thermo_scandump` (campi nuovi);
- i test e `docs/agenti/raw-mzml.md`.

**Non tocca**: il valore di base di `--peaks` (`file`) e gli altri lettori.

## Passi
1. **Descrittore** (D1): decodifica indice, carica e flag per picco; esponili nel `Lettore` e in `scandump/1`, poi confronta con Charges e Flags di RFR (R-b). Un campo verificato su tutto il corpus entra in `VERIFICATI` di `crates/mzlab-lettori/tests/scandump_thermo.rs`.
2. **Risoluzione, rumore e linea di base**: la risoluzione dallo stream «unknown»; per rumore e linea di base, i nodi oppure i valori per picco come RFR (D4), con la formula verificata al bit.
3. **Zeri** (D2): ricava la regola dai dump RFR del profilo. Opzione `--set conv.zeri=file|rfr` (base `file`): con `rfr` l'array è uguale a msconvert (F3) e ogni spettro dichiara quanti zeri aggiunti.
4. **`--peaks tutto`**: il profilo come array standard; la lista e le colonne come `binaryDataArray` non standard (MS:1000786 con nome: `centroid m/z`, `centroid intensity`, `charge`, `flags`, `resolution`, `noise`, `baseline`), con `arrayLength`. Coefficienti e fudge per scansione in `userParam` per F4.
5. **F4**: un test che dall'mzML ricalcola i bin del file per il 100 % dei punti di LAB-B.

## Accettazione
- F1 e F2 al 100 % contro RFR su LAB-A, LAB-B, INF-A e BAF. Gli scarti si riportano con il numero e la causa, senza tolleranze nascoste.
- F3 con `zeri=rfr` al 100 % contro msconvert.
- F4 al 100 %.
- `--peaks file` invariato al byte (F10). Il modulo `thermo` dentro il +10 % dichiarato.

## Prove
- E6 (lettori di terzi con array non standard) chiesto alla locale con R-d prima di proporre di cambiare il valore di base.
- Se E6 fallisce: le colonne vanno nel blocco colonnare del `.mzidx` (T-7d) e l'mzML resta standard. Decide il Coordinatore.
