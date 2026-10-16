# GR-2 — Dig LR per condizione: cinetiche separate, `gruppi/1` nell'uscita, dati sintetici multi-condizione

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di GR-1.

---

Pacchetto GR-2 dello studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (leggi Sintesi, §1, §2, §3, §6, §7, §8). File d'area: `docs/agenti/gruppi.md` (lo crea GR-1), `docs/agenti/mzfinder-lr.md`, `docs/agenti/campioni.md`, `docs/agenti/memoria.md`. Codice: `crates/mzlab-feat/src/legacy/lr/` (`mod.rs`, `plan.rs`, `kin.rs`, `labels.rs`, `rows.rs`), `crates/mzlab-feat/src/legacy/series.rs` (`FileIn`), `crates/mzlab-finder-lr-wasm/src/lib.rs`, `TP_Mine/files/finder-lr.js` (solo per leggere come arrivano i file).

## Obiettivo
Il Dig LR riceve per ogni file anche la condizione (e la replica), calcola cinetiche e criteri **per condizione** invece che su tutti i file insieme, e aggiunge all'uscita il blocco `gruppi` (`gruppi/1` di GR-1). Con una sola condizione l'uscita resta identica byte per byte (studio §8.1).

## Da fare
1. **Ingresso**: `FileLr`/`FileIn` prendono `cond: String` (vuoto = condiviso) e `replica: Option<u32>`; `finder_lr_plan` e l'ingresso del Dig accettano anche il `campioni/2` canonico (`{"campioni": ...}` facoltativo: se c'è, ruoli, tempi, condizioni e repliche vengono da lì con `Campione::role()`, altrimenti dai campi di oggi `type/time`). Nessun cambio a `campioni/2`.
2. **Cinetiche per condizione**: `kin::kinetics` e `decay` per una condizione (i trattati della condizione); `labels::score` usa la cinetica della condizione e i controlli della condizione più quelli condivisi. Con più condizioni la riga porta i criteri della condizione in cui è più forte (il JSON di oggi resta valido) e il dettaglio per condizione sta nel blocco `gruppi`. Con un tempo per condizione nessun «grows over time».
3. **Celle delle condizioni assenti**: verifica che `rows::cells` dia celle riempite (non assenti) per ogni file full quando la riga esiste in un'altra condizione; se l'allineamento non riempie, usa il riempimento già esistente (`gapfill.rs` o il percorso `filled` di `Aligned`) senza nuova ricerca dei picchi.
4. **`gruppi/1` nell'uscita** del Dig (`"gruppi": {...}`, solo con ≥ 2 condizioni; con 1 la chiave non c'è): costruisci `GroupTable` dalle celle, `RowMeta` con `tp_name` di `nomi::tp_names` se è già in `main` (PR #871), altrimenti con l'etichetta di oggi e una nota nel documento; `ms2_files` dagli abbinamenti MS2 se #873 è in `main`, altrimenti vuoto. Opzioni (`reference`, `min_rep_frac`, `min_area`) da `Settings` (`plan.rs`), con i default di GR-1.
5. **Polarità**: `other_polarity` (studio §2.3): riga di polarità opposta con Δ massa neutra entro `tol_da` e RT entro `rt_tol_min`.
6. **Desktop**: stessa uscita nativa (cerca la rotta del Dig LR in `crates/mzlab-desktop`, `docs/agenti/rust-desktop.md`).
7. **Dati sintetici**: sottocomando `python3 tools/dati_sintetici.py gruppi OUT_DIR` (riusa `full_scan` con un parametro per condizione: ampiezze e costanti di formazione diverse, un TP solo in «O2», uno assente in «N2», uno anche nel T0, deriva di RT ±0,1 min per corsa): 6 condizioni a endpoint + `blank` + `T0` + `STD-parent`; e `gruppi-kin`: 3 condizioni × 5 tempi × 2 repliche. Nomi dei file come `G_O2_t60`, `G_N2_t60`, `G_Reference_t60`, `G_blank`, `G_T0`, `G_STD-parent` (la condizione si ricava con `campioni::proponi`). Scrivi la verità in un JSON accanto (`gruppi_truth.json`). Stessa serie riscritta in Rust per i test di `crates/mzlab-finder-lr-wasm/tests/` (come `lr_dig.rs`).

## Prove
- `crates/mzlab-finder-lr-wasm/tests/lr_gruppi.rs`: criteri §8.2, §8.3, §8.4 sulla serie sintetica; con una sola condizione uscita uguale ai test d'oro (`lr_dig.rs`, `tests/golden/finder_lr_contratto.json`).
- Misura (scritta in `docs/agenti/misure.md`, sezione Dig LR): 20 e 40 file full sintetici, tempo e memoria di picco nel browser (`tools/bench.py` o uno script Playwright), criterio §8.5.
- `e2e_tpmine_lr_rust.py` e `e2e_tpmine_lr_tabella.py` restano verdi.

## Atteso
Dig LR per condizione con `gruppi/1`, dati sintetici, misura. Peso del `.wasm` `finder-lr` dopo `wasm-opt -Oz` sotto il suo limite di `tools/build_feat_wasm.sh` (scrivi la misura nella PR; se lo supera, fermati e chiedi).

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`). Il titolo della chat inizia con «mzLab» (per esempio «mzLab GR-1 gruppi in Rust»). Il Coordinatore è la sessione che ti ha mandato questo prompt (il suo id è nel messaggio di avvio): a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: PR, CI, pages, misure, cosa resta). Se manca l'id, scrivi lo stesso riassunto nella descrizione della PR.
- AGENTS.md è nel tuo contesto: rispettalo. Leggi SOLO lo studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (§ indicati), i file d'area e i file elencati nel prompt. Per trovare codice: `python3 tools/indice.py` poi `grep -n`, oppure il subagente `cercatore`. Letture a intervalli, niente file interi grandi.
- **Il programma propone, il chimico decide**: ogni giudizio su un TP è un suggerimento «Prev.» con la sua prova; nessuna funzione dichiara un TP confermato né assegna un livello di Schymanski; nessuna riga si toglie per un criterio di gruppo.
- **Rust per ogni calcolo** (medie, rapporti, insiemi, forme, criteri: `crates/mzlab-feat/src/gruppi/` e i moduli del prompt); JS solo interfaccia (legge `gruppi/1` e lo disegna o lo formatta); Python solo strumenti e test (`tools/`, `tests/`), mai nel sito. Codice e commenti in inglese.
- **Tabui solo in inglese**: tabelle, tooltip, messaggi, file esportati e loro nomi; nessuna chiave `lang/` per Tabui (eccezione di `docs/agenti/mzfinder.md`; i testi stanno nel catalogo inglese di `40_tabui_testi.js` o nel file nuovo). «Formula» mai da sola: «Neutral formula» o «[M+H]+ formula». Aree in notazione scientifica. Nessuna etichetta alpha/beta, niente di nascosto salvo la modalità amministratore che già contiene Tabui.
- **Velocità**: non appesantire l'avvio. I file JS nuovi stanno in `TP_Mine/lazy/` e si caricano con `TPLAZY.load` alla prima apertura (meccanismo creato da GR-3); mai un file nuovo in `TP_Mine/js/`.
- **File aperti da altre PR**: `TP_Mine/js/20_tpmine.js` e `TP_Mine/js/42_tp_table.js` sono toccati dalle PR #863, #871, #873 (e da quella dei tooltip senza «?»). Parti da file NUOVI; tocca quei due file solo se il prompt lo dice e solo dopo che quelle PR sono in `main` (`git log origin/main --oneline -- TP_Mine/js/42_tp_table.js TP_Mine/js/20_tpmine.js`; se non lo sono, fermati e scrivi al Coordinatore).
- Non toccare: `.github/workflows/`, `.claude/settings.json`, `mzlab/web/teoria/`, `crates/mzlab-core` salvo il prompt lo dica, i nomi dei dati utente (`qqq_lab`, `qqq.*`).
- **Repository pubblico**: solo dati sintetici (`tools/dati_sintetici.py`, `TP_Mine/tests/lr_synth.py`) e composti pubblici; MAI dati del repository privato copiati qui; nessun cognome di `nomi_vietati.txt` (`python3 -m pytest -q tests/test_esempi_anonimi.py`).
- **Documenti**: righe nuove nel file d'area del prompt (≤ 25 KB, righe ≤ 600 caratteri, stato attuale senza «ora/prima» né numeri di PR); aggiorna la riga del tuo pacchetto nella tabella di §9 dello studio (stato, misure).
- Funzioni ≤ 80 righe; test Rust in file a sé (`crates/<crate>/tests/*.rs`); rustfmt solo sui file che tocchi (mai `cargo fmt` del repository); `python3 tools/controlla.py` verde; un file JS nuovo con `// @ts-check` in `tools/tipi/pagina.json` e 0 errori; ogni grafico nuovo con la sua tabella `TDATI`, colori da variabili CSS (Okabe-Ito), tastiera e `aria-label`, bersagli ≥ 24×24 px (40 al tocco), zoom senza trascinamento, `prefers-reduced-motion`.
- Prove: `cargo test --release` dei crate toccati, `python3 tools/verifica.py --cambiati` (in cloud: `docs/agenti/ambiente-cloud.md`; prima degli e2e di mzFinder `python3 tools/build_site.py`, poi cancella `site/`), un e2e nel browser se tocchi la pagina (un front end non provato nel browser è rotto), `window.MZLAB_LR_MISS` vuoto.
- Flusso: ramo `claude/<argomento>` da `origin/main`, PR non bozza, merge con merge commit quando la CI della PR è verde, poi controlla che il workflow «pages» del commit su `main` sia riuscito. Un criterio dello studio §8 non raggiunto o una scelta tra due comportamenti: PR aperta con la spiegazione e domanda al Coordinatore, niente merge.
- Se il contesto supera 200k token a fine PR: blocco di consegna al Coordinatore (AGENTS.md §2) e fermati.
