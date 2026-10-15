# Prompt delle chat DEV di Attributio (pipeline untargeted HR in mzLab)

Un file per pacchetto; ognuno si copia in una chat DEV (Sonnet) da sotto la riga «---». Lo studio è `docs/studi/ATTRIBUTIO.md` (§4 piano, §2.9 attribuzioni, §3 flusso 2b e domande di prodotto Q1-Q8 con le scelte predefinite che i prompt seguono).

## Ordine e onde
| Onda | Pacchetti (in parallelo dentro l'onda) | Parte dopo il merge di |
|---|---|---|
| 1 | AT-00 diagnosi e dati d'oro · AT-08 `decisioni/1` · AT-07a formule (solo crate) | — |
| 2 | AT-01 ADAP · AT-02 risolutore · AT-03 join e riempimento | AT-00 |
| 2 | AT-09 Banco 2b + `explain_spectrum` | AT-08 |
| 3 | AT-04 pipeline, `.wasm` `untargeted`, pannello, desktop, cancello | AT-01, AT-02, AT-03 |
| 4 (in serie) | AT-05 isotopi e identità → AT-06 spettro unito e annotazione → AT-07b formule collegate | AT-04, poi il precedente |
| 5 | AT-10 esportazioni e rapporto | AT-06, AT-08, AT-09 |

## Chi tocca cosa
| Pacchetto | `.wasm` | File condivisi (chi unisce per secondo fa `git merge origin/main`) |
|---|---|---|
| AT-00 | nessuno | `AGENTS.md` §5 (una riga), `tools/valida/cancello_misure.py`, `tools/bench/` |
| AT-01, AT-02, AT-03 | nessuno (crate, feature `untargeted`) | `crates/mzlab-feat/Cargo.toml` e `lib.rs` (stessa riga: chi arriva per secondo la trova già), `LICENZE-TERZI.md` (una riga di mzmine), `docs/agenti/attributio.md` |
| AT-04 | `untargeted` (nuovo) | `tools/build_feat_wasm.sh` (lista del ciclo), `TP_Mine/files/feat-worker.js`, `feat-desktop-pool.js`, `crates/mzlab-desktop/src/feat.rs`, `32/34/35_pipeline*.js`, `TP_Mine/data/cancello.json` |
| AT-05, AT-06, AT-07b | `untargeted` (in serie) | `27_untarget_ui.js` (ordine AT-09 → AT-06 → AT-10), `26_untarget_match.js`, `28_untarget_export.js`, `37_feature_table.js`, `livelli-1.json` (AT-07b) |
| AT-07a | nessuno | — |
| AT-08 | `table` | `mzlab/web/workflow.js` (campo `decisioni` del `.mzflow`), `26_untarget_match.js` (`decide`) |
| AT-09 | `grafo` | `27_untarget_ui.js` |
| AT-10 | nessuno | `27_untarget_ui.js`, `28_untarget_export.js` |

Nessun pacchetto tocca il motore, `detect`, `proofs`, `mrm`, `finder`, `finder_msn` o `finder-lr`. Non collidono con RAW veloce (motore, lettori, `finder-idx.js`, `tpmine-worker.js`, `20_tpmine.js`), Tabui MS2 LR (`legacy/lr/`, `finder-lr`, `similarity.rs`, `finder` in M5) e formati nativi (lettori).

## Soglie (dettagli nei prompt)
- Parità per stadio contro mzmine 3.9.0: ≥ 0,99 sui sintetici (`tests/data/untargeted/mzmine39/`), ≥ 0,97 sui dati veri del privato.
- Cancello della ricetta `untargeted` (AT-04, protocollo corretto di AT-00): recall ≥ 0,95, precisione ≥ 0,95, Spearman ≥ 0,95, |log2| ≤ 0,10; anche il protocollo vecchio deve passare 0,90 / 0,80 / 0,90 / 0,25. Recall per file della rilevazione ≥ 0,95.
- Ogni `.wasm` < 512 000 B; il peso va nella descrizione della PR.
