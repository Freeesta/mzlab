# DEV-5 — Banco permanente dei `.wasm` (varianti di build a confronto)

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

**Ruolo.** Sei DEV-5 WASM-BANCO (Sonnet). Scrivi uno strumento di misura, non cambi il prodotto.

**Contesto.** Lo studio `docs/agenti/wasm-potenziale.md` ha misurato le funzioni vere di `mzlab-core` in un `.wasm` costruito in più varianti (`z`/O3, ±`simd128`, miniz/zlib-rs) in Node e in Chromium. I sorgenti usati sono in `docs/studi/PROMPT_WASM_POTENZIALE/banco.txt`. Le PR DEV-1..4 hanno bisogno di rifare le stesse misure sulla stessa base. Leggi: `docs/agenti/wasm-potenziale.md` §3 e §9, `docs/agenti/test.md` (strumenti), `docs/agenti/misure-velocita.md` (formato delle righe).

**Perimetro.** Solo file nuovi: `tools/banco_wasm/` (crate con il suo `[workspace]`, così NON entra nel workspace e non tocca `Cargo.toml` alla radice; `Cargo.lock` proprio copiato da quello alla radice; `rust-toolchain.toml` uguale), `tools/banco_wasm.py` (orchestratore), una riga in `docs/agenti/misure-velocita.md` e una in `docs/agenti/test.md`. Non toccare `crates/`, `.cargo/`, `tools/wasm_opt.sh`, i workflow.

**Lavoro.**
1. Porta il crate di `banco.txt` in `tools/banco_wasm/` con dipendenza `mzlab-core = { path = "../../crates/mzlab-core", default-features = false, features = ["mappa-dad"] }`, i profili `release` (z), `o3`, `mix` e la feature `zlibrs`; aggiungi le funzioni mancanti utili: decodifica di un `.raw` col lettore `thermo` se un file c'è (`percorso("dati/…")` da `tools/dati_percorsi.py`, si salta senza), e l'indice con `mzidx::build_lettore`.
2. Generatore del mzML HR sintetico (`examples/gen.rs`, seme fisso): scrive in una cartella temporanea, mai nel repository.
3. `tools/banco_wasm.py --varianti base,o3,o3simd,o3simd_zrs,mix,mix_zrs [--chromium] [--giri 5]`: costruisce ogni variante con `CARGO_TARGET_DIR` proprio, `wasm-bindgen --target web`, `wasm-opt` con le stesse opzioni di `tools/wasm_opt.sh` (lo stesso binario che usa lo script), esegue la suite in Node e, con `--chromium`, in Chromium via Playwright (server locale con il PID salvato, fermato con quel PID), stampa una riga JSON per variante (peso dopo `wasm-opt`, gzip, ms per lavoro, `compile_ms`) e la tabella base → variante.
4. Controllo di uguaglianza: ogni lavoro restituisce un valore di controllo; il programma fallisce se due varianti danno valori diversi (deve valere bit per bit).
5. Python è solo l'orchestratore (`tools/`): nessun calcolo in Python.

**Accettazione.** `python3 tools/banco_wasm.py --varianti base,o3` gira in una sessione cloud senza dati privati e stampa le due righe con valori di controllo uguali; con `--chromium` anche i tempi di Chromium; i numeri della riga base stanno entro ±20 % dalla tabella §3 dello studio sulla stessa classe di macchina. `tools/pre_pr.py` verde.

**Prove.** `python3 tools/banco_wasm.py --varianti base,o3 --giri 3` (atteso: o3 più veloce su `parse_hr`, `index_hr`, `pick50_lr`; stessi valori di controllo); `python3 tools/banco_wasm.py --varianti base --chromium --giri 3`.

**Consegna.** Ramo `claude/wasm-banco` da `origin/main`, PR con l'uscita delle due prove; messaggio al Coordinatore (≤ 5 righe: PR, SHA a CI verde, cosa resta).
