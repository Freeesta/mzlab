# GR-4 — Vista «Compare conditions»: matrice TP × condizione, UpSet, barre e curve, esportazioni

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte subito con la fixture (in parallelo con GR-1 e GR-3); se GR-1 non è ancora in `main`, usa la fixture dello studio §3 scritta da te in `TP_Mine/tests/gruppi_fixture.json` e sostituiscila con quella di GR-1 prima del merge.

---

Pacchetto GR-4 dello studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (leggi Sintesi, §2.6-2.7, §3, §4 punti 3-5, §8.6-8.7). File d'area: `docs/agenti/gruppi.md` (se esiste), `docs/agenti/mzfinder.md` (righe «Testi e aiuto di Tabui», `TPHELP`), `docs/agenti/dati.md` (sezione `TDATI` e grafici accessibili), `docs/agenti/mzfinder-tabella.md` (stile delle tabelle di Tabui). Codice da guardare come esempio: la funzione `chart` e `TDATI.series` usate in `TP_Mine/js/20_tpmine.js:562` e `:729-731` (solo lettura).

## Obiettivo
Un componente che riceve un `gruppi/1` e mostra la vista «Compare conditions» dello studio §4.4 con le esportazioni di §4.5. Nessun calcolo: il JS formatta e disegna ciò che dà il Rust.

## Da fare
1. **`TP_Mine/lazy/gruppi-vista.js`** (`// @ts-check`, in `tools/tipi/pagina.json`): `window.TPGRUPPI_VIEW.mount(el, gruppi, opts)` con `opts = { parentName, onOpenRow(rowId) }`.
   - **Matrice**: righe = TP (`tp_name`, m/z, RT, polarità), colonne = condizioni (riferimento per primo, segnato «ref»); cella = area in notazione scientifica, rapporto (`×2.3`, `only here`, `—`), simbolo di presenza (● / ○, mai solo colore), gettone «Prev.» con la prova nel `title` e nel testo per lettori di schermo. Ordinamento per colonna, filtro «Only in… / In all / Present in…» (dalle `sets`), tastiera (frecce nella griglia, `role=grid` o tabella semplice con pulsanti: scegli la più semplice che passa axe).
   - **UpSet**: barre verticali delle `sets` (altezza = numero di righe), matrice dei punti delle condizioni sotto, barre orizzontali dei totali per condizione; clic/Invio su una barra = filtro della matrice; `TDATI` con insiemi e conteggi.
   - **Grafico del TP scelto**: con un solo tempo per condizione barre raggruppate (repliche come punti, deviazione standard come segmento); con più tempi curve sovrapposte, una per condizione, parent tratteggiato; scelta «Area» / «% of max»; Okabe-Ito per le prime 8 condizioni (da variabili CSS), oltre 8 piccoli multipli; `TDATI` sempre.
   - **Parent per condizione**: tabella k, t½, frazione rimasta.
   - **Avvisi** di `gruppi.warnings` in testa, testi inglesi brevi.
2. **Esportazioni** (pulsanti in inglese): «Comparison (CSV, long)», «Comparison (CSV, wide)», «Sets (CSV)», «gruppi/1 (JSON)», grafici in SVG e PNG; nomi `tabui_conditions_<parent>_<yyyymmdd>.csv`, `tabui_upset_<parent>.svg`, `tabui_tp-<mz>_conditions.svg` (parent in minuscolo, spazi → `-`). Usa la funzione di download che Tabui usa già (`dl()` o quella di `20_tpmine.js`: cercala con `cercatore`). CSV con intestazioni inglesi, «Neutral formula» e «Δ ppm» solo se presenti.
3. **Pagina di prova**: `TP_Mine/tests/gruppi_vista.html` (non nel sito) o montaggio diretto dall'e2e con la fixture.
4. Riga in `docs/agenti/gruppi.md` (o in `docs/agenti/mzfinder.md` se GR-1 non l'ha ancora creato).

## Prove
- `tests_e2e/e2e_tabui_gruppi_vista.py`: montaggio con la fixture (6 condizioni endpoint) e con una fixture cinetica (3 condizioni × 5 tempi); conteggi dell'UpSet uguali alle `sets`; filtro «Only in O2» = righe attese; barre vs curve secondo i tempi; esportazioni scaricate con nome e intestazioni attese (CSV letto e confrontato); tastiera su matrice e UpSet; axe senza violazioni (stessi controlli di `e2e_axe_grafici`); ogni grafico ha la sua `TDATI`.
- Tempo del primo disegno con 300 righe × 8 condizioni ≤ 500 ms (studio §8.6), scritto nella PR.
- 9 e 12 condizioni: piccoli multipli, nessuno scorrimento orizzontale della pagina a 360 px.

## Atteso
Componente della vista con esportazioni, provato nel browser sulla fixture. Nessun innesto in `20_tpmine.js`/`42_tp_table.js` (lo fa GR-5).

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
