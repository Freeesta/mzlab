# RM-1 — Metadati fedeli: attivazione, DIA, trailer, conti

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-0; in parallelo con RM-3 (file diversi).

---

Pacchetto RM-1 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` i criteri F5, F6, F7 e F10 di `CRITERI.md` e D6 e D10 di `DUBBI.md`.

**Leggi**:
- `crates/mzlab-core/src/rawlettore.rs` (`extra_scansione`, `testata_mzml`);
- il modulo `extra` che produce i `cvParam`/`userParam` per scansione (`grep -n 'fn di_scansione'`);
- `vendor/opentfraw/src/mzml.rs` (attivazione, `is_dia`).

**Tocca**: quei file, i loro test, `vendor/opentfraw/MODIFICHE-MZLAB.md` e `docs/agenti/raw-mzml.md`.

**Non tocca**: lo scrittore (`mzmlwrite*`, è di RM-3) e gli array dei picchi (RM-2).

## Passi
1. **Confronto dei metadati**: scrivi un confronto (strumento in `tools/`, Python ammesso: legge due mzML e confronta i termini CV per spettro) del nostro mzML con quello di msconvert per LAB-A e LAB-B. Confronta: livello MS, polarità, analizzatore, attivazione ed energia, precursore (m/z, carica, finestra), RT, TIC/BPC. Tabella delle differenze.
2. **Attivazione** (D10): mappa il filtro e il trailer ai termini CV come msconvert: `@cid` → MS:1000133, `@hcd` → MS:1000422, `@etd`, `@ethcd`, `@uvpd`, supplementare. Oggi tutte le 16 038 MS2 `@cid` di LAB-B escono come HCD (MS:1000422): è il primo caso del test.
5. **Potere risolutivo**: MS:1000800 per scansione, come msconvert (oggi solo `userParam` `opentfraw.resolution`).
6. **Precursore**: ripiego quando `Monoisotopic M/Z` del trailer vale 0, confrontato con msconvert su tutto il corpus.
7. **Cromatogrammi non MS**: oltre alla pressione (MS:1003019, già scritta), controlla sul corpus temperatura (MS:1002715) e UV/PDA; scrivi ciò che il file ha. Fonte delle idee 5-7: `VALUTAZIONE_GEMINI.md` M2, M4, M11.
3. **`opentfraw.is_dia`**: oggi vale true su una MS2 mirata di LAB-B. Trova la regola giusta (finestra larga, msx, filtro DIA) o togli il campo se non è nel file.
4. Controlla che **ogni campo del trailer** e dello stato sia scritto (nessuno filtrato) e che un campo assente resti assente (F7).

## Accettazione
- F5 e F6 uguali a msconvert sul 100 % delle scansioni di LAB-A e LAB-B. Le differenze rimaste vanno elencate con la causa e la domanda al Coordinatore.
- Prima/dopo dei byte su LAB-B nella PR e versione `conv/1` aumentata (F10).
- `raw_parity.rs` e `msntree_parity.rs` verdi o aggiornati con la spiegazione.

## Prove
Test con pacchetti costruiti per ogni attivazione; il corpus della locale dopo la PR (R-d) per F5 su tutti i file.
