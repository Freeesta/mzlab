# MATCH-A — `.mzlib` v2, insieme di librerie e ricerca aperta veloce

- **Ramo**: `claude/match-libset`. **Dipende da**: nessuno. **In parallelo con**: MATCH-F. **Dopo di te**: MATCH-B.

## Regole comuni
- Studio di riferimento: `docs/agenti/match-spettrale.md` (leggi il § indicato sotto, non tutto); stato del codice delle librerie: `docs/agenti/librerie.md`. Non rileggere AGENTS.md.
- Logica in Rust (`crates/`), JS solo colla; file ≤ 500 righe; nessuna dipendenza nuova (se serve, PR aperta con la proposta: decide Federico tramite il Coordinatore); nessun dato reale nel repository pubblico (test su dati sintetici o `tests/golden/librerie/`).
- Flow e Tabui: solo inglese, solo amministratore, nessuna chiave `lang/`. Vista Dati: ogni testo nasce in `lang/it.js` e `lang/en.js`.
- Regole dei testi di Federico: pochi testi, nessun paragrafo dove l'interfaccia è chiara; tooltip solo per dire cosa fa un campo o cosa inserire, mai opinioni, avvisi o cautele (segnalazioni = badge o colonne neutre); errori = cosa è successo + cosa fare, in una riga; MS¹, MS², MSⁿ con apice; tempo «14.32 min» senza «RT»; il tipo di esperimento è preselezionato ma modificabile.
- WCAG 2.2 AA: bersagli ≥ 24 px (40 al tocco), tastiera, `aria-label`, colori da variabili CSS, Okabe-Ito, `prefers-reduced-motion`; ogni grafico nuovo con la sua tabella dei dati.
- Principio 1: a risoluzione unitaria nessun livello, nessuna ΔFormula, nessun badge che suggerisca un'identità.
- Una PR ≤ ~600 righe, ramo indicato sotto da `origin/main`; `python3 tools/pre_pr.py` prima del push; documenti: riscrivi le righe che descrivono il comportamento cambiato in `docs/agenti/librerie.md` o nel file d'area indicato (stato attuale, niente date né numeri di PR).
- Primo messaggio al Coordinatore (send_message, session_01NawSmneBkv8sosEHpoC8k4): «MATCH-A: ricevuto». A fine PR un solo messaggio (≤ 5 righe: PR, SHA di 40 caratteri a CI verde, cosa resta). Eventi della PR: skill `steward`. Sopra 200k token: blocco di consegna (≤ 15 righe) e stop.

## 1. Ruolo
DEV Rust del motore delle librerie (`crates/mzlab-lib`). Studio: §1, §5, §6.

## 2. Contesto
Il formato `.mzlib` v1, gli indici Flash su frammenti e perdite neutre e `search::{identity, flash_scores, analogs}` esistono e funzionano su UNA libreria. Banco dello studio (wasm, 1 milione di spettri sintetici): identità 0,15 ms (HR) e 2,5–3,3 ms (unit), ibrida aperta 80–140 ms perché `flash_scores` raccoglie le coppie in un `Vec` e le ordina (`score_pairs`); un prototipo con accumulatori piatti fa 17–33 ms. Servono: più librerie in una ricerca, tipo e tecnica della libreria nel file, deduplicazione senza unire i file.

## 3. Perimetro
Solo `crates/mzlab-lib/` (src, tests, examples) e `docs/agenti/librerie.md`. Non toccare `mzlab-lib-wasm`, la pagina, il Flow, i passi `spectra.match`/`table.match` (li adattano B e C).

## 4. Lavoro
1. **v2 del formato** (`format.rs`, `build.rs`, `read.rs`): `ri: f32` nel record con `FLAG_HAS_RI` (letto da `RI`/`RetentionIndex`/`Kovats` in `parse.rs`); sezione `SEC_LIBINFO` (JSON: `kind` experimental|in_silico, `tech` lc_msms|gc_ei|msn, contatori per polarità e analyzer, `source`, `version`, `sha256`); sezione per spettro `inchikey14` (`u64`, FNV-1a dei primi 14 caratteri, 0 se manca). `MIN_READER` resta 1: un file v1 si legge come prima (RI assente, info vuota). Aggiorna la specifica byte per byte.
2. **Rilevamento automatico** nel builder: polarità, `tech` (`gc_ei` se manca il precursore e il tipo/ionizzazione è EI o c'è RI), `kind` (`in_silico` da parole chiave note o da `BuildOpts.kind`), analyzer da `Instrument_type`.
3. **Accumulatori piatti** in `flash_scores_with`: `f32` + contatore per spettro, elenco dei toccati, conflitti uno-a-uno risolti solo dove servono. Punteggi e coppie identici alla versione a coppie (test di parità su ≥ 2000 query sintetiche, tolleranza 1e-9; tieni la vecchia come riferimento nel test).
4. **`libset.rs`**: `LibSet` = elenco di `Library` aperte con id `u16` (≤ 16); `search(query, opts, mask)` cerca ogni libreria nella maschera con buffer condivisi e fonde i top per punteggio; `dedup` costruisce il sidecar (`group_id` e maschera di provenienza per spettro, chiave `(inchikey14, pol, prec a 0,001, hash dei 6 picchi più alti)`), formato binario piccolo con intestazione e SHA-256 delle librerie, ricostruito se una libreria cambia.
5. Banco `examples/libset_bench.rs` (libreria sintetica come `identity_bench.rs`, 1 o 3 librerie): righe da copiare in `docs/agenti/misure-rust.md`.

## 5. Accettazione
`cargo test -p mzlab-lib` verde, compresi i nuovi test: v1 letto da v2, RI e info scritte e rilette, parità accumulatori, `LibSet` con 3 librerie = unione dei risultati delle 3 ricerche, dedup che riconosce lo stesso spettro in due librerie. Ibrida aperta ≤ 40 ms/query nativo a 1M unit sintetico (oggi ~106).

## 6. Prove
`cargo test -p mzlab-lib && cargo run --release -p mzlab-lib --example libset_bench -- 1000000 hr` (atteso: identità < 0,1 ms, ibrida aperta in calo ≥ ×3 rispetto a `flash_scores` a coppie). `cargo clippy -p mzlab-lib -- -D warnings`.

## 7. Consegna
PR «Spectral libraries: .mzlib v2, LibSet and flat accumulators», messaggio al Coordinatore con i tempi del banco.
