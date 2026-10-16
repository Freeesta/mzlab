# Prompt delle chat DEV: gruppi di esperimenti in Tabui (condizioni a confronto)

Un file per pacchetto; ognuno si copia in una chat DEV da sotto la riga «---» ed è autosufficiente (le regole comuni sono in fondo a ogni file). Lo studio è `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md`: Sintesi per Federico, §1 cosa c'è già, §2 decisioni, §3 contratto `gruppi/1`, §4 flusso, §5 Rust e JS, §6 HR, §7 rischi, §8 criteri di successo, §9 ordine, §10 domande. Titolo di ogni chat: inizia con «mzLab» (per esempio «mzLab GR-1 gruppi in Rust»). Pensati per DEV Sonnet.

## Ordine
| Passo | Pacchetto | Parte dopo | Stima (giorni-chat) |
|---|---|---|---|
| 1 | GR-1 modulo Rust `gruppi/`, `gruppi/1`, schema, fixture | subito | 2 |
| 1 (parallelo) | GR-3 tabella dei file con «Condition», riferimento, caricamento pigro | subito | 1,5-2 |
| 1 (parallelo) | GR-4 vista «Compare conditions» ed esportazioni | subito (fixture) | 2-2,5 |
| 2 | GR-2 Dig LR per condizione, dati sintetici, `gruppi/1` nel `.wasm` `finder-lr` | merge di GR-1 | 2 |
| 2 (parallelo) | GR-6 HR: `gruppi/1` da `tptab/1`, rete per condizione | merge di GR-1 | 1,5 |
| 3 | GR-5 innesto nella tabella TP, MS2 per condizione, e2e completo, misura con 40 file | merge di GR-2, GR-3, GR-4 e delle PR #863, #871, #873 | 1,5-2 |

## Chi tocca cosa
- GR-1: `crates/mzlab-feat/src/gruppi/` (nuovo), `crates/mzlab-feat/src/lib.rs` (una riga), `crates/mzlab-feat/tests/gruppi*.rs`, `crates/mzlab-feat/tests/data/gruppi_fixture.json`, `docs/schemi/gruppi-1.json`, `docs/agenti/gruppi.md` (nuovo), AGENTS.md §5 (una riga).
- GR-2: `crates/mzlab-feat/src/legacy/lr/`, `legacy/series.rs`, `crates/mzlab-finder-lr-wasm/` (`lib.rs`, `tests/lr_gruppi.rs`), la rotta desktop del Dig LR, `tools/dati_sintetici.py` (sottocomandi `gruppi`, `gruppi-kin`), `docs/agenti/mzfinder-lr.md`, `docs/agenti/misure.md`.
- GR-3: `TP_Mine/lazy/gruppi-campioni.js` (nuovo), `tools/build_site.py` (`pack_mzfinder`), `mzlab/web/tpmine-loader.js`, `tools/tipi/pagina.json`, `tests_e2e/e2e_tabui_gruppi_file.py`; `20_tpmine.js` solo se #871 e #873 sono in `main`.
- GR-4: `TP_Mine/lazy/gruppi-vista.js` (nuovo), `tools/tipi/pagina.json`, `tests_e2e/e2e_tabui_gruppi_vista.py`, una fixture di prova.
- GR-5: `TP_Mine/js/20_tpmine.js`, `TP_Mine/js/42_tp_table.js`, eventualmente `tour.js`, `tests_e2e/e2e_tabui_gruppi.py`, `docs/agenti/misure.md`.
- GR-6: `crates/mzlab-feat/src/tp/` (`tabella.rs`, `rete.rs`, eventuale `gruppi.rs`), `crates/mzlab-feat-table-wasm` se serve, `crates/mzlab-feat/tests/tp_gruppi.rs`, `docs/agenti/mzfeat-tp.md`.

GR-3 e GR-4 toccano entrambi `tools/tipi/pagina.json` (una riga ciascuno): chi unisce per secondo fa `git merge origin/main` e prova di nuovo.

## Cosa NON fanno questi pacchetti
Nessun TP «confermato» dal programma e nessun livello di Schymanski; nessuna chiave `lang/` né testo italiano in Tabui; nessun dato reale; nessun cambio a `campioni/2` (due fattori per condizione = pacchetto futuro `campioni/3`, solo se Federico lo chiede, domanda Q2); niente nella vista Dati, nel Disegno, in Teoria o nella relazione; nessun file nuovo in `TP_Mine/js/` (si carica tutto all'avvio).
