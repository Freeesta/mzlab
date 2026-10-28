# MATCH-E — Analoghi: tabella delle modifiche, ΔFormula vincolata al padre, kit di proprietà

- **Ramo**: `claude/match-analoghi`. **Dipende da**: MATCH-B unita. **In parallelo con**: MATCH-C, MATCH-D (le colonne si collegano dopo la loro unione, in una seconda PR piccola se servono).

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-E: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV Rust chimica. Leggi `docs/agenti/livelli.md` (solo la scala), `docs/agenti/mzfeat-tp.md` (righe `regole_tp/2`). Studio: §9, §10.

## 2. Contesto
Esistono: `COMMON_DELTAS` (42 modifiche) e `delta_formulas` in `crates/mzlab-feat/src/attributio/review/matching.rs` (modifiche note entro `tol_mda`, poi CHNOPS libera); `TP_Mine/data/regole_tp_2.json` (regole con `delta_formula` e `direzione_rt`); `isoscore.rs` (pattern isotopico), `kendrick.rs` (KMD), RDBE (`msngrafo/candidati.rs`), TPSA e Crippen logP (`crates/mzlab-mol/src/descr.rs`), passo `rt.predict` (RT predetto e direzione). Manca: una tabella unica, il vincolo sul padre, NOSC, e il collegamento con `libmatch/1`.

## 3. Perimetro
`crates/mzlab-core/` (nuovo `modifiche.rs` + `data/modifiche-1.json`, NOSC), `matching.rs` (diventa lettore della tabella, stessi risultati), `crates/mzlab-lib/src/matchres.rs` (campo `delta_formula[]` e proprietà del candidato), test. Non toccare `regole_tp_2.json` (si legge), Tabui, pannello o Flow oltre il campo nel risultato.

## 4. Lavoro
1. `modifiche-1.json` + schema: le 42 di `COMMON_DELTAS` + +O 15,9949, ±CH2 14,0157, −H2 2,0157, ±H2O 18,0106, −Cl+H −33,9610, −Cl+OH −19,9720, +CF2 49,9968, +SO3 79,9568, +glucuronide 176,0321 (masse ricalcolate dalle masse atomiche del motore, non copiate), senza doppioni con `delta_formula` di `regole_tp_2.json` (test che le confronta).
2. `delta_formulas_constrained(delta, tol_mda, parent_counts, n)`: modifiche note, poi coppie di modifiche note, poi CHNOPS(+F,Cl,Br) libera; scarta ogni proposta che toglie un elemento assente nel padre o più atomi di quelli presenti; più semplice prima; ≤ 3. Senza padre = comportamento di oggi (test di parità con `delta_formulas`).
3. In `matchres`: per ogni analogo HR, `delta_formula[]` con il padre = formula della query se nota, altrimenti la formula del composto di libreria come vincolo inverso; in LR nessuna proposta.
4. Kit sul candidato (solo colonne, mai filtri): RDBE, NOSC (nuovo), KMD CH2/CF2, punteggio isotopico se c'è l'MS¹, TPSA e logP se c'è lo SMILES; `rt_dir_ok` resta di `rt.predict` + `table.attribute` (MATCH-C).

## 5. Accettazione
`cargo test -p mzlab-core -p mzlab-feat -p mzlab-lib` verde; casi: Δ = −33,9610 con padre clorurato → «−Cl+H», con padre senza Cl → nessuna proposta «−Cl»; +O e +2O; NOSC di composti noti (es. CH4 = −4, CO2 = +4, glucosio = 0).

## 6. Prove
`cargo test -p mzlab-core modifiche && cargo test -p mzlab-feat matching && cargo test -p mzlab-lib matchres`.

## 7. Consegna
PR «Analogues: one modification table, parent-constrained delta formulas, candidate properties».
