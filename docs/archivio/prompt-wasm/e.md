# WASM-E · un file grande indicizzato da più worker su SharedArrayBuffer
**Ramo**: `claude/indice-parallelo` da `origin/main`. **Dipende da**: WASM-A, WASM-C, WASM-D in `main`; decisione di Federico (§7 dello studio, punto 3). **Prima di**: nulla.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-E: ricevuto».

1. **Ruolo**: DEV Opus (o Sonnet con revisione ARCH), motore wasm + pool di worker. Toolchain stabile, nessuna dipendenza (niente `wasm-bindgen-rayon`, niente nightly).
2. **Contesto**: oggi un file si indicizza in UN worker (`idx-worker.js`, `build_mzidx`); il pool (`idxpool.js`) scala solo fra file diversi. L'app nativa ordina già in parallelo (`crates/mzlab-core/src/mzidx_ordine.rs:161-175`, `in_parallelo`, solo `not(wasm32)`), con byte uguali per ogni numero di thread.
   - Thread wasm «veri» NO: in Rust 1.97 `+atomics` con `--shared-memory` si collega anche in stabile, ma la `std` precompilata è senza `atomics` (allocatore e TLS senza lock) → corruzione con più thread (studio §3b).
   - Si usano invece più istanze wasm (una per worker, memoria propria) che leggono e scrivono le loro fette di un `SharedArrayBuffer` con copie `memcpy`.
   - Misura (`banco-browser.txt`, 128 MB di Float64, lavoro per elemento uguale): 1 worker 869 / 462 / 548 ms (Chromium 153 / Firefox 157 / WebKit 27.2), 4 worker con copie a messaggi 265 / 842 / 300 ms, 4 worker su un `SharedArrayBuffer` 103 / 55 / 66 ms.
   - Firefox e WebKit danno UN solo `SyncAccessHandle` per file (ignorano `mode: "read-only"`, studio §3e): un solo worker legge il file.
3. **Perimetro**: `crates/mzlab-core/src/mzidx_ordine.rs` (rendere la divisione in fette un'API usabile anche in wasm, stessa funzione dell'app), `crates/mzlab-wasm` (le esportazioni per «ordina la fetta i di n»), `mzlab/web/idx-worker.js`, `mzlab/web/idxpool.js`; test di parità; righe in `docs/agenti/mzidx.md` e `misure-velocita.md`. Non toccare i lettori, `sw.js`, l'app desktop oltre al codice condiviso di `mzidx_ordine.rs`.
4. **Lavoro**:
   - Primo passo, prima di scrivere codice: misura le fasi di `build_mzidx` sul mzML HR del banco (`tools/banco_wasm.py`, generatore `gen`, 87 MB) in Chromium: lettura/parse, ordinamento, scrittura. Se la parte che si divide in fette vale meno del 40 % del tempo, fermati e manda i numeri al Coordinatore.
   - Con `capacita.isolamento`: N = `min(4, hardwareConcurrency - 1)` worker; il worker che legge mette i picchi in un `SharedArrayBuffer`, gli altri ordinano le loro fette (stesse fette di `in_parallelo`), il primo scrive. Senza isolamento N = 1: è lo stesso codice con una fetta (nessun secondo percorso), e la capacità `indiceParallelo` mostra l'avviso comune di WASM-A.
   - Budget di memoria (`memoria.md`): il `SharedArrayBuffer` conta nel budget come oggi lo spill.
5. **Accettazione**:
   - `.mzidx` identico byte per byte con N = 1, 2, 4 e uguale a quello dell'app nativa (test di parità su `dati_sintetici.py` e sul mzML HR del banco).
   - Tempo d'indice del mzML HR in Chromium sceso di almeno il 30 % con N = 4 rispetto a N = 1 (tabella nella PR, anche Firefox e WebKit in locale).
   - `e2e_lr`, `e2e13`, CI verde con `sito-rust`.
6. **Prove**: `cargo test -p mzlab-core mzidx`, `python3 tools/bench.py`, `python3 tools/verifica.py --cambiati`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/indice-parallelo`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
