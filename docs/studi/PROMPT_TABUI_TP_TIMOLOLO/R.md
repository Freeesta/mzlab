# R — Interpretazione per riga in Rust (`interp/1`): TT-R1, TT-R2, TT-R3

Prompt pronto per una chat DEV (Sonnet): copia tutto ciò che segue la riga «---», poi le regole di [COMUNI.md](COMUNI.md). Tre PR in fila nella stessa chat. Parallelo alla chat LOCALE.

---

Pacchetti TT-R1, TT-R2, TT-R3 dello studio `docs/studi/TABUI_TP_TIMOLOLO.md` (leggi §0, §4-§7, §10, §11, §14). File d'area: `docs/agenti/mzfeat-tp.md`, `docs/agenti/posizioni.md`, `docs/agenti/mzfeat-tabella.md`, `docs/agenti/mzfeat-wasm.md` (solo le righe su worker e `.wasm`), `docs/agenti/mzfinder.md` (righe di `finder_msn`). Non rileggere per intero `legacy/dig*.rs`, `famalign.rs`, `TP_Mine/py/`.

## Obiettivo
Per ogni riga della feature table della Pipeline che passa i cancelli di `tp/1`, un `interp/1` (§5 dello studio) calcolato in Rust, scritto nel `.mzfeat` come blocco `interp`, servito da una rotta `table_interp` e da un `.wasm` `interp`; poi la posizione con confidenza dichiarata e le varianti da disegnare; infine le prove sui dati veri.

