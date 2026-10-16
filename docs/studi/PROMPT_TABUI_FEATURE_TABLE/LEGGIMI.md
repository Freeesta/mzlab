# Prompt Tabui feature table — ordine dei pacchetti

Studio: `docs/studi/TABUI_FEATURE_TABLE.md`. Decisione di Federico (§1): tabella tipo mzmine con poche colonne; un clic espande la riga sul posto con tutto ciò che si sa del risultato. Ogni file `FT-*.md` è un prompt pronto per una chat DEV (Sonnet): si copia la parte dopo «---» (le regole comuni sono in fondo a ciascuno). Le decisioni minori D1-D4 (§8 dello studio) sono prese come (a) finché il Coordinatore non dice altro. FT-02 (`TPTAB`), FT-03 (`riga/1`, `riga.rs`) e FT-04 (`TPRIGA`, `44_tp_riga.js`) sono fatti: i loro prompt sono stati rimossi e FT-05…FT-09 li usano.

## File toccati
| Pacchetto | File toccati | Dipende da |
|---|---|---|
| FT-06 tabella HR | `TP_Mine/js/21_tpmine_hr.js`, `tests_e2e/{e2e_tpmine_tabella,e2e_tpmine_ms2}.py`, `docs/agenti/{mzfinder,mzfinder-tabella,livelli}.md` | FT-01, FT-02, FT-04; serie con M5 |
| FT-07 tabella LR | `TP_Mine/js/20_tpmine.js`, `tests_e2e/e2e_tpmine_lr_tabella.py`, selettori degli e2e LR, `docs/agenti/{mzfinder,mzfinder-lr,mzfinder-tabella}.md` | FT-01, FT-02, FT-04, M4; serie con M5 e RV-11 |
| FT-08 livello deciso | `TP_Mine/js/45_tabui_decisioni.js` (nuovo), una riga di `44_tp_riga.js`, `tests_e2e/e2e_tabui_decisioni.py`, `docs/agenti/mzfinder-tabella.md` | FT-04 e una fra FT-05/06/07 |
| FT-09 tabelle esterne e Attributio | `TP_Mine/js/{27_untarget_ui.js,29_untarget_identity.js,51_attributio_table.js}`, `tests_e2e/{e2e_untarget,e2e_attributio}.py`, `docs/agenti/{mzfinder-untargeted,attributio}.md` | FT-02, FT-04, MZ-05, AT-04 |

## Cosa si lancia insieme
- **Onda 1**: fatta (FT-01…FT-04).
- **Onda 3**: FT-05 (dopo MZ-04: è il pacchetto centrale), FT-06 (se M5 non è aperta), FT-07 (dopo M4, con M5 e RV-11 chiuse), FT-08. Toccano file diversi e possono correre insieme; il documento comune `docs/agenti/mzfinder-tabella.md` si unisce con `git merge origin/main` tenendo le righe di tutti.
- **Onda 4**: FT-09 dopo AT-04.
- Con gli altri studi: M3 (solo Rust del LR) e AT-02 (`livelli-1.*`) non incrociano nessun pacchetto FT; M4/M5 e RV-11 condividono `20_tpmine.js`/`21_tpmine_hr.js` con FT-06/FT-07 (in serie); MZ-04/MZ-05 condividono `37_feature_table.js`, `29_untarget_identity.js` e `crates/mzlab-feat` con FT-03/FT-05/FT-09 (dopo); AT-04/AT-07 precedono FT-09.
