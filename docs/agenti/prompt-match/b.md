# MATCH-B — Algoritmi e risultato `libmatch/1` in Rust

- **Ramo**: `claude/match-risultato`. **Dipende da**: MATCH-A unita. **Dopo di te**: MATCH-C, MATCH-D, MATCH-E.

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-B: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV Rust del motore delle librerie. Studio: §2, §7, §8.

## 2. Contesto
`scores.rs` ha entropia, coseno, forward/reverse, intensità spiegata, coseno modificato e GNPS; `search::analogs` usa la ricerca ibrida Flash per i candidati. Il raggruppamento per composto (primo blocco InChIKey), l'ordinamento e i badge stanno oggi in `mzlab/web/libreria.js`. MATCH-A ha aggiunto `LibSet` (più librerie, maschera, dedup), RI e `SEC_LIBINFO`. Lo stesso risultato servirà al Flow (C) e al pannello (D): deve nascere in Rust, una volta.

## 3. Perimetro
`crates/mzlab-lib/src/{scores.rs, matchres.rs (nuovo), libset.rs}`, test e `docs/schemi/libmatch-1.json` (nuovo), `docs/agenti/librerie.md`. Niente pagina, wasm o Flow.

## 4. Lavoro
1. **`stein_cosine`**: coseno pesato I^a·(m/z)^b con forward e reverse, ricerca senza precursore (GC-EI); verifica sulla fonte i pesi (Kim et al. 2012: a = 0,53, b = 1,3; proposta di Federico a = 0,5, b = 1,3) e scrivi la fonte nella rustdoc; ΔRI gaussiano `exp(-(ΔRI/σ)²/2)`, σ predefinito 10, moltiplicato al punteggio se query e voce hanno RI.
2. **`hybrid_entropy`** esposto come punteggio di coppia (picchi diretti + spostati di Δprec), uguale a quello della ricerca Flash ibrida (test di parità con `hybrid_pairs`).
3. **`matchres.rs`**: da `LibSet::search` a `MatchResult` compound-centric (studio §8): composti per `inchikey14` (o nome normalizzato), badge exact (Δprec entro `exact_ppm`/finestra) | analog | in_silico (`kind` della libreria), spettri espandibili, `matched` n/N, `direct`, `shifted`, `reverse`, `explained`, `delta_ppm`, `delta_mz`, `pairs[]` (direct|shifted|loss), maschera delle librerie; Top `top_compounds` (10). Serializzazione JSON `libmatch/1` stabile (ordine dei campi fisso) con schema in `docs/schemi/`.
4. `confidence` NON si calcola qui: il campo resta per la cascata (C e D); scrivi solo le prove che la cascata legge (`experimental`, `hr`, punteggio, picchi).
5. Test su dati sintetici: due composti con lo stesso InChIKey14 in due librerie → un composto con due spettri; un analogo +O → badge analog con picchi spostati; libreria in silico → mai exact; EI senza precursore trovato da `stein_cosine`.

## 5. Accettazione
`cargo test -p mzlab-lib` verde; `libmatch-1.json` valida l'uscita dei test (test che la legge); nessun cambio ai numeri di `identity`/`analogs` esistenti (golden `tests/golden/librerie/` invariati).

## 6. Prove
`cargo test -p mzlab-lib && python3 -m pytest tests/test_librerie*.py -q` (atteso verde o saltati per dati privati assenti).

## 7. Consegna
PR «Spectral libraries: Stein cosine, hybrid entropy and the libmatch/1 result».
