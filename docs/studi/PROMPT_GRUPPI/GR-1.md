# GR-1 — Modulo Rust `gruppi/`: presenza, rapporti, insiemi e forme per condizione (`gruppi/1`)

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte subito, in parallelo con GR-3 e GR-4.

---

Pacchetto GR-1 dello studio `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md` (leggi Sintesi, §1, §2, §3, §8). File d'area: `docs/agenti/campioni.md` (modello `campioni/2`, `role()`, `per_tp`), `docs/agenti/mzfinder-lr.md` (celle del Dig LR). Codice da leggere a intervalli: `crates/mzlab-core/src/campioni/mod.rs` (tipi), `crates/mzlab-feat/src/legacy/lr/kin.rs` (`kinetics`, `decay`), `crates/mzlab-feat/src/legacy/lr/labels.rs` (`RATIO_MAX`, `Cell`), `crates/mzlab-feat/src/kinetic/forms.rs`.

## Obiettivo
Un modulo Rust puro, senza file né pagina, che riceve la matrice righe × file di un allineamento unico, il `campioni/2` canonico e le opzioni, e restituisce il JSON `gruppi/1` dello studio §3. Lo useranno il Dig LR (GR-2) e la tabella HR (GR-6).

## Da fare
1. `crates/mzlab-feat/src/gruppi/` (nuovo; `mod.rs` tipi e `build`, `presenza.rs`, `forme.rs`, `insiemi.rs`, `prev.rs`, `json.rs`), dichiarato in `lib.rs`.
   - Ingresso: `GroupTable { n_rows, n_files, area: Vec<f64>, height: Vec<f64>, state: Vec<u8> (detected/filled/absent), apex_rt: Vec<f32>, rows: Vec<RowMeta> }` con `RowMeta { id, tp_name, mz, polarity, rt, neutral_formula: Option, ppm: Option, is_parent, ms2_files: Vec<usize> }`; `campioni: &[mzlab_core::campioni::Campione]` nello stesso ordine delle colonne; `GroupOpts` (§3, con `Default`).
   - Uscita: `GroupsOut` serializzato da `gruppi_json(...) -> Result<String, Err>` con lo schema di §3 (chiavi inglesi, ordine fisso, numeri con lo stesso formato degli altri JSON del crate). Errori `error.gruppi.noConditions|badOpts|sizeMismatch`.
2. Regole esattamente come §3: condizioni = valori distinti non vuoti di `serie` dei campioni `sample` (ordine di prima comparsa); controlli della condizione = i suoi più quelli con `serie == ""`; repliche raggruppate per (condizione, `tempo_min`); presenza con `min_rep_frac` e `min_area`; celle riempite assenti per la presenza ma con area in `kin`; `ratio_ref` all'endpoint comune più alto; `sets` esatti ordinati per numero di righe e poi per nome; `shape`, `t_half_rise`, `rate_rel`; `prev` con `verdict`; `parent` per condizione con `kin::decay` (riusala, non riscriverla) e frazione rimasta.
3. Avvisi `refMissing`, `condNoControls`, `condOnePoint`, `condNoReplicates`, `mixedTimes` (tempi diversi fra condizioni: i rapporti usano il tempo comune più alto; se non c'è un tempo comune, `ratio_ref` null).
4. `norm`: solo `"raw"`; `"istd"` → `error.gruppi.badOpts` con `params.norm` (studio §2.5, domanda Q1).
5. Schema `docs/schemi/gruppi-1.json` (stile di `docs/schemi/campioni-2.json`) e fixture `crates/mzlab-feat/tests/data/gruppi_fixture.json` (ingresso e uscita attesa: 6 condizioni a endpoint + bianco + T0 + standard del parent; 12 righe con almeno: un TP in tutte, uno solo in «O2», uno nel T0, uno nello standard del parent, uno con repliche discordi). GR-3 e GR-4 useranno l'uscita di questa fixture.
6. Documento d'area nuovo `docs/agenti/gruppi.md` (modello, regole, avvisi, prove) e UNA riga in AGENTS.md §5 («gruppi di esperimenti in Tabui: condizioni, `gruppi/1`, `crates/mzlab-feat/src/gruppi/` → `docs/agenti/gruppi.md`»).

## Prove (`crates/mzlab-feat/tests/gruppi.rs`, `tests/gruppi_forme.rs`)
- Fixture: uscita uguale byte per byte all'attesa.
- Presenza con repliche (2 su 3 rilevate = presente con `min_rep_frac` 0,5; 1 su 3 = assente); cella riempita = assente con area in `kin`.
- `only_in`, `in_all`, `sets` su 3 casi; riferimento assente nella riga → `ratio_ref` null; condizione assente → 0.
- Forme: rising, bell (calo 40 %), decay, flat, single; `t_half_rise` interpolato; `rate_rel` > 1 per la condizione più veloce.
- Controlli condivisi e per condizione (il controllo della condizione vince); `in_parent_std`.
- Determinismo: 5 permutazioni delle colonne (con `campioni` permutati allo stesso modo) danno gli stessi byte, salvo gli indici dei file che seguono la permutazione: confronta dopo aver rimappato gli indici.
- Prestazioni (test `#[ignore]` lanciato a mano e scritto nel documento d'area): 2 000 righe × 60 file ≤ 50 ms nativo (studio §8.5).

## Atteso
Modulo `gruppi` con test verdi, schema, fixture e documento d'area. Nessun `.wasm` toccato (lo collegano GR-2 e GR-6). Peso dei `.wasm`: invariato.

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
