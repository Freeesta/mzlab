# Audit di pulizia (fase 1: nessuna cancellazione)

Documento temporaneo del lavoro PULIZIA-MZLAB. Classi: **CANCELLA** (provato inutilizzato), **ARCHIVIA** (valore storico: `docs/archivio/` con una riga di indice), **TIENI**, **DECIDE FEDERICO** (dubbio). Quanto sta in `mzlab/web` e `crates/` non si tocca finché non c'è il via.

Limite delle prove: il clone del cloud ha la storia di `main` troncata (≈50 commit), quindi `git merge-base --is-ancestor` non vale (tutti i rami risultano «non in main»). Lo stato «in main» dei rami si prova con la PR unita (squash) e, dove la PR è chiusa non unita, con un grep del contenuto in `main`.

## 1. Branch remoti (36 oltre `main`)

### 1a. CANCELLA: PR unita (squash), lavoro già in main
| Ramo | PR | Nota |
|---|---|---|
| claude/flow-live-7b | #1481 unita 22/10 | avanzamento conversione |
| claude/flow-live-studio | #1449 unita 22/10 | studio solo documenti |
| claude/flow-msn-risultati | #1854 unita 27/10 | |
| claude/flow-msn-test-robusti | #1879 unita 27/10 | |
| claude/flow-review | #1169 unita 20/10 | |
| claude/infusione-msn-ui | #1263 unita 20/10 | |
| claude/notte-completa | #1848 unita 27/10 | |
| claude/raw-fast-parallel-1 | #1731 unita 25/10 | |
| claude/tooltip-inventario | #2041 unita 28/10 | di un'altra chat attiva: cancellare solo a fine suo lavoro |

### 1b. CANCELLA: temporanei e prove, senza lavoro da salvare
| Ramo | Prova |
|---|---|
| tmp-bis-39884b9, tmp-bis-435b64621, tmp-bis-97afe79e5, tmp-bis-b62325c05, tmp-bis-b68e8bfaf, tmp-bis-be52b6096 | bisect del 27/10, nessuna PR, un commit «tmp bisect» sopra main |
| claude/ci-r3-prova-core | #1442 «PROVA NON UNIRE» (costante sbagliata voluta) |
| claude/ci-solo-doc-prova | #1384 «Prova CI solo documenti (NON unire)» |
| claude/ci-misura-cache-calda | #1399 chiusa: misura della cache calda, nota nel modulo `doppi` |
| claude/flow-canvas-shots | nessuna PR, commit di testa «anteprime della tela (temporaneo, da non unire)» |

### 1c. CANCELLA con prova nel codice di main (PR chiusa non unita ma il contenuto c'è)
| Ramo | Prova in main |
|---|---|
| claude/associazioni-single-instance | nessuna PR; `crates/mzlab-desktop/Cargo.toml` e `src/apri.rs` usano single-instance |
| claude/oro-mzmine410-dati2, claude/oro-mzmine410-locale | nessuna PR; esiste `tests/data/untargeted/mzmine410/` |
| claude/perf-rowstats | nessuna PR; esiste `crates/mzlab-feat/examples/rowstats.rs` |
| claude/wasm-o3 | #1944 chiusa; `Cargo.toml` ha già `opt-level = 3` sui crate caldi |
| claude/tpmine-load-order | #1784 chiusa; `TP_Mine/js/20_tpmine.js:346` documenta e rispetta l'ordine di caricamento |
| claude/c42-ci-sito-artifact | #1026 chiusa; `.github/workflows/sito-rust.yml` costruisce già i `.wasm` in tre job paralleli e li passa come artifact |
| claude/attributio-rg02 | #1064 chiusa; `crates/mzlab-feat/src/attributio/` ha gli hazard (`checks/mod.rs`, `export/publication.rs`) |

