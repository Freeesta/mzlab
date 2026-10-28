# MATCH-C — Nodo «Attribution» del Flow: `table.match` su più librerie e `table.attribute@1`

- **Ramo**: `claude/match-attribution`. **Dipende da**: MATCH-B unita. **In parallelo con**: MATCH-D, MATCH-E. **Dopo di te**: le colonne di MATCH-E.

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-C: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV Rust del Flow (`crates/mzlab-cli`, descrittori `step/1` in `crates/mzlab-feat/src/steps/`). Leggi `docs/agenti/rust-flow.md` (righe di `table.match`, `annot.level`, `rt.predict`), `docs/agenti/cli.md`. Studio: §3, §8.

## 2. Contesto
`table.match@1` (`flow_tablematch.rs`) confronta l'MS² di ogni riga del `.mzfeat` con UNA libreria ed esce in `candidates/1`; `annot.level` legge `candidates/1`; le tabelle pos e neg restano separate (`polpair/1` è una prova). MATCH-B ha dato `LibSet` e `libmatch/1`. Federico vuole il modulo «Attribution» dopo la tabella delle feature con gli MS² associati, prima/accanto a Tabui TP.

## 3. Perimetro
`crates/mzlab-cli/src/flow_tablematch*.rs`, nuovo `flow_attribute.rs` (+ `_tests.rs`), una riga di registro in `flow.rs`, descrittori in `crates/mzlab-feat/src/steps/`, schemi `docs/schemi/steps-*.json` rigenerati con lo strumento, palette del Flow solo per il gruppo «Attribution» e il preset di catena, viste del Flow per leggere `attrib/1` (`flow-views-attributio.js`, solo colla). Non toccare Tabui (`TP_Mine/`) oltre la lettura di `attrib/1`, né il pannello laterale.

## 4. Lavoro
1. `table.match@1`: porta `library` `.many()`; parametri `library_preset` (all|experimental|environmental|metabolomics|natural_products|forensic; preset da MATCH-F se già unita, altrimenti solo `all`/`experimental`) e `search_mode` (identity|identity+analog); uscita `candidates/1` invariata per `annot.level` + blocco `libmatch` per query. Una sola libreria = numeri identici a oggi (test).
2. `table.attribute@1` (`flow_attribute.rs`, registrato con UNA riga in `flow.rs`): ingressi `table`, `hits`, facoltativi `polpair`, `rt` (uscita di `rt.predict`), `parent`; uscita `attrib/1` con le colonne dello studio §3 (`best_name`, `best_inchikey14`, `match_type`, `score`, `score_kind`, `matched`, `shifted`, `reverse`, `explained`, `delta_ppm`, `delta_mz`, `delta_formula` vuota finché MATCH-E non c'è, `n_compounds`, `library`, `confidence` da `annot.level` se collegato, `rt_dir_ok`, `other_polarity_hit`).
   Parametri `min_score`, `min_matched`, `top_compounds`, `analog_max_delta`, `chrom_mode` (rplc|hilic).
3. Palette: gruppo «Attribution» nel pacchetto `attributio` e preset di catena `table.match → table.attribute → annot.level`. Tooltip = cosa fa il campo.
4. CLI: ogni parametro con `--set ISTANZA.NOME=VALORE`; `flow_rule_tests.rs` verde; riga in `docs/agenti/cli.md` e in `rust-flow.md`.

## 5. Accettazione
`cargo test -p mzlab-cli flow_attribute flow_tablematch flow_rule` verde; un piano `mzflow/2` sintetico (tabella + 2 librerie + polpair) produce `attrib/1` uguale nella CLI e nell'esecutore web (`flow-web`), test di parità.

## 6. Prove
`cargo test -p mzlab-cli && python3 tools/verifica.py --cambiati` (atteso: nessun rosso nuovo; `window.MZLAB_LR_MISS` vuoto).

## 7. Consegna
PR «Flow: Attribution (multi-library table.match and table.attribute)».
