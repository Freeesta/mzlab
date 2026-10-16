# Prompt delle chat DEV: motore di reattività del genitore nel browser (Tabui)

Un file per pacchetto; ognuno si copia in una chat DEV da sotto la riga «---» ed è autosufficiente (le regole comuni sono in fondo a ogni file). Lo studio è `docs/studi/CC_MOTORE_NEL_BROWSER.md`: Sintesi per Federico, §2 ricerca (A-E), §3 misura su 10 molecole, §4 raccomandazione, §5 rischi e licenze, §7 criteri di successo, §8 architettura (`crates/mzlab-chem`, `.wasm` `chem`, `reatt/1`), §9 piano. Titolo di ogni chat: inizia con «mzLab» (per esempio «mzLab CC-B2 motore GFN2 in Rust»). Tutto parte solo con il sì di Federico (Sintesi punto 8a).

## Ordine
| Passo | Pacchetto | Parte dopo | `.wasm` | Stima (giorni-chat) |
|---|---|---|---|---|
| 1 | CC-B1 crate `mzlab-chem`, geometria 3D (ETKDG + MMFF94), dati di riferimento tblite | sì di Federico | nessuno | 2,5-3 |
| 1 (parallelo) | CC-B3 viewer 3D e pannello nel riquadro «Parent compound» (con `reatt/1` di prova) | sì di Federico | nessuno | 3-3,5 |
| 2 | CC-B2 GFN2-xTB in Rust: carica, ALPB, Fukui, ripiego EHT, `.wasm` `chem` | merge di CC-B1 | `chem` (nuovo) | 5-6 |
| 3 | CC-B4 avvio automatico dopo i TP, worker a bassa priorità, stato | merge di CC-B2 e CC-B3 | `chem` (solo carico) | 1-1,5 |
| 3 (parallelo) | CC-B5 «Prev. f(−)» in `posiz/1` e nel pannello «Substitution position» | merge di CC-B2 (e CC-B3) | `grafo` | 1-1,5 |
| 3 (parallelo, facoltativo) | CC-B6 IP, EA, ω; BDE solo se validata | merge di CC-B2 + sì di Federico | `chem` | 1,5-2 |
| 4 (facoltativi) | CC-B7 tautomeri e ionizzazione qualitativa; CC-B8 polarità per l'ordine di eluizione (`CC_CATALOGO_ROWAN.md`): prompt da scrivere solo dopo il sì di Federico | merge di CC-B2 (+ coppie di isomeri con RT per B8) | `chem` | 2-3; ~2 |

CC-B1 e CC-B3 possono correre insieme (file diversi). CC-B4, CC-B5 e CC-B6 possono correre insieme: B4 tocca il worker e gli innesti in `20_tpmine.js`/`21_tpmine_hr.js`, B5 `posizioni/` e `44_posizioni.js`, B6 `indici.rs` e la scheda «Info» di `46_reattivita.js`; chi unisce per secondo fa `git merge origin/main` e prova di nuovo.

## Chi tocca cosa
- CC-B1: `Cargo.toml` (workspace), `crates/mzlab-chem/` (nuovo: `mol3d.rs`, `geom/`, `LICENSE-sci-form`, `tests/geom.rs`, `tests/data/cc_ref.json`), `tools/cc_riferimento.py`, `docs/agenti/reattivita.md` (nuovo), AGENTS.md §5 (una riga).
- CC-B2: `crates/mzlab-chem/src/xtb/`, `eht/`, `reatt.rs`, `tests/parita_gfn2.rs`, `tests/siti.rs`, `crates/mzlab-chem-wasm/` (nuovo), `tools/build_feat_wasm.sh` (voce `chem`), `crates/mzlab-desktop/src/feat_grafo.rs` (o rotta sorella), `tests_e2e/e2e_chem_tempo.py`.
- CC-B3: `TP_Mine/js/46_reattivita.js`, `TP_Mine/js/47_viewer3d.js` (nuovi), l'innesto in `20_tpmine.js` (`structure()`), `tools/tipi/pagina.json`, `tests_e2e/e2e_reattivita.py`.
- CC-B4: `TP_Mine/files/chem-worker.js` (nuovo), innesti in `20_tpmine.js` e `21_tpmine_hr.js`, desktop, `tests_e2e/e2e_reattivita_auto.py`.
- CC-B5: `crates/mzlab-feat/src/chem/posizioni/`, `crates/mzlab-feat-grafo-wasm/src/lib.rs`, `TP_Mine/js/44_posizioni.js`, `tests/posizioni_prior.rs`, `tests_e2e/e2e_posizioni.py`, `docs/agenti/posizioni.md`.
- CC-B6: `crates/mzlab-chem/src/indici.rs`, `tests/indici.rs`, scheda «Info» di `46_reattivita.js`.

## Cosa NON fanno questi pacchetti
Nessun programma di terzi nel sito (xtb, tblite, OCC, xtbloom, RDKit, Pyodide); nessuna rete; nessun livello assegnato dal programma; niente nella vista Dati, nel Disegno degli studenti, in Teoria o nella relazione; nessun cambio ai pacchetti di Attributio.
