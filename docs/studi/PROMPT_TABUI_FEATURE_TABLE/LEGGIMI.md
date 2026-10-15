# Prompt Tabui feature table — ordine dei pacchetti

Studio: `docs/studi/TABUI_FEATURE_TABLE.md`. Ogni file `FT-*.md` è un prompt pronto per una chat DEV (Sonnet): si copia la parte dopo «---» (le regole comuni sono in fondo a ciascuno). Le decisioni D1-D5 (§9 dello studio) sono prese come (a) finché il Coordinatore non dice altro. Nessun pacchetto tocca Rust o i `.wasm`.

## File toccati
| Pacchetto | File toccati | Dipende da |
|---|---|---|
| FT-01 testi it/en e aiuto | `TP_Mine/js/{40_tabui_testi.js,41_tabui_aiuto.js}` (nuovi), `TP_Mine/tests/test_tabui_testi.py`, `tests_e2e/e2e_tabui_aiuto.py`, `docs/agenti/mzfinder.md` | — |
| FT-02 componente `TPTAB` | `TP_Mine/js/42_tp_table.js`, `tests_e2e/e2e_tabui_tptab.py`, `docs/agenti/mzfinder-tabella.md` | — |
| FT-03 tabella HR | `TP_Mine/js/21_tpmine_hr.js`, `tests_e2e/{e2e_tpmine_tabella,e2e_tpmine_ms2}.py`, `docs/agenti/{mzfinder,mzfinder-tabella,livelli}.md` | FT-01, FT-02; serie con M5 |
| FT-04 tabella LR | `TP_Mine/js/20_tpmine.js`, `tests_e2e/e2e_tpmine_lr_tabella.py`, selettori degli e2e LR, `docs/agenti/{mzfinder,mzfinder-tabella}.md` | FT-01, FT-02, M4; serie con M5 e RV-11 |
| FT-05 Feature table della Pipeline | `TP_Mine/js/37_feature_table.js`, una riga di `34_pipeline_run.js`, `tests_e2e/e2e_pipeline.py`, `docs/agenti/mzfeat-wasm.md` | FT-01, FT-02, MZ-04, MZ-05 |
| FT-06 matrice «Samples» | `TP_Mine/js/44_tp_matrix.js` (nuovo), una riga di `42_tp_table.js`, `tests_e2e/e2e_tabui_tptab.py`, `docs/agenti/mzfinder-tabella.md` | FT-02 |
| FT-07 livello deciso | `TP_Mine/js/45_tabui_decisioni.js` (nuovo), una riga in `20_tpmine.js`, `21_tpmine_hr.js`, `37_feature_table.js`, `tests_e2e/e2e_tabui_decisioni.py`, `docs/agenti/mzfinder-tabella.md` | FT-03, FT-04, FT-05 |
| FT-08 tabelle esterne e Attributio | `TP_Mine/js/{27_untarget_ui.js,51_attributio_table.js}`, `tests_e2e/{e2e_untarget,e2e_attributio}.py`, `docs/agenti/{mzfinder-untargeted,attributio}.md` | FT-02, AT-04 (e AT-07 se aperto) |

## Cosa si lancia insieme
- **Onda 1**: FT-01 e FT-02 insieme (file diversi; FT-02 usa `TPHELP`/`TT` solo se ci sono).
- **Onda 2** (dopo FT-01 e FT-02): FT-03 se M5 non è aperta; FT-06 subito; FT-04 appena M4 è unita e M5/RV-11 non sono aperte; FT-05 appena MZ-04 e MZ-05 sono unite. FT-03, FT-04, FT-05, FT-06 toccano file diversi e possono correre insieme; l'unico documento in comune è `docs/agenti/mzfinder-tabella.md`: chi unisce per secondo fa `git merge origin/main` e tiene le righe di tutti.
- **Onda 3**: FT-07 dopo FT-03, FT-04, FT-05; FT-08 dopo AT-04.
- Con gli altri studi: M3 (solo Rust) e AT-02 (`livelli-1.*`, crate) non incrociano nessun pacchetto FT; M4/M5 e RV-11 condividono `20_tpmine.js`/`21_tpmine_hr.js` con FT-03/FT-04 (in serie); MZ-04/MZ-05 condividono `37_feature_table.js` con FT-05 (in serie); AT-04/AT-07 precedono FT-08.
