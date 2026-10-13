# mzFinder: lo Scavo a bassa risoluzione in Rust

Lo Scavo («Dig») di mzFinder su una serie a risoluzione unitaria (3200 QTRAP, full scan) gira in Rust: le righe della tabella sono i picchi della pipeline di `crates/mzlab-feat` (profilo `lr`: rilevamento, allineamento, ipotesi L7 con `regole_tp.json`), poi un adattatore scrive il JSON di `api.run` / `api.detail` del motore Python (`TP_Mine/py/tpmine/engine.py`), che `20_tpmine.js` e `24_tpmine_diretta.js` leggono senza modifiche.

## File
- `crates/mzlab-feat/src/legacy/lr/`: `plan.rs` (file, colonne, progenitore, impostazioni; `plan_json` = `{rust, lc, minutes, formula}`), `rows.rs` (calibrazione sul progenitore, candidati dal registro delle ipotesi, isomeri, ioni inattesi, isotopi), `labels.rs` (criteri e punteggio di `score_candidate`), `kin.rs` (decadimento del progenitore, generazione cinetica), `summary.rs` (`summary`, `live`, livelli 4/5), `detail.rs` (`detail` con gli XIC dai `.mzidx`, `frag_tree`, `frag_tree_compare`), `mod.rs` (`dig`).
- `crates/mzlab-finder-lr-wasm`: `.wasm` a sé (`wasm-feat/finder-lr/`, voce `finder_lr` del manifest, limite 512 000 B): `finder_lr_plan`, `finder_lr_dig` → `FinderLrDig{summary(timing), live(), detail(id, read, sizes), frag_tree(), frag_tree_compare(id)}`. Esempio nativo `finder_lr` (stessa pipeline del browser, per l'e2e).
- `TP_Mine/files/finder-lr.js` (modulo importato da `tpmine-worker.js`): `run(ctx, args)` → JSON di `api.run` oppure `false` (si passa a Pyodide); `ofDig(ctx, fn, args)` per `detail`, `frag_tree`, `frag_tree_compare` quando `last === "rust-lr"`.

## Quando gira in Rust
- Tutti i file di tipo `full`, ognuno con il suo `.mzidx` in OPFS (`mzlab/idx/`), progenitore con formula o SMILES (`plan.rust`), trasformazioni predefinite. Altrimenti (file MS2/EPI o MRM, solo m/z del progenitore, trasformazioni proprie) risponde Pyodide.
- Ordine nel worker: detect (fase `calibration`), ipotesi (`candidates`), allineamento e Scavo (`xic`, con il carico `live()` dei candidati che passano), poi `unexpected`, `ms2`, `tables`.

## Semantica
- Calibrazione: riga del progenitore entro 0.6 Da (area massima); offset = osservato - calcolato, applicato se `auto_offset` e |offset| ≤ 0.6.
- Candidati: ipotesi `[M+H]+` del registro (fonti regola/rottura, passi ≥ 1) per (passi, m/z); un'ipotesi entro la tolleranza del progenitore diventa sua alternativa. Isomeri: fino a 4 picchi (≥ 10 % del più forte), id + 10000·j, nome «etichetta @ RT min».
- Criteri sulle celle della tabella (stato DETECTED = picco): picco nei trattati, bianco e t0 ≤ 0.2, crescita (Spearman ≥ 0.8 fino al massimo, salita ≥ ×2); punteggio = quota superata (≤ 20 senza picco); forte ≥ 75, possibile ≥ 50, debole.
- Isotopo: ione più forte (area ≥ 2×) a -1.00335 / -2.0067 / -1.99580 Da entro tolleranza e RT: criterio fallito, punteggio ≤ 40, debole.
- Ioni inattesi: righe libere che passano i criteri, id da 100001, al più `max_unexpected` per punteggio e area. `insource` = inatteso più leggero del progenitore entro 0.5·`rt_tol_min` da lui; `isf` = null; `mrm` = null.
- Senza MS2: livelli 4/5 come `confidence`, `ms2` vuoto, `frag_tree` con la nota «no MS2 of the parent in the files».

## Prove
- `cargo test --release -p mzlab-finder-lr-wasm` (`tests/lr_dig.rs`): la serie di `TP_Mine/tests/lr_synth.py` riscritta in Rust; chiavi e tipi uguali a `tests/golden/finder_lr_contratto.json` (`tools/finder_lr_riferimento.py --golden`, motore Python), ≥ 95 % delle righe Python progenitore/forte/possibile ritrovate entro ±0.35 Da e ±0.25 min, ogni candidato con le sue prove, offset ≈ 0.30, isotopo e bianco non promossi, determinismo.
- `tests_e2e/e2e_tpmine_lr_rust.py`: Scavo nel browser uguale all'esempio nativo (run e detail), nessuna richiesta `pyodide*`, `wasm-feat/finder-lr` scaricato.
