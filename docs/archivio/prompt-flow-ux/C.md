# FLOW-C — Raw files: tabella ordinata, tipo di acquisizione, esplora, esporta mzML

- **Ramo**: `claude/flow-raw-files` da `origin/main`.
- **Dipende da**: l'unione della PR di FLOW-INFUSIONE-RAW (stesso nodo). **In parallelo con**: A, B, E.

## Regole comuni (valgono per tutto il lavoro)
- Flow è solo per l'amministratore e SOLO in inglese: nessuna chiave in `lang/`, `flow/` sta fuori da `tools/controlla_i18n.py`.
- **Regola dei testi di Federico**: pochi testi. Niente paragrafi esplicativi dove l'interfaccia si capisce da sola. I tooltip servono SOLO a dire cosa fa un campo o un pulsante, o cosa inserirvi: mai opinioni, avvisi o cautele. Un testo del tuo perimetro che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n, se ne ha una).
- Logica in Rust, JS solo colla; file ≤ 500 righe; WCAG 2.2 AA (bersagli ≥ 24 px, 40 al tocco, tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`); nessuna dipendenza nuova; nessun dato reale nel repository.
- FLOW-INFUSIONE-RAW (session_01Ly8ApkcMYtS51kFeTjDzWe) possiede il caricamento `.raw`, i messaggi d'errore del nodo Raw files e il «null» della riga di stato: non toccarli.
- Contesto completo: `docs/agenti/flow-ux.md` (stato del codice §1, layout §3). Leggilo una volta; non rileggere AGENTS.md.
- Primo messaggio al COO (send_message, session_01NawSmneBkv8sosEHpoC8k4): «FLOW-C: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`.

## 1. Ruolo
DEV Sonnet del nodo Raw files del Flow. Leggi `docs/agenti/flow-grafica.md`, la parte sulla barra del banco di `docs/agenti/dati.md` e `docs/agenti/flow-ux.md` §1.

## 2. Contesto
Il nodo Raw files dice solo «N files in Data: …» (`flow/lazy/flow-inspect.js:19`), con Add files / Add a folder (`flow-import.js:86-90`). Federico vuole: i file elencati in ordine con il tipo di acquisizione (LC-MS, infusione, DDA/DIA, MSn, SRM/MRM, polarità, strumento), esplorare il file (metodo, intestazioni, scansioni, albero dei precursori) senza troppi pulsanti, ed esportare il mzML.
Data ha già tutto: `api/method` (`showMethod`, `mzlab/web/eventi.js:442`), la barra informativa del banco (`mzlab/web/banco.js:349`: `hdr`, `lst`, `fil`, `alb` = `infusione-albero.js`), `to_mzml` in `crates/mzlab-lettori-wasm/src/lib.rs:238` con il worker `lettori-worker.js`. Riuso, non riscrittura.

## 3. Perimetro
Tocchi: file nuovo `flow/lazy/flow-raw-files.js` (aggiunto a `LATE` in `flow/js/flow-tool.js`), la riga 19 di `flow-inspect.js` (chiama il file nuovo), un'esportazione minima in `mzlab/web/banco.js` (una funzione che disegna il pannello `fil|lst|alb` del file `k` in un contenitore dato, senza duplicarne il codice), Rust solo se la classificazione dell'acquisizione non esiste già, un e2e nuovo, righe in `flow-grafica.md` e `dati.md`.
Non tocchi: il caricamento `.raw` e i messaggi d'errore del nodo (FLOW-INFUSIONE-RAW), `flow-shell.js`, `flow-form.js`, `flow-names.js`, `flow-fix.js`.

## 4. Lavoro
1. Tabella dei file (una riga per file, ordinata per nome): name, type (LC-MS / Infusion / DDA / DIA / MSn / SRM-MRM), MS levels, polarity, instrument, size. Dati da `ST.files` + `api/method`. Se il tipo non è già calcolato in Rust, aggiungilo in `mzlab-core` come campo della rotta esistente, con il suo test di parità; JS non classifica.
2. Un pulsante «Explore» per riga: apre sotto la tabella le schede File and instrument / Scans / Precursor tree (quest'ultima solo se il file ha MSn) con la funzione esportata da `banco.js`. Una sola esplorazione aperta alla volta.
3. «Export mzML» per riga: un `.mzML` si scarica così com'è; un formato del fornitore passa da `to_mzml` nel worker dei lettori; nome `<stem>.mzML`.
4. Regola dei testi: niente frasi sotto la tabella; i pulsanti hanno solo il verbo e un tooltip che dice cosa fanno.

## 5. Accettazione
- Con l'infusione Orbitrap MS1–MS8 la riga dice «Infusion · MS1–MS8 · positive · Orbitrap Fusion».
- «Explore» mostra gli stessi valori della scheda Data per lo stesso file; il mzML scaricato si riapre in Data.
- Tabella con `caption`, intestazioni `th scope="col"`, tutto da tastiera.

## 6. Prove
- e2e nuovo `tests_e2e/e2e_flow_rawfiles.py` con file sintetici (`tools/dati_sintetici.py`, `TP_Mine/tests/msn_synth.py`); parità Rust se aggiunta; `tsc`; `python3 tools/pre_pr.py`; `python3 tools/verifica.py --cambiati`.

## 7. Consegna
PR da `claude/flow-raw-files`; messaggio al COO come nelle regole comuni.