### 1d. DECIDE FEDERICO: lavoro forse NON in main
| Ramo | Contiene |
|---|---|
| claude/ms2-insilico (#1059), claude/ms2-rearrange (#1060), claude/ms2-rt-chimerico (#1061) | Attributio MS-02/03/04 (frammentazione in silico, trasferimenti H, RT e intrusi chimerici); PR chiuse il 19/10 non unite; in main esistono pezzi (`mzlab-attributio-wasm`) ma non provo che ci sia tutto |
| claude/c35-conformeri (#1030) | media di Boltzmann sui conformeri (reattività, opzione spenta); `grep boltzmann crates` in main: vuoto, quindi NON in main |
| claude/c43b-teoria-primo-paint (#1016) | niente lampo bianco al primo paint (snippet del tema, hash CSP) |
| claude/e2e-calc (#1985 chiusa, #931 unita) | e2e6 / e2e_calc che seguono la calcolatrice nella barra laterale (probabile sostituita da CALC-COSA-E2) |
| claude/spectral-match-1 (#1667 chiusa; #1588 unita) | ricerca per identità nelle librerie più veloce (potatura dei migliori) |

### 1e. TIENI
claude/perf-lotto12 (PR #2042 aperta), claude/gemini-lotto14-verifica (verificatrici Gemini, nessuna PR, non toccare), `stat-ci` (usato da `tools/ci_metrics.py`).

## 2. File e cartelle tracciati

### 2a. CANCELLA (git rm, la storia resta)
| Voce | Prova | Peso |
|---|---|---|
| `tests_e2e/*.whl` (5 wheel: playwright 1.64.1, greenlet, axe_playwright_python, pyee, typing_extensions) | nessun riferimento in codice, workflow, docs (`grep` su nome e su «.whl»); l'ambiente li installa con pip; entrati con la PR #1991 | 49,1 MB |
| `fuzz/corpus/` (2099 file) | già in `.gitignore`; lo rigenera `fuzz/semi.sh`; tracciato per errore dalla stessa PR; `git rm --cached -r` | 1,2 MB |

I due blocchi non cambiano comportamento (le wheel non sono usate; il corpus si ricrea). Richiede un controllo: che `tools/dati_mappa.tsv` e i workflow `fuzz.yml`/`notte.yml` non leggano `fuzz/corpus` tracciato (prima di agire rilancio il grep).

### 2b. ARCHIVIA o CANCELLA (decide Federico: sono one-shot)
| Voce | Cosa è | Prova |
|---|---|---|
| `esperimenti/mirino-ui/*.png` + `tools/mirino_ui_prova.py` + `tools/righello_prova.py` | prove grafiche «prima/dopo» del mirino XIC e del righello, scrivono PNG | nessun riferimento ai due script; i PNG non sono citati |
| `tools/hr_velocita.py` | misura `api/spectra` JSON contro `api/scanbin` sul sito Pyodide, che non esiste più | nessun riferimento; descrive un sito Python/Pyodide eliminato |
| `tools/redirect-qqq-lab/` (404.html, index.html, sw.js) | redirect del vecchio sito `qqq-lab` | nessun riferimento nel repo; potrebbe servire al repository vecchio: DECIDE |
| `docs/controlli/numeri/` (RAPPORTO, RAPPORTO_BIS, due CSV identici, `controllo_numeri.py`, `controlla_intestazioni.py`, `uscita_cli/`) | controllo indipendente dei numeri, eseguito due volte | citata solo da `tools/indice.py`; `confronto_numeri_bis.csv` è identico a `confronto_numeri.csv` (stesso hash) per dimostrare la concordanza |
| `docs/manuale/` (4 file, 56 KB) | manuale studenti fermo al commit `b79e8cb` (v1.0.1, 16/10) | nessun link in entrata; rischia di descrivere un'interfaccia superata |
| `docs/ospitare.md` | piano B per ospitare altrove | nessun link in entrata; potrebbe valere come documento di emergenza: TIENI se Federico lo vuole |

### 2c. ARCHIVIA (valore storico, in `docs/archivio/`)
Prompt già eseguiti. Regola di `AGENTS.md`: `docs/agenti` descrive solo lo stato attuale.
- `docs/agenti/prompt-canali/`, `prompt-flow-ux/`, `prompt-match/`, `prompt-wasm/` (24 file, ≈190 KB). Citati da pochi file (`INDICE.md`, `canali-dati.md`, `flow-ux.md`, `match-spettrale.md`, `docs/studi/wasm-potenza.md`): vanno aggiornati i riferimenti.
- `docs/studi/PROMPT_*` (8 cartelle, ≈330 KB) e studi senza alcun link in entrata: `ATTRIBUTIO_LISTE_REGOLATORIE.md`, `CC_CATALOGO_ROWAN.md`, `CI_RUST_BINARI_PARALLELI.md`, `DAD_SPETTRO_UV.md`, `TABUI_FEATURE_TABLE.md`, `processo-macchina.md`, `msn-grafo-v2/`, `visualizzatore-veloce/`. `docs/studi/` pesa 2,1 MB in tutto.
- NON toccare: `docs/studi/gemini-*-verifica.md`.

### 2d. TIENI (capito e usato)
- `TP_Mine/` = sorgenti di mzFinder/Tabui (`tools/build_site.py:75`), `flow/` = pagina Flow (`build_site.py:91`), `paper/` = articolo JOSS (`CITATION.cff`, zenodo), `vendor/opentfraw/` (dipendenza di `mzlab-core`), `docs/bozze/teoria_cc05.md` (citata da `tests/data/reattivita` e dallo studio CC), `docs/letteratura_cc/`, `docs/zenodo.md`, `docs/azioni.md`, `docs/utenti/`, `docs/schemi/`.
- `tests/data/gruppi/*.mzML` (38 file sintetici): ignorati da `.gitignore` ma forzati apposta, usati da `lr_gruppi.rs`, `e2e_tabui_gruppi.py`. `tests/golden/ref/msntree_rotta/msn.mzML` e `crates/mzlab-lettori/tests/rosina/sintetico.mzML`: idem.
- File grandi > 500 KB giustificati: `mzlab/web/vendor/` (Ketcher, openchemlib, editor libro), `mzlab/web/esempi/*.mzML` (esempi pubblici), `tests/golden/**` (verità di parità).
- Tutti gli script `tools/*.py` e i test `tests/`, `tests_e2e/e2e*.py` sono raccolti per glob (`tools/verifica.py:377`, pytest): non sono morti per assenza di riferimenti. Unici script di `tools/` senza chiamanti: quelli di 2b e `genera_anteprima.py`, `genera_icone.py` (generatori manuali, TIENI).
- `mzlab/web/*.js`: nessun file orfano (i `lang/it|en/*.js`, `teoria/pratica/ei-dati-l*.js`, `teoria/strumenti/massa.js` si caricano per nome dinamico).
- Duplicati: fixture di test con lo stesso contenuto (mzmine39/410, eventi LLM, `msn_figura/testi-*`): sono voluti per tenere i test indipendenti, TIENI. `crates/mzlab-desktop/icons/icon.png` = `mzlab/web/app-icon-512.png`: TIENI (due consumatori). Test `#[ignore]` (11) tutti con motivo o misure lunghe: TIENI.

## 3. Cartelle NON tracciate (non sono nel repo)
`graphify-out/` (93 MB, già in `.gitignore`), `target/`, `.verifica/`, `site/`: solo da citare; si possono cancellare in locale senza effetti.

## 4. Link rotti nei documenti
Nuovo `tools/controlla_link_docs.py` (solo rapporto, non in `pre_pr`): 67 riferimenti rotti in 35 documenti. Quelli veri sono in:
- `docs/studi/` (prompt e studi: `tests/…` di crate, `TP_Mine/py/` e `TP_Mine/js/46_…` spariti, `tools/rigenera_reatt.py`, `tools/controlla_dati.py`, `mzlab/web/tutor/*`, `docs/agenti/assistente-dati.md`): si risolvono archiviando.
- `docs/agenti/`: `flow-pacchetto-tabui.md` (`ms2link.rs`), `match-spettrale.md` (`flow_attribute.rs`), `mzfinder-validazione.md` (`TP_Mine/data/cancello_tp.json`), `flow-web.md` (`mzlab/flow/prov.json`), `prompt-match/d.md`, `prompt-wasm/f.md`, `rilascio.md` (`CHECKLIST-vX.Y.Z.md`: il file è `CHECKLIST-TEMPLATE.md`), `libro.md` e `llm.md` (segnaposto `NN-slug`, `E1..E10`: non rotti), `test.md` (`mappa.md` è generata e ignorata: non rotto).
- `docs/inventari/TABUI_INGLESE.md` (`TP_Mine/py` non esiste più).
Alcuni potrebbero appartenere a PR di altre chat non ancora unite (tutor, LLM, CLI): le correggo solo dopo il via, una per una.

## 5. AGENTS.md, CLAUDE.md, CHANGELOG.md
- Non c'è un CLAUDE.md. `AGENTS.md` (14,5 KB): nessun percorso rotto. Unica voce da rivedere: «Python = riferimento» e `mzlab/chem` «Python» (§4) coesistono con la regola «Tutto in Rust»; è già dichiarato, TIENI.
- `CHANGELOG.md` (4,5 KB): «Unreleased» ordinato, nessuna parte obsoleta.
- `docs/agenti/` ha 167 file; nessuno orfano; solo `rust-flow.md` è al limite dei 25 KB.

## 6. Non fatto in questa fase
- Codice morto Rust (`dead_code`/clippy): non girato (il cloud non ha `target/`, la build del workspace è lunga e `crates/` è occupata da altre chat). Da fare dopo il via.
- Dati di laboratorio: nessuno trovato nei file tracciati (i mzML tracciati sono sintetici o esempi anonimi); nessun cognome controllato contro `nomi_vietati.txt` (il privato non è raggiungibile qui).
