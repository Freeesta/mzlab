# Prompt Tabui MS2 LR — ordine dei pacchetti

Studio: `docs/studi/TABUI_MS2_LR.md`. Ogni file `M*.md` è un prompt pronto per una chat DEV: si copia la parte dopo «---» (le regole comuni sono in fondo a ciascuno). Le decisioni D1-D5 sono confermate come (a); D6 si decide in M5.

## File toccati e `.wasm`
| Pacchetto | File toccati | `.wasm` che cambia | Dipende da |
|---|---|---|---|
| M0 | `crates/mzlab-feat/src/legacy/lr/ms2.rs`; test in `crates/mzlab-finder-lr-wasm/tests/`; `docs/agenti/mzfinder-lr.md` | `finder-lr` (tetto 600 000 B) | — |
| M1 | `crates/mzlab-core/src/wiff/{mod.rs,lettore.rs}`, `crates/mzlab-core/src/mzmlwrite.rs` (+ `mzmlwrite/tests.rs`), `crates/mzlab-core/tests/wiff_parity.rs`, `tools/wiff_riferimento.py` (solo codificatore dei file di prova), dati d'oro degli mzML scritti, `docs/agenti/{wiff,mzidx,rust}.md` | motore (tetto 1 703 936 B); gli altri moduli che includono `mzlab-core` non usano `mzmlwrite`/`wiff`: controlla che il loro peso non cambi | — |
| M2 | `crates/mzlab-feat/src/legacy/lr/{ms2link*.rs,mod.rs,ms2.rs,summary.rs,tree.rs,plan.rs}`, `crates/mzlab-finder-lr-wasm/{src/lib.rs,examples/finder_lr.rs,tests/}`, `TP_Mine/files/{finder-lr.js,tpmine-worker.js}`, `TP_Mine/tests/lr_synth.py`, `docs/agenti/mzfinder-lr.md` | `finder-lr` | M0 |
| M3 | `crates/mzlab-feat/src/legacy/lr/{ms2link*.rs o ms2target.rs,rows.rs}`, test in `crates/mzlab-finder-lr-wasm/tests/`, `docs/agenti/mzfinder-lr.md`; privato: `scratchpad/consegne/TABUI_MS2_LR_DATI.md` | `finder-lr` | M2 |
| M4 | `TP_Mine/js/20_tpmine.js`, `tests_e2e/e2e_tpmine_ms2_lr.py`, `docs/agenti/mzfinder.md` | nessuno | M2 |
| M5 | `crates/mzlab-feat/src/legacy/{similarity.rs,lr/ms2.rs o ms2link*.rs,lr/summary.rs}`, test in `crates/mzlab-feat/tests/` e `crates/mzlab-finder-lr-wasm/tests/`, `TP_Mine/js/{20_tpmine.js,21_tpmine_hr.js}`, `tests_e2e/e2e_tpmine_ms2_lr.py` | `finder-lr` e `finder` HR (tetto 512 000 B: `similarity.rs` sta anche nel Dig HR) | M3 e M4 |

## Cosa si lancia insieme
- **Onda 1**: M0 e M1 insieme (file e `.wasm` diversi).
- **Onda 2** (dopo il merge di M0): M2. M1 può essere ancora in corso.
- **Onda 3** (dopo il merge di M2): M3 e M4 insieme. M3 tocca solo Rust e `finder-lr`; M4 solo `20_tpmine.js` e un e2e. L'unico punto comune è `docs/agenti/mzfinder-lr.md` (M3) e `mzfinder.md` (M4): file diversi, nessun conflitto.
- **Onda 4** (dopo il merge di M3 e M4): M5.
- In serie, per il `.wasm`: M0 → M2 → M3 → M5 cambiano tutti `finder-lr` (e M5 anche `finder`); M1 è l'unico che cambia il motore. Nessun altro pacchetto di questa serie tocca il motore, quindi M1 non deve aspettare nessuno; va solo messo in serie con altre chat (di altri studi) che cambiano il motore nello stesso momento.

Dati veri e verità dei test: repository privato `Freeesta/mzlab-dati`, `scratchpad/consegne/TABUI_MS2_LR_DATI.md` e `scratchpad/tabui_ms2_lr_truth.json`.
