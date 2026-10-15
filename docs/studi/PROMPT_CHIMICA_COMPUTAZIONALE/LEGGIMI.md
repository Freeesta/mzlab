# Prompt delle chat DEV: posizione della sostituzione negli isomeri dei TP

Un file per pacchetto; ognuno si copia in una chat DEV da sotto la riga «---» ed è autosufficiente (le regole comuni sono in fondo a ogni file). Lo studio è `docs/studi/CHIMICA_COMPUTAZIONALE.md`: §3 prove da MS, §4 fattibilità, §5 rischi, §6 piano, §7 domande D1-D5 con le scelte predefinite che i prompt seguono. Titolo di ogni chat: inizia con «mzLab» (per esempio «mzLab CC-01 posizioni della sostituzione»).

## Ordine
| Passo | Pacchetto | Parte dopo | `.wasm` | Stima (giorni-chat) |
|---|---|---|---|---|
| 1 | CC-01 posizioni compatibili con la MS2 (Rust, contratto `posiz/1`, `docs/agenti/posizioni.md`) | — | nessuno | 1,5 |
| 2 | CC-02 pannello «Substitution position» in Tabui (amministratore) | merge di CC-01 | `grafo` | 1,5 |
| 3 (facoltativo) | CC-03 stima di reattività «Prev.», solo se passa la validazione | merge di CC-02 + sì di Federico | `grafo` | 2 |
| 3 (facoltativo) | CC-04 importazione di calcoli esterni (`siti/1`, xtb `--vfukui`) | merge di CC-02 + sì di Federico | `grafo` | 1 |

CC-01 e CC-02 vanno in serie. CC-03 e CC-04 possono correre in parallelo tra loro; toccano entrambi `grafo` e `posiz/1`: chi unisce per secondo fa `git merge origin/main` e prova di nuovo.

## Chi tocca cosa
- CC-01: `crates/mzlab-feat/src/chem/posizioni.rs` (nuovo), `crates/mzlab-feat/tests/posizioni.rs`, `docs/agenti/posizioni.md` (nuovo), AGENTS.md §5 (una riga).
- CC-02: `crates/mzlab-feat-grafo-wasm/src/lib.rs`, `crates/mzlab-desktop/src/feat_grafo.rs`, `TP_Mine/js/44_posizioni.js` (nuovo) e il punto del dettaglio di riga in Tabui, `tools/tipi/pagina.json`, `tools/build_feat_wasm.sh` (solo il tetto di `grafo`, se serve), `tests_e2e/e2e_posizioni.py`, `docs/agenti/posizioni.md`, una riga in `docs/agenti/mzfinder.md`.
- CC-03: `crates/mzlab-feat/src/chem/reattivita.rs`, `tests/reattivita.rs`, `tests/data/oh_regio.csv`, poi (se pubblicato) `44_posizioni.js`.
- CC-04: `crates/mzlab-feat/src/chem/siti.rs`, `tests/siti.rs`, `tests/data/` (uscite xtb), `44_posizioni.js`.

## Cosa NON fanno questi pacchetti
Nessun calcolo quantistico nel sito; nessun livello assegnato dal programma; niente nella vista Dati, nel Disegno degli studenti, in Teoria o nella relazione; nessun cambio ai pacchetti di Attributio (il Banco 2b di AT-07 potrà leggere `posiz/1` quando nasce: è una decisione del Coordinatore, non di questi pacchetti).
