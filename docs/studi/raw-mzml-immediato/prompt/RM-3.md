# RM-3 — Scrittore veloce: niente reset, niente format!, SHA-1 e indice nella stessa passata

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-0; in parallelo con RM-1.

---

Pacchetto RM-3 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md`, poi in `docs/studi/raw-mzml-immediato/` i criteri V1, V2, V5, V6, V7, V10, F8, F9 e F10 di `CRITERI.md`, `MISURE.md` §2-3, D7, D8, D9 e le decisioni 3 e 5 di `DUBBI.md`, e la tabella di partenza di RM-0 in `docs/agenti/raw-mzml.md`.

**Leggi**: `crates/mzlab-core/src/mzmlwrite.rs`, `mzmlwrite/{codifica,pool,thread,matrice,testa}.rs` e `lettori_mzml.rs`.

**Tocca**: quei file, i loro test, `docs/agenti/raw-mzml.md` e il banco di RM-0 (solo per aggiungere misure).

**Non tocca**: il lettore Thermo (`rawlettore.rs`, `vendor/opentfraw`) e il contenuto degli spettri.

## Passi, uno per volta, ognuno con la misura prima e dopo del banco di RM-0
1. **Compressore riusato**: niente `reset` con memset per array. Un flusso per worker riusato, o zlib-rs (backend `zlib-rs` di flate2, già nel `Cargo.lock`). Misura µs per array e dimensione sul corpus; decidi V10 con la sua tabella; versione `conv/1` nuova se i byte cambiano (decisione 5).
2. **XML senza allocare**: un buffer per worker riusato, `itoa` per gli interi, i float con lo stesso formato di oggi (byte identici), escape senza `String` intermedie.
   - Obiettivo V7: ≤ 1 allocazione ogni 1000 scansioni.
3. **Intestazione fuori dal percorso critico**: SHA-1 del `.raw` e metodo calcolati in un thread mentre gli spettri si preparano. La testata si scrive per prima, quindi:
   - o si calcola l'SHA-1 durante la lettura;
   - o si riserva lo spazio e lo si riempie alla fine con una scrittura a offset (solo nativo; nel browser si misura).
4. **Indicizzato di base** (decisione 3): offset raccolti dallo scrittore ordinato e SHA-1 dell'uscita in un thread dedicato che consuma i byte in ordine.
5. **Back-pressure in byte**: canale limitato a un budget in MB (non in spettri) fra i worker e lo scrittore. V6 sul denso da 2 GB e da 8 GB.

6. **Dalla valutazione Gemini** (`VALUTAZIONE_GEMINI.md` G15, G17-G19, G22): livello di compressione scelto dalla dimensione dell'array (stored o livello 0 sotto una soglia misurata); zlib-ng escluso (C, niente `.wasm`, byte diversi); ryu escluso (cambia i byte); base64-simd (dipendenza nuova) solo se il denso mostra un guadagno ≥ 3 %; ogni altro passo entra con il suo guadagno oltre il rumore.

## Accettazione
- Con il solo scrittore (produttore di oggi), sul container a 4 core:
  - LAB-B ≤ 0,9 s a 1 thread;
  - nessun caso più lento con N thread che con 1 (V5);
  - V6 e V7 rispettati.
- F8 (validatore nostro + R-d), F9 su 1/2/4 thread e F10.
- Tabella nella PR: ogni passo con il tempo, la dimensione e le impronte.

## Prove
- Test al bit: lo stesso mzML con 1 e N thread.
- Test che un lettore zlib standard (`flate2` con miniz e zlib-rs) rilegge ogni array.
- Fuzzing dell'escape XML.
