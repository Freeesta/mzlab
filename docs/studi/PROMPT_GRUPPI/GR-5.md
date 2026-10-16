# GR-5 — Innesto nella tabella TP di Tabui, MS2 per condizione, e2e completo, misura con 40 file

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di GR-2, GR-3, GR-4 e delle PR #863, #871, #873 (controlla con `git log origin/main --oneline -- TP_Mine/js/20_tpmine.js TP_Mine/js/42_tp_table.js` e la lista delle PR aperte; se una manca, fermati e scrivi al Coordinatore).

---

Pacchetto GR-5 dello studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (leggi Sintesi, §2, §4, §7, §8). File d'area: `docs/agenti/gruppi.md`, `docs/agenti/mzfinder.md` (righe «Scheda dei risultati LR», «MS2 abbinate»), `docs/agenti/mzfinder-lr.md`, `docs/agenti/mzfinder-ms2.md`, `docs/agenti/mzfinder-tabella.md`. Codice: `TP_Mine/js/20_tpmine.js` (`render()`, `mountTable()`, `show()`, tabella dei file), `TP_Mine/js/42_tp_table.js`, `TP_Mine/lazy/gruppi-campioni.js`, `TP_Mine/lazy/gruppi-vista.js`.

## Obiettivo
Dalla tabella dei file al confronto, tutto collegato in Tabui LR: un Dig con molte condizioni, la colonna «Conditions» e il filtro nella tabella TP, la linguetta «Compare conditions», l'apertura della riga dalla vista, le MS2 per condizione.

## Da fare
1. **Tabella dei file e innesto esclusivo**: l'innesto completo della tabella dei file (`TPGRUPPI_FILES`, `TPLAZY.load("gruppi-campioni.js")`, `TPLAZY.load("gruppi-vista.js")`) e il passaggio al Dig di `campioni` e `reference` sono di proprietà esclusiva di GR-5 su `20_tpmine.js` e `42_tp_table.js` (GR-3 e GR-4 non toccano quei file); in `20_tpmine.js` usa `TPGRUPPI_FILES` con ≥ 2 file full e passa `campioni` e `reference` al Dig.
2. **Tabella TP** (`42_tp_table.js`): con `gruppi` nell'uscita, colonna compatta «Conditions» (un punto per condizione, `aria-label` «present in …; absent in …») e filtro «Only in… / In all / Present in…»; con una condizione nulla cambia (stesso DOM di oggi: controllalo nell'e2e).
3. **Linguetta «Compare conditions»** accanto a quelle di oggi, solo con ≥ 2 condizioni; alla prima apertura `TPLAZY.load("gruppi-vista.js")`; `onOpenRow` = `show(id)` (torna a «Table» e apre la riga).
4. **Riga aperta**: nella scheda della riga, sezione «By condition» con i valori della riga in `gruppi/1` (area, rapporto, `prev`) e il grafico del TP della vista (riusa `TPGRUPPI_VIEW`, nessun grafico nuovo).
5. **MS2 per condizione**: nella scheda «MS2 matches» il file MS2 mostra la sua condizione (da `NB.campioni`); `prev.ms2` = `linked` se un MS2 della condizione o condiviso è abbinato alla riga (dato dal Rust di GR-2: se manca, aggiungilo lì, non in JS).
6. **Esportazioni della tabella**: Excel/CSV di oggi con, se ci sono condizioni, le colonne «Present in» e «Only in»; testi inglesi.
7. Aggiorna `tour.js` solo se un passo perde il bersaglio (`e2e_tour` verde).

## Prove
- `tests_e2e/e2e_tabui_gruppi.py`: serie `tools/dati_sintetici.py gruppi` (endpoint) e `gruppi-kin`: dalla tabella dei file al confronto; matrice uguale a `gruppi_truth.json` (studio §8.2); un nome per TP in tutte le condizioni (§8.3); filtro «Only in O2»; apertura di una riga dalla matrice; MS2 della condizione mostrata; con una sola condizione nessuna linguetta nuova e tabella invariata; `window.MZLAB_LR_MISS` vuoto; nessuna richiesta a `lazy/` prima della linguetta.
- Misura con 40 file full sintetici (20 condizioni × 2): Dig, primo disegno della vista, memoria di picco; criterio §8.5 e §8.6 in `docs/agenti/misure.md`.
- Gli e2e di Tabui LR esistenti restano verdi (`e2e_tpmine_lr_*`, `e2e_tabui_tptab`, `e2e_tpmine_ms2_lr`).

## Atteso
Flusso completo nel sito, misure scritte, studio §9 aggiornato (GR-5 fatto). Se un criterio di §8 non passa: PR aperta con i numeri e domanda al Coordinatore.

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
