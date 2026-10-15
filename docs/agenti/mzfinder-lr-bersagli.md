# Bersagli MS2 di Tabui LR

Lo studente acquisisce una MS2 su uno ione perché l'ha visto nel full scan: se il Dig LR non lo elenca, la MS2 non deve restare orfana. Codice: `crates/mzlab-feat/src/legacy/lr/ms2target.rs`, chiamato da `ms2link::ms2_link`; studio: `docs/studi/TABUI_MS2_LR.md` §6; abbinamenti: `docs/agenti/mzfinder-lr.md`.

## Regole
- `ms2_link` toglie le righe `ms2_target` di una chiamata precedente, abbina gli esperimenti (primo giro), poi per ogni picco della traccia `no_row` / `no_row_at_rt` (non `front`, non `parent_rt`, non `user`) cerca nella tabella allineata (`LrDig.table`) la riga con |m/z − P| ≤ `PREC_TOL` (m/z letto o m/z − offset) e |RT − (apice − deriva del file)| ≤ `rt_tol_min`, dopo `rt_min`, con l'area massima nei file della serie (`ms2target::near`). Se la riga è già una `Entry` non nasce nulla; se non c'è: il picco resta tra i non abbinati con motivo `no_feature`.
- Una riga per riga della tabella (due picchi, o due CE, sulla stessa riga danno una riga sola). Id da 200001 in ordine di riga della tabella; `unexpected: true`, `ms2_target: true`, nome «MS2 target m/z {P:.1} @ {RT:.2} min». Celle, criteri e punteggio come gli inattesi (`evaluate`, controllo isotopo e generazione cinetica con `finish_one`), SENZA il filtro dei criteri e di `max_unexpected`: l'etichetta può essere `debole`, è onesto e visibile.
- Poi il secondo giro degli abbinamenti, con le righe nuove: il picco diventa `auto` e il `why` finisce con «row made from this MS2» (solo per l'esperimento che ha fatto nascere la riga). Un bersaglio marcato isotopo (`isotope_of`) resta compatibile (`ms2link/rules.rs::compatible`): lo studente lo ha scelto, il criterio «not an isotope peak» mostra il dubbio.
- Se l'ISF era già stato calcolato dalla pagina (`d.isf` non vuoto) viene ricalcolato con le righe nuove. `relink` ricomincia da capo: una correzione «non questo ione» (`row: null`) su quel picco impedisce la riga; togliendola, la riga torna con lo stesso id.
- Riassunto: `rows[].ms2_target: true` (la chiave c'è solo quando è vera, il contratto d'oro non cambia), `kind: "unexpected"`; `live()` non cambia (solo candidati).

## Casi dei file veri (verità privata)
- Un ione che la verità dà come isotopo di una riga (`expect: "isotope"`) ha il legame `isotope` e nessuna riga propria. Il test dei dati (`lr_ms2link_dati.rs`) vuole, per un bersaglio della verità (`expect: "target"`), un legame `auto`, `ambiguous` o `user` del picco all'apice (±0,1 min) a una riga qualsiasi con RT di riferimento entro 0,1 min dall'apice full; `isotope` vale solo per `expect: "isotope"`; per `no_signal` nessun legame alla riga del Dig a quel tempo.
- Il motivo per cui il Dig non elenca ioni forti (sintetico, provato in `lr_ms2link/target.rs`): un ione entro `tol_da` dall'm/z di un'ipotesi è di quell'ipotesi per `rows::assign` e non è mai un inatteso (`rows::unexpected` scarta ogni riga entro `tol_da` da un m/z noto, anche a un altro tempo); come isomero entra solo se ha ≥ 10 % dell'area del più forte e tra i primi 4 (`rows::candidates`). Un ione piccolo allo stesso m/z di un'ipotesi ma a un altro tempo sparisce quindi dalla tabella del Dig. Nessuna modifica del Dig: correggerlo richiede il sì del Coordinatore.

## Diagnosi sui dati veri (la fa Federico in locale; numeri solo nel repository privato)
```
MZLAB_DATI=<percorso> cargo run --release -p mzlab-finder-lr-wasm --example finder_lr -- OUT FILES.json PARENT.json RULES.json --diag --ms2 N MZML...
```
(`FILES.json` e `PARENT.json` come per l'e2e `e2e_tpmine_lr_rust.py`; `RULES.json` = `TP_Mine/data/regole_tp.json`). `--diag` stampa, per ogni picco di traccia senza riga PRIMA dei bersagli, le righe della tabella entro 0,5 Da e 0,5 min dall'apice (m/z, RT, punteggio, celle, criteri) e il passo del Dig che le perde: ipotesi vicina (isomeri), criteri, assenza dalla tabella (rilevamento). Scrivere l'esito in `scratchpad/consegne/TABUI_MS2_LR_DATI.md` del privato.