## TT-R1 (prima PR): modifica, MS2, confronto, albero, filtri
1. `crates/mzlab-feat/src/interp/` (nuovo; `mod.rs`, `ms2link.rs`, `confronto.rs`, `albero.rs`, `filtri.rs`, `json.rs`), dietro la feature `interp` del crate.
2. `ms2link.rs`: per una riga, tutte le scansioni DDA di tutti i file della serie (non l'infusione) con `legacy/ms2.rs::select` e i bordi del picco dal `.mzfeat`; per ognuna file, scansione, RT, precursore, finestra di isolamento, CE e intervallo di scansione dal filtro (se l'indice non li ha, dal testo del filtro: dillo nella PR), purezza = intensità del precursore entro ±5 ppm / intensità totale nella finestra, nell'MS1 più vicina prima e dopo, interpolata al tempo (`chimeric` sotto 0,8). Consenso con `legacy/ms2.rs::consensus` sulle scansioni non chimeriche. Isomeri: se le tracce (intensità per scansione) dei 5 frammenti più intensi formano due gruppi con correlazione di Pearson < 0,5 fra i gruppi, `isomers: "separated"` e un consenso per gruppo; con meno di 4 scansioni `single`.
3. `confronto.rs`: `similarity::pair_components` contro la MS2 DDA del genitore (stessa via, riga del genitore) e, per formula ≤ 5 ppm, contro i nodi della libreria dell'albero (`legacy/msn/library.rs::build` con la DDA del genitore); `equal`, `shifted`, `losses`, `cos_mod`, `entropy`. Mai confrontare intensità con l'infusione (decisione D4).
4. `albero.rs`: per ogni nodo dell'albero del genitore (`legacy/msn.rs::build_tree` sull'`.mzidx` del file di ruolo `infusion` in `campioni/2`, ione [M+H]+ del genitore, calibrazione `auto`) lo stato `same|shifted|absent|not_checkable` (`not_checkable` se fuori dall'intervallo di scansione della MS2). La libreria si costruisce una volta e va nel meta `interp` (`msnlib`).
5. Modifica: Δ della formula neutra dall'ipotesi migliore (`ipotesi/1`) e controllo isotopico: M+1/M e M+2/M osservati all'apice contro attesi dalla formula, `s_count` `consistent` se M+2 è entro il 30 % relativo dell'atteso con lo zolfo del genitore ± la Δ.
6. Calibrazione per file (F5): errore mediano in ppm del genitore e dei suoi frammenti noti in ogni file (`legacy/msn/space.rs::fit`), tolleranza della formula = max(3, 2 × MAD) ppm, mai oltre 5; riga fuori = nascosta con motivo `F5`.
7. `filtri.rs`: registro `filtri/1` (§6 dello studio) con F1-F5; F1-F4 si leggono dal `.mzfeat` e da `tp/1`, non si ricalcolano.
8. `store.rs`: meta `interp` + un JSON compresso per riga (stessa via di `write_table_tp`); tabella senza blocco = come oggi.
9. `.wasm` `interp`: feature di `crates/mzlab-feat-wasm`, cartella `wasm-feat/interp/`, voce nel manifest, riga in `tools/build_feat_wasm.sh` col tetto comune 512 000 B; funzioni `interp_build(...)` (tutta la tabella, scrive il blocco) e `table_interp(id)`; worker `TP_Mine/files/feat-worker.js`, `feat-desktop-pool.js`, `crates/mzlab-desktop/src/feat.rs`. Si carica solo con SMILES del genitore e modalità amministratore. **Misura il peso alla prima build**: oltre 470 000 B l'albero lo costruisce il `.wasm` `finder_msn` (esporta `msnlib`) e `interp` legge solo la libreria (studio §11, cancello); scrivi la misura in `docs/agenti/misure.md`.
10. Schemi `docs/schemi/interp-1.json` e `filtri-1.json`; file d'area nuovo `docs/agenti/interp.md` (una riga per funzione) e una riga nella tabella §5 di AGENTS.md («interpretazione per riga dei TP: `interp/1`, `filtri/1`»).

## TT-R2 (seconda PR): la posizione
1. `crates/mzlab-feat/src/chem/posizioni.rs`: trasformazione `-2H` (legame C-C sp3 con un H su ciascun lato; classi per simmetria come le altre) e un accesso alle maschere dei siti.
2. `crates/mzlab-feat/src/legacy/msn/localize.rs`: entrata nuova `localize_sites(..., sites: &[u64])` che punteggia le maschere date invece di `candidate_sites`; `localize` vecchia invariata (parità). `default_groups` dà nomi inglesi («ring A», «chain A») in una funzione nuova usata da `interp`; i test di parità della vecchia non cambiano.
3. `interp/sito.rs`: fusione di §7 dello studio (siti da `posizioni`, voti da `localize` con l'albero, altrimenti da `posizioni::match_spectrum`), `support` = softmax della verosimiglianza fra le classi non escluse, `confidence` `site|region|uncertain|cannot_say` con le soglie e i motivi della tabella di §7 (costanti con nome e commento sul perché), perdite = parte tenuta.
4. `interp/varianti.rs`: `variants(parent_molfile, transform, class)` → molfile e SMILES del TP (atomo nuovo sulla bisettrice esterna, lunghezza di legame media del genitore, ordine degli atomi conservato), formula controllata con `check_formulas`; nessuna variante per `cannot_say`, Δ multiple e apertura d'anello. Il molfile del genitore entra dalla pagina (Ketcher); senza, `variants` vuoto e `site` resta.
5. `proposal.sublevel` come `posiz/1` (3a solo con una classe esclusa e al più 3 rimaste); mai un livello oltre.
6. Riferimento: `tools/interp_riferimento.py` (RDKit, solo test, salta senza RDKit) enumera i siti +O/−CH2/−2H di 20 SMILES di prova e i test Rust controllano lo stesso insieme di classi.

## TT-R3 (terza PR): prove sui dati veri, esche
1. `crates/mzlab-feat/tests/interp_dati.rs` (salta senza `MZLAB_DATI`): sui 13 mzML della serie e sull'infusione, i criteri di accettazione di §10 dello studio leggendo `scratchpad/timolol_truth.json` (TP, RT, regioni) e `timolol_msn_truth.json` (nodi, `localization.cases`, `ambiguous_ids`, `isomers`); nessun numero vero nel test.
2. Esche: `interp/filtri.rs` accetta una lista di Δ esca (le Δ delle regole + 0,0137 Da × k, k = 1..5: ogni Δ esca deve stare oltre 10 ppm da ogni Δ vera delle regole, verificalo nel test; un abbinamento a un'esca è un abbinamento a caso) e conta le esche che passano F1-F5; per `site`, 50 librerie con gli insiemi di atomi permutati fra i nodi danno la distribuzione nulla del margine. `decoys` nel registro.
3. `tools/valida/tp_timololo.py` (misura, nessuna funzione di prodotto): metriche per l'ultimo giro (TP in vista, regioni giuste, `site` sbagliati, FDR, concordanza con l'oro di mzmine della chat LOCALE) scritte nel privato in `scratchpad/consegne/TABUI_TP_TIMOLOLO_MISURE.md`; nel pubblico solo i nomi delle metriche in `docs/agenti/misure.md`.

## Prove
- `cargo test --release -p mzlab-feat --features interp`, test di parità di `localize` e `posizioni` invariati; serie sintetica: estendi `tools/dati_sintetici.py` (genitore con un sito unico e un isomero di posizione, MS2 chimeriche, infusione MSn sintetica come `finder_msn_e2e.json`).
- Peso del `.wasm` `interp` prima/dopo in `docs/agenti/misure.md`; gli altri `.wasm` non cambiano peso.

## Atteso
- TT-R1: `table_interp(id)` dà modifica, MS2 collegate, confronto, albero; `filtri/1` nel meta. TT-R2: `site` con confidenza, motivi e varianti. TT-R3: misure nel privato, FDR nel registro.
