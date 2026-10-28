# MATCH-D — Pannello laterale: ricerca immediata in tutte le librerie all'apertura di uno spettro MSⁿ

- **Ramo**: `claude/match-pannello`. **Dipende da**: MATCH-B unita e la PR di IDENT-LIBRERIE-UI (pannello «Identification with the libraries») unita o leggibile. **In parallelo con**: MATCH-C, MATCH-E.

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-D: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV wasm + pagina delle librerie. Leggi `docs/agenti/alta-risoluzione.md` (righe «Motore», «Cartella», «Cos'è?»), `docs/agenti/wasm-tetti.md` (tetto 614 400 B di `mzlab-lib-wasm`). Studio: §4, §8.

## 2. Contesto
IDENT-LIBRERIE-UI ha fatto il pannello: cartella scelta dall'utente, elenco delle librerie con selezione automatica pos/neg, menu di download e link. Prima di tutto leggi la sua PR o il suo ramo (`git ls-remote origin 'refs/heads/claude/ident-librerie*'`, PR aperte con «librar» nel titolo): ti innesti sul suo codice, non lo rifai. Il worker `libreria-worker.js` apre già le `.mzlib` con `Lib.open`; MATCH-B ha dato `LibSet` e `libmatch/1`.

## 3. Perimetro
`crates/mzlab-lib-wasm/src/` (export nuovo), `mzlab/web/libreria-worker.js` (messaggio `search`), il file del pannello di IDENT-LIBRERIE-UI solo per chiamare la ricerca e disegnare i risultati, `libreria.js` solo dove lo specchio legge `pairs[]`, chiavi `lang/`. Non toccare la cartella, il download, il catalogo (MATCH-F) né il Flow.

## 4. Lavoro
1. wasm: `LibSet` esportato (`add(lib)`, `set_mask(u16)`, `search(query_json, opts_json) -> libmatch/1`, `cancel(token)`); il `.wasm` resta sotto il tetto (misura e scrivi il numero).
2. Worker: all'apertura di uno spettro MSⁿ (n ≥ 2) il pannello manda `search` con polarità e tolleranze del profilo (HR: precursore ppm, picchi 10 ppm min 0,005 Da; LR: 0,5/0,3 Da); prima l'identità, poi gli analoghi in coda; un nuovo spettro annulla la ricerca in corso.
3. Pannello: Top 10 composti (riga = nome, badge exact/analog/in_silico, punteggio, `matched` n/N, Δppm), espandibile agli spettri; clic su uno spettro = specchio (diretti pieni, spostati tratteggiati, perdite puntinate) con tabella dei dati; `confidence` dalla cascata esistente (`LIVELLI.assess` finché la decisione 2 dello studio non cambia); in LR niente livello né analoghi (decisione 6). Nessuna frase d'avviso; nessun pulsante «Cerca».
4. Tour: se il pannello cambia, `tour.js` e `tests_e2e/e2e_tour.py` restano validi.

## 5. Accettazione
Nuovo `tests_e2e/e2e_libreria_pannello.py` (sito costruito, libreria sintetica generata nel test): aprendo uno spettro MS² appare il Top con il composto atteso in prima riga entro 1 s; cambio di spettro annulla; tastiera e `e2e_axe_grafici` verdi; `window.MZLAB_LR_MISS` vuoto.

## 6. Prove
`python3 tools/build_site.py && python3 tools/verifica.py --solo e2e_libreria_pannello,e2e_libreria_wasm,e2e_tour` poi cancella `site/`.

## 7. Consegna
PR «Libraries panel: instant search in every loaded library».
