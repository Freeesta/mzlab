# WASM-C · blocco di lettura da OPFS 1 MiB → 4 MiB
**Ramo**: `claude/blocco-opfs` da `origin/main`. **Dipende da**: nulla. **Prima di**: E.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-C: ricevuto».

1. **Ruolo**: DEV Sonnet, ponte JS ↔ wasm della lettura dei file. Nessun algoritmo nuovo.
2. **Contesto**: il motore chiede i byte a blocchi di `BLOCK = 1 << 20` (`crates/mzlab-wasm/src/lib.rs:73`, `BlockReader::fill`); in JS i lettori riusano un buffer da 1 MiB (`rust-worker.js:75` `readerFor`, `lettura-sync.js:28` `openSource`).
   - Misura (`docs/studi/wasm-potenza.md` §3e, `banco-browser.txt`, file OPFS da 256 MB, mediana di 3): `SyncAccessHandle.read` in Chromium 153 dà 461-521 MB/s a 1 MiB, 818-894 a 2 MiB, 1191-1273 a 4 MiB, 1432-1630 a 8 MiB; Firefox 157 4655 → 5689 MB/s; WebKit 27.2 piatto (~6-7 GB/s).
   - La lettura mzML HR costa ~620-780 ms per 87 MB (`wasm-potenziale.md` §3): a 1 MiB la sola lettura OPFS in Chromium vale ~170 ms, a 4 MiB ~70 ms.
   - Leggi `docs/agenti/memoria.md` (budget: il buffer conta) e `docs/agenti/mzidx.md`.
3. **Perimetro**: `crates/mzlab-wasm/src/lib.rs` (costante `BLOCK` e, se serve, la soglia `4 * BLOCK` di riga 130), `mzlab/web/rust-worker.js` (`readerFor`), `mzlab/web/lettura-sync.js`; una riga in `docs/agenti/memoria.md` e in `misure-velocita.md`. Non toccare `mzlab-core`, i lettori nativi, l'app desktop (legge da disco: nessun blocco JS).
4. **Lavoro**:
   - Misura prima con `tools/bench.py` (o `tools/bench_mappa.py`, campo `rd`: tempo nei lettori) su `B_FullMass-t15` e sul mzML HR del banco (`tools/banco_wasm.py`, generatore `gen`), in Chromium.
   - Porta `BLOCK` a 4 MiB (una costante sola, condivisa col buffer JS: niente doppio numero), controlla che la memoria di picco resti nel budget di `memoria.md` (riga 130: `4 * BLOCK` = 16 MiB di coda; se il budget piccolo (telefono, `SMALL`) non regge, la coda resta a 4 MiB totali).
   - Misura dopo, stessi file, stesso browser.
5. **Accettazione**:
   - Indici `.mzidx` e risposte delle rotte identici byte per byte prima/dopo (golden e test di parità invariati, nessun test modificato).
   - In Chromium: tempo nei lettori (`rd.ms` di `tools/bench_mappa.py`) sceso di almeno il 40 % e apertura del mzML HR (`tools/bench.py`) più breve di almeno 60 ms; Firefox non più lento; tabella prima/dopo nella PR (ms, picco di memoria).
   - CI verde con `sito-rust`.
6. **Prove**: `cargo test -p mzlab-wasm`, `python3 tools/bench.py` prima/dopo, `python3 tools/verifica.py --cambiati`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/blocco-opfs`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
