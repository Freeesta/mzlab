# GR-6 — Alta risoluzione: `gruppi/1` dalla tabella dei prodotti HR, rete per condizione

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di GR-1, in parallelo con GR-2.

---

Pacchetto GR-6 dello studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (leggi Sintesi, §2, §3, §6, §8.1, §8.8). File d'area: `docs/agenti/gruppi.md`, `docs/agenti/mzfeat-tp.md` (`tp/1`, `tptab/1`, rete), `docs/agenti/mzfeat-tabella.md`, `docs/agenti/campioni.md`, `docs/agenti/memoria.md`. Codice: `crates/mzlab-feat/src/tp/tabella.rs`, `tp/rete.rs`, `tp/mod.rs`, il binding `tp_table` di `crates/mzlab-feat-table-wasm`.

## Obiettivo
La tabella dei prodotti HR dà lo stesso `gruppi/1` del Dig LR, con in più formula neutra ed errore ppm, quando il `campioni/2` ha ≥ 2 condizioni; la rete osservata si calcola dentro ogni condizione. Con una condizione l'uscita non cambia.

## Da fare
1. In `tp/tabella.rs` (o un file nuovo `tp/gruppi.rs`): costruisci `GroupTable` dalle aree della tabella delle feature e dalle righe di `tp_table` (`neutral_formula` dall'ipotesi migliore se c'è, `ppm` dell'ione), chiama `gruppi::build` e aggiungi `"gruppi"` a `tptab/1` solo con ≥ 2 condizioni.
2. `tp/rete.rs`: con più condizioni, la correlazione «lungo la serie» si calcola per condizione (punti = tempi trattati della condizione, almeno `points`) e un arco vale se vale in almeno una condizione; con < 4 tempi per condizione nessun arco (scrivilo nella nota della riga). Con una condizione stesso risultato di oggi.
3. `tp/tabella.rs::series`, `in_t0`, `in_blank`: con più condizioni restano quelli della condizione in cui la riga è più forte (compatibilità) e il dettaglio sta in `gruppi`.
4. Binding: nessuna funzione nuova se `tp_table` basta; se serve, `tp_groups(...)` nello stesso `.wasm` `table`, con il contratto di memoria (`docs/agenti/memoria.md`) e la rotta nativa sorella nel desktop.
5. Righe in `docs/agenti/mzfeat-tp.md` e `docs/agenti/gruppi.md`.

## Prove
- `crates/mzlab-feat/tests/tp_gruppi.rs`: fixture HR sintetica (6 condizioni, endpoint; e 3 × 5 tempi) costruita come in `tests/tp_vista.rs`; `gruppi/1` con `neutral_formula` e `ppm`; matrice uguale alla verità; con una condizione `tptab/1` uguale byte per byte a oggi (`tp_vista.rs`, `tp_ruoli.rs` verdi); rete per condizione su un caso con correlazione opposta fra condizioni.
- Peso del `.wasm` `table` dopo `wasm-opt -Oz` sotto il suo limite (misura nella PR).

## Atteso
`gruppi/1` dalla parte HR. L'interfaccia HR (tabella dei prodotti `TPTAB` in `21_tpmine_hr.js`) userà la stessa vista di GR-4 in un pacchetto successivo, da scrivere solo quando GR-5 è unito.

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
