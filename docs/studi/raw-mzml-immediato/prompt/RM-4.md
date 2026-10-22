# RM-4 — Produttore parallelo nel nativo: mmap, worker per intervalli, scrittore ordinato

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-1 e RM-3.

---

Pacchetto RM-4 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` i criteri V1-V6, V9 e F9 di `CRITERI.md`, `RISPOSTE.md` §3.2 (righe lettura, decodifica, metadati, parallelismo) e la tabella aggiornata da RM-3 in `docs/agenti/raw-mzml.md`.

**Leggi**:
- `crates/mzlab-core/src/rawlettore.rs`, `raw.rs` e `lettori_mzml.rs`;
- `vendor/opentfraw/src/reader.rs` (`con_blocco`, cursore);
- `mzmlwrite/pool.rs`.

**Tocca**:
- un modulo nuovo per la pipeline nativa (lettore → N worker che fanno decodifica + metadati + XML + compressione → scrittore ordinato);
- un modulo `unsafe` isolato per `mmap`, con la versione sicura (`read` a offset) accanto e confrontata al bit (COMUNI);
- i test e `docs/agenti/raw-mzml.md`.

**Non tocca**: il contenuto degli spettri e il percorso `.wasm` (RM-5).

## Passi
1. Cursore per worker sullo stesso file (`mmap` o `pread`) con l'indice delle scansioni; nessuno stato condiviso modificabile.
2. Tabelle dei trailer e degli eventi decodificate una volta e condivise in sola lettura; `extra_scansione` per scansione nei worker.
3. Scrittore ordinato: riceve spettri finiti in qualunque ordine e scrive nell'ordine del file, con gli offset; budget in byte (back-pressure).
4. Misura 1/2/4/N thread su LAB-A, LAB-B, BAF e il denso; curva di Amdahl con la parte seriale misurata.

## Accettazione
- V1 ≤ 0,6 s, V2 ≤ 0,5 s, V3 ≤ 3× T_min e V4 sul container a 4 core.
- V5 ≥ 0,7·N fino a 4 core, oppure la roofline che mostra disco o SHA-1 saturi.
- V6 costante con la dimensione (2 GB e 8 GB); V9 ≤ 50 ms.
- F9 al bit con 1/2/4/N thread e ingressi permutati.
- Tabella nella PR: tempi, RSS per fase e impronte.

## Prove
- Fuzzing del modulo `mmap` contro la versione sicura.
- Test con un file troncato e un indice che punta fuori: errore `err.file.*`, mai panico.
- R-c e R-d alla locale dopo la PR (tempi a cache fredda sul Mac contro msconvert e TRFP).
