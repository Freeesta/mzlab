# DEV-2 — `opt-level = 3` sui crate caldi dei `.wasm`

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte SOLO dopo la decisione di Federico sui tetti (§8 di `docs/agenti/wasm-potenziale.md`) e dopo DEV-1 e DEV-3 in `main`.

---

**Ruolo.** Sei DEV-2 WASM-O3 (Sonnet). Cambi il profilo di ottimizzazione e i tetti decisi; nessun algoritmo nuovo.

**Contesto.** Tutti i `.wasm` si costruiscono col profilo `release` a `opt-level = "z"`. Lo studio `docs/agenti/wasm-potenziale.md` §3 misura con `opt-level = 3` (Chromium 153): lettura mzML ×1,5, indice ×1,4, XIC + picchi ×2,7, inflate ×1,5; uscite identiche. Con O3 solo su `mzlab-core`, `miniz_oxide`, `quick-xml`, `base64` (`[profile.release.package.<crate>]`) il banco è veloce come O3 su tutto; pesi: motore 1 765 995 → 2 336 860 B (+32 %), HR +2,8 %, Attributio +0,7 %. Il livello di `wasm-opt` non conta: resta `-Oz`. Regola dei tetti: misura × 1,10 arrotondata a 4 KiB, solo con decisione di Federico. Leggi `docs/agenti/wasm-tetti.md`, `docs/agenti/lettori-modulo.md`, `docs/agenti/rust.md` (profili).

**Perimetro.** `Cargo.toml` alla radice (solo le sezioni `[profile.release.package.*]`); i tetti in `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh`, le righe `tools/wasm_opt.sh … <tetto> engine` di `pages.yml`, `sito-rust.yml`, `release.yml`, le copie dei tetti nei test (`tests/test_lettori_wasm.py` e quanto trovi con `grep -rn 1884160`), e le righe di `wasm-tetti.md`, `lettori-modulo.md`, `rust.md`. Non toccare `.cargo/`, `tools/wasm_opt.sh`, il codice.

**Lavoro.**
1. Misura per crate: costruisci motore, HR, Attributio, `untargeted`, `interp`, `table`, librerie e i lettori `thermo`, `sciex`, `aperti` con O3 a turno su `mzlab-core`, `mzlab-feat`, `mzlab-lettori`, `opentfraw`, `mzlab-lib`, `miniz_oxide`, `quick-xml`, `base64` (`cargo build --config 'profile.release.package.<crate>.opt-level=3'`); per ognuno peso dopo `wasm-opt` e tempi col banco di DEV-5 (`tools/banco_wasm.py`) o `tools/bench.py`/`tools/bench_raw.py`. Tieni un crate solo se il guadagno sul suo lavoro è ≥ 10 % (regola: niente peso senza misura).
2. Scrivi in `Cargo.toml` le sezioni scelte (commento in inglese di una riga col perché). `desktop` e `cli` ereditano da `release`: controlla che restino a 3 ovunque.
3. Tetti: solo quelli che Federico ha approvato, alla misura × 1,10 arrotondata a 4 KiB; ogni altro modulo deve restare sotto il suo tetto attuale.
4. Parità: test di parità, golden e e2e identici; peso e tempi prima/dopo nella PR; righe in `misure-velocita.md` e `wasm-tetti.md`.

**Accettazione.** CI verde con `sito-rust`; nessun tetto alzato senza la riga di decisione di Federico; tabella prima/dopo (byte, gzip, ms) per ogni `.wasm`; `tools/pre_pr.py` verde.

**Prove.** `tools/banco_wasm.py --varianti base,mix --chromium`; `python3 tools/verifica.py --cambiati`; `sito-rust.yml` con `solo=e2e_lr,e2e13`.

**Consegna.** Ramo `claude/wasm-o3`, PR aperta; messaggio al Coordinatore (≤ 5 righe).
