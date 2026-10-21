# Tutor sulla Teoria (`crates/mzlab-tutor`)

Il crate interno taglia le pagine della Teoria in blocchi citabili (`teoria-blocchi/1`, schema `docs/schemi/teoria-blocchi-1.json`) e li cerca con BM25. Dipende solo da `mzlab-libro`, `mzlab-core` (SHA-256 di `budget`) e `serde_json`; nessun `.wasm` del sito cambia.

## Funzioni

Moduli: `blocchi.rs` (blocchi, JSON), `pagina.rs` (lettore di una pagina), `html.rs` (token e entità), `cerca.rs` (BM25).

- `blocchi::build(&[(nome, html)]) -> Result<Value>`: legge `<main>` di ogni pagina in ordine di nome; salta i rimandi (`http-equiv="refresh"`), `index.html` e le pagine `pratica*`; ignora script, stili, `nav`, `svg`, `.sim`, `.eispec`; chiavi ordinate, stessi byte a ogni corsa.
- Blocco: paragrafi consecutivi sotto lo stesso h2/h3 fino a ~800 caratteri (un paragrafo più lungo si taglia a fine frase, mai oltre 1 100 + parola); tabelle come righe `a | b`; `<sup>` dopo una cifra diventa `^`; l'etichetta in grassetto di un riquadro (`div.box`) apre il suo paragrafo.
- `id = file#ancora.k` (k = ordine sotto l'ancora), `anchor` = id scritto nel titolo o `slug` del testo (lo stesso che dà `teoria.js` a run time), vuoto prima del primo h2; `chapter` = kicker («Capitolo 12»); `sha` = primi 12 esadecimali di SHA-256 del testo normalizzato.
- `kind`: `testo`; `glossario` (`<dt id="g-…">` + `<dd>` = un blocco «termine: definizione»); `figura` (`details.figdesc` unito alla didascalia, la sua summary non è testo); `esercizio` (`details.q` e `details.poe`: solo la domanda in `text`).
- Esercizi: `hidden_sha` (SHA della risposta) e `hidden_shingles` (`blocchi::shingles`: hash di 6 esadecimali delle 5-grammi di parole normalizzate, ordinati, senza doppioni; chi confronta una risposta calcola gli stessi hash). Il testo della risposta non esce mai.
- `blocchi::to_text(&Value)`: testo canonico (un blocco per riga, a capo finale); è ciò che scrive il binario.
- `cerca::Index::new(&Value)` e `search(query, where_, k) -> Vec<(id, score)>`: BM25 (k1 1,2, b 0,75) su parole normalizzate come `cerca.js` (`words`), senza parole vuote it/en, con radice leggera (`cerca::stem`), titolo contato due volte; i sinonimi di `mzlab_libro::sinonimi()` entrano nella domanda a metà peso; +20 % ai blocchi della sezione aperta `where_` (`file#ancora`, o solo `file`). Ordine e punteggi sono deterministici.
- `cerca::recall_at(&Index, &[caso], k)`: quota dei casi con `expect_anchors` il cui `file#ancora` (o solo `file`) compare fra i primi k blocchi.
- Binario `teoria-blocchi DIR OUT [--controlla]`: scrive OUT; con `--controlla` confronta con OUT e torna 1 se diverso (2 per errori di lettura).
- `mzlab-libro` espone ora `slug`, `norm` e `sinonimi()` (nessun cambio di comportamento).

## Casi di recupero

`tests/data/tutor/eval/*.json`: 68 casi sintetici (`question`, `lang`, `expect_anchors`, `intent`): concetto it 32 e en 11, esercizio copiato da un `details.q` 10, dati del laboratorio 5, fuori tema 5, non coperto 3, iniezione 2 (gli ultimi senza ancore). Quando una sezione cambia titolo la sua ancora cambia e il test di richiamo la segnala.

## Misure (`cargo test -p mzlab-tutor -- --nocapture`)

- 582 blocchi, 390 KB (limite 400 KB), ~ 90 KB compressi.
- richiamo@5 italiano 0,905 su 57 casi (soglia 0,85); inglese 0,455 su 11 casi, solo misurato: le domande inglesi contro un testo italiano contano sui sinonimi e sui termini tecnici comuni; il passo di scelta del Worker deve tradurre le parole chiave in italiano.

## Richiesta per il workflow (non applicata)

Passo da aggiungere a `.github/workflows/pages.yml` dopo la costruzione dei `.wasm`, prima di pubblicare `site/` (il file non si committa):

```yaml
      - name: Teoria blocks for the tutor (never committed)
        run: |
          mkdir -p site/static/tutor
          cargo run --locked --release -p mzlab-tutor --bin teoria-blocchi -- mzlab/web/teoria site/static/tutor/blocchi-1.json
```
