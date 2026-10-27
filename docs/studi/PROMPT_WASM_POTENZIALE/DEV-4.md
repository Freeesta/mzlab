# DEV-4 — zlib-rs per l'inflate degli array mzML/mzXML nei `.wasm`

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo la decisione di Federico (solo decompressione oppure anche compressione con `conv/1` nuova, §8 di `docs/agenti/wasm-potenziale.md`) e dopo DEV-2 (tetti).

---

**Ruolo.** Sei DEV-4 WASM-ZLIBRS (Sonnet). Cambi il motore di inflate; nessuna dipendenza nuova fuori dal `Cargo.lock`.

**Contesto.** `mzlab-core` e `mzlab-lettori` usano `flate2` 1.1.10 con `rust_backend` (`miniz_oxide`). `zlib-rs` 0.6.8 è già nel `Cargo.lock` (feature `banco-zlib-rs` di `mzlab-core`, solo per un banco). Lo studio misura nel `.wasm`: inflate ×1,35-2,3, indice HR −9 %, +43 KB per modulo; decompressione deterministica per definizione. La compressione con zlib-rs dà byte diversi da miniz: tocca il convertitore (`mzmlwrite/codifica.rs`) e i golden (`docs/studi/raw-mzml-immediato/DUBBI.md` punto 5). Leggi `docs/agenti/wasm-tetti.md`, `docs/agenti/rust.md`, `docs/agenti/lettori-modulo.md`.

**Perimetro.** `crates/mzlab-core/Cargo.toml`, `crates/mzlab-lettori/Cargo.toml`, il punto unico dove si decomprimono gli array (trovalo con `grep -rn ZlibDecoder crates/`), e solo se Federico sceglie «anche compressione» `mzmlwrite/codifica.rs` con la versione nuova del convertitore; righe di `rust.md`, `wasm-tetti.md`, `misure-velocita.md`. Non toccare `.cargo/`, `tools/`, i workflow, i tetti (li ha già fissati DEV-2: se non bastano, fermati e scrivi al Coordinatore).

**Lavoro.**
1. Variante «solo decompressione» (predefinita salvo diversa decisione): inflate degli array con l'API di `zlib-rs` (o `flate2` con backend `zlib-rs` se la feature non cambia la compressione: controllalo, `flate2` unifica le feature nel grafo), la compressione resta `miniz_oxide`. Una sola funzione di decompressione usata da tutti i lettori.
2. Prova di equivalenza: decompressione di tutti gli array dei file sintetici (`tools/dati_sintetici.py`) e, se presenti, di `dati/` privati (`percorso(...)`) identica byte per byte fra miniz e zlib-rs; un fuzz target o un test con flussi troncati e corrotti (stesso errore `error.*` di oggi).
3. Pesi e tempi prima/dopo (banco di DEV-5 e `tools/bench.py`); `cargo deny check` verde (licenza Zlib già ammessa in `deny.toml`). Se un modulo che non legge mzML cresce, la feature è finita nel posto sbagliato.

**Accettazione.** CI verde con `sito-rust`; golden invariati nella variante «solo decompressione»; nessun tetto superato; tabella prima/dopo in PR.

**Prove.** `cargo test --release -p mzlab-core`; `python3 tools/verifica.py --cambiati`; `tools/banco_wasm.py --varianti mix,mix_zrs`.

**Consegna.** Ramo `claude/wasm-zlibrs`, PR aperta; messaggio al Coordinatore (≤ 5 righe).
