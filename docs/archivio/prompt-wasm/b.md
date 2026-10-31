# WASM-B · `opt-level = 3` sui crate caldi dei `.wasm`
**Ramo**: `claude/wasm-o3` da `origin/main`. **Dipende da**: decisione di Federico sui tetti (§7 dello studio, punto 1). **Prima di**: nulla. Sostituisce `docs/studi/PROMPT_WASM_POTENZIALE/DEV-2.md`.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-B: ricevuto».

1. **Ruolo**: DEV Sonnet, profili di build e tetti. Nessun algoritmo nuovo, nessuna dipendenza.
2. **Contesto**: tutti i `.wasm` si costruiscono a `opt-level = "z"` (`Cargo.toml:12-16`); solo `mzlab-picchi` è a 3 (`Cargo.toml:19`).
   - Misura (`docs/studi/wasm-potenza.md` §3c, banco `tools/banco_wasm.py` con O3 su `mzlab-core`, `miniz_oxide`, `quick-xml`, `base64`, mediana di 3, stesse somme di controllo): indice HR 4254 → 3840 ms (Chromium 153), 4962 → 4452 (Firefox 157), 4174 → 4219 (WebKit 27.2); 200 composizioni 4144 → 3860, 4071 → 3142, 4173 → 3114; centroidi 2966 → 2922, 4696 → 4223, 4062 → 2823; Node 22 indice ×1,42, centroidi ×3,3.
   - Peso del banco 231 807 → 259 869 B gzip (+12 %); nel motore vero `wasm-potenziale.md` §3 stimava +32 % (1 765 995 → 2 336 860 B).
   - Il livello di `wasm-opt` non conta: resta `-Oz`. Regola dei tetti: misura × 1,10 arrotondata a 4 KiB, solo con decisione di Federico. Leggi `docs/agenti/wasm-tetti.md`, `docs/agenti/lettori-modulo.md`, `docs/agenti/rust.md`.
3. **Perimetro**: `Cargo.toml` alla radice (solo `[profile.release.package.*]`); i tetti in `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh`, le righe `tools/wasm_opt.sh … <tetto>` di `pages.yml`, `sito-rust.yml`, `release.yml` e le copie nei test (`grep -rn` del tetto attuale); righe di `wasm-tetti.md`, `rust.md`, `misure-velocita.md`. Non toccare `.cargo/`, `tools/wasm_opt.sh`, il codice.
4. **Lavoro**:
   - Per ogni `.wasm` (motore, HR, Attributio, `untargeted`, `interp`, `table`, librerie, lettori `thermo`, `sciex`, `aperti`) misura peso e tempi con O3 a turno su `mzlab-core`, `mzlab-feat`, `mzlab-lettori`, `opentfraw`, `mzlab-lib`, `miniz_oxide`, `zlib-rs`, `quick-xml`, `base64` (`cargo build --config 'profile.release.package.<crate>.opt-level=3'`). Tieni un crate solo se fa guadagnare almeno il 10 % al suo lavoro in Firefox O in WebKit (in Chromium il guadagno è piccolo: misurato 0-10 %).
   - Tempi nei tre browser: `banco_wasm.py --chromium` e, per Firefox e WebKit, `run.html` dello stesso banco servito dalla cartella `--lavoro` (come nello studio §2; Playwright in una cartella temporanea).
   - Scrivi le sezioni scelte in `Cargo.toml` (una riga di commento in inglese col perché). `desktop` e `cli` restano a 3.
5. **Accettazione**:
   - Golden, parità e e2e identici byte per byte; nessun test modificato.
   - Tabella prima/dopo (byte, gzip, ms in Chromium, Firefox, WebKit) per ogni `.wasm` nella PR; nessun tetto alzato senza la riga di decisione di Federico.
   - CI verde con `sito-rust`; `tools/pre_pr.py` verde.
6. **Prove**: `python3 tools/banco_wasm.py --varianti base,mix --chromium --lavoro /tmp/banco`, `python3 tools/verifica.py --cambiati`, `sito-rust.yml` con `solo=e2e_lr,e2e13`.
7. **Consegna**: PR aperta su `claude/wasm-o3`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
